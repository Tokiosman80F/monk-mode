//! The service's brain, with no networking or real OS calls: one `step` per second, one `handle`
//! per incoming message. The OS sits behind `Platform` so all of this runs in tests.

use std::collections::HashMap;
use std::io;

use chrono::{DateTime, Utc};
use monk_core::engine::{
    compute_blocked, pause_pomodoro, start_pomodoro, stop_pomodoro, tick, ActiveTab, BlockedSet,
    Event,
};
use monk_core::lock::{
    effective_now, next_lock, validate_change, validate_settings, NextLockError,
};
use monk_core::models::{FocusSession, Lock, Now, Settings, State, UsageDay};
use monk_core::planner::LeftoverAction;
use monk_core::protocol::{ErrorCode, Message};
use monk_core::stats::build_stats;
use monk_core::tasks::{apply_small_items, apply_tasks, plan_day, resolve_leftover};
use monk_platform::registry::LockRegistryError;
use serde_json::{json, Value};
use tracing::{error, warn};

use crate::db::Db;

/// How often the hosts file and browser list are re-checked (F6).
const CHECK_EVERY_SEC: u64 = 5;
/// How long a browser may run without the extension before allowance sites are blocked (F4).
const EXTENSION_GRACE_SEC: u64 = 30;
/// A heartbeat or tab report older than this no longer counts. The extension sends one every 5 s.
const FRESH_SEC: u64 = 15;
/// After a sleep or a stall, the gap isn't study time.
const MAX_TICK_SEC: u32 = 5;
const LAST_SEEN_SAVE_EVERY_SEC: u64 = 60;
/// XP and badges add up from every saved day, so this is effectively all of them.
const HISTORY_DAYS: u32 = 3650;

pub trait Platform: Send + 'static {
    fn enforce_hosts(&mut self, domains: &[String]) -> io::Result<bool>;
    fn running_browsers(&mut self) -> Vec<String>;
    fn read_lock_copy(&self) -> Result<Option<DateTime<Utc>>, LockRegistryError>;
    fn write_lock_copy(&mut self, ends_at: DateTime<Utc>) -> Result<(), LockRegistryError>;
    fn sign(&self, ends_at: DateTime<Utc>) -> String;
    fn verify(&self, ends_at: DateTime<Utc>, signature: &str) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Client {
    Ui,
    /// An authenticated extension connection. `conn` tells two browsers apart.
    Ext {
        conn: u64,
        browser: String,
    },
}

pub type HandlerError = (ErrorCode, String);

fn invalid(message: impl ToString) -> HandlerError {
    (ErrorCode::InvalidRequest, message.to_string())
}

/// The real lock, from the database and registry copies, with both repaired to match. Deleting
/// or editing one copy can't unlock: only a validly signed copy counts, and the later one wins.
pub fn reconcile_lock<P: Platform>(db: &Db, platform: &mut P, now: DateTime<Utc>) -> Option<Lock> {
    let row = db.load_lock().unwrap_or_else(|e| {
        error!("Couldn't read the lock from the database: {e}");
        None
    });
    let registry = platform.read_lock_copy().unwrap_or_else(|e| {
        warn!("Ignoring the registry lock copy: {e}");
        None
    });
    let db_end = row
        .as_ref()
        .filter(|(l, sig)| platform.verify(l.ends_at, sig))
        .map(|(l, _)| l.ends_at);

    // Neither copy verifies but a row exists: honor it rather than assume unlocked.
    let ends_at = db_end
        .max(registry)
        .or_else(|| row.as_ref().map(|(l, _)| l.ends_at))?;

    let lock = Lock {
        created_at: row.map_or(now, |(l, _)| l.created_at),
        ends_at,
    };
    if db_end != Some(ends_at) {
        if let Err(e) = db.save_lock(&lock, &platform.sign(ends_at)) {
            error!("Couldn't repair the database lock copy: {e}");
        }
    }
    if registry != Some(ends_at) {
        if let Err(e) = platform.write_lock_copy(ends_at) {
            error!("Couldn't repair the registry lock copy: {e}");
        }
    }
    Some(lock)
}

pub struct Daemon<P: Platform> {
    state: State,
    db: Db,
    platform: P,
    to_now: fn(DateTime<Utc>) -> Now,
    last_seen: DateTime<Utc>,
    /// Seconds counted by this process. Unlike wall time, nobody can set it back.
    uptime: u64,
    tabs: HashMap<u64, (ActiveTab, u64)>,
    heartbeats: HashMap<String, u64>,
    unconnected_since: Option<u64>,
    extension_ok: bool,
    missing_browsers: Vec<String>,
    last_check: Option<u64>,
    last_seen_saved_at: u64,
    enforced: Option<BlockedSet>,
    saved_day: UsageDay,
    last_push: String,
    ui_quit: bool,
    open_app: bool,
}

/// What the morning prompt needs for `today`: whether it's planned, yesterday's unfinished
/// tasks, and the monthly goal's target.
fn load_morning(db: &Db, state: &mut State, today: chrono::NaiveDate) {
    state.planned = db.is_planned(today).unwrap_or_else(|e| {
        error!("Couldn't read the day log: {e}");
        false
    });
    let yesterday = today.pred_opt().unwrap_or(today);
    state.leftovers = db.load_open_tasks(yesterday).unwrap_or_else(|e| {
        error!("Couldn't read yesterday's tasks: {e}");
        Vec::new()
    });
    state.today_target = db.load_target(today).unwrap_or_else(|e| {
        error!("Couldn't read today's target: {e}");
        None
    });
}

impl<P: Platform> Daemon<P> {
    pub fn new(
        db: Db,
        mut platform: P,
        to_now: fn(DateTime<Utc>) -> Now,
        utc: DateTime<Utc>,
    ) -> Self {
        let last_seen = db.load_last_seen().unwrap_or_else(|e| {
            error!("Couldn't read the last seen time: {e}");
            None
        });
        let last_seen = last_seen.map_or(utc, |seen| effective_now(utc, seen));
        let now = to_now(last_seen);

        let mut state = State::new(now.app_day);
        state.settings = db.load_settings().ok().flatten().unwrap_or_else(|| {
            let defaults = Settings::default();
            if let Err(e) = db.save_settings(&defaults) {
                error!("Couldn't save default settings: {e}");
            }
            defaults
        });
        state.lock = reconcile_lock(&db, &mut platform, last_seen);
        // Tasks are stored by day, so a 06:00 missed while the service was off needs no work
        // for them. Small work still has to move to today.
        let today = now.app_day;
        if let Err(e) = db.import_old_tasks(today) {
            error!("Couldn't move old tasks: {e}");
        }
        if let Err(e) = db.carry_small_items(today) {
            error!("Couldn't carry small work: {e}");
        }
        state.tasks = db.load_tasks(today).unwrap_or_else(|e| {
            error!("Couldn't read tasks: {e}");
            Vec::new()
        });
        state.small_items = db.load_small_items(today).unwrap_or_else(|e| {
            error!("Couldn't read small work: {e}");
            Vec::new()
        });
        state.next_id = db.max_item_id().unwrap_or_else(|e| {
            error!("Couldn't read the last id: {e}");
            0
        }) + 1;
        load_morning(&db, &mut state, today);

        let mut history = db.load_days(HISTORY_DAYS).unwrap_or_else(|e| {
            error!("Couldn't read usage history: {e}");
            Vec::new()
        });
        if let Some(today) = history.pop_if(|d| d.date >= now.app_day) {
            state.today = today;
        }
        state.history = history;
        if let Err(e) = db.save_last_seen(last_seen) {
            error!("Couldn't save the last seen time: {e}");
        }

        let saved_day = state.today.clone();
        Self {
            state,
            db,
            platform,
            to_now,
            last_seen,
            uptime: 0,
            tabs: HashMap::new(),
            heartbeats: HashMap::new(),
            unconnected_since: None,
            extension_ok: true,
            missing_browsers: Vec::new(),
            last_check: None,
            last_seen_saved_at: 0,
            enforced: None,
            saved_day,
            last_push: String::new(),
            ui_quit: false,
            open_app: false,
        }
    }

    fn now(&mut self, utc: DateTime<Utc>) -> Now {
        self.last_seen = effective_now(utc, self.last_seen);
        (self.to_now)(self.last_seen)
    }

    pub fn is_locked(&self) -> bool {
        self.state
            .lock
            .as_ref()
            .is_some_and(|l| l.is_active(self.last_seen))
    }

    pub fn ui_quit(&self) -> bool {
        self.ui_quit
    }

    /// Whether the block page asked for the app since the last call.
    pub fn take_open_app(&mut self) -> bool {
        std::mem::take(&mut self.open_app)
    }

    pub fn ui_connected(&mut self) {
        self.ui_quit = false;
    }

    pub fn ext_connected(&mut self, browser: &str) {
        self.heartbeats.insert(browser.to_string(), self.uptime);
    }

    pub fn ext_disconnected(&mut self, conn: u64) {
        self.tabs.remove(&conn);
    }

    fn is_fresh(&self, at: u64) -> bool {
        self.uptime.saturating_sub(at) <= FRESH_SEC
    }

    /// The tab being watched right now, across all connected browsers.
    fn counting_tab(&self) -> Option<ActiveTab> {
        self.tabs
            .values()
            .filter(|(tab, at)| self.is_fresh(*at) && tab.focused && tab.visible && !tab.idle)
            .max_by_key(|(_, at)| *at)
            .map(|(tab, _)| tab.clone())
    }

    fn check_browsers(&mut self) {
        let mut missing = self.platform.running_browsers();
        missing.retain(|browser| {
            !self
                .heartbeats
                .get(browser)
                .is_some_and(|at| self.is_fresh(*at))
        });
        self.unconnected_since = if !missing.is_empty() {
            Some(self.unconnected_since.unwrap_or(self.uptime))
        } else {
            None
        };
        self.extension_ok = self
            .unconnected_since
            .is_none_or(|since| self.uptime - since <= EXTENSION_GRACE_SEC);
        self.missing_browsers = missing;
    }

    /// Runs once per second. Returns what happened, for logging and notifications.
    pub fn step(&mut self, elapsed_sec: u32, utc: DateTime<Utc>) -> Vec<Event> {
        let elapsed_sec = elapsed_sec.min(MAX_TICK_SEC);
        self.uptime += u64::from(elapsed_sec);
        let now = self.now(utc);
        let tab = self.counting_tab();
        let events = tick(&mut self.state, elapsed_sec, now, tab.as_ref());
        self.record(&events);

        let check_due = self
            .last_check
            .is_none_or(|at| self.uptime - at >= CHECK_EVERY_SEC);
        if check_due {
            self.last_check = Some(self.uptime);
            self.check_browsers();
        }

        let blocked = compute_blocked(&self.state, now, self.extension_ok);
        if check_due || self.enforced.as_ref() != Some(&blocked) {
            match self.platform.enforce_hosts(&blocked.hosts_domains) {
                Ok(_) => self.enforced = Some(blocked),
                Err(e) => error!("Couldn't apply the hosts file: {e}"),
            }
        }
        if check_due {
            self.save_today();
        }
        if self.uptime - self.last_seen_saved_at >= LAST_SEEN_SAVE_EVERY_SEC {
            self.last_seen_saved_at = self.uptime;
            self.log_db(self.db.save_last_seen(self.last_seen));
        }
        events
    }

    fn record(&mut self, events: &[Event]) {
        for event in events {
            match event {
                Event::DayRolled(day) => {
                    self.log_db(self.db.save_day(day));
                    self.save_today();
                    let today = self.state.today.date;
                    self.log_db(self.db.carry_small_items(today));
                    load_morning(&self.db, &mut self.state, today);
                }
                Event::FocusEnded(session) => self.save_session(session),
                _ => {}
            }
        }
    }

    fn save_session(&self, session: &FocusSession) {
        self.log_db(self.db.insert_focus_session(session));
    }

    fn save_today(&mut self) {
        if self.saved_day != self.state.today {
            self.log_db(self.db.save_day(&self.state.today));
            self.saved_day = self.state.today.clone();
        }
    }

    fn log_db<T>(&self, result: crate::db::Result<T>) {
        if let Err(e) = result {
            error!("Database write failed: {e}");
        }
    }

    /// Keeps today's numbers when the service is told to stop.
    pub fn flush(&mut self) {
        self.save_today();
        self.log_db(self.db.save_last_seen(self.last_seen));
    }

    fn snapshot(&self) -> (&State, BlockedSet) {
        let now = (self.to_now)(self.last_seen);
        (
            &self.state,
            compute_blocked(&self.state, now, self.extension_ok),
        )
    }

    pub fn state_json(&self) -> Value {
        let (state, blocked) = self.snapshot();
        let mut connected_browsers: Vec<&String> = self
            .heartbeats
            .iter()
            .filter(|(_, at)| self.is_fresh(**at))
            .map(|(browser, _)| browser)
            .collect();
        connected_browsers.sort();
        json!({
            "state": state,
            "blocked": blocked,
            "connected_browsers": connected_browsers,
            "missing_browsers": self.missing_browsers,
        })
    }

    /// The `state_changed` message, if anything changed since the last one.
    pub fn push_if_changed(&mut self) -> Option<String> {
        let push = json!({ "type": "state_changed", "payload": self.state_json() }).to_string();
        if push == self.last_push {
            return None;
        }
        self.last_push = push.clone();
        Some(push)
    }

    pub fn handle(
        &mut self,
        client: &Client,
        message: Message,
        utc: DateTime<Utc>,
    ) -> Result<Value, HandlerError> {
        let now = self.now(utc);
        match (client, message) {
            (_, Message::GetState) => Ok(self.state_json()),

            (Client::Ui, Message::UpdateSettings(new)) => {
                validate_settings(&new).map_err(invalid)?;
                validate_change(
                    &self.state.settings,
                    &new,
                    self.state.lock.as_ref(),
                    now.utc,
                )
                .map_err(|e| (ErrorCode::LockedLoosening, e.to_string()))?;
                self.db
                    .save_settings(&new)
                    .map_err(|e| invalid(format!("Couldn't save settings: {e}")))?;
                self.state.settings = new;
                Ok(self.state_json())
            }
            (Client::Ui, Message::StartLock { ends_at }) => {
                let lock =
                    next_lock(self.state.lock.as_ref(), ends_at, now.utc).map_err(|e| match e {
                        NextLockError::Invalid(e) => invalid(e),
                        NextLockError::Loosening(e) => (ErrorCode::LockedLoosening, e.to_string()),
                    })?;
                self.db
                    .save_lock(&lock, &self.platform.sign(lock.ends_at))
                    .map_err(|e| invalid(format!("Couldn't save the lock: {e}")))?;
                if let Err(e) = self.platform.write_lock_copy(lock.ends_at) {
                    // The database copy holds the lock. The next start repairs this one.
                    error!("Couldn't write the registry lock copy: {e}");
                }
                self.state.lock = Some(lock);
                Ok(self.state_json())
            }
            (Client::Ui, Message::UpdateSmallItems(items)) => {
                apply_small_items(&mut self.state, items, now.app_day).map_err(invalid)?;
                self.db
                    .save_small_items(now.app_day, &self.state.small_items)
                    .map_err(|e| invalid(format!("Couldn't save small work: {e}")))?;
                Ok(self.state_json())
            }
            (Client::Ui, Message::ResolveLeftover { task_id, action }) => {
                let today = now.app_day;
                resolve_leftover(&mut self.state, task_id, action, today).map_err(invalid)?;
                let stored = match action {
                    LeftoverAction::Carry => self.db.move_task(task_id, today),
                    LeftoverAction::Move { date } => self.db.move_task(task_id, date),
                    LeftoverAction::Drop => self.db.drop_task(task_id),
                };
                if let Err(e) = stored {
                    // Most likely the other day is full. Show what's really stored.
                    load_morning(&self.db, &mut self.state, today);
                    self.state.tasks = self.db.load_tasks(today).unwrap_or_default();
                    return Err(invalid(format!("Couldn't move that task: {e}")));
                }
                Ok(self.state_json())
            }
            (Client::Ui, Message::PlanDay) => {
                plan_day(&mut self.state).map_err(invalid)?;
                self.log_db(self.db.mark_planned(now.app_day, now.utc));
                Ok(self.state_json())
            }
            (Client::Ui, Message::UpdateTasks(tasks)) => {
                apply_tasks(&mut self.state, tasks, now.app_day).map_err(invalid)?;
                self.db
                    .save_tasks(now.app_day, &self.state.tasks)
                    .map_err(|e| invalid(format!("Couldn't save tasks: {e}")))?;
                self.save_today();
                Ok(self.state_json())
            }
            (Client::Ui, Message::StartPomodoro { task_id }) => {
                start_pomodoro(&mut self.state, task_id, now).map_err(invalid)?;
                Ok(self.state_json())
            }
            (Client::Ui, Message::PausePomodoro { paused }) => {
                pause_pomodoro(&mut self.state, paused).map_err(invalid)?;
                Ok(self.state_json())
            }
            (Client::Ui, Message::StopPomodoro) => {
                if let Some(session) = stop_pomodoro(&mut self.state, now) {
                    self.save_session(&session);
                }
                Ok(self.state_json())
            }
            (Client::Ui, Message::GetStats { days }) => {
                let days = days.clamp(1, HISTORY_DAYS) as usize;
                let stats = build_stats(&self.state.history, &self.state.today, days);
                Ok(json!(stats))
            }
            (Client::Ui, Message::UiQuit) => {
                self.ui_quit = true;
                Ok(Value::Null)
            }

            (Client::Ext { browser, .. }, Message::ExtHeartbeat) => {
                self.ext_connected(browser);
                Ok(Value::Null)
            }
            (Client::Ext { conn, .. }, Message::ExtActiveTab(tab)) => {
                self.tabs.insert(*conn, (tab, self.uptime));
                Ok(Value::Null)
            }
            (Client::Ext { .. }, Message::ExtOpenApp) => {
                self.open_app = true;
                Ok(Value::Null)
            }
            (Client::Ext { .. }, Message::ExtBlockedAttempt { .. }) => {
                self.state.today.blocked_attempts += 1;
                Ok(Value::Null)
            }

            (_, _) => Err((
                ErrorCode::Unauthorized,
                "This client can't send that message.".to_string(),
            )),
        }
    }

    #[cfg(test)]
    pub fn state(&self) -> &State {
        &self.state
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use monk_core::models::{PomodoroState, Priority, Task};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    pub struct Fake {
        pub hosts: Vec<String>,
        pub writes: u32,
        pub fail_hosts: bool,
        pub browsers: Vec<String>,
        pub lock_copy: Option<DateTime<Utc>>,
        pub lock_copy_tampered: bool,
    }

    /// A fake OS the test keeps a handle to, so it can look inside after the daemon takes it.
    #[derive(Clone, Default)]
    pub struct FakePlatform(pub Arc<Mutex<Fake>>);

    impl Platform for FakePlatform {
        fn enforce_hosts(&mut self, domains: &[String]) -> io::Result<bool> {
            let mut fake = self.0.lock().unwrap();
            fake.writes += 1;
            if fake.fail_hosts {
                return Err(io::Error::other("denied"));
            }
            let changed = fake.hosts != domains;
            fake.hosts = domains.to_vec();
            Ok(changed)
        }
        fn running_browsers(&mut self) -> Vec<String> {
            self.0.lock().unwrap().browsers.clone()
        }
        fn read_lock_copy(&self) -> Result<Option<DateTime<Utc>>, LockRegistryError> {
            let fake = self.0.lock().unwrap();
            if fake.lock_copy_tampered {
                return Err(LockRegistryError::Tampered);
            }
            Ok(fake.lock_copy)
        }
        fn write_lock_copy(&mut self, ends_at: DateTime<Utc>) -> Result<(), LockRegistryError> {
            let mut fake = self.0.lock().unwrap();
            fake.lock_copy = Some(ends_at);
            fake.lock_copy_tampered = false;
            Ok(())
        }
        fn sign(&self, ends_at: DateTime<Utc>) -> String {
            format!("signed:{}", ends_at.to_rfc3339())
        }
        fn verify(&self, ends_at: DateTime<Utc>, signature: &str) -> bool {
            self.sign(ends_at) == signature
        }
    }

    pub fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 5, 12, 0, 0).unwrap()
    }

    pub fn utc_now(utc: DateTime<Utc>) -> Now {
        Now::new(utc, &Utc)
    }

    /// Every site on the same daily limit.
    fn limits(daily_min: u32) -> Settings {
        let mut settings = Settings::default();
        for site in &mut settings.allowance_sites {
            site.daily_min = daily_min;
        }
        settings
    }

    struct Rig {
        daemon: Daemon<FakePlatform>,
        fake: FakePlatform,
        utc: DateTime<Utc>,
        dir: tempfile::TempDir,
    }

    impl Rig {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let fake = FakePlatform::default();
            let db = Db::open(dir.path()).unwrap();
            let daemon = Daemon::new(db, fake.clone(), utc_now, t0());
            Self {
                daemon,
                fake,
                utc: t0(),
                dir,
            }
        }

        /// A new daemon on the same database and OS, as after a service restart.
        fn restart(&mut self) {
            self.daemon.flush();
            let db = Db::open(self.dir.path()).unwrap();
            self.daemon = Daemon::new(db, self.fake.clone(), utc_now, self.utc);
        }

        fn run(&mut self, seconds: u32) -> Vec<Event> {
            let mut events = Vec::new();
            for _ in 0..seconds {
                self.utc += Duration::seconds(1);
                events.extend(self.daemon.step(1, self.utc));
            }
            events
        }

        fn ui(&mut self, message: Message) -> Result<Value, HandlerError> {
            self.daemon.handle(&Client::Ui, message, self.utc)
        }

        fn ext(
            &mut self,
            conn: u64,
            browser: &str,
            message: Message,
        ) -> Result<Value, HandlerError> {
            let client = Client::Ext {
                conn,
                browser: browser.to_string(),
            };
            self.daemon.handle(&client, message, self.utc)
        }

        fn tab(&mut self, conn: u64, focused: bool) {
            let tab = ActiveTab {
                url: "https://www.youtube.com/watch?v=1".to_string(),
                focused,
                visible: true,
                incognito: false,
                idle: false,
            };
            self.ext(conn, "chrome.exe", Message::ExtActiveTab(tab))
                .unwrap();
        }

        fn hosts_has(&self, domain: &str) -> bool {
            let fake = self.fake.0.lock().unwrap();
            fake.hosts.iter().any(|d| d == domain)
        }

        fn used(&self) -> u32 {
            self.daemon.state().today.allowance_used_sec
        }

        fn task(&mut self, title: &str) -> Option<u32> {
            let id = self.daemon.state().next_id;
            let mut tasks = self.daemon.state().tasks.clone();
            tasks.push(Task {
                id,
                title: title.to_string(),
                priority: Priority::High,
                done: false,
                done_on: None,
                allocated_min: 0,
                subtasks: vec![],
                focus_area: None,
                carried_from_date: None,
            });
            self.ui(Message::UpdateTasks(tasks)).unwrap();
            Some(id)
        }

        fn lock_for_90_days(&mut self) {
            self.ui(Message::StartLock {
                ends_at: self.utc + Duration::days(90),
            })
            .unwrap();
        }
    }

    // Main loop (T3.2)

    #[test]
    fn enforces_always_blocked_sites_on_the_first_step() {
        let mut rig = Rig::new();
        rig.run(1);
        assert!(rig.hosts_has("tiktok.com"));
        assert!(!rig.hosts_has("youtube.com"));
    }

    #[test]
    fn hosts_are_rewritten_every_5_seconds_even_with_no_rule_change() {
        let mut rig = Rig::new();
        rig.run(1);
        let first = rig.fake.0.lock().unwrap().writes;
        rig.run(10);
        assert_eq!(rig.fake.0.lock().unwrap().writes, first + 2);
    }

    #[test]
    fn hand_edited_hosts_are_restored_within_5_seconds() {
        let mut rig = Rig::new();
        rig.run(1);
        rig.fake.0.lock().unwrap().hosts.clear();
        rig.run(5);
        assert!(rig.hosts_has("tiktok.com"));
    }

    #[test]
    fn using_up_the_allowance_blocks_on_that_same_tick() {
        let mut rig = Rig::new();
        rig.ui(Message::UpdateSettings(limits(1))).unwrap();
        // 59 seconds, re-reporting the tab like the real extension does.
        for _ in 0..59 {
            rig.tab(1, true);
            rig.run(1);
        }
        assert!(!rig.hosts_has("youtube.com"));
        rig.tab(1, true);
        let events = rig.run(1);
        assert!(events.contains(&Event::AllowanceUsedUp));
        assert!(rig.hosts_has("youtube.com"));
    }

    #[test]
    fn a_failed_hosts_write_is_retried_on_the_next_tick() {
        let mut rig = Rig::new();
        rig.fake.0.lock().unwrap().fail_hosts = true;
        rig.run(1);
        rig.fake.0.lock().unwrap().fail_hosts = false;
        rig.run(1);
        assert!(rig.hosts_has("tiktok.com"));
    }

    #[test]
    fn a_stale_tab_report_stops_counting() {
        let mut rig = Rig::new();
        rig.tab(1, true);
        rig.run(30);
        assert_eq!(rig.used(), 15);
    }

    #[test]
    fn a_long_gap_is_capped_so_sleep_is_not_study_time() {
        let mut rig = Rig::new();
        let task_id = rig.task("React");
        rig.ui(Message::StartPomodoro { task_id }).unwrap();
        rig.utc += Duration::hours(3);
        rig.daemon.step(3 * 3600, rig.utc);
        assert_eq!(rig.daemon.state().today.focus_sec["React"], 5);
    }

    #[test]
    fn a_focused_browser_is_not_cancelled_out_by_an_unfocused_one() {
        let mut rig = Rig::new();
        rig.tab(1, true);
        rig.tab(2, false);
        rig.run(1);
        assert_eq!(rig.used(), 1);
    }

    #[test]
    fn a_closed_connection_stops_counting_at_once() {
        let mut rig = Rig::new();
        rig.tab(1, true);
        rig.daemon.ext_disconnected(1);
        rig.run(1);
        assert_eq!(rig.used(), 0);
    }

    #[test]
    fn usage_survives_a_restart() {
        let mut rig = Rig::new();
        rig.tab(1, true);
        rig.run(3);
        rig.restart();
        assert_eq!(rig.used(), 3);
    }

    #[test]
    fn six_am_saves_yesterday_and_a_restart_keeps_it_in_history() {
        let mut rig = Rig::new();
        rig.tab(1, true);
        rig.run(2);
        rig.daemon.ext_disconnected(1);
        rig.utc = Utc.with_ymd_and_hms(2026, 11, 6, 6, 0, 0).unwrap();
        let events = rig.run(1);
        assert!(matches!(events[..], [Event::DayRolled(_)]));
        assert_eq!(rig.used(), 0);

        rig.restart();
        let history = &rig.daemon.state().history;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].allowance_used_sec, 2);
    }

    #[test]
    fn a_clock_turned_back_while_stopped_does_not_refill_the_allowance() {
        let mut rig = Rig::new();
        rig.tab(1, true);
        rig.run(3);
        rig.restart();
        rig.utc -= Duration::days(30);
        rig.restart();
        rig.run(1);
        assert_eq!(rig.used(), 3);
        assert_eq!(rig.daemon.state().today.date, t0().date_naive());
    }

    // Handlers (T3.5)

    #[test]
    fn loosening_while_locked_returns_the_locked_error() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        let result = rig.ui(Message::UpdateSettings(limits(40)));
        assert_eq!(result.unwrap_err().0, ErrorCode::LockedLoosening);
        assert_eq!(rig.daemon.state().settings, Settings::default());
    }

    #[test]
    fn tightening_while_locked_is_saved() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        rig.ui(Message::UpdateSettings(limits(5))).unwrap();
        rig.restart();
        assert_eq!(rig.daemon.state().settings.allowance_sites[0].daily_min, 5);
    }

    #[test]
    fn invalid_settings_are_rejected_even_without_a_lock() {
        let mut rig = Rig::new();
        let result = rig.ui(Message::UpdateSettings(limits(181)));
        assert_eq!(result.unwrap_err().0, ErrorCode::InvalidRequest);
    }

    #[test]
    fn shortening_a_lock_is_refused() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        let result = rig.ui(Message::StartLock {
            ends_at: rig.utc + Duration::days(1),
        });
        assert_eq!(result.unwrap_err().0, ErrorCode::LockedLoosening);
    }

    #[test]
    fn pomodoro_messages_drive_the_state_and_block_sites() {
        let mut rig = Rig::new();
        let task_id = rig.task("React");
        rig.ui(Message::StartPomodoro { task_id }).unwrap();
        rig.run(1);
        assert!(rig.hosts_has("youtube.com"));
        rig.ui(Message::StopPomodoro).unwrap();
        assert_eq!(rig.daemon.state().pomodoro, PomodoroState::Idle);
        rig.run(1);
        assert!(!rig.hosts_has("youtube.com"));
        let unknown = rig.ui(Message::StartPomodoro { task_id: Some(99) });
        assert_eq!(unknown.unwrap_err().0, ErrorCode::InvalidRequest);
    }

    #[test]
    fn get_state_and_get_stats_return_their_data() {
        let mut rig = Rig::new();
        let state = rig.ui(Message::GetState).unwrap();
        assert_eq!(
            state["state"]["settings"]["allowance_sites"][0]["daily_min"],
            30
        );
        assert!(state["blocked"]["hosts_domains"].is_array());
        let stats = rig.ui(Message::GetStats { days: 7 }).unwrap();
        assert_eq!(stats["days"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn blocked_attempts_are_counted_and_saved() {
        let mut rig = Rig::new();
        let message = Message::ExtBlockedAttempt {
            url: "https://tiktok.com".to_string(),
        };
        rig.ext(1, "chrome.exe", message.clone()).unwrap();
        rig.ext(1, "chrome.exe", message).unwrap();
        rig.restart();
        assert_eq!(rig.daemon.state().today.blocked_attempts, 2);
    }

    #[test]
    fn the_extension_cannot_send_ui_messages() {
        let mut rig = Rig::new();
        let result = rig.ext(
            1,
            "chrome.exe",
            Message::UpdateSettings(Settings::default()),
        );
        assert_eq!(result.unwrap_err().0, ErrorCode::Unauthorized);
        let result = rig.ext(1, "chrome.exe", Message::StopPomodoro);
        assert_eq!(result.unwrap_err().0, ErrorCode::Unauthorized);
    }

    #[test]
    fn the_ui_cannot_pretend_to_be_the_extension() {
        let mut rig = Rig::new();
        assert_eq!(
            rig.ui(Message::ExtHeartbeat).unwrap_err().0,
            ErrorCode::Unauthorized
        );
    }

    #[test]
    fn the_block_page_can_ask_for_the_app_once() {
        let mut rig = Rig::new();
        assert!(!rig.daemon.take_open_app());
        rig.ext(1, "chrome.exe", Message::ExtOpenApp).unwrap();
        assert!(rig.daemon.take_open_app());
        assert!(!rig.daemon.take_open_app());
    }

    #[test]
    fn ui_quit_is_remembered_until_the_ui_returns() {
        let mut rig = Rig::new();
        assert!(!rig.daemon.ui_quit());
        rig.ui(Message::UiQuit).unwrap();
        assert!(rig.daemon.ui_quit());
        rig.daemon.ui_connected();
        assert!(!rig.daemon.ui_quit());
    }

    #[test]
    fn tasks_and_their_xp_survive_a_restart() {
        let mut rig = Rig::new();
        rig.task("Ship");
        let mut tasks = rig.daemon.state().tasks.clone();
        tasks[0].done = true;
        rig.ui(Message::UpdateTasks(tasks)).unwrap();
        assert_eq!(rig.daemon.state().progress.xp, 50);

        rig.restart();
        rig.run(1);
        assert_eq!(rig.daemon.state().tasks[0].title, "Ship");
        assert_eq!(rig.daemon.state().progress.xp, 50);
    }

    #[test]
    fn six_am_clears_the_list_even_if_the_service_was_off() {
        let mut rig = Rig::new();
        rig.task("Ship");
        rig.task("Open");
        let mut tasks = rig.daemon.state().tasks.clone();
        tasks[0].done = true;
        rig.ui(Message::UpdateTasks(tasks)).unwrap();
        rig.utc += Duration::days(1);
        rig.restart();
        assert!(rig.daemon.state().tasks.is_empty());
        // Yesterday's XP stays.
        rig.run(1);
        assert_eq!(rig.daemon.state().progress.xp, 50);
        // New tasks don't reuse yesterday's ids.
        assert!(rig.daemon.state().next_id > 2);
    }

    #[test]
    fn small_work_carries_over_at_six_am_while_running() {
        let mut rig = Rig::new();
        let item = |id, done| monk_core::planner::SmallItem {
            id,
            date: t0().date_naive(),
            title: format!("Email {id}"),
            done,
            carried_from_date: None,
        };
        rig.ui(Message::UpdateSmallItems(vec![
            item(1, false),
            item(2, true),
        ]))
        .unwrap();
        rig.utc = Utc.with_ymd_and_hms(2026, 11, 6, 5, 59, 59).unwrap();
        rig.run(1);
        assert_eq!(rig.daemon.state().small_items.len(), 1);
        rig.restart();
        assert_eq!(rig.daemon.state().small_items[0].id, 1);
        assert_eq!(
            rig.daemon.state().small_items[0].carried_from_date,
            Some(t0().date_naive())
        );
    }

    #[test]
    fn the_morning_offers_yesterdays_unfinished_tasks_and_waits_for_a_plan() {
        let mut rig = Rig::new();
        rig.task("Open");
        rig.task("Finished");
        let mut tasks = rig.daemon.state().tasks.clone();
        tasks[1].done = true;
        rig.ui(Message::UpdateTasks(tasks)).unwrap();
        assert!(rig.ui(Message::PlanDay).is_ok());

        // The service is off at 06:00 and starts later that morning.
        rig.utc = Utc.with_ymd_and_hms(2026, 11, 6, 9, 0, 0).unwrap();
        rig.restart();
        let state = rig.daemon.state();
        assert!(!state.planned);
        assert_eq!(state.leftovers.len(), 1);
        let id = state.leftovers[0].id;
        assert_eq!(
            rig.ui(Message::PlanDay).unwrap_err().0,
            ErrorCode::InvalidRequest
        );

        let carry = LeftoverAction::Carry;
        rig.ui(Message::ResolveLeftover {
            task_id: id,
            action: carry,
        })
        .unwrap();
        rig.ui(Message::PlanDay).unwrap();
        rig.restart();
        let state = rig.daemon.state();
        assert!(state.planned);
        assert!(state.leftovers.is_empty());
        assert_eq!(state.tasks[0].title, "Open");
        assert_eq!(state.tasks[0].carried_from_date, Some(t0().date_naive()));
    }

    #[test]
    fn moving_a_leftover_onto_a_full_day_is_refused() {
        let mut rig = Rig::new();
        rig.task("Leftover");
        rig.utc = Utc.with_ymd_and_hms(2026, 11, 6, 9, 0, 0).unwrap();
        rig.restart();
        let id = rig.daemon.state().leftovers[0].id;
        // Fill Nov 8 in the database directly.
        let later = chrono::NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
        let full: Vec<Task> = (50..54)
            .map(|id| Task {
                id,
                title: format!("T{id}"),
                priority: Priority::Low,
                done: false,
                done_on: None,
                allocated_min: 0,
                subtasks: vec![],
                focus_area: None,
                carried_from_date: None,
            })
            .collect();
        rig.daemon.db.save_tasks(later, &full).unwrap();
        let action = LeftoverAction::Move { date: later };
        let error = rig
            .ui(Message::ResolveLeftover {
                task_id: id,
                action,
            })
            .unwrap_err();
        assert!(error.1.contains("at most 4"), "{}", error.1);
        assert_eq!(rig.daemon.state().leftovers.len(), 1);
    }

    #[test]
    fn the_service_refuses_a_fifth_task() {
        let mut rig = Rig::new();
        for title in ["A", "B", "C", "D"] {
            rig.task(title);
        }
        let mut tasks = rig.daemon.state().tasks.clone();
        let mut fifth = tasks[0].clone();
        fifth.id = 99;
        tasks.push(fifth);
        let error = rig.ui(Message::UpdateTasks(tasks)).unwrap_err();
        assert_eq!(error.0, ErrorCode::InvalidRequest);
        assert_eq!(rig.daemon.state().tasks.len(), 4);
    }

    #[test]
    fn the_extension_cannot_change_tasks() {
        let mut rig = Rig::new();
        let result = rig.ext(1, "chrome.exe", Message::UpdateTasks(vec![]));
        assert_eq!(result.unwrap_err().0, ErrorCode::Unauthorized);
    }

    // Pushes (T3.6)

    #[test]
    fn pushes_only_when_something_changed() {
        let mut rig = Rig::new();
        rig.run(1);
        let first = rig.daemon.push_if_changed().unwrap();
        let parsed: monk_core::protocol::Push = serde_json::from_str(&first).unwrap();
        let monk_core::protocol::Push::StateChanged(snapshot) = parsed;
        assert_eq!(snapshot.state.settings, Settings::default());

        rig.run(1);
        assert_eq!(rig.daemon.push_if_changed(), None);
        rig.tab(1, true);
        rig.run(1);
        assert!(rig.daemon.push_if_changed().is_some());
    }

    // Extension-missing rule (T3.7)

    fn with_browser(browser: &str) -> Rig {
        let rig = Rig::new();
        rig.fake.0.lock().unwrap().browsers = vec![browser.to_string()];
        rig
    }

    #[test]
    fn no_browser_running_means_no_block() {
        let mut rig = Rig::new();
        rig.run(60);
        assert!(!rig.hosts_has("youtube.com"));
    }

    #[test]
    fn a_browser_without_the_extension_blocks_only_after_30_seconds() {
        let mut rig = with_browser("chrome.exe");
        rig.run(30);
        assert!(!rig.hosts_has("youtube.com"));
        rig.run(10);
        assert!(rig.hosts_has("youtube.com"));
    }

    #[test]
    fn heartbeats_keep_the_sites_open() {
        let mut rig = with_browser("chrome.exe");
        for _ in 0..30 {
            rig.ext(1, "chrome.exe", Message::ExtHeartbeat).unwrap();
            rig.run(5);
        }
        assert!(!rig.hosts_has("youtube.com"));
    }

    #[test]
    fn the_extension_coming_back_lifts_the_block() {
        let mut rig = with_browser("chrome.exe");
        rig.run(40);
        assert!(rig.hosts_has("youtube.com"));
        rig.daemon.ext_connected("chrome.exe");
        rig.run(5);
        assert!(!rig.hosts_has("youtube.com"));
    }

    #[test]
    fn the_extension_vanishing_blocks_after_the_grace_period() {
        let mut rig = with_browser("chrome.exe");
        rig.daemon.ext_connected("chrome.exe");
        rig.run(10);
        assert!(!rig.hosts_has("youtube.com"));
        rig.run(60);
        assert!(rig.hosts_has("youtube.com"));
    }

    #[test]
    fn snapshots_list_connected_and_missing_browsers() {
        let mut rig = with_browser("chrome.exe");
        rig.fake
            .0
            .lock()
            .unwrap()
            .browsers
            .push("msedge.exe".to_string());
        rig.daemon.ext_connected("chrome.exe");
        rig.run(1);
        let snapshot: monk_core::protocol::Snapshot =
            serde_json::from_value(rig.daemon.state_json()).unwrap();
        assert_eq!(snapshot.connected_browsers, ["chrome.exe"]);
        assert_eq!(snapshot.missing_browsers, ["msedge.exe"]);
    }

    #[test]
    fn chrome_with_the_extension_does_not_excuse_edge_without_it() {
        let mut rig = with_browser("chrome.exe");
        rig.fake
            .0
            .lock()
            .unwrap()
            .browsers
            .push("msedge.exe".to_string());
        for _ in 0..10 {
            rig.ext(1, "chrome.exe", Message::ExtHeartbeat).unwrap();
            rig.run(5);
        }
        assert!(rig.hosts_has("youtube.com"));
    }

    // Lock copies (F5)

    fn locked_end(rig: &Rig) -> Option<DateTime<Utc>> {
        rig.daemon.state().lock.as_ref().map(|l| l.ends_at)
    }

    #[test]
    fn starting_a_lock_writes_both_copies() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        assert!(rig.daemon.is_locked());
        let end = locked_end(&rig);
        assert_eq!(rig.fake.0.lock().unwrap().lock_copy, end);
        rig.restart();
        assert_eq!(locked_end(&rig), end);
    }

    #[test]
    fn deleting_the_registry_copy_does_not_unlock_and_is_repaired() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        let end = locked_end(&rig);
        rig.fake.0.lock().unwrap().lock_copy = None;
        rig.restart();
        assert_eq!(locked_end(&rig), end);
        assert_eq!(rig.fake.0.lock().unwrap().lock_copy, end);
    }

    #[test]
    fn a_tampered_registry_copy_does_not_unlock_and_is_repaired() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        let end = locked_end(&rig);
        rig.fake.0.lock().unwrap().lock_copy_tampered = true;
        rig.restart();
        assert_eq!(locked_end(&rig), end);
        assert_eq!(rig.fake.0.lock().unwrap().lock_copy, end);
    }

    #[test]
    fn deleting_the_database_does_not_unlock_and_is_repaired() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        let end = locked_end(&rig);
        // A fresh data directory stands in for a deleted database.
        rig.dir = tempfile::tempdir().unwrap();
        let db = Db::open(rig.dir.path()).unwrap();
        rig.daemon = Daemon::new(db, rig.fake.clone(), utc_now, rig.utc);
        assert_eq!(locked_end(&rig), end);
        rig.restart();
        assert_eq!(locked_end(&rig), end);
    }

    #[test]
    fn an_edited_database_date_loses_to_the_signed_copy() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        let end = locked_end(&rig);
        let edited = Lock {
            created_at: rig.utc,
            ends_at: rig.utc + Duration::days(1),
        };
        Db::open(rig.dir.path())
            .unwrap()
            .save_lock(&edited, "forged")
            .unwrap();
        rig.restart();
        assert_eq!(locked_end(&rig), end);
    }

    #[test]
    fn the_lock_ends_on_time_without_being_touched() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        rig.utc += Duration::days(90);
        rig.run(1);
        assert!(!rig.daemon.is_locked());
        rig.ui(Message::UpdateSettings(limits(40))).unwrap();
    }

    #[test]
    fn turning_the_clock_back_does_not_end_a_lock_early_or_unlock_settings() {
        let mut rig = Rig::new();
        rig.lock_for_90_days();
        rig.utc += Duration::days(89);
        rig.run(1);
        // A month back, then past the "end": the ratchet held at day 89, so a day still remains.
        rig.utc -= Duration::days(30);
        rig.run(1);
        assert!(rig.daemon.is_locked());
        let result = rig.ui(Message::UpdateSettings(limits(40)));
        assert_eq!(result.unwrap_err().0, ErrorCode::LockedLoosening);
    }
}
