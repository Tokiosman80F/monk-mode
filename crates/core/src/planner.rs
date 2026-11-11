//! The planner: monthly and weekly goals, daily tasks, small work and penalties. Dates are app
//! days, see `models::app_day`.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Tasks are the work that really counts, so a day holds at most this many.
pub const MAX_TASKS_PER_DAY: usize = 4;
const MAX_FOCUS_AREAS: usize = 6;
const MAX_NAME_CHARS: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum WeekStart {
    Saturday,
    Sunday,
    Monday,
}

/// Something the user works on, with the hours a day they aim to give it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FocusArea {
    pub name: String,
    pub target_min: u32,
}

/// From the one-time setup. The daily study hours are the existing daily focus target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Profile {
    pub name: String,
    pub focus_areas: Vec<FocusArea>,
    pub week_start: WeekStart,
}

impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        let name_ok = |n: &str| !n.trim().is_empty() && n.chars().count() <= MAX_NAME_CHARS;
        if !name_ok(&self.name) {
            return Err(format!(
                "Your name must be 1 to {MAX_NAME_CHARS} characters."
            ));
        }
        if self.focus_areas.is_empty() || self.focus_areas.len() > MAX_FOCUS_AREAS {
            return Err(format!("Pick 1 to {MAX_FOCUS_AREAS} focus areas."));
        }
        let mut seen = std::collections::HashSet::new();
        for area in &self.focus_areas {
            if !name_ok(&area.name) {
                return Err(format!(
                    "A focus area needs a name of 1 to {MAX_NAME_CHARS} characters."
                ));
            }
            if !seen.insert(area.name.trim().to_lowercase()) {
                return Err(format!("{} is on the list twice.", area.name));
            }
            if area.target_min > 24 * 60 {
                return Err(format!(
                    "{} can't have more than 24 hours a day.",
                    area.name
                ));
            }
        }
        Ok(())
    }
}

/// What the morning prompt does with one of yesterday's unfinished tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum LeftoverAction {
    /// Onto today's list.
    Carry,
    /// Onto a later day's list.
    Move { date: NaiveDate },
    /// Kept in the history as dropped.
    Drop,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        let area = |name: &str, hours: u32| FocusArea {
            name: name.to_string(),
            target_min: hours * 60,
        };
        Profile {
            name: "Tokiosman".to_string(),
            focus_areas: vec![area("React", 3), area("Products", 6), area("LinkedIn", 2)],
            week_start: WeekStart::Saturday,
        }
    }

    #[test]
    fn a_profile_needs_a_name_and_distinct_focus_areas() {
        assert!(profile().validate().is_ok());
        let mut p = profile();
        p.name = " ".to_string();
        assert!(p.validate().is_err());
        let mut p = profile();
        p.focus_areas.clear();
        assert!(p.validate().is_err());
        let mut p = profile();
        p.focus_areas[1].name = "react".to_string();
        assert!(p.validate().unwrap_err().contains("twice"));
        let mut p = profile();
        p.focus_areas[0].target_min = 25 * 60;
        assert!(p.validate().is_err());
    }
}
