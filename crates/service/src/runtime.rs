//! Wires everything together: the 1-second loop, both servers, logging, the UI watchdog.

use std::future::Future;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, Utc};
use monk_core::engine::Event;
use monk_core::models::Now;
use monk_core::protocol::{ErrorCode, Reply, Request};
use monk_platform::registry::{LockRegistry, LockRegistryError};
use monk_platform::{browser, hosts, session};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

use crate::daemon::{Client, Daemon, Platform};
use crate::db::{data_dir, Db};
use crate::{pipe, ws};

pub type Shared<P> = Arc<Mutex<Daemon<P>>>;

const APP_EXE: &str = "monk-mode-app.exe";
const WATCHDOG_EVERY_SEC: u64 = 5;
const LOG_DAYS: usize = 7;
const TOKEN_FILE: &str = "ext_token";

pub struct WindowsPlatform {
    pub registry: LockRegistry,
}

impl Platform for WindowsPlatform {
    fn enforce_hosts(&mut self, domains: &[String]) -> io::Result<bool> {
        let changed = hosts::enforce(domains)?;
        if changed {
            info!("Hosts file updated: {} domains", domains.len());
        }
        Ok(changed)
    }
    fn running_browsers(&mut self) -> Vec<String> {
        browser::running_browsers()
    }
    fn read_lock_copy(&self) -> Result<Option<DateTime<Utc>>, LockRegistryError> {
        self.registry.read()
    }
    fn write_lock_copy(&mut self, ends_at: DateTime<Utc>) -> Result<(), LockRegistryError> {
        self.registry.write(ends_at)
    }
    fn sign(&self, ends_at: DateTime<Utc>) -> String {
        self.registry.sign(ends_at)
    }
    fn verify(&self, ends_at: DateTime<Utc>, signature: &str) -> bool {
        self.registry.verify(ends_at, signature)
    }
}

pub fn local_now(utc: DateTime<Utc>) -> Now {
    Now::new(utc, &Local)
}

/// Handles one incoming line or frame and returns the reply to send.
pub fn dispatch<P: Platform>(
    shared: &Shared<P>,
    client: &Client,
    text: &str,
    utc: DateTime<Utc>,
) -> String {
    let reply = match serde_json::from_str::<Request>(text) {
        Err(_) => Reply::err(String::new(), ErrorCode::InvalidRequest, "Bad message."),
        Ok(request) => {
            let result = shared
                .lock()
                .expect("daemon lock")
                .handle(client, request.message, utc);
            match result {
                Ok(data) => Reply::ok(request.id, data),
                Err((code, message)) => Reply::err(request.id, code, message),
            }
        }
    };
    serde_json::to_string(&reply).expect("replies always serialize")
}

/// One tick of the loop: advance the daemon and tell every client if anything changed.
pub fn publish<P: Platform>(
    shared: &Shared<P>,
    pushes: &broadcast::Sender<String>,
    elapsed_sec: u32,
    utc: DateTime<Utc>,
) -> Vec<Event> {
    let (events, push) = {
        let mut daemon = shared.lock().expect("daemon lock");
        let events = daemon.step(elapsed_sec, utc);
        (events, daemon.push_if_changed())
    };
    if let Some(push) = push {
        // An error only means nobody is connected.
        let _ = pushes.send(push);
    }
    events
}

/// The token the extension must present. Created once and kept in the data directory.
pub fn ensure_token(dir: &Path) -> io::Result<String> {
    let path = dir.join(TOKEN_FILE);
    match std::fs::read_to_string(&path) {
        Ok(token) if !token.trim().is_empty() => return Ok(token.trim().to_string()),
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(io::Error::other)?;
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::create_dir_all(dir)?;
    std::fs::write(&path, &token)?;
    Ok(token)
}

/// The extension can't read the data directory, so it gets a copy in its own folder. Does nothing
/// if that folder isn't there, as in development.
pub fn write_extension_token(extension_dir: &Path, token: &str) {
    if !extension_dir.is_dir() {
        return;
    }
    let json = serde_json::json!({ "token": token }).to_string();
    if let Err(e) = std::fs::write(extension_dir.join("token.json"), json) {
        error!("Couldn't give the extension its token: {e}");
    }
}

pub fn file_logger(
    dir: &Path,
) -> Result<RollingFileAppender, tracing_appender::rolling::InitError> {
    RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("monk-service")
        .filename_suffix("log")
        .max_log_files(LOG_DAYS)
        .build(dir.join("logs"))
}

fn init_logging(dir: &Path) {
    match file_logger(dir) {
        Ok(writer) => tracing_subscriber::fmt()
            .with_writer(writer)
            .with_ansi(false)
            .init(),
        Err(e) => eprintln!("Couldn't start logging: {e}"),
    }
}

fn watchdog<P: Platform>(shared: &Shared<P>, exe: &Path) {
    if shared.lock().expect("daemon lock").ui_quit() || session::process_running(APP_EXE) {
        return;
    }
    match session::launch_in_user_session(exe) {
        Ok(()) => info!("Watchdog restarted the app"),
        // Normal before anyone has signed in.
        Err(e) => debug!("Watchdog couldn't start the app: {e}"),
    }
}

pub struct RunOptions {
    /// Restart the app if it's closed. Needs SYSTEM, so only the real service turns it on.
    pub watchdog: bool,
    /// Kept up to date for the service's stop handler.
    pub locked: Arc<AtomicBool>,
}

/// Runs until `shutdown` completes. Runs forever in the real service unless told to stop.
pub async fn run(options: RunOptions, shutdown: impl Future<Output = ()>) -> Result<(), String> {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    init_logging(&dir);
    info!("Service starting, data in {}", dir.display());

    let registry = LockRegistry::machine().map_err(|e| e.to_string())?;
    let db = Db::open(&dir).map_err(|e| e.to_string())?;
    let token = ensure_token(&dir).map_err(|e| format!("Couldn't create the token: {e}"))?;
    let exe_dir = std::env::current_exe().map_err(|e| e.to_string())?;
    write_extension_token(exe_dir.with_file_name("extension").as_path(), &token);
    let daemon = Daemon::new(db, WindowsPlatform { registry }, local_now, Utc::now());
    let shared: Shared<WindowsPlatform> = Arc::new(Mutex::new(daemon));
    let (pushes, _) = broadcast::channel(16);

    let listener = TcpListener::bind(("127.0.0.1", ws::PORT))
        .await
        .map_err(|e| format!("Couldn't open port {}: {e}", ws::PORT))?;
    tokio::spawn(ws::serve(
        listener,
        shared.clone(),
        Arc::new(token),
        pushes.clone(),
    ));
    {
        let (shared, pushes) = (shared.clone(), pushes.clone());
        tokio::spawn(async move {
            if let Err(e) = pipe::serve(pipe::PIPE_NAME, shared, pushes).await {
                error!("Named pipe server stopped: {e}");
            }
        });
    }

    let app_exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name(APP_EXE);
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last = Instant::now();
    let mut seconds: u64 = 0;
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            _ = &mut shutdown => break,
        }
        // Whole seconds only, and the remainder carries over, so nothing is lost to rounding.
        let elapsed = last.elapsed().as_secs();
        if elapsed == 0 {
            continue;
        }
        last += Duration::from_secs(elapsed);
        seconds += elapsed;

        let events = publish(
            &shared,
            &pushes,
            elapsed.min(u64::from(u32::MAX)) as u32,
            Utc::now(),
        );
        for event in events {
            info!("{}", describe(&event));
        }
        options.locked.store(
            shared.lock().expect("daemon lock").is_locked(),
            Ordering::Relaxed,
        );
        let open_app = shared.lock().expect("daemon lock").take_open_app();
        if options.watchdog && open_app {
            if let Err(e) = session::launch_in_user_session(&app_exe) {
                warn!("Couldn't open the app: {e}");
            }
        }
        if options.watchdog && seconds % WATCHDOG_EVERY_SEC < elapsed {
            watchdog(&shared, &app_exe);
        }
    }

    shared.lock().expect("daemon lock").flush();
    warn!("Service stopping");
    Ok(())
}

fn describe(event: &Event) -> String {
    match event {
        Event::Warn2Min => "2 minutes of allowance left".to_string(),
        Event::Warn30s => "30 seconds of allowance left".to_string(),
        Event::AllowanceUsedUp => "Allowance used up".to_string(),
        Event::DayRolled(day) => format!("New day. Finished {}", day.date),
        Event::FocusEnded(session) => format!("Focus session ended: {}", session.block_name),
        Event::BreakEnded => "Break ended".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_is_created_once_and_then_reused() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("new");
        let token = ensure_token(&nested).unwrap();
        assert_eq!(token.len(), 64);
        assert_eq!(ensure_token(&nested).unwrap(), token);
        assert_ne!(ensure_token(dir.path()).unwrap(), token);
    }

    #[test]
    fn an_empty_token_file_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(TOKEN_FILE), "  ").unwrap();
        assert_eq!(ensure_token(dir.path()).unwrap().len(), 64);
    }

    #[test]
    fn the_extension_gets_its_token_only_if_its_folder_exists() {
        let dir = tempfile::tempdir().unwrap();
        write_extension_token(&dir.path().join("missing"), "abc");
        assert!(!dir.path().join("missing").exists());
        write_extension_token(dir.path(), "abc");
        let written = std::fs::read_to_string(dir.path().join("token.json")).unwrap();
        assert_eq!(written, r#"{"token":"abc"}"#);
    }

    #[test]
    fn logs_go_to_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let writer = file_logger(dir.path()).unwrap();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || info!("hello from the test"));

        let file = std::fs::read_dir(dir.path().join("logs"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let name = file.file_name().into_string().unwrap();
        assert!(
            name.starts_with("monk-service") && name.ends_with(".log"),
            "{name}"
        );
        let contents = std::fs::read_to_string(file.path()).unwrap();
        assert!(contents.contains("hello from the test"));
    }
}
