-- Removes what was never used: goals, penalties, check-in notes and the old total of allowance
-- time from before each site had its own limit.

DROP TRIGGER goal_locked_update;
DROP TRIGGER goal_locked_delete;
DROP TRIGGER goal_day_locked_insert;
DROP TRIGGER goal_day_locked_update;
DROP TRIGGER goal_day_locked_delete;
DROP TABLE goal_day;
DROP TABLE goal;
DROP TABLE penalty;

ALTER TABLE day_log DROP COLUMN checkin_note;
ALTER TABLE day_log DROP COLUMN completion_pct;
ALTER TABLE usage_daily DROP COLUMN allowance_used_sec;
