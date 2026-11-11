#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mini;
mod notify;
mod pipe;
mod sound;
mod tray;

use std::path::PathBuf;
use std::process::Command;

use chrono::{DateTime, Utc};
use monk_core::models::{Settings, Task};
use monk_core::planner::{LeftoverAction, SmallItem};
use monk_core::protocol::{ErrorCode, Message, ReplyError};
use pipe::Pipe;
use serde_json::Value;
use tauri::{RunEvent, State};
use tauri_plugin_autostart::MacosLauncher;

const AUTOSTART_ARG: &str = "--autostart";

type CommandResult = Result<Value, ReplyError>;

#[tauri::command]
async fn get_state(pipe: State<'_, Pipe>) -> CommandResult {
    pipe.call(Message::GetState).await
}

#[tauri::command]
async fn update_settings(pipe: State<'_, Pipe>, settings: Settings) -> CommandResult {
    pipe.call(Message::UpdateSettings(settings)).await
}

#[tauri::command]
async fn start_lock(pipe: State<'_, Pipe>, ends_at: DateTime<Utc>) -> CommandResult {
    pipe.call(Message::StartLock { ends_at }).await
}

#[tauri::command]
async fn update_small_items(pipe: State<'_, Pipe>, items: Vec<SmallItem>) -> CommandResult {
    pipe.call(Message::UpdateSmallItems(items)).await
}

#[tauri::command]
async fn resolve_leftover(
    pipe: State<'_, Pipe>,
    task_id: u32,
    action: LeftoverAction,
) -> CommandResult {
    pipe.call(Message::ResolveLeftover { task_id, action })
        .await
}

#[tauri::command]
async fn plan_day(pipe: State<'_, Pipe>) -> CommandResult {
    pipe.call(Message::PlanDay).await
}

#[tauri::command]
async fn update_tasks(pipe: State<'_, Pipe>, tasks: Vec<Task>) -> CommandResult {
    pipe.call(Message::UpdateTasks(tasks)).await
}

#[tauri::command]
async fn start_pomodoro(pipe: State<'_, Pipe>, task_id: Option<u32>) -> CommandResult {
    pipe.call(Message::StartPomodoro { task_id }).await
}

#[tauri::command]
async fn stop_pomodoro(pipe: State<'_, Pipe>) -> CommandResult {
    pipe.call(Message::StopPomodoro).await
}

#[tauri::command]
async fn pause_pomodoro(pipe: State<'_, Pipe>, paused: bool) -> CommandResult {
    pipe.call(Message::PausePomodoro { paused }).await
}

#[tauri::command]
async fn get_stats(pipe: State<'_, Pipe>, days: u32) -> CommandResult {
    pipe.call(Message::GetStats { days }).await
}

fn local_error(message: String) -> ReplyError {
    ReplyError {
        code: ErrorCode::InvalidRequest,
        message,
    }
}

/// Files installed next to the app, which the installer puts in the same folder.
fn beside_app(name: &str) -> Result<PathBuf, ReplyError> {
    std::env::current_exe()
        .map(|exe| exe.with_file_name(name))
        .map_err(|e| local_error(e.to_string()))
}

/// Setup wizard: asks Windows for admin rights, then installs and starts the service.
#[tauri::command]
fn install_service() -> Result<(), ReplyError> {
    let exe = beside_app("monk-service.exe")?;
    if !exe.is_file() {
        return Err(local_error(format!("{} wasn't found.", exe.display())));
    }
    let path = exe.display().to_string().replace('\'', "''");
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("Start-Process -Verb RunAs -Wait -FilePath '{path}' -ArgumentList install"),
        ])
        .status()
        .map_err(|e| local_error(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(local_error(
            "Windows didn't get permission to install it.".to_string(),
        ))
    }
}

/// Setup wizard: shows the folder to choose in "Load unpacked".
#[tauri::command]
fn open_extension_folder() -> Result<String, ReplyError> {
    let folder = beside_app("extension")?;
    if !folder.is_dir() {
        return Err(local_error(format!("{} wasn't found.", folder.display())));
    }
    Command::new("explorer")
        .arg(&folder)
        .spawn()
        .map_err(|e| local_error(e.to_string()))?;
    Ok(folder.display().to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_window(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .manage(Pipe::default())
        .manage(mini::Mini::default())
        .invoke_handler(tauri::generate_handler![
            get_state,
            update_settings,
            start_lock,
            update_tasks,
            update_small_items,
            resolve_leftover,
            plan_day,
            start_pomodoro,
            stop_pomodoro,
            pause_pomodoro,
            get_stats,
            install_service,
            open_extension_folder,
            mini::dismiss_mini,
        ])
        .setup(|app| {
            tray::build(app)?;

            // Not in development, or `tauri dev` would register a debug build to start at login.
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
            }

            if !std::env::args().any(|arg| arg == AUTOSTART_ARG) {
                tray::show_window(app.handle());
            }
            tauri::async_runtime::spawn(pipe::run(app.handle().clone()));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while starting Monk Mode")
        .run(|_app, event| {
            // Closing the last window leaves the app in the tray. Quit passes an exit code.
            if let RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
