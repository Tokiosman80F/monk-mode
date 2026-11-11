use std::collections::HashSet;

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::domain::clean_domain;
use crate::models::{Lock, LockDay, Settings};

const MAX_ALLOWANCE_MIN: u32 = 180;
const MINUTES_PER_DAY: u32 = 24 * 60;
const SECONDS_PER_DAY: i64 = 24 * 3600;

/// A change the commitment lock forbids (F5).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LooseningError {
    #[error("{0} can't be removed while locked.")]
    RemovedSite(String),
    #[error("{0} can't move from always blocked to allowance while locked.")]
    MovedToAllowance(String),
    #[error("The daily limit for {0} can't go up while locked.")]
    RaisedAllowance(String),
    #[error("The Shorts and Reels blocker can't be turned off while locked.")]
    ShortsBlockerDisabled,
    #[error("The lock can't be shortened.")]
    ShortenedLock,
}

/// Settings that make no sense, locked or not.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvalidSettings {
    #[error("A daily limit must be {MAX_ALLOWANCE_MIN} minutes or less.")]
    AllowanceTooHigh,
    #[error("{0} is on the list twice.")]
    DuplicateSite(String),
    #[error("A focus session must be at least 1 minute.")]
    NoFocusLength,
    #[error("The daily focus target can't be more than 24 hours.")]
    TargetTooHigh,
    #[error("\"{0}\" is not a clean domain.")]
    BadDomain(String),
    #[error("A lock must end in the future.")]
    LockEndsInPast,
}

/// Run on every incoming settings change. Requiring clean domains also keeps `validate_change`
/// honest: `WWW.YouTube.com` can't stand in for `youtube.com`.
pub fn validate_settings(s: &Settings) -> Result<(), InvalidSettings> {
    if s.allowance_sites
        .iter()
        .any(|site| site.daily_min > MAX_ALLOWANCE_MIN)
    {
        return Err(InvalidSettings::AllowanceTooHigh);
    }
    if s.pomodoro_focus_min == 0 {
        return Err(InvalidSettings::NoFocusLength);
    }
    if s.daily_focus_target_min > MINUTES_PER_DAY {
        return Err(InvalidSettings::TargetTooHigh);
    }
    let allowance = s.allowance_sites.iter().map(|site| &site.domain);
    for site in allowance.clone().chain(&s.always_blocked) {
        if clean_domain(site).as_deref() != Ok(site.as_str()) {
            return Err(InvalidSettings::BadDomain(site.clone()));
        }
    }
    // A second entry with a bigger number would be a way to raise a limit.
    let mut seen = HashSet::new();
    for site in allowance {
        if !seen.insert(site) {
            return Err(InvalidSettings::DuplicateSite(site.clone()));
        }
    }
    Ok(())
}

/// While a lock is active, rules may only get stricter.
pub fn validate_change(
    old: &Settings,
    new: &Settings,
    lock: Option<&Lock>,
    now: DateTime<Utc>,
) -> Result<(), LooseningError> {
    if !lock.is_some_and(|l| l.is_active(now)) {
        return Ok(());
    }
    for site in &old.always_blocked {
        if !new.always_blocked.contains(site) {
            return Err(if new.allowance_sites.iter().any(|s| &s.domain == site) {
                LooseningError::MovedToAllowance(site.clone())
            } else {
                LooseningError::RemovedSite(site.clone())
            });
        }
    }
    for site in &old.allowance_sites {
        match new.allowance_sites.iter().find(|s| s.domain == site.domain) {
            Some(kept) if kept.daily_min > site.daily_min => {
                return Err(LooseningError::RaisedAllowance(site.domain.clone()));
            }
            Some(_) => {}
            None if new.always_blocked.contains(&site.domain) => {}
            None => return Err(LooseningError::RemovedSite(site.domain.clone())),
        }
    }
    if old.shorts_blocker_enabled && !new.shorts_blocker_enabled {
        return Err(LooseningError::ShortsBlockerDisabled);
    }
    Ok(())
}

/// The lock that results from asking for `ends_at`. An active lock can only be extended, and
/// keeps its start date so "Day X of Y" doesn't reset.
pub fn next_lock(
    current: Option<&Lock>,
    ends_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Lock, NextLockError> {
    if ends_at <= now {
        return Err(InvalidSettings::LockEndsInPast.into());
    }
    match current.filter(|l| l.is_active(now)) {
        Some(l) if ends_at < l.ends_at => Err(LooseningError::ShortenedLock.into()),
        Some(l) => Ok(Lock {
            created_at: l.created_at,
            ends_at,
        }),
        None => Ok(Lock {
            created_at: now,
            ends_at,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NextLockError {
    #[error(transparent)]
    Invalid(#[from] InvalidSettings),
    #[error(transparent)]
    Loosening(#[from] LooseningError),
}

/// Turning the system clock back must not shorten a lock or refill the allowance.
pub fn effective_now(now: DateTime<Utc>, last_seen: DateTime<Utc>) -> DateTime<Utc> {
    now.max(last_seen)
}

/// "Day X of Y" while a lock is active. Day 1 is the day it started.
pub fn lock_day(lock: &Lock, now: DateTime<Utc>) -> Option<LockDay> {
    if !lock.is_active(now) {
        return None;
    }
    let length_sec = (lock.ends_at - lock.created_at).num_seconds();
    let total = ((length_sec + SECONDS_PER_DAY - 1) / SECONDS_PER_DAY).max(1);
    let day = (now - lock.created_at).num_days() + 1;
    Some(LockDay {
        day: day.clamp(1, total) as u32,
        total: total as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{compute_blocked, tick};
    use crate::models::{AllowanceSite, Now, State};
    use chrono::{Duration, TimeZone};

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 5, 12, 0, 0).unwrap()
    }

    fn locked() -> Lock {
        Lock {
            created_at: t0(),
            ends_at: t0() + Duration::days(90),
        }
    }

    fn check(change: impl FnOnce(&mut Settings)) -> Result<(), LooseningError> {
        let old = Settings::default();
        let mut new = old.clone();
        change(&mut new);
        validate_change(&old, &new, Some(&locked()), t0())
    }

    fn remove(list: &mut Vec<String>, site: &str) {
        list.retain(|s| s != site);
    }

    fn remove_limit(settings: &mut Settings, site: &str) {
        settings.allowance_sites.retain(|s| s.domain != site);
    }

    fn set_limit(settings: &mut Settings, domain: &str, daily_min: u32) {
        let site = settings
            .allowance_sites
            .iter_mut()
            .find(|s| s.domain == domain)
            .unwrap();
        site.daily_min = daily_min;
    }

    // Allowed while locked

    #[test]
    fn may_add_an_allowance_site() {
        assert_eq!(
            check(|s| s.allowance_sites.push(AllowanceSite {
                domain: "twitch.tv".to_string(),
                daily_min: 60,
            })),
            Ok(())
        );
    }

    #[test]
    fn may_add_an_always_blocked_site() {
        assert_eq!(
            check(|s| s.always_blocked.push("twitch.tv".to_string())),
            Ok(())
        );
    }

    #[test]
    fn may_move_a_site_from_allowance_to_always_blocked() {
        assert_eq!(
            check(|s| {
                remove_limit(s, "reddit.com");
                s.always_blocked.push("reddit.com".to_string());
            }),
            Ok(())
        );
    }

    #[test]
    fn may_lower_the_allowance() {
        assert_eq!(check(|s| set_limit(s, "youtube.com", 5)), Ok(()));
    }

    // Forbidden while locked

    #[test]
    fn may_not_remove_an_allowance_site() {
        assert_eq!(
            check(|s| remove_limit(s, "reddit.com")),
            Err(LooseningError::RemovedSite("reddit.com".to_string()))
        );
    }

    #[test]
    fn may_not_remove_an_always_blocked_site() {
        assert_eq!(
            check(|s| remove(&mut s.always_blocked, "tiktok.com")),
            Err(LooseningError::RemovedSite("tiktok.com".to_string()))
        );
    }

    #[test]
    fn may_not_move_a_site_from_always_blocked_to_allowance() {
        assert_eq!(
            check(|s| {
                remove(&mut s.always_blocked, "tiktok.com");
                s.allowance_sites.push(AllowanceSite {
                    domain: "tiktok.com".to_string(),
                    daily_min: 1,
                });
            }),
            Err(LooseningError::MovedToAllowance("tiktok.com".to_string()))
        );
    }

    #[test]
    fn may_not_raise_the_allowance() {
        assert_eq!(
            check(|s| set_limit(s, "youtube.com", 31)),
            Err(LooseningError::RaisedAllowance("youtube.com".to_string()))
        );
    }

    #[test]
    fn may_not_turn_off_the_shorts_blocker() {
        assert_eq!(
            check(|s| s.shorts_blocker_enabled = false),
            Err(LooseningError::ShortsBlockerDisabled)
        );
    }

    #[test]
    fn may_not_shorten_the_lock() {
        let shorter = locked().ends_at - Duration::days(1);
        assert_eq!(
            next_lock(Some(&locked()), shorter, t0()),
            Err(LooseningError::ShortenedLock.into())
        );
    }

    #[test]
    fn extending_the_lock_keeps_its_start_date() {
        let longer = locked().ends_at + Duration::days(1);
        let lock = next_lock(Some(&locked()), longer, t0() + Duration::days(5)).unwrap();
        assert_eq!(lock.created_at, t0());
        assert_eq!(lock.ends_at, longer);
    }

    #[test]
    fn a_new_lock_must_end_in_the_future() {
        assert_eq!(
            next_lock(None, t0(), t0()),
            Err(InvalidSettings::LockEndsInPast.into())
        );
        assert_eq!(
            next_lock(None, t0() + Duration::days(7), t0())
                .unwrap()
                .created_at,
            t0()
        );
    }

    // No lock, or an expired one

    #[test]
    fn anything_goes_without_a_lock() {
        let old = Settings::default();
        let new = Settings {
            shorts_blocker_enabled: false,
            allowance_sites: vec![],
            always_blocked: vec![],
            ..old.clone()
        };
        assert_eq!(validate_change(&old, &new, None, t0()), Ok(()));
    }

    #[test]
    fn anything_goes_once_the_lock_has_ended() {
        let old = Settings::default();
        let mut new = old.clone();
        set_limit(&mut new, "youtube.com", 60);
        let end = locked().ends_at;
        assert_eq!(validate_change(&old, &new, Some(&locked()), end), Ok(()));
        assert!(!locked().is_active(end));
        assert!(locked().is_active(end - Duration::seconds(1)));
    }

    // Input validation

    #[test]
    fn defaults_are_valid() {
        assert_eq!(validate_settings(&Settings::default()), Ok(()));
    }

    #[test]
    fn bad_settings_are_rejected() {
        let d = Settings::default;
        let mut high = d();
        set_limit(&mut high, "youtube.com", 181);
        assert_eq!(
            validate_settings(&high),
            Err(InvalidSettings::AllowanceTooHigh)
        );
        let no_focus = Settings {
            pomodoro_focus_min: 0,
            ..d()
        };
        assert_eq!(
            validate_settings(&no_focus),
            Err(InvalidSettings::NoFocusLength)
        );
        let target = Settings {
            daily_focus_target_min: 1441,
            ..d()
        };
        assert_eq!(
            validate_settings(&target),
            Err(InvalidSettings::TargetTooHigh)
        );
        let mut twice = d();
        twice.allowance_sites.push(AllowanceSite {
            domain: "youtube.com".to_string(),
            daily_min: 60,
        });
        assert_eq!(
            validate_settings(&twice),
            Err(InvalidSettings::DuplicateSite("youtube.com".to_string()))
        );
    }

    #[test]
    fn unclean_domains_cannot_be_used_to_sneak_a_site_out() {
        let mut s = Settings::default();
        s.always_blocked[0] = "WWW.TikTok.com".to_string();
        assert_eq!(
            validate_settings(&s),
            Err(InvalidSettings::BadDomain("WWW.TikTok.com".to_string()))
        );
    }

    // Tamper-safe clock

    #[test]
    fn effective_now_never_goes_backwards() {
        let later = t0() + Duration::hours(1);
        assert_eq!(effective_now(t0(), later), later);
        assert_eq!(effective_now(later, t0()), later);
    }

    #[test]
    fn clock_rollback_does_not_end_a_lock_early() {
        // The lock has 1 day left, and the clock is turned back a month: still locked either way.
        let lock = locked();
        let last_seen = lock.ends_at - Duration::days(1);
        let rolled_back = last_seen - Duration::days(30);
        assert!(lock.is_active(effective_now(rolled_back, last_seen)));

        // A lock that has ended stays ended, rather than coming back.
        assert!(!lock.is_active(effective_now(rolled_back, lock.ends_at)));
    }

    #[test]
    fn clock_rollback_does_not_reset_the_allowance() {
        let now = Now::new(t0(), &Utc);
        let mut state = State::new(now.app_day);
        state
            .today
            .site_used_sec
            .insert("youtube.com".to_string(), 1800);

        let rolled_back = t0() - Duration::days(30);
        let safe = Now::new(effective_now(rolled_back, t0()), &Utc);
        tick(&mut state, 1, safe, None);

        assert_eq!(state.today.site_used("youtube.com"), 1800);
        let blocked = compute_blocked(&state, safe, true);
        assert!(blocked.hosts_domains.iter().any(|d| d == "youtube.com"));
    }

    // Day X of Y

    #[test]
    fn counts_lock_days() {
        let lock = locked();
        let day = |after: Duration| lock_day(&lock, t0() + after);
        assert_eq!(day(Duration::zero()), Some(LockDay { day: 1, total: 90 }));
        assert_eq!(
            day(Duration::hours(23)),
            Some(LockDay { day: 1, total: 90 })
        );
        assert_eq!(day(Duration::days(1)), Some(LockDay { day: 2, total: 90 }));
        assert_eq!(
            day(Duration::days(90) - Duration::seconds(1)),
            Some(LockDay { day: 90, total: 90 })
        );
        assert_eq!(day(Duration::days(90)), None);
    }

    #[test]
    fn a_corrupt_lock_ending_before_it_starts_does_not_panic() {
        let lock = Lock {
            created_at: t0(),
            ends_at: t0() - Duration::days(1),
        };
        let now = t0() - Duration::days(2);
        assert_eq!(lock_day(&lock, now), Some(LockDay { day: 1, total: 1 }));
    }

    #[test]
    fn a_partial_last_day_still_counts_as_a_day() {
        let lock = Lock {
            created_at: t0(),
            ends_at: t0() + Duration::hours(36),
        };
        assert_eq!(lock_day(&lock, t0()), Some(LockDay { day: 1, total: 2 }));
    }
}
