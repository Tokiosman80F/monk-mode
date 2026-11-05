//! Which notifications a change of state calls for (F1, F7, F11).

use monk_core::models::{AllowanceSite, PomodoroState, State};

const TWO_MIN_SEC: u32 = 120;
const THIRTY_SEC: u32 = 30;

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

/// The task a one-click start from the tray counts towards: the most important unfinished one.
/// With none, the session runs without a task.
pub fn next_task(state: &State) -> Option<u32> {
    state
        .tasks
        .iter()
        .filter(|task| !task.done)
        .min_by_key(|task| task.priority)
        .map(|task| task.id)
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

    match (&previous.pomodoro, &current.pomodoro) {
        // Focus to idle with no break to follow is also a finish, but so is pressing Stop.
        (PomodoroState::Focus { .. }, PomodoroState::Break { .. }) => {
            notice("Focus session done".to_string(), "Take a break.");
        }
        (PomodoroState::Break { .. }, PomodoroState::Idle) => {
            notice("Break is over".to_string(), "Back to work.");
        }
        _ => {}
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

        previous.pomodoro = focus();
        assert!(titles(&previous, &current).is_empty());
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
            subtasks: vec![],
        };
        let mut s = state(0);
        assert_eq!(next_task(&s), None);
        s.tasks = vec![
            task(1, Priority::Low, false),
            task(2, Priority::High, true),
            task(3, Priority::Medium, false),
        ];
        assert_eq!(next_task(&s), Some(3));
        s.tasks[1].done = false;
        assert_eq!(next_task(&s), Some(2));
    }
}
