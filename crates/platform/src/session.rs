//! For the watchdog: is the app running, and starting it on the signed-in user's desktop.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use windows::core::{w, Error, Result, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, E_FAIL, HANDLE};
use windows::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
use windows::Win32::System::RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken};
use windows::Win32::System::Threading::{
    CreateProcessAsUserW, CREATE_UNICODE_ENVIRONMENT, PROCESS_INFORMATION, STARTUPINFOW,
};

const NO_SESSION: u32 = 0xFFFF_FFFF;

/// If the check can't run, reports running, so a failed check can't spawn copies of the app.
pub fn process_running(name: &str) -> bool {
    let Ok(mut system) = System::new() else {
        return true;
    };
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    system.processes().values().any(|p| {
        p.name()
            .to_str()
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    })
}

fn to_wide(text: impl IntoIterator<Item = u16>) -> Vec<u16> {
    text.into_iter().chain(std::iter::once(0)).collect()
}

/// Starts `exe` as the user at the console. Needs SYSTEM, and fails if nobody is signed in.
pub fn launch_in_user_session(exe: &Path) -> Result<()> {
    let application = to_wide(exe.as_os_str().encode_wide());
    let quote = '"' as u16;
    let mut command_line = to_wide(
        std::iter::once(quote)
            .chain(exe.as_os_str().encode_wide())
            .chain(std::iter::once(quote)),
    );

    // Safety: every pointer outlives the call it's passed to, and each handle and block is
    // released exactly once before returning.
    unsafe {
        let session = WTSGetActiveConsoleSessionId();
        if session == NO_SESSION {
            return Err(Error::from(E_FAIL));
        }
        let mut token = HANDLE::default();
        WTSQueryUserToken(session, &mut token)?;

        let mut environment = std::ptr::null_mut();
        let result = CreateEnvironmentBlock(&mut environment, Some(token), false).and_then(|()| {
            let startup = STARTUPINFOW {
                cb: size_of::<STARTUPINFOW>() as u32,
                lpDesktop: PWSTR(w!(r"winsta0\default").as_ptr().cast_mut()),
                ..Default::default()
            };
            let mut process = PROCESS_INFORMATION::default();
            let created = CreateProcessAsUserW(
                Some(token),
                PCWSTR(application.as_ptr()),
                Some(PWSTR(command_line.as_mut_ptr())),
                None,
                None,
                false,
                CREATE_UNICODE_ENVIRONMENT,
                Some(environment.cast_const()),
                PCWSTR::null(),
                &startup,
                &mut process,
            );
            if created.is_ok() {
                let _ = CloseHandle(process.hThread);
                let _ = CloseHandle(process.hProcess);
            }
            let _ = DestroyEnvironmentBlock(environment);
            created
        });
        let _ = CloseHandle(token);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_running_process_and_ignores_a_made_up_one() {
        let this = std::env::current_exe().unwrap();
        let name = this.file_name().unwrap().to_str().unwrap();
        assert!(process_running(name));
        assert!(!process_running("no-such-process-monk.exe"));
    }

    #[test]
    fn launching_without_system_rights_fails_cleanly() {
        // A normal user lacks the privilege, so this must error, not start anything or crash.
        assert!(launch_in_user_session(Path::new(r"C:\Windows\System32\notepad.exe")).is_err());
    }
}
