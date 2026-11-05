use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::domain::{expand_subdomains, url_matches_domain};
use crate::lock::lock_day;
use crate::models::{AllowanceSite, FocusKind, FocusSession, Now, PomodoroState, State, UsageDay};
use crate::progress::progress;
use crate::stats::streak;
use crate::tasks::prune_tasks;

const WARN_2MIN_SEC: u32 = 120;
const WARN_30S_SEC: u32 = 30;
/// A session started with no task picked, such as from the tray with an empty list.
pub const FREE_FOCUS: &str = "Free focus";

const SHORTS_PATTERNS: [&str; 5] = [
    "||youtube.com/shorts/",
    "||instagram.com/reels/",
    "||instagram.com/reel/",
    "||facebook.com/reel/",
    "||facebook.com/watch",
];

/// What the extension reports about the tab the user is looking at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ActiveTab {
    pub url: String,
    pub focused: bool,
    pub visible: bool,
    pub incognito: bool,
    /// From `chrome.idle`: the user is away or the PC is locked.
    pub idle: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Warn2Min,
    Warn30s,
    AllowanceUsedUp,
    DayRolled(UsageDay),
    FocusEnded(FocusSession),
    BreakEnded,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BlockedSet {
    pub hosts_domains: Vec<String>,
    /// `declarativeNetRequest` URL filters.
    pub url_patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PomodoroError {
    #[error("A session is already running.")]
    AlreadyRunning,
    #[error("That task no longer exists.")]
    UnknownTask,
}

/// Advances the state by `elapsed_sec`. Call once per service loop.
pub fn tick(
    state: &mut State,
    elapsed_sec: u32,
    now: Now,
    active_tab: Option<&ActiveTab>,
) -> Vec<Event> {
    let mut events = Vec::new();
    roll_over(state, now, &mut events);
    tick_pomodoro(state, elapsed_sec, now, &mut events);
    tick_allowance(state, elapsed_sec, active_tab, &mut events);
    state.streak_days = streak(
        &state.history,
        &state.today,
        state.settings.daily_focus_target_min,
    );
    state.progress = progress(
        &state.history,
        &state.today,
        state.settings.daily_focus_target_min,
    );
    state.lock_day = state.lock.as_ref().and_then(|l| lock_day(l, now.utc));
    events
}

/// Only ever moves forward, so a time zone change can't hand back a used-up allowance.
fn roll_over(state: &mut State, now: Now, events: &mut Vec<Event>) {
    if now.local_date > state.today.date {
        let finished = std::mem::replace(&mut state.today, UsageDay::new(now.local_date));
        state.history.push(finished.clone());
        prune_tasks(&mut state.tasks, now.local_date);
        events.push(Event::DayRolled(finished));
    }
}

fn is_focusing(state: &State) -> bool {
    matches!(state.pomodoro, PomodoroState::Focus { .. })
}

/// A tab counts towards every limit it matches, so adding `m.example.com` with a bigger limit
/// can't get around the limit on `example.com`.
fn tick_allowance(
    state: &mut State,
    elapsed_sec: u32,
    tab: Option<&ActiveTab>,
    events: &mut Vec<Event>,
) {
    let Some(tab) = tab else { return };
    if !tab.focused || !tab.visible || tab.idle || is_focusing(state) {
        return;
    }

    let mut counted = 0;
    for site in &state.settings.allowance_sites {
        if !url_matches_domain(&tab.url, &site.domain) {
            continue;
        }
        let used = state
            .today
            .site_used_sec
            .entry(site.domain.clone())
            .or_default();
        let before = (site.daily_min * 60).saturating_sub(*used);
        if before == 0 {
            continue;
        }
        let added = elapsed_sec.min(before);
        *used += added;
        counted = counted.max(added);
        let after = before - added;

        if before > WARN_2MIN_SEC && after <= WARN_2MIN_SEC {
            events.push(Event::Warn2Min);
        }
        if before > WARN_30S_SEC && after <= WARN_30S_SEC {
            events.push(Event::Warn30s);
        }
        if after == 0 {
            events.push(Event::AllowanceUsedUp);
        }
    }
    state.today.allowance_used_sec += counted;
}

/// `task_id` of `None` starts a session with no task.
pub fn start_pomodoro(
    state: &mut State,
    task_id: Option<u32>,
    now: Now,
) -> Result<(), PomodoroError> {
    if state.pomodoro != PomodoroState::Idle {
        return Err(PomodoroError::AlreadyRunning);
    }
    let task_title = match task_id {
        Some(id) => state
            .tasks
            .iter()
            .find(|t| t.id == id)
            .ok_or(PomodoroError::UnknownTask)?
            .title
            .clone(),
        None => FREE_FOCUS.to_string(),
    };
    state.pomodoro = PomodoroState::Focus {
        task_title,
        started_at: now.utc,
        remaining_sec: state.settings.pomodoro_focus_min * 60,
    };
    Ok(())
}

/// Stops any session. Returns the focus session to record, if one was running.
pub fn stop_pomodoro(state: &mut State, now: Now) -> Option<FocusSession> {
    match std::mem::replace(&mut state.pomodoro, PomodoroState::Idle) {
        PomodoroState::Focus {
            task_title,
            started_at,
            ..
        } => Some(FocusSession {
            id: None,
            task_title,
            started_at,
            ended_at: now.utc,
            kind: FocusKind::Pomodoro,
        }),
        _ => None,
    }
}

fn tick_pomodoro(state: &mut State, elapsed_sec: u32, now: Now, events: &mut Vec<Event>) {
    let break_sec = state.settings.pomodoro_break_min * 60;
    match &mut state.pomodoro {
        PomodoroState::Idle => {}
        PomodoroState::Focus {
            task_title,
            remaining_sec,
            ..
        } => {
            let counted = elapsed_sec.min(*remaining_sec);
            *remaining_sec -= counted;
            *state.today.focus_sec.entry(task_title.clone()).or_default() += counted;
            if *remaining_sec == 0 {
                if let Some(session) = stop_pomodoro(state, now) {
                    events.push(Event::FocusEnded(session));
                }
                if break_sec > 0 {
                    state.pomodoro = PomodoroState::Break {
                        remaining_sec: break_sec,
                    };
                }
            }
        }
        PomodoroState::Break { remaining_sec } => {
            *remaining_sec = remaining_sec.saturating_sub(elapsed_sec);
            if *remaining_sec == 0 {
                state.pomodoro = PomodoroState::Idle;
                events.push(Event::BreakEnded);
            }
        }
    }
}

/// Everything that must be blocked right now. `extension_connected` is false only when a
/// browser has been running without the extension for too long (F4).
pub fn compute_blocked(state: &State, now: Now, extension_connected: bool) -> BlockedSet {
    // A stored day that's already over has no bearing on today's allowance.
    let is_today = state.today.date == now.local_date;
    let used_up = |site: &AllowanceSite| {
        let used = if is_today {
            state.today.site_used(&site.domain)
        } else {
            0
        };
        used >= site.daily_min * 60
    };
    let block_all = is_focusing(state) || !extension_connected;
    let allowance = &state.settings.allowance_sites;

    let mut sites: Vec<&String> = state.settings.always_blocked.iter().collect();
    sites.extend(
        allowance
            .iter()
            // A site closes with its own limit, or with the limit of a site it's part of.
            .filter(|site| {
                block_all
                    || allowance.iter().any(|parent| {
                        used_up(parent) && url_matches_domain(&site.domain, &parent.domain)
                    })
            })
            .map(|site| &site.domain),
    );

    let mut hosts_domains: Vec<String> = sites.iter().flat_map(|d| expand_subdomains(d)).collect();
    hosts_domains.sort();
    hosts_domains.dedup();

    let mut url_patterns: Vec<String> = sites.iter().map(|d| format!("||{d}^")).collect();
    if state.settings.shorts_blocker_enabled {
        url_patterns.extend(SHORTS_PATTERNS.iter().map(|p| p.to_string()));
    }

    BlockedSet {
        hosts_domains,
        url_patterns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn at(y: i32, m: u32, d: u32, h: u32) -> Now {
        let utc = Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap();
        Now::new(utc, &Utc)
    }

    fn now() -> Now {
        at(2026, 11, 5, 12)
    }

    /// Every site on a 10-minute limit.
    fn state() -> State {
        let mut s = State::new(now().local_date);
        set_limits(&mut s, 10);
        s
    }

    fn set_limits(s: &mut State, minutes: u32) {
        for site in &mut s.settings.allowance_sites {
            site.daily_min = minutes;
        }
    }

    fn with_task(s: &mut State, title: &str) -> Option<u32> {
        let id = s.tasks.len() as u32 + 1;
        s.tasks.push(crate::models::Task {
            id,
            title: title.to_string(),
            priority: crate::models::Priority::Medium,
            done: false,
            done_on: None,
            subtasks: vec![],
        });
        Some(id)
    }

    fn tab(url: &str) -> ActiveTab {
        ActiveTab {
            url: url.to_string(),
            focused: true,
            visible: true,
            incognito: false,
            idle: false,
        }
    }

    fn youtube() -> ActiveTab {
        tab("https://www.youtube.com/watch?v=1")
    }

    // Allowance

    #[test]
    fn focused_allowance_site_counts() {
        let mut s = state();
        tick(&mut s, 5, now(), Some(&youtube()));
        assert_eq!(s.today.allowance_used_sec, 5);
    }

    #[test]
    fn multiple_ticks_add_up() {
        let mut s = state();
        for _ in 0..3 {
            tick(&mut s, 2, now(), Some(&youtube()));
        }
        assert_eq!(s.today.allowance_used_sec, 6);
    }

    #[test]
    fn background_or_unfocused_tab_does_not_count() {
        let mut s = state();
        let hidden = ActiveTab {
            visible: false,
            ..youtube()
        };
        let unfocused = ActiveTab {
            focused: false,
            ..youtube()
        };
        tick(&mut s, 5, now(), Some(&hidden));
        tick(&mut s, 5, now(), Some(&unfocused));
        tick(&mut s, 5, now(), None);
        assert_eq!(s.today.allowance_used_sec, 0);
    }

    #[test]
    fn idle_does_not_count() {
        let mut s = state();
        let idle = ActiveTab {
            idle: true,
            ..youtube()
        };
        tick(&mut s, 5, now(), Some(&idle));
        assert_eq!(s.today.allowance_used_sec, 0);
    }

    #[test]
    fn non_allowance_site_does_not_count() {
        let mut s = state();
        tick(&mut s, 5, now(), Some(&tab("https://react.dev/learn")));
        assert_eq!(s.today.allowance_used_sec, 0);
    }

    #[test]
    fn nothing_counts_during_a_focus_session_but_breaks_do() {
        let mut s = state();
        start_pomodoro(&mut s, None, now()).unwrap();
        tick(&mut s, 5, now(), Some(&youtube()));
        assert_eq!(s.today.allowance_used_sec, 0);

        s.pomodoro = PomodoroState::Break { remaining_sec: 60 };
        tick(&mut s, 5, now(), Some(&youtube()));
        assert_eq!(s.today.allowance_used_sec, 5);
    }

    #[test]
    fn warnings_fire_once_when_their_threshold_is_crossed() {
        let mut s = state(); // 600 s allowance
        assert!(tick(&mut s, 479, now(), Some(&youtube())).is_empty());
        assert_eq!(tick(&mut s, 1, now(), Some(&youtube())), [Event::Warn2Min]);
        assert!(tick(&mut s, 89, now(), Some(&youtube())).is_empty());
        assert_eq!(tick(&mut s, 1, now(), Some(&youtube())), [Event::Warn30s]);
        assert!(tick(&mut s, 29, now(), Some(&youtube())).is_empty());
        assert_eq!(
            tick(&mut s, 1, now(), Some(&youtube())),
            [Event::AllowanceUsedUp]
        );
    }

    #[test]
    fn used_up_stops_counting_and_stays_quiet() {
        let mut s = state();
        let events = tick(&mut s, 10_000, now(), Some(&youtube()));
        assert_eq!(
            events,
            [Event::Warn2Min, Event::Warn30s, Event::AllowanceUsedUp]
        );
        assert_eq!(s.today.allowance_used_sec, 600);
        assert!(tick(&mut s, 5, now(), Some(&youtube())).is_empty());
        assert_eq!(s.today.allowance_used_sec, 600);
    }

    // Day rollover

    #[test]
    fn midnight_resets_the_day_and_saves_history() {
        let mut s = state();
        tick(&mut s, 600, now(), Some(&youtube()));
        let events = tick(&mut s, 1, at(2026, 11, 6, 0), None);
        assert!(matches!(&events[..], [Event::DayRolled(d)] if d.allowance_used_sec == 600));
        assert_eq!(s.today, UsageDay::new(date(2026, 11, 6)));
        assert_eq!(s.history.len(), 1);
        assert_eq!(s.history[0].date, date(2026, 11, 5));
    }

    #[test]
    fn dst_changes_only_roll_at_local_midnight() {
        let edt = FixedOffset::west_opt(4 * 3600).unwrap();
        let est = FixedOffset::west_opt(5 * 3600).unwrap();
        let utc =
            |d, h, m| -> DateTime<Utc> { Utc.with_ymd_and_hms(2026, 11, d, h, m, 0).unwrap() };

        let mut s = State::new(date(2026, 11, 1));
        // New York, Nov 1 2026: 01:30 happens twice as the clocks go back.
        assert!(tick(&mut s, 1, Now::new(utc(1, 5, 30), &edt), None).is_empty());
        assert!(tick(&mut s, 1, Now::new(utc(1, 6, 30), &est), None).is_empty());
        // This day has 25 hours: 24 hours after midnight is still 23:00 on Nov 1.
        assert!(tick(&mut s, 1, Now::new(utc(2, 4, 0), &est), None).is_empty());
        assert_eq!(s.today.date, date(2026, 11, 1));
        // Real local midnight.
        let events = tick(&mut s, 1, Now::new(utc(2, 5, 0), &est), None);
        assert!(matches!(&events[..], [Event::DayRolled(_)]));
        assert_eq!(s.today.date, date(2026, 11, 2));
    }

    #[test]
    fn moving_to_an_earlier_date_does_not_reset_the_day() {
        let mut s = state();
        tick(&mut s, 600, now(), Some(&youtube()));
        assert!(tick(&mut s, 1, at(2026, 11, 4, 23), None).is_empty());
        assert_eq!(s.today.allowance_used_sec, 600);
    }

    // Pomodoro

    #[test]
    fn full_cycle_counts_focus_to_the_chosen_block() {
        let mut s = state();
        let task = with_task(&mut s, "React");
        start_pomodoro(&mut s, task, now()).unwrap();

        tick(&mut s, 50 * 60 - 1, now(), None);
        assert!(matches!(s.pomodoro, PomodoroState::Focus { .. }));

        let events = tick(&mut s, 1, now(), None);
        assert!(matches!(&events[..], [Event::FocusEnded(f)] if f.task_title == "React"));
        assert_eq!(
            s.pomodoro,
            PomodoroState::Break {
                remaining_sec: 10 * 60
            }
        );
        assert_eq!(s.today.focus_sec["React"], 50 * 60);

        assert_eq!(tick(&mut s, 10 * 60, now(), None), [Event::BreakEnded]);
        assert_eq!(s.pomodoro, PomodoroState::Idle);
    }

    #[test]
    fn zero_length_break_goes_straight_to_idle() {
        let mut s = state();
        s.settings.pomodoro_break_min = 0;
        let task = with_task(&mut s, "React");
        start_pomodoro(&mut s, task, now()).unwrap();
        tick(&mut s, 50 * 60, now(), None);
        assert_eq!(s.pomodoro, PomodoroState::Idle);
    }

    #[test]
    fn stopping_early_keeps_time_already_counted() {
        let mut s = state();
        let task = with_task(&mut s, "Building");
        start_pomodoro(&mut s, task, now()).unwrap();
        tick(&mut s, 300, now(), None);
        let session = stop_pomodoro(&mut s, now()).unwrap();
        assert_eq!(session.task_title, "Building");
        assert_eq!(s.pomodoro, PomodoroState::Idle);
        assert_eq!(s.today.focus_sec["Building"], 300);
        assert_eq!(stop_pomodoro(&mut s, now()), None);
    }

    #[test]
    fn focus_cannot_overcount_past_the_session_length() {
        let mut s = state();
        let task = with_task(&mut s, "React");
        start_pomodoro(&mut s, task, now()).unwrap();
        tick(&mut s, 100_000, now(), None);
        assert_eq!(s.today.focus_sec["React"], 50 * 60);
    }

    #[test]
    fn rollover_during_a_session_splits_focus_across_days() {
        let mut s = state();
        let task = with_task(&mut s, "React");
        start_pomodoro(&mut s, task, now()).unwrap();
        tick(&mut s, 60, now(), None);
        tick(&mut s, 60, at(2026, 11, 6, 0), None);
        assert_eq!(s.history[0].focus_sec["React"], 60);
        assert_eq!(s.today.focus_sec["React"], 60);
        assert!(matches!(s.pomodoro, PomodoroState::Focus { .. }));
    }

    #[test]
    fn start_rejects_a_second_session_and_unknown_tasks() {
        let mut s = state();
        assert_eq!(
            start_pomodoro(&mut s, Some(99), now()),
            Err(PomodoroError::UnknownTask)
        );
        let task = with_task(&mut s, "React");
        start_pomodoro(&mut s, task, now()).unwrap();
        assert_eq!(
            start_pomodoro(&mut s, task, now()),
            Err(PomodoroError::AlreadyRunning)
        );
    }

    #[test]
    fn a_session_can_start_with_no_task() {
        let mut s = state();
        start_pomodoro(&mut s, None, now()).unwrap();
        tick(&mut s, 60, now(), None);
        assert_eq!(s.today.focus_sec[FREE_FOCUS], 60);
    }

    #[test]
    fn a_new_day_clears_finished_tasks_only() {
        let mut s = state();
        with_task(&mut s, "Open");
        with_task(&mut s, "Finished");
        s.tasks[1].done = true;
        s.tasks[1].done_on = Some(now().local_date);
        tick(&mut s, 1, at(2026, 11, 6, 0), None);
        assert_eq!(s.tasks.len(), 1);
        assert_eq!(s.tasks[0].title, "Open");
    }

    // Per-site limits

    fn set_limit(s: &mut State, domain: &str, minutes: u32) {
        let site = s
            .settings
            .allowance_sites
            .iter_mut()
            .find(|site| site.domain == domain)
            .unwrap();
        site.daily_min = minutes;
    }

    #[test]
    fn each_site_runs_out_on_its_own() {
        let mut s = state();
        set_limit(&mut s, "youtube.com", 30);
        let instagram = tab("https://www.instagram.com/");
        tick(&mut s, 600, now(), Some(&instagram));
        assert_eq!(s.today.site_used("instagram.com"), 600);
        assert_eq!(s.today.site_used("youtube.com"), 0);

        let set = compute_blocked(&s, now(), true);
        assert!(has(&set, "instagram.com"));
        assert!(!has(&set, "youtube.com"));
        assert!(!has(&set, "facebook.com"));

        tick(&mut s, 1799, now(), Some(&youtube()));
        assert!(!has(&compute_blocked(&s, now(), true), "youtube.com"));
        tick(&mut s, 1, now(), Some(&youtube()));
        assert!(has(&compute_blocked(&s, now(), true), "youtube.com"));
    }

    #[test]
    fn a_subdomain_counts_towards_its_parent_and_closes_with_it() {
        let mut s = state();
        set_limit(&mut s, "music.youtube.com", 60);
        let music = tab("https://music.youtube.com/watch?v=1");
        tick(&mut s, 600, now(), Some(&music));
        assert_eq!(s.today.site_used("youtube.com"), 600);
        assert_eq!(s.today.site_used("music.youtube.com"), 600);
        // One tab, so the day total only counts it once.
        assert_eq!(s.today.allowance_used_sec, 600);

        let set = compute_blocked(&s, now(), true);
        assert!(has(&set, "youtube.com"));
        assert!(has(&set, "music.youtube.com"));
    }

    #[test]
    fn a_parent_stays_open_when_only_its_subdomain_runs_out() {
        let mut s = state();
        set_limit(&mut s, "music.youtube.com", 1);
        tick(&mut s, 60, now(), Some(&tab("https://music.youtube.com/")));
        let set = compute_blocked(&s, now(), true);
        assert!(has(&set, "music.youtube.com"));
        assert!(!has(&set, "youtube.com"));
    }

    // Blocked set

    fn has(set: &BlockedSet, domain: &str) -> bool {
        set.hosts_domains.iter().any(|d| d == domain)
    }

    #[test]
    fn always_blocked_and_shorts_are_always_present() {
        let set = compute_blocked(&state(), now(), true);
        assert!(has(&set, "tiktok.com"));
        assert!(has(&set, "www.tiktok.com"));
        assert!(has(&set, "tiktokv.com"));
        assert!(has(&set, "tiktokcdn.com"));
        assert!(set.url_patterns.iter().any(|p| p == "||tiktok.com^"));
        for p in SHORTS_PATTERNS {
            assert!(set.url_patterns.iter().any(|x| x == p), "{p}");
        }
    }

    #[test]
    fn allowance_sites_stay_open_while_allowance_is_left() {
        let mut s = state();
        tick(&mut s, 599, now(), Some(&youtube()));
        let set = compute_blocked(&s, now(), true);
        assert!(!has(&set, "youtube.com"));
        assert!(!set.url_patterns.iter().any(|p| p == "||youtube.com^"));
    }

    #[test]
    fn allowance_sites_block_when_used_up() {
        let mut s = state();
        tick(&mut s, 600, now(), Some(&youtube()));
        let set = compute_blocked(&s, now(), true);
        assert!(has(&set, "youtube.com"));
        assert!(has(&set, "m.youtube.com"));
        assert!(!has(&set, "open.spotify.com"));
        assert!(set.url_patterns.iter().any(|p| p == "||youtube.com^"));
    }

    #[test]
    fn zero_allowance_blocks_from_the_start() {
        let mut s = state();
        set_limits(&mut s, 0);
        assert!(has(&compute_blocked(&s, now(), true), "youtube.com"));
    }

    #[test]
    fn allowance_sites_block_during_focus_but_not_during_a_break() {
        let mut s = state();
        let task = with_task(&mut s, "React");
        start_pomodoro(&mut s, task, now()).unwrap();
        assert!(has(&compute_blocked(&s, now(), true), "youtube.com"));
        s.pomodoro = PomodoroState::Break { remaining_sec: 60 };
        assert!(!has(&compute_blocked(&s, now(), true), "youtube.com"));
    }

    #[test]
    fn allowance_sites_block_when_the_extension_is_missing() {
        assert!(has(&compute_blocked(&state(), now(), false), "youtube.com"));
    }

    #[test]
    fn yesterdays_used_allowance_does_not_block_a_new_day() {
        let mut s = state();
        tick(&mut s, 600, now(), Some(&youtube()));
        assert!(!has(
            &compute_blocked(&s, at(2026, 11, 6, 0), true),
            "youtube.com"
        ));
    }

    #[test]
    fn hosts_list_has_no_duplicates_and_shorts_can_be_turned_off() {
        let mut s = state();
        s.settings.allowance_sites.push(AllowanceSite {
            domain: "tiktok.com".to_string(),
            daily_min: 0,
        });
        set_limits(&mut s, 0);
        s.settings.shorts_blocker_enabled = false;
        let set = compute_blocked(&s, now(), true);
        let mut sorted = set.hosts_domains.clone();
        sorted.dedup();
        assert_eq!(sorted, set.hosts_domains);
        assert!(!set.url_patterns.iter().any(|p| p.contains("/shorts/")));
    }
}
