CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    json TEXT NOT NULL
);

CREATE TABLE lock (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    ends_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    signature TEXT NOT NULL
);

CREATE TABLE usage_daily (
    date TEXT PRIMARY KEY,
    allowance_used_sec INTEGER NOT NULL DEFAULT 0,
    blocked_attempts INTEGER NOT NULL DEFAULT 0,
    focus_sec TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE focus_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_name TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT NOT NULL,
    kind TEXT NOT NULL
);

CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
