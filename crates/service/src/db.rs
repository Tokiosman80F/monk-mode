//! SQLite storage. The schema version lives in SQLite's own `user_version`.

use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use monk_core::models::{FocusKind, FocusSession, Lock, Priority, Settings, Task, UsageDay};
use monk_core::planner::{SmallItem, MAX_TASKS_PER_DAY};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

const MIGRATIONS: [&str; 3] = [
    include_str!("../migrations/001_init.sql"),
    include_str!("../migrations/002_tasks_and_limits.sql"),
    include_str!("../migrations/003_planner.sql"),
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

fn parse_date(text: &str) -> Result<NaiveDate> {
    text.parse().map_err(|_| DbError::BadDate(text.to_string()))
}

fn priority_text(priority: Priority) -> &'static str {
    match priority {
        Priority::High => "high",
        Priority::Medium => "medium",
        Priority::Low => "low",
    }
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

    /// The tasks of one day. Other days stay in the table as the archive.
    pub fn load_tasks(&self, date: NaiveDate) -> Result<Vec<Task>> {
        self.query_tasks(date, "status != 'dropped'")
    }

    /// A day's unfinished tasks, for the next morning's prompt.
    pub fn load_open_tasks(&self, date: NaiveDate) -> Result<Vec<Task>> {
        self.query_tasks(date, "status = 'open'")
    }

    fn query_tasks(&self, date: NaiveDate, filter: &str) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, title, priority, status, done_at, estimate_min, subtasks, focus_area,
                    carried_from_date
             FROM task WHERE date = ?1 AND {filter} ORDER BY id"
        ))?;
        let rows = stmt.query_map([date.to_string()], |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, u32>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?,
            ))
        })?;
        let mut tasks = Vec::new();
        for row in rows {
            let (id, title, priority, status, done_at, estimate_min, subtasks, focus_area, carried) =
                row?;
            tasks.push(Task {
                id,
                title,
                priority: serde_json::from_value(priority.into())?,
                done: status == "done",
                done_on: done_at.as_deref().map(parse_date).transpose()?,
                allocated_min: estimate_min,
                subtasks: serde_json::from_str(&subtasks)?,
                focus_area,
                carried_from_date: carried.as_deref().map(parse_date).transpose()?,
            });
        }
        Ok(tasks)
    }

    /// Moves an unfinished task to another day, remembering the day it started on.
    pub fn move_task(&self, id: u32, to: NaiveDate) -> Result<()> {
        self.conn.execute(
            "UPDATE task SET carried_from_date = COALESCE(carried_from_date, date), date = ?2
             WHERE id = ?1 AND status = 'open'",
            params![id, to.to_string()],
        )?;
        Ok(())
    }

    /// Keeps the task in the history, off every list.
    pub fn drop_task(&self, id: u32) -> Result<()> {
        self.conn
            .execute("UPDATE task SET status = 'dropped' WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Records the first time a day's plan was confirmed.
    pub fn mark_planned(&self, date: NaiveDate, at: DateTime<Utc>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO day_log (date, planned_at) VALUES (?1, ?2)
             ON CONFLICT (date) DO UPDATE SET planned_at = COALESCE(planned_at, excluded.planned_at)",
            params![date.to_string(), at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn is_planned(&self, date: NaiveDate) -> Result<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT planned_at IS NOT NULL FROM day_log WHERE date = ?1",
                [date.to_string()],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(false))
    }

    /// The day's row from a saved monthly goal: its target, or "Rest day".
    pub fn load_target(&self, date: NaiveDate) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT CASE WHEN d.status = 'rest' THEN 'Rest day' ELSE d.target END
                 FROM goal_day d JOIN goal g ON g.id = d.goal_id
                 WHERE g.level = 'month' AND g.locked_at IS NOT NULL AND d.date = ?1
                   AND (d.target != '' OR d.status = 'rest')
                 LIMIT 1",
                [date.to_string()],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Replaces one day's tasks, leaving every other day alone.
    pub fn save_tasks(&self, date: NaiveDate, tasks: &[Task]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let day = date.to_string();
        let ids: Vec<u32> = tasks.iter().map(|t| t.id).collect();
        // Removals first, so the 4-a-day trigger counts the list being saved.
        tx.execute(
            "DELETE FROM task WHERE date = ?1 AND id NOT IN (SELECT value FROM json_each(?2))",
            params![day, serde_json::to_string(&ids)?],
        )?;
        for task in tasks {
            let fields = params![
                task.id,
                day,
                task.title,
                priority_text(task.priority),
                if task.done { "done" } else { "open" },
                task.done_on.map(|d| d.to_string()),
                task.allocated_min,
                serde_json::to_string(&task.subtasks)?,
                task.focus_area,
                task.carried_from_date.map(|d| d.to_string())
            ];
            // An update, not an upsert: the insert trigger would count the row being replaced.
            let updated = tx.execute(
                "UPDATE task SET title = ?3, priority = ?4, status = ?5, done_at = ?6,
                     estimate_min = ?7, subtasks = ?8, focus_area = ?9, carried_from_date = ?10
                 WHERE id = ?1 AND date = ?2",
                fields,
            )?;
            if updated == 0 {
                tx.execute(
                    "INSERT INTO task (id, date, title, priority, status, done_at, estimate_min,
                         subtasks, focus_area, carried_from_date)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    fields,
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Moves the one-list tasks of older versions into the table: finished ones to the day they
    /// were finished, the rest to today. More than a day's limit go to yesterday, so the morning
    /// prompt can offer them.
    pub fn import_old_tasks(&self, today: NaiveDate) -> Result<()> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = 'tasks'", [], |r| {
                r.get(0)
            })
            .optional()?;
        let Some(json) = json else { return Ok(()) };
        let tasks: Vec<Task> = serde_json::from_str(&json)?;
        let mut days: std::collections::BTreeMap<NaiveDate, Vec<Task>> = Default::default();
        for task in tasks {
            let date = match task.done_on {
                Some(day) if task.done && day < today => day,
                _ => today,
            };
            days.entry(date).or_default().push(task);
        }
        if let Some(todays) = days.get_mut(&today) {
            if todays.len() > MAX_TASKS_PER_DAY {
                let extra = todays.split_off(MAX_TASKS_PER_DAY);
                let yesterday = today.pred_opt().unwrap_or(today);
                days.entry(yesterday).or_default().extend(extra);
            }
        }
        for (date, tasks) in &days {
            // A day can't take more than its limit, so extras past it are left out.
            let fits = &tasks[..tasks.len().min(MAX_TASKS_PER_DAY)];
            self.save_tasks(*date, fits)?;
        }
        self.conn
            .execute("DELETE FROM meta WHERE key = 'tasks'", [])?;
        Ok(())
    }

    pub fn load_small_items(&self, date: NaiveDate) -> Result<Vec<SmallItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, done, carried_from_date FROM small_item WHERE date = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([date.to_string()], |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?;
        let mut items = Vec::new();
        for row in rows {
            let (id, title, done, carried) = row?;
            items.push(SmallItem {
                id,
                date,
                title,
                done,
                carried_from_date: carried.as_deref().map(parse_date).transpose()?,
            });
        }
        Ok(items)
    }

    pub fn save_small_items(&self, date: NaiveDate, items: &[SmallItem]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let ids: Vec<u32> = items.iter().map(|i| i.id).collect();
        tx.execute(
            "DELETE FROM small_item WHERE date = ?1 AND id NOT IN (SELECT value FROM json_each(?2))",
            params![date.to_string(), serde_json::to_string(&ids)?],
        )?;
        for item in items {
            tx.execute(
                "INSERT INTO small_item (id, date, title, done, carried_from_date)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (id) DO UPDATE SET date = excluded.date, title = excluded.title,
                     done = excluded.done, carried_from_date = excluded.carried_from_date",
                params![
                    item.id,
                    date.to_string(),
                    item.title,
                    item.done,
                    item.carried_from_date.map(|d| d.to_string())
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Moves unticked small work from earlier days to `today`. Safe to run more than once, and
    /// catches up if the service was off at 06:00.
    pub fn carry_small_items(&self, today: NaiveDate) -> Result<()> {
        self.conn.execute(
            "UPDATE small_item SET carried_from_date = COALESCE(carried_from_date, date), date = ?1
             WHERE done = 0 AND date < ?1",
            [today.to_string()],
        )?;
        Ok(())
    }

    /// The highest id any task or small item has used.
    pub fn max_item_id(&self) -> Result<u32> {
        Ok(self.conn.query_row(
            "SELECT MAX(COALESCE((SELECT MAX(id) FROM task), 0),
                        COALESCE((SELECT MAX(id) FROM small_item), 0))",
            [],
            |r| r.get(0),
        )?)
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

    fn run(db: &Db, sql: &str) -> std::result::Result<usize, String> {
        db.conn.execute(sql, []).map_err(|e| e.to_string())
    }

    fn add_task(db: &Db, id: u32, date: &str) -> std::result::Result<usize, String> {
        run(
            db,
            &format!("INSERT INTO task (id, date, title) VALUES ({id}, '{date}', 'T')"),
        )
    }

    #[test]
    fn the_database_refuses_a_fifth_task_on_a_day() {
        let (_dir, db) = temp_db();
        for id in 1..=4 {
            add_task(&db, id, "2026-11-12").unwrap();
        }
        let fifth = add_task(&db, 5, "2026-11-12");
        assert!(fifth.unwrap_err().contains("at most 4 tasks"));
        // Another day is fine, but moving a task onto a full day is not.
        add_task(&db, 5, "2026-11-13").unwrap();
        let moved = run(&db, "UPDATE task SET date = '2026-11-12' WHERE id = 5");
        assert!(moved.unwrap_err().contains("at most 4 tasks"));
        // A dropped task frees its place.
        run(&db, "UPDATE task SET status = 'dropped' WHERE id = 1").unwrap();
        run(&db, "UPDATE task SET date = '2026-11-12' WHERE id = 5").unwrap();
    }

    #[test]
    fn the_database_refuses_to_change_a_saved_goal() {
        let (_dir, db) = temp_db();
        run(
            &db,
            "INSERT INTO goal (id, level, period_start, period_end, title)
             VALUES (1, 'month', '2026-11-12', '2026-11-30', 'Ship it')",
        )
        .unwrap();
        run(
            &db,
            "INSERT INTO goal_day (goal_id, date, target) VALUES (1, '2026-11-12', 'Plan')",
        )
        .unwrap();
        // Still a draft, so it can change.
        run(&db, "UPDATE goal_day SET target = 'Plan v2'").unwrap();
        run(&db, "UPDATE goal SET locked_at = '2026-11-11T12:00:00Z'").unwrap();

        for sql in [
            "UPDATE goal SET title = 'Easier'",
            "DELETE FROM goal",
            "UPDATE goal_day SET target = 'Rest'",
            "DELETE FROM goal_day",
            "INSERT INTO goal_day (goal_id, date, target) VALUES (1, '2026-11-13', 'x')",
        ] {
            assert!(
                run(&db, sql).unwrap_err().contains("can't be changed"),
                "{sql}"
            );
        }
        // Marking how the day went is still allowed.
        run(&db, "UPDATE goal_day SET status = 'done'").unwrap();
    }

    fn stored_task(id: u32, done_on: Option<NaiveDate>) -> Task {
        Task {
            id,
            title: format!("Task {id}"),
            priority: Priority::High,
            done: done_on.is_some(),
            done_on,
            allocated_min: 60,
            subtasks: vec![],
            focus_area: None,
            carried_from_date: None,
        }
    }

    #[test]
    fn tasks_are_kept_per_day_and_old_days_stay_archived() {
        let (_dir, db) = temp_db();
        let monday = vec![stored_task(1, Some(date(9))), stored_task(2, None)];
        db.save_tasks(date(9), &monday).unwrap();
        db.save_tasks(date(10), &[stored_task(3, None)]).unwrap();
        assert_eq!(db.load_tasks(date(9)).unwrap(), monday);
        assert_eq!(db.load_tasks(date(10)).unwrap().len(), 1);
        assert_eq!(db.max_item_id().unwrap(), 3);
        // Saving a day's list changes only that day.
        db.save_tasks(date(10), &[]).unwrap();
        assert_eq!(db.load_tasks(date(9)).unwrap(), monday);
    }

    #[test]
    fn a_full_day_can_still_be_edited() {
        let (_dir, db) = temp_db();
        let mut four: Vec<_> = (1..=4).map(|id| stored_task(id, None)).collect();
        db.save_tasks(date(9), &four).unwrap();
        four[0].done = true;
        four[3].title = "Renamed".to_string();
        db.save_tasks(date(9), &four).unwrap();
        assert_eq!(db.load_tasks(date(9)).unwrap(), four);
        // Swapping one task for another works too.
        four[1] = stored_task(5, None);
        db.save_tasks(date(9), &four).unwrap();
        four.sort_by_key(|t| t.id);
        assert_eq!(db.load_tasks(date(9)).unwrap(), four);
        four.push(stored_task(6, None));
        assert!(db.save_tasks(date(9), &four).is_err());
    }

    #[test]
    fn old_single_list_tasks_move_into_the_table() {
        let (_dir, db) = temp_db();
        let old = vec![
            stored_task(1, None),
            stored_task(2, Some(date(8))),
            stored_task(3, Some(date(9))),
        ];
        db.conn
            .execute(
                "INSERT INTO meta (key, value) VALUES ('tasks', ?1)",
                [serde_json::to_string(&old).unwrap()],
            )
            .unwrap();
        db.import_old_tasks(date(9)).unwrap();
        assert_eq!(db.load_tasks(date(8)).unwrap(), [old[1].clone()]);
        assert_eq!(
            db.load_tasks(date(9)).unwrap(),
            [old[0].clone(), old[2].clone()]
        );
        // Running it again does nothing.
        db.import_old_tasks(date(9)).unwrap();
        assert_eq!(db.load_tasks(date(9)).unwrap().len(), 2);
    }

    #[test]
    fn a_task_can_move_to_a_later_day_or_be_dropped() {
        let (_dir, db) = temp_db();
        let mut area = stored_task(1, None);
        area.focus_area = Some("React".to_string());
        db.save_tasks(date(9), &[area.clone(), stored_task(2, None)])
            .unwrap();
        assert_eq!(
            db.load_open_tasks(date(9)).unwrap()[0]
                .focus_area
                .as_deref(),
            Some("React")
        );

        db.move_task(1, date(12)).unwrap();
        let moved = &db.load_tasks(date(12)).unwrap()[0];
        assert_eq!(moved.carried_from_date, Some(date(9)));
        db.drop_task(2).unwrap();
        assert!(db.load_tasks(date(9)).unwrap().is_empty());
        assert!(db.load_open_tasks(date(9)).unwrap().is_empty());
        // Dropped, not deleted.
        let kept: u32 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM task WHERE status = 'dropped'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, 1);
    }

    #[test]
    fn a_plan_is_recorded_once_and_a_saved_goal_gives_the_day_target() {
        let (_dir, db) = temp_db();
        assert!(!db.is_planned(date(9)).unwrap());
        db.mark_planned(date(9), t(9)).unwrap();
        db.mark_planned(date(9), t(10)).unwrap();
        assert!(db.is_planned(date(9)).unwrap());
        let at: String = db
            .conn
            .query_row("SELECT planned_at FROM day_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(parse_time(&at).unwrap(), t(9));

        db.conn
            .execute_batch(
                "INSERT INTO goal (id, level, period_start, period_end, title)
                 VALUES (1, 'month', '2026-11-09', '2026-11-30', 'Ship');
                 INSERT INTO goal_day (goal_id, date, target) VALUES (1, '2026-11-09', 'Hooks');
                 INSERT INTO goal_day (goal_id, date, target, status)
                     VALUES (1, '2026-11-10', '', 'rest');",
            )
            .unwrap();
        // A draft goal gives nothing.
        assert_eq!(db.load_target(date(9)).unwrap(), None);
        db.conn
            .execute("UPDATE goal SET locked_at = '2026-11-09T00:00:00Z'", [])
            .unwrap();
        assert_eq!(db.load_target(date(9)).unwrap().as_deref(), Some("Hooks"));
        assert_eq!(
            db.load_target(date(10)).unwrap().as_deref(),
            Some("Rest day")
        );
        assert_eq!(db.load_target(date(11)).unwrap(), None);
    }

    #[test]
    fn unticked_small_work_carries_to_today_once() {
        let (_dir, db) = temp_db();
        let item = |id, done| SmallItem {
            id,
            date: date(9),
            title: format!("Call {id}"),
            done,
            carried_from_date: None,
        };
        db.save_small_items(date(9), &[item(1, false), item(2, true)])
            .unwrap();
        db.carry_small_items(date(11)).unwrap();
        db.carry_small_items(date(11)).unwrap();
        let today = db.load_small_items(date(11)).unwrap();
        assert_eq!(today.len(), 1);
        assert_eq!(today[0].id, 1);
        assert_eq!(today[0].carried_from_date, Some(date(9)));
        // The ticked one stays on its own day.
        assert_eq!(db.load_small_items(date(9)).unwrap()[0].id, 2);
    }

    #[test]
    fn a_day_gets_one_penalty_at_most() {
        let (_dir, db) = temp_db();
        let penalty = "INSERT INTO penalty (date, reason, days_added, created_at)
                       VALUES ('2026-11-12', 'no_plan', 1, '2026-11-13T00:00:00Z')";
        run(&db, penalty).unwrap();
        assert!(run(&db, penalty).is_err());
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
        assert!(db.load_tasks(date(5)).unwrap().is_empty());
        let task = Task {
            id: 1,
            title: "Ship".to_string(),
            priority: monk_core::models::Priority::High,
            done: false,
            done_on: None,
            allocated_min: 180,
            subtasks: vec![],
            focus_area: None,
            carried_from_date: None,
        };
        db.save_tasks(date(5), std::slice::from_ref(&task)).unwrap();
        assert_eq!(db.load_tasks(date(5)).unwrap(), [task]);
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
