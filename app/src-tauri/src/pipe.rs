//! Client for the service's named pipe: requests with replies, plus `state_changed` pushes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use monk_core::protocol::{
    ErrorCode, Message, Push, Reply, ReplyError, Request, Snapshot, PIPE_NAME,
};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, WriteHalf};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::sync::oneshot;

use crate::notify;
use crate::tray::Tray;

const RETRY: Duration = Duration::from_secs(2);
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

type Writer = WriteHalf<NamedPipeClient>;

#[derive(Default)]
pub struct Pipe {
    writer: tokio::sync::Mutex<Option<Writer>>,
    pending: Mutex<HashMap<String, oneshot::Sender<Reply>>>,
    next_id: AtomicU64,
    last: Mutex<Option<Snapshot>>,
}

fn unavailable() -> ReplyError {
    ReplyError {
        code: ErrorCode::ServiceUnavailable,
        message: "The Monk Mode service isn't running.".to_string(),
    }
}

impl Pipe {
    pub async fn call(&self, message: Message) -> Result<Value, ReplyError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        let request = Request {
            id: id.clone(),
            message,
        };
        let line = serde_json::to_string(&request).expect("requests always serialize") + "\n";
        let (reply_tx, reply_rx) = oneshot::channel();
        self.pending
            .lock()
            .expect("pending lock")
            .insert(id.clone(), reply_tx);

        let sent = match self.writer.lock().await.as_mut() {
            Some(writer) => writer.write_all(line.as_bytes()).await.is_ok(),
            None => false,
        };
        let reply = if sent {
            tokio::time::timeout(REPLY_TIMEOUT, reply_rx).await.ok()
        } else {
            None
        };
        self.pending.lock().expect("pending lock").remove(&id);

        match reply {
            Some(Ok(Reply { ok: true, data, .. })) => Ok(data.unwrap_or(Value::Null)),
            Some(Ok(Reply {
                error: Some(error), ..
            })) => Err(error),
            _ => Err(unavailable()),
        }
    }

    pub fn last_snapshot(&self) -> Option<Snapshot> {
        self.last.lock().expect("snapshot lock").clone()
    }
}

/// Stays connected for as long as the app runs, retrying while the service is down.
pub async fn run(app: AppHandle) {
    loop {
        if let Ok(client) = ClientOptions::new().open(PIPE_NAME) {
            session(&app, client).await;
        }
        let _ = app.emit("service-status", false);
        tokio::time::sleep(RETRY).await;
    }
}

async fn session(app: &AppHandle, client: NamedPipeClient) {
    let pipe = app.state::<Pipe>();
    let (reader, writer) = tokio::io::split(client);
    *pipe.writer.lock().await = Some(writer);
    let _ = app.emit("service-status", true);

    // Replies come through the loop below, so this can't wait inside it.
    let first = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(data) = first.state::<Pipe>().call(Message::GetState).await {
            if let Ok(snapshot) = serde_json::from_value::<Snapshot>(data) {
                apply_snapshot(&first, snapshot);
            }
        }
    });

    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if let Ok(Push::StateChanged(snapshot)) = serde_json::from_str::<Push>(&line) {
            apply_snapshot(app, snapshot);
        } else if let Ok(reply) = serde_json::from_str::<Reply>(&line) {
            let waiting = pipe.pending.lock().expect("pending lock").remove(&reply.id);
            if let Some(waiting) = waiting {
                let _ = waiting.send(reply);
            }
        }
    }
    *pipe.writer.lock().await = None;
}

fn apply_snapshot(app: &AppHandle, snapshot: Snapshot) {
    let pipe = app.state::<Pipe>();
    let previous = pipe
        .last
        .lock()
        .expect("snapshot lock")
        .replace(snapshot.clone());

    if let Some(previous) = previous {
        for (title, body) in notify::notices(&previous.state, &snapshot.state) {
            let _ = app.notification().builder().title(title).body(body).show();
        }
    }
    if let Some(tray) = app.try_state::<Tray>() {
        tray.set_allowance(&notify::allowance_text(&snapshot.state));
    }
    let _ = app.emit("state-changed", snapshot);
}
