//! Tray icon and menu (F9).

use monk_core::protocol::Message;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{App, AppHandle, Manager, WebviewWindow, WebviewWindowBuilder, Wry};

use crate::notify;
use crate::pipe::Pipe;

pub struct Tray {
    allowance: MenuItem<Wry>,
    icon: TrayIcon<Wry>,
}

impl Tray {
    pub fn set_allowance(&self, text: &str) {
        let _ = self.allowance.set_text(text);
    }

    /// Hovering over the tray icon shows the time left in a session.
    pub fn set_timer(&self, text: Option<&str>) {
        let tooltip = text.map_or("Monk Mode".to_string(), |t| format!("Monk Mode: {t}"));
        let _ = self.icon.set_tooltip(Some(tooltip));
    }
}

fn focus(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Closing the window destroys it, which frees WebView2's memory while the app sits in the tray,
/// so opening may need to build it again.
pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        focus(&window);
        return;
    }
    let app = app.clone();
    // Building a window from an event handler deadlocks on Windows, so it gets its own task.
    tauri::async_runtime::spawn(async move {
        if let Some(config) = app.config().app.windows.first() {
            if let Ok(window) = WebviewWindowBuilder::from_config(&app, config)
                .and_then(tauri::WebviewWindowBuilder::build)
            {
                focus(&window);
            }
        }
    });
}

async fn start_pomodoro(app: AppHandle) {
    let pipe = app.state::<Pipe>();
    let last_focus = pipe.last_focus();
    let task_id = pipe
        .last_snapshot()
        .and_then(|snapshot| notify::next_task(&snapshot.state, last_focus.as_deref()));
    let _ = pipe.call(Message::StartPomodoro { task_id }).await;
    show_window(&app);
}

async fn quit(app: AppHandle) {
    // So the service's watchdog leaves the app closed. Quitting works even if this fails.
    let _ = app.state::<Pipe>().call(Message::UiQuit).await;
    app.exit(0);
}

pub fn build(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let pomodoro = MenuItem::with_id(app, "pomodoro", "Start Pomodoro", true, None::<&str>)?;
    let allowance = MenuItem::with_id(app, "allowance", "Checking allowance", false, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &pomodoro, &allowance, &separator, &quit_item])?;

    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Monk Mode")
        .on_menu_event(|app, event| {
            let app = app.clone();
            match event.id.as_ref() {
                "open" => show_window(&app),
                "pomodoro" => {
                    tauri::async_runtime::spawn(start_pomodoro(app));
                }
                "quit" => {
                    tauri::async_runtime::spawn(quit(app));
                }
                _ => {}
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    let icon = tray.build(app)?;

    app.manage(Tray { allowance, icon });
    Ok(())
}
