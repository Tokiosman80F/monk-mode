//! The planner: monthly and weekly goals, daily tasks, small work and penalties. Dates are app
//! days, see `models::app_day`.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Tasks are the work that really counts, so a day holds at most this many.
pub const MAX_TASKS_PER_DAY: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum GoalLevel {
    Month,
    Week,
}

/// Can't be changed once `locked_at` is set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Goal {
    pub id: u32,
    pub level: GoalLevel,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub title: String,
    pub locked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DayStatus {
    Planned,
    Done,
    Partial,
    Missed,
    Rest,
}

/// One day's target in a goal. The target is fixed with the goal; the status is filled in later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GoalDay {
    pub goal_id: u32,
    pub date: NaiveDate,
    pub target: String,
    pub status: DayStatus,
}

/// A quick 5 to 10 minute job. Not a task: no limit, and it never counts as a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SmallItem {
    pub id: u32,
    pub date: NaiveDate,
    pub title: String,
    pub done: bool,
    pub carried_from_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DayLog {
    pub date: NaiveDate,
    /// When the first task was written.
    pub planned_at: Option<DateTime<Utc>>,
    pub checkin_note: Option<String>,
    pub completion_pct: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PenaltyReason {
    NoPlan,
}

/// One per day at most, so a missed day is never charged twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Penalty {
    pub date: NaiveDate,
    pub reason: PenaltyReason,
    pub days_added: u32,
    pub lock_end_before: Option<DateTime<Utc>>,
    pub lock_end_after: Option<DateTime<Utc>>,
}
