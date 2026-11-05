use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::UsageDay;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Stats {
    /// Oldest first, ending with today.
    pub days: Vec<UsageDay>,
    pub focus_totals_sec: BTreeMap<String, u32>,
    pub blocked_attempts: u32,
}

/// Consecutive days that hit the focus target, ending today. A today that hasn't hit the target
/// yet doesn't break the streak, since the day isn't over.
pub fn streak(history: &[UsageDay], today: &UsageDay, target_min: u32) -> u32 {
    let hit_target = |d: &UsageDay| d.total_focus_sec() >= target_min * 60;
    let mut count = u32::from(hit_target(today));
    let mut expected = today.date.pred_opt();
    for day in history.iter().rev() {
        if Some(day.date) != expected || !hit_target(day) {
            break;
        }
        count += 1;
        expected = day.date.pred_opt();
    }
    count
}

/// Stats for the last `days` days, including today.
pub fn build_stats(history: &[UsageDay], today: &UsageDay, days: usize) -> Stats {
    let past = days.saturating_sub(1);
    let start = history.len().saturating_sub(past);
    let mut selected = history[start..].to_vec();
    selected.push(today.clone());

    let mut focus_totals_sec = BTreeMap::new();
    for (block, sec) in selected.iter().flat_map(|d| &d.focus_sec) {
        *focus_totals_sec.entry(block.clone()).or_default() += sec;
    }
    let blocked_attempts = selected.iter().map(|d| d.blocked_attempts).sum();

    Stats {
        days: selected,
        focus_totals_sec,
        blocked_attempts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    const TARGET_MIN: u32 = 660;

    fn day(n: u32, focus_min: u32) -> UsageDay {
        let mut d = UsageDay::new(NaiveDate::from_ymd_opt(2026, 11, n).unwrap());
        d.focus_sec.insert("React".to_string(), focus_min * 60);
        d
    }

    #[test]
    fn counts_consecutive_days_that_hit_the_target() {
        let history = [day(1, 660), day(2, 700), day(3, 660)];
        assert_eq!(streak(&history, &day(4, 0), TARGET_MIN), 3);
    }

    #[test]
    fn today_adds_to_the_streak_once_it_hits_the_target() {
        let history = [day(3, 660)];
        assert_eq!(streak(&history, &day(4, 659), TARGET_MIN), 1);
        assert_eq!(streak(&history, &day(4, 660), TARGET_MIN), 2);
    }

    #[test]
    fn a_partial_day_breaks_the_streak() {
        let history = [day(1, 660), day(2, 300), day(3, 660)];
        assert_eq!(streak(&history, &day(4, 0), TARGET_MIN), 1);
    }

    #[test]
    fn a_missing_day_breaks_the_streak() {
        let history = [day(1, 660), day(3, 660)];
        assert_eq!(streak(&history, &day(4, 0), TARGET_MIN), 1);
        // Yesterday wasn't recorded at all, so older days don't count.
        assert_eq!(streak(&[day(1, 660)], &day(4, 660), TARGET_MIN), 1);
    }

    #[test]
    fn focus_across_blocks_adds_up() {
        let mut today = day(4, 330);
        today.focus_sec.insert("Building".to_string(), 330 * 60);
        assert_eq!(streak(&[], &today, TARGET_MIN), 1);
    }

    #[test]
    fn empty_history_with_no_focus_is_zero() {
        assert_eq!(streak(&[], &day(4, 0), TARGET_MIN), 0);
    }

    #[test]
    fn stats_cover_the_requested_days_including_today() {
        let mut old = day(1, 100);
        old.blocked_attempts = 50;
        let mut yesterday = day(2, 60);
        yesterday.blocked_attempts = 2;
        let mut today = day(3, 30);
        today.blocked_attempts = 1;
        today.focus_sec.insert("Building".to_string(), 600);

        let stats = build_stats(&[old, yesterday], &today, 2);
        assert_eq!(stats.days.len(), 2);
        assert_eq!(stats.days[1], today);
        assert_eq!(stats.focus_totals_sec["React"], 90 * 60);
        assert_eq!(stats.focus_totals_sec["Building"], 600);
        assert_eq!(stats.blocked_attempts, 3);
    }

    #[test]
    fn stats_handle_short_history() {
        assert_eq!(build_stats(&[], &day(1, 0), 30).days.len(), 1);
        assert_eq!(build_stats(&[], &day(1, 0), 0).days.len(), 1);
    }
}
