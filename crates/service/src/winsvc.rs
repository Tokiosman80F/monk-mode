//! The Windows Service wrapper: install, uninstall and the service entry point.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use monk_core::lock::effective_now;
use monk_platform::hosts;
use monk_platform::registry::LockRegistry;
use tokio::sync::watch;
use windows_service::service::{
    ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept,
    ServiceErrorControl, ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::{define_windows_service, service_dispatcher};

use crate::daemon::reconcile_lock;
use crate::db::{data_dir, Db};
use crate::runtime::{self, RunOptions, WindowsPlatform};

const SERVICE_NAME: &str = "MonkModeService";
const DISPLAY_NAME: &str = "Monk Mode Blocker";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
const STOP_WAIT: Duration = Duration::from_secs(20);

/// SYSTEM and admins may write, users may only read. Without this, a normal user could edit the
/// database while the service is running.
fn secure_data_dir() -> Result<(), String> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create {}: {e}", dir.display()))?;
    let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let status = Command::new(PathBuf::from(system_root).join(r"System32\icacls.exe"))
        .arg(&dir)
        .args([
            "/inheritance:r",
            "/grant:r",
            "*S-1-5-18:(OI)(CI)F",
            "*S-1-5-32-544:(OI)(CI)F",
            "*S-1-5-32-545:(OI)(CI)RX",
            "/T",
            "/Q",
        ])
        .stdout(Stdio::null())
        .status()
        .map_err(|e| format!("Couldn't run icacls: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Couldn't secure {}: {status}", dir.display()))
    }
}

/// Safe to run again: an existing service is just started if it isn't running.
pub fn install() -> Result<(), String> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .map_err(|e| format!("Couldn't reach the service manager. Is this running as admin? {e}"))?;

    secure_data_dir()?;
    runtime::ensure_token(&data_dir()).map_err(|e| format!("Couldn't create the token: {e}"))?;

    let existing = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::START,
    );
    if let Ok(service) = existing {
        let status = service.query_status().map_err(|e| e.to_string())?;
        if status.current_state == ServiceState::Stopped {
            service
                .start::<&OsStr>(&[])
                .map_err(|e| format!("Couldn't start the service: {e}"))?;
        }
        println!("{SERVICE_NAME} was already installed.");
        return Ok(());
    }

    let info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(DISPLAY_NAME),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: std::env::current_exe().map_err(|e| e.to_string())?,
        launch_arguments: vec![OsString::from("run")],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };
    let service = manager
        .create_service(&info, ServiceAccess::CHANGE_CONFIG | ServiceAccess::START)
        .map_err(|e| format!("Couldn't create the service: {e}"))?;
    service
        .set_description("Enforces your Monk Mode site blocks and commitment lock.")
        .map_err(|e| e.to_string())?;

    let restart = |seconds| ServiceAction {
        action_type: ServiceActionType::Restart,
        delay: Duration::from_secs(seconds),
    };
    service
        .update_failure_actions(ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(86_400)),
            reboot_msg: None,
            command: None,
            actions: Some(vec![restart(1), restart(5), restart(5)]),
        })
        .map_err(|e| format!("Couldn't set the restart rule: {e}"))?;
    service
        .set_failure_actions_on_non_crash_failures(true)
        .map_err(|e| e.to_string())?;

    service
        .start::<&OsStr>(&[])
        .map_err(|e| format!("Couldn't start the service: {e}"))?;
    println!("{SERVICE_NAME} installed and started.");
    Ok(())
}

/// Refuses while locked. Otherwise removes the service, the hosts block and the registry copy.
pub fn uninstall() -> Result<(), String> {
    let registry = LockRegistry::machine().map_err(|e| e.to_string())?;
    let db = Db::open(&data_dir()).map_err(|e| e.to_string())?;
    let now = db
        .load_last_seen()
        .map_err(|e| e.to_string())?
        .map_or(Utc::now(), |seen| effective_now(Utc::now(), seen));
    let mut platform = WindowsPlatform { registry };
    if let Some(lock) = reconcile_lock(&db, &mut platform, now) {
        if lock.is_active(now) {
            return Err(format!(
                "Locked until {}. Monk Mode can't be removed before then.",
                lock.ends_at.format("%Y-%m-%d")
            ));
        }
    }

    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| {
            format!("Couldn't reach the service manager. Is this running as admin? {e}")
        })?;
    // A service that's already gone mustn't leave the app impossible to remove.
    let service = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
    );
    if let Ok(service) = service {
        let status = service.query_status().map_err(|e| e.to_string())?;
        if status.current_state != ServiceState::Stopped {
            service
                .stop()
                .map_err(|e| format!("Couldn't stop the service: {e}"))?;
            let deadline = Instant::now() + STOP_WAIT;
            while service
                .query_status()
                .map_err(|e| e.to_string())?
                .current_state
                != ServiceState::Stopped
            {
                if Instant::now() > deadline {
                    return Err("The service didn't stop in time.".to_string());
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        }
        service
            .delete()
            .map_err(|e| format!("Couldn't delete the service: {e}"))?;
    }

    hosts::enforce(&[]).map_err(|e| format!("Couldn't clean the hosts file: {e}"))?;
    platform
        .registry
        .clear()
        .map_err(|e| format!("Couldn't remove the registry copy: {e}"))?;
    println!("{SERVICE_NAME} removed.");
    Ok(())
}

define_windows_service!(ffi_service_main, service_main);

/// Hands this process to the service manager. Only works when started by it.
pub fn run_as_service() -> Result<(), String> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main).map_err(|e| e.to_string())
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        // Service mode has no console, and logging may be what failed.
        tracing::error!("Service failed: {e}");
        std::process::exit(1);
    }
}

fn run_service() -> Result<(), String> {
    let locked = Arc::new(AtomicBool::new(false));
    let (stop_tx, mut stop_rx) = watch::channel(false);

    let handler_locked = locked.clone();
    let handle = service_control_handler::register(SERVICE_NAME, move |control| match control {
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        // While locked `sc stop` gets an error. Windows shutting down is always allowed.
        ServiceControl::Stop if handler_locked.load(Ordering::Relaxed) => {
            ServiceControlHandlerResult::NotImplemented
        }
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = stop_tx.send(true);
            ServiceControlHandlerResult::NoError
        }
        _ => ServiceControlHandlerResult::NotImplemented,
    })
    .map_err(|e| e.to_string())?;

    let report = |state, accepted, exit| {
        handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: state,
            controls_accepted: accepted,
            exit_code: ServiceExitCode::Win32(exit),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
    };
    report(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        0,
    )
    .map_err(|e| e.to_string())?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let options = RunOptions {
        watchdog: true,
        locked,
    };
    let result = runtime.block_on(runtime::run(options, async move {
        let _ = stop_rx.wait_for(|stop| *stop).await;
    }));

    let exit = u32::from(result.is_err());
    let _ = report(ServiceState::Stopped, ServiceControlAccept::empty(), exit);
    result
}
