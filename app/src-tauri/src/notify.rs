//! Which notifications a change of state calls for (F1, F7).

use monk_core::models::{PomodoroState, State};

const TWO_MIN_SEC: u32 = 120;
const THIRTY_SEC: u32 = 30;

pub fn allowance_left_sec(state: &State) -> u32 {
    (state.settings.daily_allowance_min * 60).saturating_sub(state.today.allowance_used_sec)
}

pub fn allowance_text(state: &State) -> String {
    let left = allowance_left_sec(state);
    match left {
        0 => "No allowance left today".to_string(),
        1..=59 => format!("{left} seconds left today"),
        60..=119 => "1 minute left today".to_string(),
        _ => format!("{} minutes left today", left / 60),
    }
}

/// The study block a one-click start from the tray counts towards: the first one not finished.
pub fn next_block(state: &State) -> Option<String> {
    let blocks = &state.settings.study_blocks;
    let unfinished = blocks.iter().find(|block| {
        let done = state.today.focus_sec.get(&block.name).copied().unwrap_or(0);
        done < block.target_min * 60
    });
    unfinished.or(blocks.first()).map(|b| b.name.clone())
}

/// (title, body) for each notification to show.
pub fn notices(previous: &State, current: &State) -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();

    if previous.today.date == current.today.date {
        let before = allowance_left_sec(previous);
        let after = allowance_left_sec(current);
        if before > TWO_MIN_SEC && after <= TWO_MIN_SEC && after > THIRTY_SEC {
            out.push(("2 minutes left", "That's nearly it for today."));
        }
        if before > THIRTY_SEC && after <= THIRTY_SEC && after > 0 {
            out.push(("30 seconds left", "Time to wrap up."));
        }
        if before > 0 && after == 0 {
            out.push((
                "Allowance used up",
                "Those sites are closed until midnight.",
            ));
        }
    }

    match (&previous.pomodoro, &current.pomodoro) {
        // Focus to idle with no break to follow is also a finish, but so is pressing Stop.
        (PomodoroState::Focus { .. }, PomodoroState::Break { .. }) => {
            out.push(("Focus session done", "Take a break."));
        }
        (PomodoroState::Break { .. }, PomodoroState::Idle) => {
            out.push(("Break is over", "Back to work."));
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};

    fn state(used: u32) -> State {
        let mut s = State::new(NaiveDate::from_ymd_opt(2026, 11, 5).unwrap());
        s.today.allowance_used_sec = used;
        s
    }

    fn titles(previous: &State, current: &State) -> Vec<&'static str> {
        notices(previous, current).iter().map(|n| n.0).collect()
    }

    #[test]
    fn warns_once_as_each_threshold_is_crossed() {
        assert_eq!(titles(&state(479), &state(480)), ["2 minutes left"]);
        assert_eq!(titles(&state(480), &state(481)), Vec::<&str>::new());
        assert_eq!(titles(&state(569), &state(570)), ["30 seconds left"]);
        assert_eq!(titles(&state(599), &state(600)), ["Allowance used up"]);
        assert_eq!(titles(&state(600), &state(600)), Vec::<&str>::new());
    }

    #[test]
    fn a_big_jump_only_shows_the_latest_news() {
        assert_eq!(titles(&state(0), &state(600)), ["Allowance used up"]);
        assert_eq!(titles(&state(0), &state(580)), ["30 seconds left"]);
    }

    #[test]
    fn a_new_day_is_not_a_warning() {
        let previous = state(0);
        let mut current = state(600);
        current.today.date = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        assert_eq!(titles(&previous, &current), Vec::<&str>::new());
    }

    fn focus() -> PomodoroState {
        PomodoroState::Focus {
            block_name: "React".to_string(),
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
        assert_eq!(titles(&previous, &current), Vec::<&str>::new());
    }

    #[test]
    fn allowance_text_reads_naturally() {
        assert_eq!(allowance_text(&state(0)), "10 minutes left today");
        assert_eq!(allowance_text(&state(500)), "1 minute left today");
        assert_eq!(allowance_text(&state(560)), "40 seconds left today");
        assert_eq!(allowance_text(&state(600)), "No allowance left today");
    }

    #[test]
    fn next_block_is_the_first_unfinished_one() {
        let mut s = state(0);
        assert_eq!(next_block(&s).as_deref(), Some("React"));
        s.today.focus_sec.insert("React".to_string(), 3 * 3600);
        assert_eq!(next_block(&s).as_deref(), Some("Building"));
        s.today.focus_sec.insert("Building".to_string(), 6 * 3600);
        s.today
            .focus_sec
            .insert("LinkedIn course".to_string(), 2 * 3600);
        assert_eq!(next_block(&s).as_deref(), Some("React"));
        s.settings.study_blocks.clear();
        assert_eq!(next_block(&s), None);
    }
}
