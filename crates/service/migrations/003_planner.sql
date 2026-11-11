-- The planner. Dates are app days (06:00 to 05:59 local time). The triggers back up the
-- service's own checks, so a 5th task or an edit to a saved goal fails even from a bad caller.

CREATE TABLE goal (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    level TEXT NOT NULL CHECK (level IN ('month', 'week')),
    period_start TEXT NOT NULL,
    period_end TEXT NOT NULL,
    title TEXT NOT NULL,
    locked_at TEXT,
    CHECK (period_end >= period_start)
);

CREATE TABLE goal_day (
    goal_id INTEGER NOT NULL REFERENCES goal (id),
    date TEXT NOT NULL,
    target TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN ('planned', 'done', 'partial', 'missed', 'rest')),
    PRIMARY KEY (goal_id, date)
);

CREATE TABLE task (
    id INTEGER PRIMARY KEY,
    date TEXT NOT NULL,
    title TEXT NOT NULL,
    focus_area TEXT,
    estimate_min INTEGER NOT NULL DEFAULT 0,
    priority TEXT NOT NULL DEFAULT 'medium',
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'done', 'dropped')),
    subtasks TEXT NOT NULL DEFAULT '[]',
    carried_from_date TEXT,
    done_at TEXT
);
CREATE INDEX task_by_date ON task (date);

CREATE TABLE small_item (
    id INTEGER PRIMARY KEY,
    date TEXT NOT NULL,
    title TEXT NOT NULL,
    done INTEGER NOT NULL DEFAULT 0,
    carried_from_date TEXT
);
CREATE INDEX small_item_by_date ON small_item (date);

CREATE TABLE day_log (
    date TEXT PRIMARY KEY,
    planned_at TEXT,
    checkin_note TEXT,
    completion_pct INTEGER CHECK (completion_pct BETWEEN 0 AND 100)
);

CREATE TABLE penalty (
    date TEXT PRIMARY KEY,
    reason TEXT NOT NULL CHECK (reason IN ('no_plan')),
    days_added INTEGER NOT NULL CHECK (days_added > 0),
    lock_end_before TEXT,
    lock_end_after TEXT,
    created_at TEXT NOT NULL
);

CREATE TRIGGER task_at_most_four BEFORE INSERT ON task
WHEN NEW.status != 'dropped'
    AND (SELECT COUNT(*) FROM task WHERE date = NEW.date AND status != 'dropped') >= 4
BEGIN
    SELECT RAISE(ABORT, 'A day can have at most 4 tasks.');
END;

CREATE TRIGGER task_moved_at_most_four BEFORE UPDATE OF date, status ON task
WHEN NEW.status != 'dropped'
    AND (SELECT COUNT(*) FROM task
         WHERE date = NEW.date AND status != 'dropped' AND id != NEW.id) >= 4
BEGIN
    SELECT RAISE(ABORT, 'A day can have at most 4 tasks.');
END;

CREATE TRIGGER goal_locked_update BEFORE UPDATE ON goal
WHEN OLD.locked_at IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'A saved goal can''t be changed.');
END;

CREATE TRIGGER goal_locked_delete BEFORE DELETE ON goal
WHEN OLD.locked_at IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'A saved goal can''t be changed.');
END;

CREATE TRIGGER goal_day_locked_insert BEFORE INSERT ON goal_day
WHEN (SELECT locked_at FROM goal WHERE id = NEW.goal_id) IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'A saved goal can''t be changed.');
END;

CREATE TRIGGER goal_day_locked_update BEFORE UPDATE OF goal_id, date, target ON goal_day
WHEN (SELECT locked_at FROM goal WHERE id = OLD.goal_id) IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'A saved goal can''t be changed.');
END;

CREATE TRIGGER goal_day_locked_delete BEFORE DELETE ON goal_day
WHEN (SELECT locked_at FROM goal WHERE id = OLD.goal_id) IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'A saved goal can''t be changed.');
END;
