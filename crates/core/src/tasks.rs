//! Today's tasks and the XP they earn (F7, F11).

use std::collections::HashSet;

use chrono::NaiveDate;
use thiserror::Error;

use crate::models::{Priority, State, Task};
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
    let mut ids = HashSet::new();
    for task in tasks {
        check_title(&task.title)?;
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
    for task in &mut tasks {
        let old = state.tasks.iter().find(|t| t.id == task.id);
        let was_done = old.is_some_and(|t| t.done);
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

/// Finished tasks stay for the day they were finished. Unfinished ones carry over.
pub fn prune_tasks(tasks: &mut Vec<Task>, today: NaiveDate) {
    tasks.retain(|t| !t.done || t.done_on == Some(today));
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
    fn a_new_day_drops_finished_tasks_and_keeps_the_rest() {
        let mut s = State::new(date(5));
        apply(
            &mut s,
            vec![task(1, Priority::High, true), task(2, Priority::Low, false)],
        )
        .unwrap();
        prune_tasks(&mut s.tasks, date(5));
        assert_eq!(s.tasks.len(), 2);
        prune_tasks(&mut s.tasks, date(6));
        assert_eq!(s.tasks.len(), 1);
        assert_eq!(s.tasks[0].id, 2);
    }
}
