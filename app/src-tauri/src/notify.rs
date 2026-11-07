//! Which notifications a change of state calls for (F1, F7, F11).

use monk_core::models::{AllowanceSite, PomodoroState, State};

use crate::sound::Chime;

const TWO_MIN_SEC: u32 = 120;
const THIRTY_SEC: u32 = 30;
/// Focus going straight to idle this close to the end ran out, rather than being stopped. The
/// service pushes every second, so the last push before the end can show 1 or 2 seconds.
const RAN_OUT_SEC: u32 = 2;

fn left_sec(state: &State, site: &AllowanceSite) -> u32 {
    (site.daily_min * 60).saturating_sub(state.today.site_used(&site.domain))
}

/// The tray's read-only line: the site in use that's closest to its limit.
pub fn allowance_text(state: &State) -> String {
    let closest = state
        .settings
        .allowance_sites
        .iter()
        .filter(|site| state.today.site_used(&site.domain) > 0)
        .min_by_key(|site| left_sec(state, site));
    let Some(site) = closest else {
        return "No limits used today".to_string();
    };
    let left = left_sec(state, site);
    let domain = &site.domain;
    match left {
        0 => format!("{domain}: closed today"),
        1..=59 => format!("{domain}: {left} seconds left"),
        60..=119 => format!("{domain}: 1 minute left"),
        _ => format!("{domain}: {} minutes left", left / 60),
    }
}

/// The task a one-click start from the tray counts towards: the one last focused on, if it's still
/// open, so a break doesn't move you off it. Otherwise the most important unfinished one. With
/// none, the session runs without a task.
pub fn next_task(state: &State, last_focus: Option<&str>) -> Option<u32> {
    let open = || state.tasks.iter().filter(|task| !task.done);
    open()
        .find(|task| Some(task.title.as_str()) == last_focus)
        .or_else(|| open().min_by_key(|task| task.priority))
        .map(|task| task.id)
}

/// Time left in words: whole minutes, rounded up, then seconds for the last minute.
fn time_left(seconds: u32) -> String {
    if seconds < 60 {
        format!("{seconds} sec left")
    } else {
        format!("{} min left", seconds.div_ceil(60))
    }
}

/// The tray tooltip's line while a session runs, like "Focus: 49 min left".
pub fn timer_text(pomodoro: &PomodoroState) -> Option<String> {
    match pomodoro {
        PomodoroState::Idle => None,
        PomodoroState::Focus { paused: true, .. } => Some("Focus: paused".to_string()),
        PomodoroState::Focus { remaining_sec, .. } => {
            Some(format!("Focus: {}", time_left(*remaining_sec)))
        }
        PomodoroState::Break { remaining_sec } => {
            Some(format!("Break: {}", time_left(*remaining_sec)))
        }
    }
}

/// The sound for a session that ran to its end. Stopping one by hand plays nothing.
pub fn chime(previous: &State, current: &State) -> Option<Chime> {
    match (&previous.pomodoro, &current.pomodoro) {
        (PomodoroState::Focus { .. }, PomodoroState::Break { .. }) => Some(Chime::FocusDone),
        (PomodoroState::Focus { remaining_sec, .. }, PomodoroState::Idle)
            if *remaining_sec <= RAN_OUT_SEC =>
        {
            Some(Chime::FocusDone)
        }
        (PomodoroState::Break { remaining_sec }, PomodoroState::Idle)
            if *remaining_sec <= RAN_OUT_SEC =>
        {
            Some(Chime::BreakDone)
        }
        _ => None,
    }
}

/// (title, body) for each notification to show.
pub fn notices(previous: &State, current: &State) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut notice = |title: String, body: &str| out.push((title, body.to_string()));

    if previous.today.date == current.today.date {
        for site in &current.settings.allowance_sites {
            let domain = &site.domain;
            let before = left_sec(previous, site);
            let after = left_sec(current, site);
            if before > TWO_MIN_SEC && after <= TWO_MIN_SEC && after > THIRTY_SEC {
                notice(
                    format!("{domain}: 2 minutes left"),
                    "That's nearly it for today.",
                );
            }
            if before > THIRTY_SEC && after <= THIRTY_SEC && after > 0 {
                notice(format!("{domain}: 30 seconds left"), "Time to wrap up.");
            }
            if before > 0 && after == 0 {
                notice(format!("{domain} is closed"), "It opens again at midnight.");
            }
        }
    }

    match chime(previous, current) {
        Some(Chime::FocusDone) if matches!(current.pomodoro, PomodoroState::Break { .. }) => {
            notice("Focus session done".to_string(), "Take a break.");
        }
        Some(Chime::FocusDone) => notice("Focus session done".to_string(), "Well done."),
        Some(Chime::BreakDone) => notice("Break is over".to_string(), "Back to work."),
        None => {}
    }

    // Level 0 is the state before the service's first tick, not a real level.
    let (old_level, new_level) = (previous.progress.level, current.progress.level);
    if old_level > 0 && new_level > old_level {
        notice(format!("Level {new_level}"), "Keep going.");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use monk_core::models::{Priority, Task};

    const REDDIT: &str = "reddit.com";

    /// Reddit has a 10-minute limit by default.
    fn state(used: u32) -> State {
        let mut s = State::new(NaiveDate::from_ymd_opt(2026, 11, 5).unwrap());
        s.today.site_used_sec.insert(REDDIT.to_string(), used);
        s
    }

    fn titles(previous: &State, current: &State) -> Vec<String> {
        notices(previous, current)
            .into_iter()
            .map(|n| n.0)
            .collect()
    }

    #[test]
    fn warns_once_as_each_threshold_is_crossed() {
        assert_eq!(
            titles(&state(479), &state(480)),
            ["reddit.com: 2 minutes left"]
        );
        assert!(titles(&state(480), &state(481)).is_empty());
        assert_eq!(
            titles(&state(569), &state(570)),
            ["reddit.com: 30 seconds left"]
        );
        assert_eq!(titles(&state(599), &state(600)), ["reddit.com is closed"]);
        assert!(titles(&state(600), &state(600)).is_empty());
    }

    #[test]
    fn a_big_jump_only_shows_the_latest_news() {
        assert_eq!(titles(&state(0), &state(600)), ["reddit.com is closed"]);
        assert_eq!(
            titles(&state(0), &state(580)),
            ["reddit.com: 30 seconds left"]
        );
    }

    #[test]
    fn a_new_day_is_not_a_warning() {
        let previous = state(0);
        let mut current = state(600);
        current.today.date = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        assert!(titles(&previous, &current).is_empty());
    }

    #[test]
    fn a_new_level_is_announced_but_the_first_answer_is_not() {
        let mut previous = state(0);
        let mut current = state(0);
        current.progress.level = 3;
        assert!(titles(&previous, &current).is_empty());
        previous.progress.level = 2;
        assert_eq!(titles(&previous, &current), ["Level 3"]);
        previous.progress.level = 3;
        assert!(titles(&previous, &current).is_empty());
    }

    fn focus() -> PomodoroState {
        PomodoroState::Focus {
            task_title: "React".to_string(),
            started_at: Utc.with_ymd_and_hms(2026, 11, 5, 9, 0, 0).unwrap(),
            remaining_sec: 1,
            paused: false,
        }
    }

    #[test]
    fn pomodoro_end_and_break_end_notify_but_stopping_does_not() {
        let mut previous = state(0);
        let mut current = state(0);
        previous.pomodoro = focus();
        current.pomodoro = PomodoroState::Break { remaining_sec: 600 };
        assert_eq!(titles(&previous, &current), ["Focus session done"]);

        previous.pomodoro = PomodoroState::Break { remaining_sec: 1 };
        current.pomodoro = PomodoroState::Idle;
        assert_eq!(titles(&previous, &current), ["Break is over"]);

        previous.pomodoro = focus_left(600);
        assert!(titles(&previous, &current).is_empty());
    }

    fn chimes(previous: PomodoroState, current: PomodoroState) -> Option<Chime> {
        let (mut before, mut after) = (state(0), state(0));
        before.pomodoro = previous;
        after.pomodoro = current;
        chime(&before, &after)
    }

    fn focus_left(remaining_sec: u32) -> PomodoroState {
        PomodoroState::Focus {
            task_title: "React".to_string(),
            started_at: Utc.with_ymd_and_hms(2026, 11, 5, 9, 0, 0).unwrap(),
            remaining_sec,
            paused: false,
        }
    }

    #[test]
    fn a_chime_plays_when_focus_or_a_break_runs_out() {
        let on_break = PomodoroState::Break { remaining_sec: 600 };
        assert_eq!(chimes(focus_left(1), on_break), Some(Chime::FocusDone));
        // With no break set, focus goes straight to idle.
        assert_eq!(
            chimes(focus_left(1), PomodoroState::Idle),
            Some(Chime::FocusDone)
        );
        assert_eq!(
            chimes(
                PomodoroState::Break { remaining_sec: 1 },
                PomodoroState::Idle
            ),
            Some(Chime::BreakDone)
        );
    }

    #[test]
    fn stopping_early_or_skipping_a_break_is_silent() {
        assert_eq!(chimes(focus_left(1200), PomodoroState::Idle), None);
        assert_eq!(
            chimes(
                PomodoroState::Break { remaining_sec: 300 },
                PomodoroState::Idle
            ),
            None
        );
        assert_eq!(chimes(PomodoroState::Idle, focus_left(3000)), None);
        assert_eq!(chimes(focus_left(10), focus_left(9)), None);
    }

    #[test]
    fn a_session_with_no_break_also_says_it_is_done() {
        let mut previous = state(0);
        previous.pomodoro = focus_left(1);
        assert_eq!(titles(&previous, &state(0)), ["Focus session done"]);
    }

    #[test]
    fn timer_text_counts_down_in_minutes_then_seconds() {
        assert_eq!(timer_text(&PomodoroState::Idle), None);
        assert_eq!(
            timer_text(&focus_left(49 * 60)).as_deref(),
            Some("Focus: 49 min left")
        );
        // A part-started minute still counts, so it never says 0 min.
        assert_eq!(
            timer_text(&focus_left(49 * 60 - 30)).as_deref(),
            Some("Focus: 49 min left")
        );
        assert_eq!(
            timer_text(&focus_left(60)).as_deref(),
            Some("Focus: 1 min left")
        );
        assert_eq!(
            timer_text(&PomodoroState::Break { remaining_sec: 45 }).as_deref(),
            Some("Break: 45 sec left")
        );
    }

    #[test]
    fn allowance_text_reads_naturally() {
        assert_eq!(
            allowance_text(&State::new(NaiveDate::MIN)),
            "No limits used today"
        );
        assert_eq!(allowance_text(&state(1)), "reddit.com: 9 minutes left");
        assert_eq!(allowance_text(&state(500)), "reddit.com: 1 minute left");
        assert_eq!(allowance_text(&state(560)), "reddit.com: 40 seconds left");
        assert_eq!(allowance_text(&state(600)), "reddit.com: closed today");
    }

    #[test]
    fn next_task_is_the_most_important_unfinished_one() {
        let task = |id, priority, done| Task {
            id,
            title: "x".to_string(),
            priority,
            done,
            done_on: None,
            allocated_min: 0,
            subtasks: vec![],
        };
        let mut s = state(0);
        assert_eq!(next_task(&s, None), None);
        s.tasks = vec![
            task(1, Priority::Low, false),
            task(2, Priority::High, true),
            task(3, Priority::Medium, false),
        ];
        assert_eq!(next_task(&s, None), Some(3));
        s.tasks[1].done = false;
        assert_eq!(next_task(&s, None), Some(2));
    }

    #[test]
    fn next_task_stays_on_the_task_last_focused_on() {
        let task = |id, title: &str, priority, done| Task {
            id,
            title: title.to_string(),
            priority,
            done,
            done_on: None,
            allocated_min: 0,
            subtasks: vec![],
        };
        let mut s = state(0);
        s.tasks = vec![
            task(1, "Urgent thing", Priority::High, false),
            task(2, "Building project", Priority::Low, false),
        ];
        assert_eq!(next_task(&s, Some("Building project")), Some(2));
        // Once it's finished, or for a free session, the most important comes next.
        s.tasks[1].done = true;
        assert_eq!(next_task(&s, Some("Building project")), Some(1));
        assert_eq!(next_task(&s, Some("Free focus")), Some(1));
    }
}
