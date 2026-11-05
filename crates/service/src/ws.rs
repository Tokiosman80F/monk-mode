//! WebSocket server for the browser extension, on 127.0.0.1 only. A connection must come from
//! our extension's origin, and its first message must be `ext_hello` with the shared token.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use monk_core::protocol::{ErrorCode, Message, Reply, Request};
use monk_platform::browser::SUPPORTED;
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::handshake::server::{
    ErrorResponse, Request as HttpRequest, Response as HttpResponse,
};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message as Frame;
use tracing::{info, warn};

use crate::daemon::{Client, Platform};
use crate::runtime::{dispatch, Shared};

/// Derived from the public key in `extension/public/manifest.json`.
pub const EXTENSION_ID: &str = "klcjcepmegnckiffabjmnjclnpojhelp";
pub const PORT: u16 = 47613;
const MAX_MESSAGE_BYTES: usize = 1 << 20;

static NEXT_CONN: AtomicU64 = AtomicU64::new(1);

pub async fn serve<P: Platform>(
    listener: TcpListener,
    shared: Shared<P>,
    token: Arc<String>,
    pushes: broadcast::Sender<String>,
) {
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            Err(e) => {
                warn!("WebSocket accept failed: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };
        let (shared, token, pushes) = (shared.clone(), token.clone(), pushes.subscribe());
        tokio::spawn(async move {
            connection(stream, shared, &token, pushes).await;
        });
    }
}

#[allow(clippy::result_large_err)] // The signature tungstenite's callback requires.
fn check_origin(
    request: &HttpRequest,
    response: HttpResponse,
) -> Result<HttpResponse, ErrorResponse> {
    let expected = format!("chrome-extension://{EXTENSION_ID}");
    let origin = request
        .headers()
        .get("origin")
        .and_then(|o| o.to_str().ok());
    if origin == Some(expected.as_str()) {
        Ok(response)
    } else {
        warn!("Refused a WebSocket from origin {origin:?}");
        let mut refused = ErrorResponse::new(Some("Forbidden".to_string()));
        *refused.status_mut() = StatusCode::FORBIDDEN;
        Err(refused)
    }
}

/// Checks a first frame. On success returns the browser and the reply to send.
fn authenticate(text: &str, token: &str) -> Result<(String, Request), Reply> {
    let request: Request = serde_json::from_str(text)
        .map_err(|_| Reply::err(String::new(), ErrorCode::InvalidRequest, "Bad message."))?;
    let unauthorized =
        |message: &str| Reply::err(request.id.clone(), ErrorCode::Unauthorized, message);
    match &request.message {
        Message::ExtHello {
            token: given,
            browser,
        } => {
            if given != token {
                return Err(unauthorized("Wrong token."));
            }
            if !SUPPORTED.contains(&browser.as_str()) {
                return Err(unauthorized("Unsupported browser."));
            }
            Ok((browser.clone(), request))
        }
        _ => Err(unauthorized("Say ext_hello first.")),
    }
}

async fn connection<P: Platform>(
    stream: TcpStream,
    shared: Shared<P>,
    token: &str,
    mut pushes: broadcast::Receiver<String>,
) {
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE_BYTES))
        .max_frame_size(Some(MAX_MESSAGE_BYTES));
    let handshake =
        tokio_tungstenite::accept_hdr_async_with_config(stream, check_origin, Some(config));
    let Ok(socket) = handshake.await else { return };
    let (mut sink, mut frames) = socket.split();

    let first = match frames.next().await {
        Some(Ok(Frame::Text(text))) => text,
        _ => return,
    };
    let (browser, hello) = match authenticate(first.as_str(), token) {
        Ok(ok) => ok,
        Err(reply) => {
            warn!("Refused an extension: {:?}", reply.error);
            let _ = sink
                .send(Frame::text(serde_json::json!(reply).to_string()))
                .await;
            return;
        }
    };

    info!("ext_hello from {browser}");
    let conn = NEXT_CONN.fetch_add(1, Ordering::Relaxed);
    let client = Client::Ext {
        conn,
        browser: browser.clone(),
    };
    let state: Value = {
        let mut daemon = shared.lock().expect("daemon lock");
        daemon.ext_connected(&browser);
        daemon.state_json()
    };
    let reply = Reply::ok(hello.id, state);
    if sink
        .send(Frame::text(serde_json::json!(reply).to_string()))
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            frame = frames.next() => match frame {
                Some(Ok(Frame::Text(text))) => {
                    let reply = dispatch(&shared, &client, text.as_str(), Utc::now());
                    if sink.send(Frame::text(reply)).await.is_err() {
                        break;
                    }
                }
                Some(Ok(Frame::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
            push = pushes.recv() => match push {
                Ok(json) => {
                    if sink.send(Frame::text(json)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    shared.lock().expect("daemon lock").ext_disconnected(conn);
    info!("{browser} extension disconnected");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::tests::{t0, utc_now, FakePlatform};
    use crate::daemon::Daemon;
    use crate::db::Db;
    use crate::runtime::publish;
    use serde_json::json;
    use std::net::SocketAddr;
    use std::sync::Mutex;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

    const GOOD_ORIGIN: &str = "chrome-extension://klcjcepmegnckiffabjmnjclnpojhelp";

    struct Rig {
        addr: SocketAddr,
        shared: Shared<FakePlatform>,
        pushes: broadcast::Sender<String>,
        _dir: tempfile::TempDir,
    }

    async fn rig() -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        let daemon = Daemon::new(db, FakePlatform::default(), utc_now, t0());
        let shared = Arc::new(Mutex::new(daemon));
        let (pushes, _) = broadcast::channel(16);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(serve(
            listener,
            shared.clone(),
            Arc::new("secret".to_string()),
            pushes.clone(),
        ));
        Rig {
            addr,
            shared,
            pushes,
            _dir: dir,
        }
    }

    async fn dial(
        addr: SocketAddr,
        origin: Option<&str>,
    ) -> Result<Socket, tokio_tungstenite::tungstenite::Error> {
        let mut request = format!("ws://{addr}/").into_client_request().unwrap();
        if let Some(origin) = origin {
            request
                .headers_mut()
                .insert("origin", origin.parse().unwrap());
        }
        connect_async(request).await.map(|(socket, _)| socket)
    }

    async fn send(socket: &mut Socket, value: Value) {
        socket.send(Frame::text(value.to_string())).await.unwrap();
    }

    async fn recv(socket: &mut Socket) -> Option<Value> {
        let wait = std::time::Duration::from_secs(5);
        match tokio::time::timeout(wait, socket.next())
            .await
            .expect("timed out")
        {
            Some(Ok(Frame::Text(text))) => Some(serde_json::from_str(text.as_str()).unwrap()),
            _ => None,
        }
    }

    fn hello(token: &str) -> Value {
        json!({ "id": "1", "type": "ext_hello", "payload": { "token": token, "browser": "chrome.exe" } })
    }

    #[tokio::test]
    async fn wrong_or_missing_origin_is_refused() {
        let rig = rig().await;
        for origin in [
            None,
            Some("https://evil.example"),
            Some("chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ] {
            assert!(dial(rig.addr, origin).await.is_err(), "{origin:?}");
        }
    }

    #[tokio::test]
    async fn wrong_token_is_refused_and_disconnected() {
        let rig = rig().await;
        let mut socket = dial(rig.addr, Some(GOOD_ORIGIN)).await.unwrap();
        send(&mut socket, hello("wrong")).await;
        let reply = recv(&mut socket).await.unwrap();
        assert_eq!(reply["ok"], false);
        assert_eq!(reply["error"]["code"], "UNAUTHORIZED");
        assert_eq!(recv(&mut socket).await, None);
    }

    #[tokio::test]
    async fn messages_before_hello_are_refused() {
        let rig = rig().await;
        let mut socket = dial(rig.addr, Some(GOOD_ORIGIN)).await.unwrap();
        send(&mut socket, json!({ "id": "1", "type": "get_state" })).await;
        let reply = recv(&mut socket).await.unwrap();
        assert_eq!(reply["error"]["code"], "UNAUTHORIZED");
        assert_eq!(recv(&mut socket).await, None);
    }

    #[tokio::test]
    async fn an_unsupported_browser_is_refused() {
        let rig = rig().await;
        let mut socket = dial(rig.addr, Some(GOOD_ORIGIN)).await.unwrap();
        let message = json!({ "id": "1", "type": "ext_hello", "payload": { "token": "secret", "browser": "x.exe" } });
        send(&mut socket, message).await;
        assert_eq!(recv(&mut socket).await.unwrap()["ok"], false);
    }

    #[tokio::test]
    async fn good_hello_returns_state_then_messages_work_and_pushes_arrive() {
        let rig = rig().await;
        let mut socket = dial(rig.addr, Some(GOOD_ORIGIN)).await.unwrap();
        send(&mut socket, hello("secret")).await;
        let reply = recv(&mut socket).await.unwrap();
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["id"], "1");
        assert!(reply["data"]["blocked"]["hosts_domains"].is_array());

        send(&mut socket, json!({ "id": "2", "type": "ext_heartbeat" })).await;
        let reply = recv(&mut socket).await.unwrap();
        assert_eq!((&reply["id"], &reply["ok"]), (&json!("2"), &json!(true)));

        // The extension has no right to UI messages.
        send(&mut socket, json!({ "id": "3", "type": "stop_pomodoro" })).await;
        assert_eq!(
            recv(&mut socket).await.unwrap()["error"]["code"],
            "UNAUTHORIZED"
        );

        // Garbage gets an error, not a crash.
        socket.send(Frame::text("not json")).await.unwrap();
        assert_eq!(
            recv(&mut socket).await.unwrap()["error"]["code"],
            "INVALID_REQUEST"
        );

        publish(&rig.shared, &rig.pushes, 1, t0());
        let push = recv(&mut socket).await.unwrap();
        assert_eq!(push["type"], "state_changed");
    }
}
