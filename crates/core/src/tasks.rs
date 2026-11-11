//! Today's tasks and the XP they earn (F7, F11).

use std::collections::HashSet;

use chrono::NaiveDate;
use thiserror::Error;

use crate::models::{PomodoroState, Priority, State, Task};
use crate::planner::{LeftoverAction, SmallItem, MAX_TASKS_PER_DAY};
use crate::progress::progress;

const MAX_TITLE_CHARS: usize = 200;
const MINUTES_PER_DAY: u32 = 24 * 60;
pub const SUBTASK_XP: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TaskError {
    #[error("A task needs a name.")]
    Unnamed,
    #[error("A task name must be {MAX_TITLE_CHARS} characters or less.")]
    TooLong,
    #[error("Two tasks have the same ID.")]
    DuplicateId,
    #[error("A task can't be given more than 24 hours.")]
    TooMuchTime,
    #[error("A day can have at most {MAX_TASKS_PER_DAY} tasks. Put quick jobs in small work.")]
    TooManyTasks,
    #[error("A focus area name must be {MAX_TITLE_CHARS} characters or less.")]
    BadFocusArea,
    #[error("Write at least one task to start your day. Small work doesn't count.")]
    NoTasks,
    #[error("That task isn't one of yesterday's.")]
    UnknownLeftover,
    #[error("Pick a day after today.")]
    NotLater,
}

pub fn priority_xp(priority: Priority) -> u32 {
    match priority {
        Priority::High => 50,
        Priority::Medium => 30,
        Priority::Low => 15,
    }
}

/// What a task is worth right now, so unticking one takes back what ticking it gave.
fn task_xp(task: &Task) -> u32 {
    let subtasks = task.subtasks.iter().filter(|s| s.done).count() as u32;
    subtasks * SUBTASK_XP
        + if task.done {
            priority_xp(task.priority)
        } else {
            0
        }
}

fn check_title(title: &str) -> Result<(), TaskError> {
    if title.trim().is_empty() {
        return Err(TaskError::Unnamed);
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(TaskError::TooLong);
    }
    Ok(())
}

fn validate(tasks: &[Task]) -> Result<(), TaskError> {
    if tasks.len() > MAX_TASKS_PER_DAY {
        return Err(TaskError::TooManyTasks);
    }
    let mut ids = HashSet::new();
    for task in tasks {
        check_title(&task.title)?;
        if task
            .focus_area
            .as_ref()
            .is_some_and(|a| a.chars().count() > MAX_TITLE_CHARS)
        {
            return Err(TaskError::BadFocusArea);
        }
        if task.allocated_min > MINUTES_PER_DAY {
            return Err(TaskError::TooMuchTime);
        }
        if !ids.insert(task.id) {
            return Err(TaskError::DuplicateId);
        }
        let mut sub_ids = HashSet::new();
        for subtask in &task.subtasks {
            check_title(&subtask.title)?;
            if subtask.allocated_min > MINUTES_PER_DAY {
                return Err(TaskError::TooMuchTime);
            }
            if !sub_ids.insert(subtask.id) {
                return Err(TaskError::DuplicateId);
            }
        }
    }
    Ok(())
}

/// Replaces the task list and settles today's XP. Removing a task keeps what it earned, which is
/// why task XP has a daily cap.
pub fn apply_tasks(
    state: &mut State,
    mut tasks: Vec<Task>,
    today: NaiveDate,
) -> Result<(), TaskError> {
    validate(&tasks)?;
    let (mut gained, mut lost, mut done, mut undone) = (0, 0, 0, 0);
    let mut renames = Vec::new();
    for task in &mut tasks {
        let old = state.tasks.iter().find(|t| t.id == task.id);
        if let Some(old) = old.filter(|t| t.title != task.title) {
            renames.push((old.title.clone(), task.title.clone()));
        }
        let was_done = old.is_some_and(|t| t.done);
        // Only the service moves tasks between days.
        task.carried_from_date = old.and_then(|t| t.carried_from_date);
        task.done_on = match (task.done, was_done) {
            (false, _) => None,
            (true, true) => old.and_then(|t| t.done_on).or(Some(today)),
            (true, false) => Some(today),
        };
        done += u32::from(task.done && !was_done);
        undone += u32::from(!task.done && was_done);

        let before = old.map_or(0, task_xp);
        let after = task_xp(task);
        gained += after.saturating_sub(before);
        lost += before.saturating_sub(after);
    }
    state.tasks = tasks;
    bump_next_id(state);
    // Focus time and a running session follow a renamed task.
    for (from, to) in renames {
        if let Some(sec) = state.today.focus_sec.remove(&from) {
            *state.today.focus_sec.entry(to.clone()).or_default() += sec;
        }
        if let PomodoroState::Focus { task_title, .. } = &mut state.pomodoro {
            if *task_title == from {
                *task_title = to;
            }
        }
    }
    let day = &mut state.today;
    day.task_xp = (day.task_xp + gained).saturating_sub(lost);
    day.tasks_done = (day.tasks_done + done).saturating_sub(undone);
    state.progress = progress(
        &state.history,
        &state.today,
        state.settings.daily_focus_target_min,
    );
    Ok(())
}

/// Replaces today's small work. The service sets each item's date, and where it was carried
/// from, so a client can't move items between days.
pub fn apply_small_items(
    state: &mut State,
    mut items: Vec<SmallItem>,
    today: NaiveDate,
) -> Result<(), TaskError> {
    let mut ids = HashSet::new();
    for item in &mut items {
        check_title(&item.title)?;
        if !ids.insert(item.id) {
            return Err(TaskError::DuplicateId);
        }
        item.date = today;
        item.carried_from_date = state
            .small_items
            .iter()
            .find(|old| old.id == item.id)
            .and_then(|old| old.carried_from_date);
    }
    state.small_items = items;
    bump_next_id(state);
    Ok(())
}

/// Carries, moves or drops one of yesterday's unfinished tasks. Returns the task so the caller
/// can store the change.
pub fn resolve_leftover(
    state: &mut State,
    task_id: u32,
    action: LeftoverAction,
    today: NaiveDate,
) -> Result<Task, TaskError> {
    let index = state
        .leftovers
        .iter()
        .position(|t| t.id == task_id)
        .ok_or(TaskError::UnknownLeftover)?;
    match action {
        LeftoverAction::Carry if state.tasks.len() >= MAX_TASKS_PER_DAY => {
            return Err(TaskError::TooManyTasks)
        }
        LeftoverAction::Move { date } if date <= today => return Err(TaskError::NotLater),
        _ => {}
    }
    let mut task = state.leftovers.remove(index);
    task.carried_from_date
        .get_or_insert(today.pred_opt().unwrap_or(today));
    if action == LeftoverAction::Carry {
        state.tasks.push(task.clone());
    }
    Ok(task)
}

/// Confirms today's plan, which needs at least one task.
pub fn plan_day(state: &mut State) -> Result<(), TaskError> {
    if state.tasks.is_empty() {
        return Err(TaskError::NoTasks);
    }
    state.planned = true;
    Ok(())
}

/// The 06:00 reset. Tasks stay in the database under their own day, and only today's are shown,
/// so the list starts empty. Unticked small work moves to the new day, and unfinished tasks wait
/// in the morning prompt.
pub fn start_day(state: &mut State, today: NaiveDate) {
    state.leftovers = state.tasks.iter().filter(|t| !t.done).cloned().collect();
    state.planned = false;
    state.today_target = None;
    state.tasks.clear();
    state.small_items.retain(|item| !item.done);
    for item in &mut state.small_items {
        if item.date < today {
            item.carried_from_date.get_or_insert(item.date);
            item.date = today;
        }
    }
}

fn bump_next_id(state: &mut State) {
    let used = state
        .tasks
        .iter()
        .map(|t| t.id)
        .chain(state.small_items.iter().map(|i| i.id))
        .max()
        .unwrap_or(0);
    state.next_id = state.next_id.max(used + 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Subtask;

    fn date(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }

    fn task(id: u32, priority: Priority, done: bool) -> Task {
        Task {
            id,
            title: format!("Task {id}"),
            priority,
            done,
            done_on: None,
            allocated_min: 0,
            subtasks: vec![],
            focus_area: None,
            carried_from_date: None,
        }
    }

    fn subtask(id: u32, done: bool) -> Subtask {
        Subtask {
            id,
            title: format!("Step {id}"),
            done,
            allocated_min: 0,
        }
    }

    fn apply(state: &mut State, tasks: Vec<Task>) -> Result<(), TaskError> {
        apply_tasks(state, tasks, date(5))
    }

    #[test]
    fn finishing_a_task_earns_xp_by_priority() {
        let mut s = State::new(date(5));
        apply(&mut s, vec![task(1, Priority::High, false)]).unwrap();
        assert_eq!(s.today.task_xp, 0);
        apply(&mut s, vec![task(1, Priority::High, true)]).unwrap();
        assert_eq!(s.today.task_xp, 50);
        assert_eq!(s.today.tasks_done, 1);
        assert_eq!(s.tasks[0].done_on, Some(date(5)));
        assert_eq!(s.progress.today_xp, 50);
    }

    #[test]
    fn saving_the_same_list_again_earns_nothing_more() {
        let mut s = State::new(date(5));
        let tasks = vec![task(1, Priority::Low, true)];
        apply(&mut s, tasks.clone()).unwrap();
        apply(&mut s, tasks).unwrap();
        assert_eq!(s.today.task_xp, 15);
        assert_eq!(s.today.tasks_done, 1);
    }

    #[test]
    fn unticking_takes_the_xp_back() {
        let mut s = State::new(date(5));
        apply(&mut s, vec![task(1, Priority::Medium, true)]).unwrap();
        apply(&mut s, vec![task(1, Priority::Medium, false)]).unwrap();
        assert_eq!(s.today.task_xp, 0);
        assert_eq!(s.today.tasks_done, 0);
        assert_eq!(s.tasks[0].done_on, None);
    }

    #[test]
    fn subtasks_earn_their_own_xp() {
        let mut s = State::new(date(5));
        let mut t = task(1, Priority::Low, false);
        t.subtasks = vec![subtask(1, true), subtask(2, false)];
        apply(&mut s, vec![t.clone()]).unwrap();
        assert_eq!(s.today.task_xp, 5);
        t.subtasks[1].done = true;
        t.done = true;
        apply(&mut s, vec![t]).unwrap();
        assert_eq!(s.today.task_xp, 25);
    }

    #[test]
    fn removing_a_finished_task_keeps_its_xp() {
        let mut s = State::new(date(5));
        apply(&mut s, vec![task(1, Priority::High, true)]).unwrap();
        apply(&mut s, vec![]).unwrap();
        assert_eq!(s.today.task_xp, 50);
        assert!(s.tasks.is_empty());
    }

    #[test]
    fn a_client_cannot_backdate_a_finish() {
        let mut s = State::new(date(5));
        let mut t = task(1, Priority::Low, true);
        t.done_on = Some(date(1));
        apply(&mut s, vec![t]).unwrap();
        assert_eq!(s.tasks[0].done_on, Some(date(5)));
    }

    #[test]
    fn bad_lists_are_refused_and_change_nothing() {
        let mut s = State::new(date(5));
        let mut blank = task(1, Priority::Low, true);
        blank.title = "  ".to_string();
        assert_eq!(apply(&mut s, vec![blank]), Err(TaskError::Unnamed));

        let mut long = task(1, Priority::Low, true);
        long.title = "x".repeat(201);
        assert_eq!(apply(&mut s, vec![long]), Err(TaskError::TooLong));

        let twins = vec![task(1, Priority::Low, true), task(1, Priority::Low, true)];
        assert_eq!(apply(&mut s, twins), Err(TaskError::DuplicateId));

        let mut twin_steps = task(1, Priority::Low, false);
        twin_steps.subtasks = vec![subtask(1, false), subtask(1, false)];
        assert_eq!(apply(&mut s, vec![twin_steps]), Err(TaskError::DuplicateId));

        assert_eq!(s.today.task_xp, 0);
        assert!(s.tasks.is_empty());
    }

    #[test]
    fn time_is_kept_and_earns_nothing_by_itself() {
        let mut s = State::new(date(5));
        let mut t = task(1, Priority::High, false);
        t.allocated_min = 180;
        let mut step = subtask(1, false);
        step.allocated_min = 45;
        t.subtasks = vec![step];
        apply(&mut s, vec![t]).unwrap();
        assert_eq!(s.tasks[0].allocated_min, 180);
        assert_eq!(s.tasks[0].subtasks[0].allocated_min, 45);
        assert_eq!(s.today.task_xp, 0);
    }

    #[test]
    fn more_than_a_day_of_time_is_refused() {
        let mut s = State::new(date(5));
        let mut t = task(1, Priority::Low, false);
        t.allocated_min = 1441;
        assert_eq!(apply(&mut s, vec![t]), Err(TaskError::TooMuchTime));

        let mut t = task(1, Priority::Low, false);
        let mut step = subtask(1, false);
        step.allocated_min = 1441;
        t.subtasks = vec![step];
        assert_eq!(apply(&mut s, vec![t]), Err(TaskError::TooMuchTime));
    }

    #[test]
    fn tasks_saved_before_time_or_with_a_quadrant_still_load() {
        let stored = r#"[{"id":1,"title":"Old","priority":"high","done":false,"done_on":null,"quadrant":"do_first",
            "subtasks":[{"id":1,"title":"Step","done":false}]}]"#;
        let tasks: Vec<Task> = serde_json::from_str(stored).unwrap();
        assert_eq!(tasks[0].allocated_min, 0);
        assert_eq!(tasks[0].subtasks[0].allocated_min, 0);
    }

    #[test]
    fn renaming_a_task_keeps_its_focus_time_and_session() {
        let mut s = State::new(date(5));
        let mut t = task(1, Priority::High, false);
        apply(&mut s, vec![t.clone()]).unwrap();
        s.today.focus_sec.insert(t.title.clone(), 600);
        let now = crate::models::Now::new(chrono::Utc::now(), &chrono::Utc);
        crate::engine::start_pomodoro(&mut s, Some(1), now).unwrap();
        t.title = "Renamed".to_string();
        apply(&mut s, vec![t]).unwrap();
        assert_eq!(s.today.focus_sec.get("Renamed"), Some(&600));
        assert!(
            matches!(&s.pomodoro, PomodoroState::Focus { task_title, .. } if task_title == "Renamed")
        );
    }

    #[test]
    fn a_task_done_at_one_am_counts_toward_the_day_before() {
        use chrono::TimeZone;
        let dhaka = chrono::FixedOffset::east_opt(6 * 3600).unwrap();
        let one_am = dhaka
            .with_ymd_and_hms(2026, 11, 13, 1, 0, 0)
            .unwrap()
            .to_utc();
        let day = crate::models::app_day(one_am, &dhaka);
        let mut s = State::new(day);
        apply_tasks(&mut s, vec![task(1, Priority::High, false)], day).unwrap();
        apply_tasks(&mut s, vec![task(1, Priority::High, true)], day).unwrap();
        assert_eq!(s.tasks[0].done_on, Some(date(12)));
        assert_eq!(s.today.tasks_done, 1);
    }

    #[test]
    fn a_new_day_offers_unfinished_tasks_and_needs_a_new_plan() {
        let mut s = State::new(date(5));
        apply(
            &mut s,
            vec![task(1, Priority::High, false), task(2, Priority::Low, true)],
        )
        .unwrap();
        plan_day(&mut s).unwrap();
        start_day(&mut s, date(6));
        assert!(!s.planned);
        assert_eq!(s.leftovers.len(), 1);
        assert_eq!(s.leftovers[0].id, 1);
        // Only small work isn't a plan.
        assert_eq!(plan_day(&mut s), Err(TaskError::NoTasks));
        assert!(!s.planned);
    }

    #[test]
    fn leftovers_can_be_carried_moved_or_dropped() {
        let mut s = State::new(date(6));
        s.leftovers = (1..=3).map(|id| task(id, Priority::Low, false)).collect();
        let carried = resolve_leftover(&mut s, 1, LeftoverAction::Carry, date(6)).unwrap();
        assert_eq!(carried.carried_from_date, Some(date(5)));
        assert_eq!(s.tasks[0].id, 1);
        assert_eq!(
            resolve_leftover(&mut s, 2, LeftoverAction::Move { date: date(6) }, date(6)),
            Err(TaskError::NotLater)
        );
        let moved = LeftoverAction::Move { date: date(9) };
        resolve_leftover(&mut s, 2, moved, date(6)).unwrap();
        resolve_leftover(&mut s, 3, LeftoverAction::Drop, date(6)).unwrap();
        assert!(s.leftovers.is_empty());
        assert_eq!(s.tasks.len(), 1);
        assert_eq!(
            resolve_leftover(&mut s, 3, LeftoverAction::Drop, date(6)),
            Err(TaskError::UnknownLeftover)
        );
        // A carried task keeps its date even if the client says otherwise.
        let mut faked = s.tasks.clone();
        faked[0].carried_from_date = None;
        apply(&mut s, faked).unwrap();
        assert_eq!(s.tasks[0].carried_from_date, Some(date(5)));
    }

    #[test]
    fn carrying_onto_a_full_day_is_refused() {
        let mut s = State::new(date(6));
        apply(
            &mut s,
            (1..=4).map(|id| task(id, Priority::Low, false)).collect(),
        )
        .unwrap();
        s.leftovers = vec![task(9, Priority::High, false)];
        assert_eq!(
            resolve_leftover(&mut s, 9, LeftoverAction::Carry, date(6)),
            Err(TaskError::TooManyTasks)
        );
        assert_eq!(s.leftovers.len(), 1);
    }

    #[test]
    fn a_fifth_task_is_refused_and_changes_nothing() {
        let mut s = State::new(date(5));
        let four: Vec<_> = (1..=4).map(|id| task(id, Priority::Low, false)).collect();
        apply(&mut s, four.clone()).unwrap();
        let mut five = four.clone();
        five.push(task(5, Priority::High, false));
        assert_eq!(apply(&mut s, five), Err(TaskError::TooManyTasks));
        assert_eq!(s.tasks, four);
    }

    #[test]
    fn new_ids_never_reuse_one_already_given_out() {
        let mut s = State::new(date(5));
        apply(&mut s, vec![task(7, Priority::Low, false)]).unwrap();
        assert_eq!(s.next_id, 8);
        // Removing it doesn't free 7: the archive may still hold it.
        apply(&mut s, vec![]).unwrap();
        assert_eq!(s.next_id, 8);
    }

    fn item(id: u32, done: bool) -> SmallItem {
        SmallItem {
            id,
            date: date(1),
            title: format!("Call {id}"),
            done,
            carried_from_date: None,
        }
    }

    #[test]
    fn small_work_has_no_limit_and_is_dated_by_the_service() {
        let mut s = State::new(date(5));
        let items: Vec<_> = (1..=12).map(|id| item(id, false)).collect();
        apply_small_items(&mut s, items, date(5)).unwrap();
        assert_eq!(s.small_items.len(), 12);
        assert!(s.small_items.iter().all(|i| i.date == date(5)));
        assert!(s.small_items.iter().all(|i| i.carried_from_date.is_none()));
        // Small work isn't a task: no XP, nothing done.
        assert_eq!(s.today.task_xp, 0);
        let mut blank = item(13, false);
        blank.title = " ".to_string();
        assert_eq!(
            apply_small_items(&mut s, vec![blank], date(5)),
            Err(TaskError::Unnamed)
        );
    }

    #[test]
    fn a_new_day_clears_tasks_and_carries_unticked_small_work() {
        let mut s = State::new(date(5));
        apply(&mut s, vec![task(1, Priority::High, false)]).unwrap();
        apply_small_items(&mut s, vec![item(2, false), item(3, true)], date(5)).unwrap();
        start_day(&mut s, date(6));
        assert!(s.tasks.is_empty());
        assert_eq!(s.small_items.len(), 1);
        assert_eq!(s.small_items[0].id, 2);
        assert_eq!(s.small_items[0].date, date(6));
        assert_eq!(s.small_items[0].carried_from_date, Some(date(5)));
        // Carried again, it remembers the first day.
        start_day(&mut s, date(7));
        assert_eq!(s.small_items[0].carried_from_date, Some(date(5)));
        // The client can't change where it came from.
        let mut faked = s.small_items.clone();
        faked[0].carried_from_date = None;
        apply_small_items(&mut s, faked, date(7)).unwrap();
        assert_eq!(s.small_items[0].carried_from_date, Some(date(5)));
    }
}
