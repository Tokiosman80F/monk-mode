//! SQLite storage. The schema version lives in SQLite's own `user_version`.

use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use monk_core::models::{FocusKind, FocusSession, Lock, Settings, Task, UsageDay};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

const MIGRATIONS: [&str; 2] = [
    include_str!("../migrations/001_init.sql"),
    include_str!("../migrations/002_tasks_and_limits.sql"),
];

#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Bad date in database: {0}")]
    BadDate(String),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// `C:\ProgramData\MonkMode`, or `MONK_DATA_DIR` if set.
pub fn data_dir() -> PathBuf {
    std::env::var_os("MONK_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData\MonkMode"))
}

fn parse_time(text: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .map(|t| t.to_utc())
        .map_err(|_| DbError::BadDate(text.to_string()))
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(dir: &Path) -> Result<Self> {
        let _ = std::fs::create_dir_all(dir);
        let mut conn = Connection::open(dir.join("monk.db"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn load_settings(&self) -> Result<Option<Settings>> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT json FROM settings WHERE id = 1", [], |r| r.get(0))
            .optional()?;
        Ok(json.map(|j| Settings::from_stored(&j)).transpose()?)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (id, json) VALUES (1, ?1)",
            [serde_json::to_string(settings)?],
        )?;
        Ok(())
    }

    /// The stored lock and its signature, which the caller must check.
    pub fn load_lock(&self) -> Result<Option<(Lock, String)>> {
        let row: Option<(String, String, String)> = self
            .conn
            .query_row(
                "SELECT ends_at, created_at, signature FROM lock WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        row.map(|(ends_at, created_at, signature)| {
            let lock = Lock {
                ends_at: parse_time(&ends_at)?,
                created_at: parse_time(&created_at)?,
            };
            Ok((lock, signature))
        })
        .transpose()
    }

    pub fn save_lock(&self, lock: &Lock, signature: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO lock (id, ends_at, created_at, signature)
             VALUES (1, ?1, ?2, ?3)",
            params![
                lock.ends_at.to_rfc3339(),
                lock.created_at.to_rfc3339(),
                signature
            ],
        )?;
        Ok(())
    }

    pub fn save_day(&self, day: &UsageDay) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO usage_daily
                (date, allowance_used_sec, blocked_attempts, focus_sec,
                 site_used_sec, task_xp, tasks_done)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                day.date.to_string(),
                day.allowance_used_sec,
                day.blocked_attempts,
                serde_json::to_string(&day.focus_sec)?,
                serde_json::to_string(&day.site_used_sec)?,
                day.task_xp,
                day.tasks_done
            ],
        )?;
        Ok(())
    }

    /// The most recent `limit` days, oldest first.
    pub fn load_days(&self, limit: u32) -> Result<Vec<UsageDay>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, allowance_used_sec, blocked_attempts, focus_sec,
                    site_used_sec, task_xp, tasks_done
             FROM usage_daily ORDER BY date DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, u32>(1)?,
                r.get::<_, u32>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, u32>(5)?,
                r.get::<_, u32>(6)?,
            ))
        })?;
        let mut days = Vec::new();
        for row in rows {
            let (
                date,
                allowance_used_sec,
                blocked_attempts,
                focus_sec,
                site_used_sec,
                task_xp,
                tasks_done,
            ) = row?;
            days.push(UsageDay {
                date: date
                    .parse::<NaiveDate>()
                    .map_err(|_| DbError::BadDate(date.clone()))?,
                allowance_used_sec,
                blocked_attempts,
                focus_sec: serde_json::from_str(&focus_sec)?,
                site_used_sec: serde_json::from_str(&site_used_sec)?,
                task_xp,
                tasks_done,
            });
        }
        days.reverse();
        Ok(days)
    }

    pub fn insert_focus_session(&self, session: &FocusSession) -> Result<()> {
        let kind = match session.kind {
            FocusKind::Pomodoro => "pomodoro",
            FocusKind::Manual => "manual",
        };
        self.conn.execute(
            "INSERT INTO focus_sessions (block_name, started_at, ended_at, kind)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                session.task_title,
                session.started_at.to_rfc3339(),
                session.ended_at.to_rfc3339(),
                kind
            ],
        )?;
        Ok(())
    }

    pub fn load_tasks(&self) -> Result<Vec<Task>> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = 'tasks'", [], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(json
            .map(|j| serde_json::from_str(&j))
            .transpose()?
            .unwrap_or_default())
    }

    pub fn save_tasks(&self, tasks: &[Task]) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('tasks', ?1)",
            [serde_json::to_string(tasks)?],
        )?;
        Ok(())
    }

    pub fn load_last_seen(&self) -> Result<Option<DateTime<Utc>>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'last_seen_time'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        value.as_deref().map(parse_time).transpose()
    }

    pub fn save_last_seen(&self, time: DateTime<Utc>) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('last_seen_time', ?1)",
            [time.to_rfc3339()],
        )?;
        Ok(())
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as u32 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, 12, 0, 0).unwrap()
    }

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn temp_db() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        (dir, db)
    }

    #[test]
    fn migrations_run_on_a_fresh_dir_and_are_safe_to_repeat() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        let db = Db::open(&nested).unwrap();
        db.save_last_seen(t(1)).unwrap();
        drop(db);

        let db = Db::open(&nested).unwrap();
        assert_eq!(db.load_last_seen().unwrap(), Some(t(1)));
        let version: u32 = db
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version as usize, MIGRATIONS.len());
    }

    #[test]
    fn all_tables_exist() {
        let (_dir, db) = temp_db();
        for table in ["settings", "lock", "usage_daily", "focus_sessions", "meta"] {
            let count: i64 = db
                .conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "{table}");
        }
    }

    #[test]
    fn settings_round_trip() {
        let (_dir, db) = temp_db();
        assert_eq!(db.load_settings().unwrap(), None);
        let settings = Settings {
            pomodoro_focus_min: 5,
            ..Settings::default()
        };
        db.save_settings(&settings).unwrap();
        db.save_settings(&settings).unwrap();
        assert_eq!(db.load_settings().unwrap(), Some(settings));
    }

    #[test]
    fn lock_round_trips() {
        let (_dir, db) = temp_db();
        assert_eq!(db.load_lock().unwrap(), None);
        let lock = Lock {
            created_at: t(1),
            ends_at: t(20),
        };
        db.save_lock(&lock, "sig").unwrap();
        assert_eq!(db.load_lock().unwrap(), Some((lock, "sig".to_string())));
    }

    #[test]
    fn days_update_in_place_and_load_oldest_first() {
        let (_dir, db) = temp_db();
        for day in [3, 1, 2] {
            db.save_day(&UsageDay::new(date(day))).unwrap();
        }
        let mut updated = UsageDay::new(date(3));
        updated.allowance_used_sec = 60;
        updated.blocked_attempts = 2;
        updated.focus_sec.insert("React".to_string(), 120);
        updated.site_used_sec.insert("youtube.com".to_string(), 90);
        updated.task_xp = 50;
        updated.tasks_done = 1;
        db.save_day(&updated).unwrap();

        let days = db.load_days(10).unwrap();
        assert_eq!(days.len(), 3);
        assert_eq!(days[0].date, date(1));
        assert_eq!(days[2], updated);
        // The limit keeps the most recent days.
        let recent = db.load_days(1).unwrap();
        assert_eq!(recent, [updated]);
    }

    #[test]
    fn a_version_1_database_upgrades_and_keeps_its_data() {
        let dir = tempfile::tempdir().unwrap();
        {
            let conn = Connection::open(dir.path().join("monk.db")).unwrap();
            conn.execute_batch(MIGRATIONS[0]).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
            conn.execute(
                "INSERT INTO settings (id, json) VALUES (1, ?1)",
                [r#"{"version":1,"setup_done":true,"daily_allowance_min":10,
                    "allowance_sites":["youtube.com"],"always_blocked":["tiktok.com"],
                    "shorts_blocker_enabled":true,"study_blocks":[],"pomodoro_focus_min":50,
                    "pomodoro_break_min":10,"daily_focus_target_min":660}"#],
            )
            .unwrap();
            conn.execute(
                r#"INSERT INTO usage_daily (date, allowance_used_sec, blocked_attempts, focus_sec)
                   VALUES ('2026-11-01', 60, 2, '{"React":300}')"#,
                [],
            )
            .unwrap();
        }
        let db = Db::open(dir.path()).unwrap();
        let settings = db.load_settings().unwrap().unwrap();
        assert_eq!(settings.allowance_sites[0].domain, "youtube.com");
        assert_eq!(settings.allowance_sites[0].daily_min, 10);
        let day = &db.load_days(1).unwrap()[0];
        assert_eq!(day.focus_sec["React"], 300);
        assert_eq!(day.task_xp, 0);
    }

    #[test]
    fn tasks_round_trip() {
        let (_dir, db) = temp_db();
        assert!(db.load_tasks().unwrap().is_empty());
        let task = Task {
            id: 1,
            title: "Ship".to_string(),
            priority: monk_core::models::Priority::High,
            done: false,
            done_on: None,
            quadrant: monk_core::models::Quadrant::DoFirst,
            allocated_min: 180,
            subtasks: vec![],
        };
        db.save_tasks(std::slice::from_ref(&task)).unwrap();
        assert_eq!(db.load_tasks().unwrap(), [task]);
    }

    #[test]
    fn focus_sessions_are_recorded() {
        let (_dir, db) = temp_db();
        db.insert_focus_session(&FocusSession {
            id: None,
            task_title: "React".to_string(),
            started_at: t(1),
            ended_at: t(1),
            kind: FocusKind::Pomodoro,
        })
        .unwrap();
        let (block, kind): (String, String) = db
            .conn
            .query_row(
                "SELECT block_name, kind FROM focus_sessions WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((block.as_str(), kind.as_str()), ("React", "pomodoro"));
    }
}
