//! Messages shared by the named pipe and the WebSocket (PRD 5.2).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::engine::{ActiveTab, BlockedSet};
use crate::models::{Settings, State};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
#[ts(export)]
pub enum Message {
    GetState,
    UpdateSettings(Settings),
    StartLock {
        ends_at: DateTime<Utc>,
    },
    StartPomodoro {
        block_name: String,
    },
    StopPomodoro,
    GetStats {
        days: u32,
    },
    ExtHello {
        token: String,
        /// Process name, such as `chrome.exe`.
        browser: String,
    },
    ExtHeartbeat,
    ExtActiveTab(ActiveTab),
    ExtBlockedAttempt {
        url: String,
    },
    /// The user chose Quit in the tray, so the watchdog leaves the app closed.
    UiQuit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Request {
    pub id: String,
    #[serde(flatten)]
    pub message: Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum ErrorCode {
    LockedLoosening,
    InvalidRequest,
    Unauthorized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReplyError {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Reply {
    pub id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown", optional)]
    pub data: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<ReplyError>,
}

impl Reply {
    pub fn ok(id: String, data: serde_json::Value) -> Self {
        Self {
            id,
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(id: String, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            id,
            ok: false,
            data: None,
            error: Some(ReplyError {
                code,
                message: message.into(),
            }),
        }
    }
}

/// `state_changed`, sent by the service without a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
#[ts(export)]
pub enum Push {
    StateChanged { state: State, blocked: BlockedSet },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> Request {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn request_without_payload_matches_the_prd_shape() {
        let req = Request {
            id: "1".to_string(),
            message: Message::GetState,
        };
        let value = json!({ "id": "1", "type": "get_state" });
        assert_eq!(serde_json::to_value(&req).unwrap(), value);
        assert_eq!(parse(value), req);
    }

    #[test]
    fn request_with_payload_round_trips() {
        let req = parse(json!({
            "id": "2",
            "type": "ext_active_tab",
            "payload": {
                "url": "https://youtube.com/",
                "focused": true,
                "visible": true,
                "incognito": false,
                "idle": false
            }
        }));
        assert!(matches!(req.message, Message::ExtActiveTab(ref t) if t.focused));
        let back: Request = serde_json::from_value(serde_json::to_value(&req).unwrap()).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn unknown_message_types_are_rejected() {
        let result: Result<Request, _> =
            serde_json::from_value(json!({ "id": "3", "type": "unlock_please" }));
        assert!(result.is_err());
    }

    #[test]
    fn success_reply_matches_the_prd_shape() {
        let reply = Reply::ok("1".to_string(), json!({ "a": 1 }));
        assert_eq!(
            serde_json::to_value(&reply).unwrap(),
            json!({ "id": "1", "ok": true, "data": { "a": 1 } })
        );
    }

    #[test]
    fn error_reply_matches_the_prd_shape() {
        let reply = Reply::err("1".to_string(), ErrorCode::LockedLoosening, "Locked.");
        assert_eq!(
            serde_json::to_value(&reply).unwrap(),
            json!({
                "id": "1",
                "ok": false,
                "error": { "code": "LOCKED_LOOSENING", "message": "Locked." }
            })
        );
    }

    #[test]
    fn push_has_the_state_changed_type() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let push = Push::StateChanged {
            state: State::new(date),
            blocked: BlockedSet::default(),
        };
        let value = serde_json::to_value(&push).unwrap();
        assert_eq!(value["type"], "state_changed");
        assert_eq!(value["payload"]["state"]["today"]["date"], "2026-11-05");
        assert!(value["payload"]["state"].get("history").is_none());
    }
}
