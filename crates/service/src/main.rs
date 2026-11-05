mod daemon;
mod db;
mod pipe;
mod runtime;
mod winsvc;
mod ws;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

const USAGE: &str = "Usage: monk-service <install | uninstall | run | console>
  install    Install and start the Windows Service (admin)
  uninstall  Remove it, unless a lock is active (admin)
  run        Service entry point, started by Windows
  console    Run in this terminal until Ctrl+C. Set MONK_DATA_DIR and MONK_HOSTS_PATH to
             leave the real data and hosts file alone";

fn console() -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let options = runtime::RunOptions {
        watchdog: false,
        locked: Arc::new(AtomicBool::new(false)),
    };
    runtime.block_on(runtime::run(options, async {
        let _ = tokio::signal::ctrl_c().await;
    }))
}

fn main() {
    let command = std::env::args().nth(1);
    let result = match command.as_deref() {
        Some("install") => winsvc::install(),
        Some("uninstall") => winsvc::uninstall(),
        Some("run") => winsvc::run_as_service(),
        Some("console") => console(),
        _ => Err(USAGE.to_string()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
