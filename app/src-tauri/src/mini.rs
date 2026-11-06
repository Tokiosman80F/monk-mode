//! A small always-on-top timer that appears while a focus session or break runs, so the time
//! left is visible without opening the app.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use monk_core::models::PomodoroState;
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, WindowEvent};

const LABEL: &str = "mini";
const WIDTH: f64 = 140.0;
const HEIGHT: f64 = 34.0;
/// Gap from the screen's top-right corner, in logical pixels.
const MARGIN: f64 = 16.0;

#[derive(Default)]
pub struct Mini {
    /// Closed by hand: stays away until the next session.
    dismissed: AtomicBool,
    building: AtomicBool,
    /// Where it was dragged to, kept for the next session.
    position: Mutex<Option<PhysicalPosition<i32>>>,
}

/// Opens or closes the timer to match the session. Called on every state change.
pub fn sync(app: &AppHandle, pomodoro: &PomodoroState) {
    let Some(mini) = app.try_state::<Mini>() else {
        return;
    };
    let window = app.get_webview_window(LABEL);
    if matches!(pomodoro, PomodoroState::Idle) {
        mini.dismissed.store(false, Ordering::Relaxed);
        if let Some(window) = window {
            let _ = window.close();
        }
        return;
    }
    if window.is_some()
        || mini.dismissed.load(Ordering::Relaxed)
        || mini.building.swap(true, Ordering::Relaxed)
    {
        return;
    }

    let app = app.clone();
    // Building a window from an event handler deadlocks on Windows, so it gets its own task.
    tauri::async_runtime::spawn(async move {
        let mini = app.state::<Mini>();
        let saved = *mini.position.lock().expect("mini position lock");
        let built = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("index.html".into()))
            .title("Monk Mode timer")
            .inner_size(WIDTH, HEIGHT)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            // Shown once it's in place, so it doesn't flash in the middle of the screen.
            .visible(false)
            .build();
        mini.building.store(false, Ordering::Relaxed);
        let Ok(window) = built else { return };

        let position = saved.or_else(|| {
            let monitor = window.primary_monitor().ok().flatten()?;
            let scale = monitor.scale_factor();
            let x = monitor.position().x + monitor.size().width as i32
                - ((WIDTH + MARGIN) * scale) as i32;
            let y = monitor.position().y + (MARGIN * scale) as i32;
            Some(PhysicalPosition::new(x, y))
        });
        if let Some(position) = position {
            let _ = window.set_position(position);
        }
        let app = app.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::Moved(position) = event {
                *app.state::<Mini>()
                    .position
                    .lock()
                    .expect("mini position lock") = Some(*position);
            }
        });
        let _ = window.show();
    });
}

/// The close button on the timer.
#[tauri::command]
pub fn dismiss_mini(app: AppHandle) {
    if let Some(mini) = app.try_state::<Mini>() {
        mini.dismissed.store(true, Ordering::Relaxed);
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.close();
    }
}
