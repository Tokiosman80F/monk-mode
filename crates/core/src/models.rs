use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::planner::{Profile, SmallItem};

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// A site with its own daily limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AllowanceSite {
    pub domain: String,
    pub daily_min: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Settings {
    pub version: u32,
    pub setup_done: bool,
    pub allowance_sites: Vec<AllowanceSite>,
    pub always_blocked: Vec<String>,
    pub shorts_blocker_enabled: bool,
    pub pomodoro_focus_min: u32,
    pub pomodoro_break_min: u32,
    pub daily_focus_target_min: u32,
    /// Filled in by the planner's one-time setup. Settings saved before it have none.
    #[serde(default)]
    pub profile: Option<Profile>,
}

impl Default for Settings {
    fn default() -> Self {
        let site = |domain: &str, daily_min: u32| AllowanceSite {
            domain: domain.to_string(),
            daily_min,
        };
        Self {
            version: 2,
            setup_done: false,
            allowance_sites: vec![
                site("youtube.com", 30),
                site("facebook.com", 10),
                site("instagram.com", 10),
                site("messenger.com", 10),
                site("x.com", 10),
                site("twitter.com", 10),
                site("reddit.com", 10),
                site("open.spotify.com", 10),
                site("music.youtube.com", 10),
                site("soundcloud.com", 10),
            ],
            always_blocked: strings(&["tiktok.com", "tiktokv.com", "tiktokcdn.com"]),
            shorts_blocker_enabled: true,
            pomodoro_focus_min: 50,
            pomodoro_break_min: 10,
            daily_focus_target_min: 11 * 60,
            profile: None,
        }
    }
}

impl Settings {
    /// Reads stored settings. Version 1 had one shared allowance, so each site starts on that
    /// number, which can't loosen a running lock. Its study blocks are dropped for tasks.
    pub fn from_stored(json: &str) -> serde_json::Result<Self> {
        let mut value: serde_json::Value = serde_json::from_str(json)?;
        if value["version"] == 1 {
            let daily_min = value["daily_allowance_min"].clone();
            let sites: Vec<_> = value["allowance_sites"]
                .as_array()
                .map(|sites| {
                    sites
                        .iter()
                        .map(|domain| serde_json::json!({ "domain": domain, "daily_min": daily_min }))
                        .collect()
                })
                .unwrap_or_default();
            value["allowance_sites"] = sites.into();
            value["version"] = 2.into();
        }
        serde_json::from_value(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Subtask {
    pub id: u32,
    pub title: String,
    pub done: bool,
    /// Minutes set aside. Tasks saved before this existed have none.
    #[serde(default)]
    pub allocated_min: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub priority: Priority,
    pub done: bool,
    /// Set by the service. Finished tasks leave the list the day after.
    pub done_on: Option<NaiveDate>,
    /// Minutes set aside. Tasks saved before this existed have none.
    #[serde(default)]
    pub allocated_min: u32,
    pub subtasks: Vec<Subtask>,
    /// One of the profile's focus areas, like `React`.
    #[serde(default)]
    pub focus_area: Option<String>,
    /// Set by the service when an unfinished task moves to a later day.
    #[serde(default)]
    pub carried_from_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Badge {
    FirstFocus,
    Focus10h,
    Focus100h,
    Streak3,
    Streak7,
    Streak30,
    Streak90,
    Tasks10,
    Tasks100,
}

/// Worked out from usage history, never stored on its own (F11).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Progress {
    pub xp: u32,
    pub today_xp: u32,
    pub level: u32,
    pub level_xp: u32,
    pub next_level_xp: u32,
    pub badges: Vec<Badge>,
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
    /// Focus seconds per task title.
    pub focus_sec: BTreeMap<String, u32>,
    /// Seconds used per allowance site.
    pub site_used_sec: BTreeMap<String, u32>,
    /// XP from tasks and subtasks, before the daily cap.
    pub task_xp: u32,
    pub tasks_done: u32,
}

impl UsageDay {
    pub fn new(date: NaiveDate) -> Self {
        Self {
            date,
            allowance_used_sec: 0,
            blocked_attempts: 0,
            focus_sec: BTreeMap::new(),
            site_used_sec: BTreeMap::new(),
            task_xp: 0,
            tasks_done: 0,
        }
    }

    pub fn total_focus_sec(&self) -> u32 {
        self.focus_sec.values().sum()
    }

    pub fn site_used(&self, domain: &str) -> u32 {
        self.site_used_sec.get(domain).copied().unwrap_or(0)
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
    pub task_title: String,
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
        task_title: String,
        started_at: DateTime<Utc>,
        remaining_sec: u32,
        /// Paused sessions don't count down, but sites stay closed.
        #[serde(default)]
        paused: bool,
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
    /// Today's tasks. Earlier days stay in the database.
    pub tasks: Vec<Task>,
    /// Today's small work, including unticked items carried over.
    pub small_items: Vec<SmallItem>,
    /// The lowest id no task or small item has ever used, so new ones never clash with the archive.
    pub next_id: u32,
    /// True once today's plan is confirmed. Until then the app shows the morning prompt.
    pub planned: bool,
    /// Yesterday's unfinished tasks, for the morning prompt to carry, move or drop.
    pub leftovers: Vec<Task>,
    /// Today's row from the saved monthly goal, if there is one.
    pub today_target: Option<String>,
    pub progress: Progress,
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
            tasks: Vec::new(),
            small_items: Vec::new(),
            next_id: 1,
            planned: false,
            leftovers: Vec::new(),
            today_target: None,
            progress: Progress::default(),
            history: Vec::new(),
        }
    }
}

/// The local hour a day starts. Work at 01:00 still belongs to the day before.
pub const DAY_START_HOUR: i64 = 6;

/// The day a moment belongs to: 06:00 to 05:59 local time. Every date in the app comes from here.
pub fn app_day<Tz: TimeZone>(utc: DateTime<Utc>, tz: &Tz) -> NaiveDate {
    (utc.with_timezone(tz).naive_local() - Duration::hours(DAY_START_HOUR)).date()
}

/// The current time, plus the app day it falls on. Keeps the engine free of OS time zone calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    pub utc: DateTime<Utc>,
    pub app_day: NaiveDate,
}

impl Now {
    pub fn new<Tz: TimeZone>(utc: DateTime<Utc>, tz: &Tz) -> Self {
        Self {
            utc,
            app_day: app_day(utc, tz),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_day_runs_from_six_to_five_fifty_nine() {
        // Asia/Dhaka is UTC+6 with no daylight saving.
        let dhaka = chrono::FixedOffset::east_opt(6 * 3600).unwrap();
        let local = |d, h, m| {
            dhaka
                .with_ymd_and_hms(2026, 11, d, h, m, 0)
                .unwrap()
                .to_utc()
        };
        let nov = |d| NaiveDate::from_ymd_opt(2026, 11, d).unwrap();
        assert_eq!(app_day(local(12, 6, 0), &dhaka), nov(12));
        assert_eq!(app_day(local(12, 23, 59), &dhaka), nov(12));
        assert_eq!(app_day(local(13, 1, 0), &dhaka), nov(12));
        assert_eq!(app_day(local(13, 5, 59), &dhaka), nov(12));
        assert_eq!(app_day(local(13, 6, 0), &dhaka), nov(13));
        // Month and year boundaries.
        assert_eq!(
            app_day(local(1, 3, 0), &dhaka),
            NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()
        );
    }

    #[test]
    fn defaults_match_the_prd() {
        let s = Settings::default();
        let sites: Vec<_> = s
            .allowance_sites
            .iter()
            .map(|site| (site.domain.as_str(), site.daily_min))
            .collect();
        assert_eq!(
            sites,
            [
                ("youtube.com", 30),
                ("facebook.com", 10),
                ("instagram.com", 10),
                ("messenger.com", 10),
                ("x.com", 10),
                ("twitter.com", 10),
                ("reddit.com", 10),
                ("open.spotify.com", 10),
                ("music.youtube.com", 10),
                ("soundcloud.com", 10)
            ]
        );
        assert_eq!(
            s.always_blocked,
            ["tiktok.com", "tiktokv.com", "tiktokcdn.com"]
        );
        assert!(s.shorts_blocker_enabled);
        assert!(!s.setup_done);
        assert_eq!(s.pomodoro_focus_min, 50);
        assert_eq!(s.pomodoro_break_min, 10);
        assert_eq!(s.daily_focus_target_min, 660);
    }

    #[test]
    fn version_1_settings_keep_their_sites_and_shared_allowance() {
        let stored = r#"{
            "version": 1, "setup_done": true, "daily_allowance_min": 7,
            "allowance_sites": ["youtube.com", "reddit.com"],
            "always_blocked": ["tiktok.com"], "shorts_blocker_enabled": true,
            "study_blocks": [{ "name": "React", "target_min": 180 }],
            "pomodoro_focus_min": 25, "pomodoro_break_min": 5, "daily_focus_target_min": 600
        }"#;
        let s = Settings::from_stored(stored).unwrap();
        assert_eq!(s.version, 2);
        assert!(s.setup_done);
        assert_eq!(s.pomodoro_focus_min, 25);
        assert_eq!(
            s.allowance_sites,
            [
                AllowanceSite {
                    domain: "youtube.com".to_string(),
                    daily_min: 7
                },
                AllowanceSite {
                    domain: "reddit.com".to_string(),
                    daily_min: 7
                }
            ]
        );
    }

    #[test]
    fn current_settings_round_trip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(Settings::from_stored(&json).unwrap(), s);
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
