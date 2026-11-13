//! The planner: the profile, daily tasks and small work. Dates are app days, see
//! `models::app_day`.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Tasks are the work that really counts, so a day holds at most this many.
pub const MAX_TASKS_PER_DAY: usize = 4;
const MAX_FOCUS_AREAS: usize = 6;
const MAX_NAME_CHARS: usize = 60;

/// Something the user works on, like `React`. Tasks are labelled with one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FocusArea {
    pub name: String,
}

/// From the one-time setup. The daily study hours are the existing daily focus target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Profile {
    pub name: String,
    pub focus_areas: Vec<FocusArea>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        let area = |name: &str| FocusArea {
            name: name.to_string(),
        };
        Profile {
            name: "Tokiosman".to_string(),
            focus_areas: vec![area("React"), area("Products"), area("LinkedIn")],
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
    }
}
