//! XP, levels and badges (F11). Everything comes from usage days, so there's nothing extra to
//! store or tamper with. Earned only by focus time and finished tasks, and never taken away for
//! a slip.

use crate::models::{Badge, Progress, UsageDay};

const XP_PER_FOCUS_MIN: u32 = 1;
/// Tasks are free to add, so this stops a day of ticking made-up ones from buying levels.
const DAILY_TASK_XP_CAP: u32 = 200;
const XP_PER_LEVEL_STEP: u32 = 100;

pub fn day_xp(day: &UsageDay) -> u32 {
    day.total_focus_sec() / 60 * XP_PER_FOCUS_MIN + day.task_xp.min(DAILY_TASK_XP_CAP)
}

/// Level 1 takes 100 XP to finish, level 2 takes 200, and so on. Returns the level, the XP into
/// it, and the XP it takes to finish.
pub fn level_for(xp: u32) -> (u32, u32, u32) {
    let (mut level, mut left) = (1, xp);
    while left >= level * XP_PER_LEVEL_STEP {
        left -= level * XP_PER_LEVEL_STEP;
        level += 1;
    }
    (level, left, level * XP_PER_LEVEL_STEP)
}

/// Uses the current focus target, so raising the target can take back a streak badge, as it
/// already changes the streak itself.
fn longest_streak(days: &[&UsageDay], target_min: u32) -> u32 {
    let (mut best, mut run) = (0, 0);
    let mut previous = None;
    for day in days {
        let focus = day.total_focus_sec();
        if focus == 0 || focus < target_min * 60 {
            run = 0;
        } else {
            run = if previous == day.date.pred_opt() {
                run + 1
            } else {
                1
            };
            best = best.max(run);
        }
        previous = Some(day.date);
    }
    best
}

pub fn progress(history: &[UsageDay], today: &UsageDay, target_min: u32) -> Progress {
    let days: Vec<&UsageDay> = history.iter().chain(std::iter::once(today)).collect();
    let xp = days.iter().map(|d| day_xp(d)).sum();
    let focus_hours = days.iter().map(|d| d.total_focus_sec()).sum::<u32>() / 3600;
    let any_focus = days.iter().any(|d| d.total_focus_sec() > 0);
    let tasks: u32 = days.iter().map(|d| d.tasks_done).sum();
    let streak = longest_streak(&days, target_min);

    let earned = [
        (Badge::FirstFocus, any_focus),
        (Badge::Focus10h, focus_hours >= 10),
        (Badge::Focus100h, focus_hours >= 100),
        (Badge::Streak3, streak >= 3),
        (Badge::Streak7, streak >= 7),
        (Badge::Streak30, streak >= 30),
        (Badge::Streak90, streak >= 90),
        (Badge::Tasks10, tasks >= 10),
        (Badge::Tasks100, tasks >= 100),
    ];
    let (level, level_xp, next_level_xp) = level_for(xp);
    Progress {
        xp,
        today_xp: day_xp(today),
        level,
        level_xp,
        next_level_xp,
        badges: earned
            .into_iter()
            .filter(|(_, got)| *got)
            .map(|(badge, _)| badge)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn day(n: u32, focus_min: u32) -> UsageDay {
        let mut d = UsageDay::new(NaiveDate::from_ymd_opt(2026, 11, n).unwrap());
        if focus_min > 0 {
            d.focus_sec.insert("React".to_string(), focus_min * 60);
        }
        d
    }

    #[test]
    fn focus_minutes_and_tasks_add_up() {
        let mut d = day(1, 90);
        d.task_xp = 50;
        assert_eq!(day_xp(&d), 140);
    }

    #[test]
    fn task_xp_is_capped_per_day_but_focus_is_not() {
        let mut d = day(1, 300);
        d.task_xp = 5000;
        assert_eq!(day_xp(&d), 500);
    }

    #[test]
    fn levels_get_longer_as_they_go() {
        assert_eq!(level_for(0), (1, 0, 100));
        assert_eq!(level_for(99), (1, 99, 100));
        assert_eq!(level_for(100), (2, 0, 200));
        assert_eq!(level_for(299), (2, 199, 200));
        assert_eq!(level_for(300), (3, 0, 300));
    }

    #[test]
    fn totals_cover_history_and_today_while_today_xp_is_only_today() {
        let p = progress(&[day(1, 100)], &day(2, 20), 660);
        assert_eq!(p.xp, 120);
        assert_eq!(p.today_xp, 20);
        assert_eq!((p.level, p.level_xp, p.next_level_xp), (2, 20, 200));
    }

    #[test]
    fn nothing_done_earns_nothing() {
        let p = progress(&[], &day(1, 0), 0);
        assert_eq!(p.xp, 0);
        assert!(p.badges.is_empty());
    }

    #[test]
    fn focus_badges_come_from_total_hours() {
        let p = progress(&[day(1, 300)], &day(2, 299), 660);
        assert_eq!(p.badges, [Badge::FirstFocus]);
        let p = progress(&[day(1, 300)], &day(2, 300), 660);
        assert_eq!(p.badges, [Badge::FirstFocus, Badge::Focus10h]);
    }

    #[test]
    fn streak_badges_need_consecutive_days_and_stay_after_a_break() {
        let run = [day(1, 660), day(2, 660), day(3, 660), day(4, 0)];
        let p = progress(&run, &day(5, 0), 660);
        assert!(p.badges.contains(&Badge::Streak3));

        let gap = [day(1, 660), day(2, 660), day(4, 660)];
        let p = progress(&gap, &day(5, 0), 660);
        assert!(!p.badges.contains(&Badge::Streak3));
    }

    #[test]
    fn task_badges_count_finished_tasks() {
        let mut today = day(1, 0);
        today.tasks_done = 10;
        assert_eq!(progress(&[], &today, 660).badges, [Badge::Tasks10]);
    }
}
