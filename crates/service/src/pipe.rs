//! Named pipe server for the UI: one JSON message per line. The service runs as SYSTEM and the UI
//! as a normal user, so the pipe needs an explicit ACL.

use std::ffi::c_void;
use std::io;

use chrono::Utc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::{broadcast, mpsc};
use tracing::warn;
use windows::core::w;
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};

use crate::daemon::{Client, Platform};
use crate::runtime::{dispatch, Shared};

const MAX_LINE_BYTES: u64 = 1 << 20;

/// SYSTEM, admins and the owner get full access. Signed-in users may read and write, but `0x12019b` leaves
/// out FILE_CREATE_PIPE_INSTANCE, so a user's program can't pose as the service.
fn security_descriptor() -> io::Result<PSECURITY_DESCRIPTOR> {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // ponytail: the descriptor is never freed. One is made per process and lives as long as it.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            w!("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x12019b;;;AU)"),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    }
    .map_err(io::Error::other)?;
    Ok(descriptor)
}

fn create_instance(name: &str, descriptor: usize, first: bool) -> io::Result<NamedPipeServer> {
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor as *mut c_void,
        bInheritHandle: false.into(),
    };
    // Safety: `attributes` and the descriptor outlive the call, which copies what it needs.
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(name, (&raw mut attributes).cast())
    }
}

pub async fn serve<P: Platform>(
    name: &str,
    shared: Shared<P>,
    pushes: broadcast::Sender<String>,
) -> io::Result<()> {
    // As a number, because raw pointers can't be held across an await.
    let descriptor = security_descriptor()?.0 as usize;
    let mut server = create_instance(name, descriptor, true)?;
    loop {
        server.connect().await?;
        let next = create_instance(name, descriptor, false)?;
        let connected = std::mem::replace(&mut server, next);
        tokio::spawn(connection(connected, shared.clone(), pushes.subscribe()));
    }
}

/// One line, or None at the end of the stream. Too-long lines end the connection.
async fn read_line<R: AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Option<String> {
    let mut line = String::new();
    let read = reader.take(MAX_LINE_BYTES).read_line(&mut line).await;
    match read {
        Ok(n) if n > 0 && line.ends_with('\n') => Some(line),
        Ok(0) => None,
        Ok(_) => {
            warn!("Dropped a UI connection: line too long or cut off");
            None
        }
        Err(e) => {
            warn!("UI pipe read failed: {e}");
            None
        }
    }
}

async fn connection<P: Platform>(
    pipe: NamedPipeServer,
    shared: Shared<P>,
    mut pushes: broadcast::Receiver<String>,
) {
    shared.lock().expect("daemon lock").ui_connected();
    let (reader, mut writer) = tokio::io::split(pipe);
    let (replies, mut outgoing) = mpsc::channel::<String>(16);

    // Reading has its own task because `read_line` loses data if a `select!` cancels it.
    let handler = shared.clone();
    let read_task = tokio::spawn(async move {
        let mut reader = BufReader::new(reader);
        while let Some(line) = read_line(&mut reader).await {
            let reply = dispatch(&handler, &Client::Ui, line.trim_end(), Utc::now());
            if replies.send(reply).await.is_err() {
                break;
            }
        }
    });

    loop {
        let message = tokio::select! {
            reply = outgoing.recv() => match reply {
                Some(reply) => reply,
                None => break,
            },
            push = pushes.recv() => match push {
                Ok(push) => push,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
        };
        let frame = format!("{message}\n");
        if writer.write_all(frame.as_bytes()).await.is_err() {
            break;
        }
    }
    read_task.abort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::tests::{t0, utc_now, FakePlatform};
    use crate::daemon::Daemon;
    use crate::db::Db;
    use crate::runtime::publish;
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

    struct Rig {
        client: BufReader<NamedPipeClient>,
        shared: Shared<FakePlatform>,
        pushes: broadcast::Sender<String>,
        _dir: tempfile::TempDir,
    }

    async fn rig(test: &str) -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        let daemon = Daemon::new(db, FakePlatform::default(), utc_now, t0());
        let shared = Arc::new(Mutex::new(daemon));
        let (pushes, _) = broadcast::channel(16);
        let name = format!(r"\\.\pipe\monkmode-test-{}-{test}", std::process::id());

        let server = {
            let (name, shared, pushes) = (name.clone(), shared.clone(), pushes.clone());
            tokio::spawn(async move { serve(&name, shared, pushes).await })
        };
        let client = loop {
            match ClientOptions::new().open(&name) {
                Ok(client) => break client,
                Err(_) if !server.is_finished() => {
                    tokio::time::sleep(Duration::from_millis(10)).await
                }
                Err(e) => panic!("server stopped: {e}"),
            }
        };
        Rig {
            client: BufReader::new(client),
            shared,
            pushes,
            _dir: dir,
        }
    }

    async fn send(rig: &mut Rig, line: &str) {
        let frame = format!("{line}\n");
        rig.client
            .get_mut()
            .write_all(frame.as_bytes())
            .await
            .unwrap();
    }

    async fn recv(rig: &mut Rig) -> Value {
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(5), rig.client.read_line(&mut line))
            .await
            .expect("timed out")
            .unwrap();
        serde_json::from_str(&line).unwrap()
    }

    #[tokio::test]
    async fn get_state_round_trips() {
        let mut rig = rig("state").await;
        send(
            &mut rig,
            &json!({ "id": "7", "type": "get_state" }).to_string(),
        )
        .await;
        let reply = recv(&mut rig).await;
        assert_eq!(reply["id"], "7");
        assert_eq!(reply["ok"], true);
        assert_eq!(
            reply["data"]["state"]["settings"]["allowance_sites"][0]["daily_min"],
            30
        );
    }

    #[tokio::test]
    async fn loosening_while_locked_comes_back_as_the_locked_error() {
        let mut rig = rig("locked").await;
        let ends_at = (t0() + chrono::Duration::days(3650)).to_rfc3339();
        // Starting a lock uses the real clock here, so this ends far in the future.
        let start = json!({ "id": "1", "type": "start_lock", "payload": { "ends_at": ends_at } });
        send(&mut rig, &start.to_string()).await;
        assert_eq!(recv(&mut rig).await["ok"], true);

        let mut settings = monk_core::models::Settings::default();
        settings.allowance_sites[0].daily_min = 60;
        let update = json!({ "id": "2", "type": "update_settings", "payload": settings });
        send(&mut rig, &update.to_string()).await;
        let reply = recv(&mut rig).await;
        assert_eq!(reply["ok"], false);
        assert_eq!(reply["error"]["code"], "LOCKED_LOOSENING");
    }

    #[tokio::test]
    async fn bad_input_gets_an_error_reply_and_the_connection_survives() {
        let mut rig = rig("bad").await;
        send(&mut rig, "not json").await;
        assert_eq!(recv(&mut rig).await["error"]["code"], "INVALID_REQUEST");
        send(
            &mut rig,
            &json!({ "id": "1", "type": "unlock_please" }).to_string(),
        )
        .await;
        assert_eq!(recv(&mut rig).await["error"]["code"], "INVALID_REQUEST");
        send(
            &mut rig,
            &json!({ "id": "2", "type": "get_state" }).to_string(),
        )
        .await;
        assert_eq!(recv(&mut rig).await["ok"], true);
    }

    #[tokio::test]
    async fn clients_receive_pushes_after_a_tick() {
        let mut rig = rig("push").await;
        // A round trip first, so the server has subscribed this client.
        send(
            &mut rig,
            &json!({ "id": "1", "type": "get_state" }).to_string(),
        )
        .await;
        recv(&mut rig).await;
        publish(&rig.shared, &rig.pushes, 1, t0());
        assert_eq!(recv(&mut rig).await["type"], "state_changed");
    }

    #[tokio::test]
    async fn a_second_server_on_the_same_name_is_refused() {
        let rig = rig("second").await;
        let name = format!(r"\\.\pipe\monkmode-test-{}-second", std::process::id());
        assert!(serve(&name, rig.shared.clone(), rig.pushes.clone())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn connecting_clears_a_previous_quit() {
        let mut rig = rig("quit").await;
        send(
            &mut rig,
            &json!({ "id": "1", "type": "ui_quit" }).to_string(),
        )
        .await;
        recv(&mut rig).await;
        assert!(rig.shared.lock().unwrap().ui_quit());

        let name = format!(r"\\.\pipe\monkmode-test-{}-quit", std::process::id());
        let _second = ClientOptions::new().open(&name).unwrap();
        for _ in 0..100 {
            if !rig.shared.lock().unwrap().ui_quit() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the quit flag was not cleared");
    }

    #[test]
    fn the_acl_string_is_valid() {
        security_descriptor().unwrap();
    }
}
