use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StudyBlock {
    pub name: String,
    pub target_min: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Settings {
    pub version: u32,
    pub setup_done: bool,
    pub daily_allowance_min: u32,
    pub allowance_sites: Vec<String>,
    pub always_blocked: Vec<String>,
    pub shorts_blocker_enabled: bool,
    pub study_blocks: Vec<StudyBlock>,
    pub pomodoro_focus_min: u32,
    pub pomodoro_break_min: u32,
    pub daily_focus_target_min: u32,
}

impl Default for Settings {
    fn default() -> Self {
        let block = |name: &str, hours: u32| StudyBlock {
            name: name.to_string(),
            target_min: hours * 60,
        };
        Self {
            version: 1,
            setup_done: false,
            daily_allowance_min: 10,
            allowance_sites: strings(&[
                "youtube.com",
                "facebook.com",
                "instagram.com",
                "x.com",
                "twitter.com",
                "reddit.com",
                "open.spotify.com",
                "music.youtube.com",
                "soundcloud.com",
            ]),
            always_blocked: strings(&["tiktok.com", "tiktokv.com", "tiktokcdn.com"]),
            shorts_blocker_enabled: true,
            study_blocks: vec![
                block("React", 3),
                block("Building", 6),
                block("LinkedIn course", 2),
            ],
            pomodoro_focus_min: 50,
            pomodoro_break_min: 10,
            daily_focus_target_min: 11 * 60,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Lock {
    pub created_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

impl Lock {
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        now < self.ends_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UsageDay {
    pub date: NaiveDate,
    pub allowance_used_sec: u32,
    pub blocked_attempts: u32,
    /// Focus seconds per study block name.
    pub focus_sec: BTreeMap<String, u32>,
}

impl UsageDay {
    pub fn new(date: NaiveDate) -> Self {
        Self {
            date,
            allowance_used_sec: 0,
            blocked_attempts: 0,
            focus_sec: BTreeMap::new(),
        }
    }

    pub fn total_focus_sec(&self) -> u32 {
        self.focus_sec.values().sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FocusKind {
    Pomodoro,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FocusSession {
    /// Assigned by the database.
    pub id: Option<u32>,
    pub block_name: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub kind: FocusKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "phase", rename_all = "snake_case")]
#[ts(export)]
pub enum PomodoroState {
    Idle,
    Focus {
        block_name: String,
        started_at: DateTime<Utc>,
        remaining_sec: u32,
    },
    Break {
        remaining_sec: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LockDay {
    pub day: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct State {
    pub settings: Settings,
    pub lock: Option<Lock>,
    pub today: UsageDay,
    pub pomodoro: PomodoroState,
    pub streak_days: u32,
    pub lock_day: Option<LockDay>,
    /// Past days, oldest first. Kept off the wire: `state_changed` goes out every second.
    #[serde(skip)]
    #[ts(skip)]
    pub history: Vec<UsageDay>,
}

impl State {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            settings: Settings::default(),
            lock: None,
            today: UsageDay::new(today),
            pomodoro: PomodoroState::Idle,
            streak_days: 0,
            lock_day: None,
            history: Vec::new(),
        }
    }
}

/// The current time, plus the local date it falls on. Keeps the engine free of OS time zone calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    pub utc: DateTime<Utc>,
    pub local_date: NaiveDate,
}

impl Now {
    pub fn new<Tz: TimeZone>(utc: DateTime<Utc>, tz: &Tz) -> Self {
        Self {
            utc,
            local_date: utc.with_timezone(tz).date_naive(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_prd() {
        let s = Settings::default();
        assert_eq!(s.daily_allowance_min, 10);
        assert_eq!(
            s.allowance_sites,
            [
                "youtube.com",
                "facebook.com",
                "instagram.com",
                "x.com",
                "twitter.com",
                "reddit.com",
                "open.spotify.com",
                "music.youtube.com",
                "soundcloud.com"
            ]
        );
        assert_eq!(
            s.always_blocked,
            ["tiktok.com", "tiktokv.com", "tiktokcdn.com"]
        );
        assert!(s.shorts_blocker_enabled);
        assert!(!s.setup_done);
        let blocks: Vec<_> = s
            .study_blocks
            .iter()
            .map(|b| (b.name.as_str(), b.target_min))
            .collect();
        assert_eq!(
            blocks,
            [("React", 180), ("Building", 360), ("LinkedIn course", 120)]
        );
        assert_eq!(s.pomodoro_focus_min, 50);
        assert_eq!(s.pomodoro_break_min, 10);
        assert_eq!(s.daily_focus_target_min, 660);
    }

    #[test]
    fn new_state_starts_idle_and_unlocked() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let state = State::new(date);
        assert_eq!(state.pomodoro, PomodoroState::Idle);
        assert_eq!(state.lock, None);
        assert_eq!(state.today, UsageDay::new(date));
    }
}
