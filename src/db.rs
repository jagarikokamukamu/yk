use chrono::{DateTime, Local, NaiveDate, Weekday};
use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{
    parse_weekday_loose, weekday_to_str, Priority, Routine, RoutineFrequency, Task, TaskStatus,
};

pub fn get_db_path() -> PathBuf {
    if let Ok(val) = std::env::var("YK_DB_PATH") {
        return PathBuf::from(val);
    }

    if let Some(mut dir) = dirs::data_dir() {
        dir.push("yk");
        dir.push("tasks.db");
        return dir;
    }

    PathBuf::from("tasks.db")
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open_default() -> Result<Self> {
        let path = get_db_path();
        Self::open(&path)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let conn = Connection::open(path_ref)?;
        let db = Db { conn };
        db.init()?;
        Ok(db)
    }

    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Db { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                due_date TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
                routine_id INTEGER
            );",
            [],
        )?;

        // Ensure routine_id exists for existing tables
        let _ = self
            .conn
            .execute("ALTER TABLE tasks ADD COLUMN routine_id INTEGER;", []);

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS routines (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                priority TEXT NOT NULL,
                frequency TEXT NOT NULL,
                due_weekday TEXT,
                last_generated_date TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
            [],
        )?;

        // Ensure due_weekday exists for existing routines tables
        let _ = self
            .conn
            .execute("ALTER TABLE routines ADD COLUMN due_weekday TEXT;", []);

        Ok(())
    }

    pub fn get_all_tasks(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, status, priority, due_date, created_at, updated_at, completed_at, routine_id
             FROM tasks ORDER BY id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get(2)?;
            let status_str: String = row.get(3)?;
            let priority_str: String = row.get(4)?;
            let due_date_str: Option<String> = row.get(5)?;
            let created_at_str: String = row.get(6)?;
            let updated_at_str: String = row.get(7)?;
            let completed_at_str: Option<String> = row.get(8)?;
            let routine_id: Option<i64> = row.get(9)?;

            let status = TaskStatus::from_str_loose(&status_str);
            let priority = Priority::from_str_loose(&priority_str);

            let due_date =
                due_date_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());
            let completed_at = completed_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Local))
                    .ok()
            });

            Ok(Task {
                id,
                title,
                description,
                status,
                priority,
                due_date,
                created_at,
                updated_at,
                completed_at,
                routine_id,
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    pub fn add_task(
        &self,
        title: &str,
        description: &str,
        status: TaskStatus,
        priority: Priority,
        due_date: Option<NaiveDate>,
    ) -> Result<i64> {
        self.add_task_with_routine(title, description, status, priority, due_date, None)
    }

    pub fn add_task_with_routine(
        &self,
        title: &str,
        description: &str,
        status: TaskStatus,
        priority: Priority,
        due_date: Option<NaiveDate>,
        routine_id: Option<i64>,
    ) -> Result<i64> {
        let now = Local::now();
        let now_str = now.to_rfc3339();
        let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string());
        let completed_at_str = if status == TaskStatus::Done {
            Some(now_str.clone())
        } else {
            None
        };

        self.conn.execute(
            "INSERT INTO tasks (title, description, status, priority, due_date, created_at, updated_at, completed_at, routine_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                title,
                description,
                status.as_str(),
                priority.as_str(),
                due_date_str,
                now_str,
                now_str,
                completed_at_str,
                routine_id
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_task(
        &self,
        id: i64,
        title: &str,
        description: &str,
        status: TaskStatus,
        priority: Priority,
        due_date: Option<NaiveDate>,
    ) -> Result<()> {
        let now = Local::now();
        let now_str = now.to_rfc3339();
        let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string());

        // Check current status to preserve or update completed_at
        let current_completed_at: Option<String> = self
            .conn
            .query_row(
                "SELECT completed_at FROM tasks WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let new_completed_at = match status {
            TaskStatus::Done => current_completed_at.or_else(|| Some(now_str.clone())),
            _ => None,
        };

        self.conn.execute(
            "UPDATE tasks
             SET title = ?1, description = ?2, status = ?3, priority = ?4, due_date = ?5, updated_at = ?6, completed_at = ?7
             WHERE id = ?8",
            params![
                title,
                description,
                status.as_str(),
                priority.as_str(),
                due_date_str,
                now_str,
                new_completed_at,
                id
            ],
        )?;

        Ok(())
    }

    pub fn delete_task(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn toggle_completed(&self, id: i64) -> Result<TaskStatus> {
        let (current_status_str,): (String,) = self.conn.query_row(
            "SELECT status FROM tasks WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?,)),
        )?;

        let current_status = TaskStatus::from_str_loose(&current_status_str);
        let now = Local::now();
        let now_str = now.to_rfc3339();

        let (new_status, completed_at) = match current_status {
            TaskStatus::Done => (TaskStatus::Todo, None),
            _ => (TaskStatus::Done, Some(now_str.clone())),
        };

        self.conn.execute(
            "UPDATE tasks SET status = ?1, completed_at = ?2, updated_at = ?3 WHERE id = ?4",
            params![new_status.as_str(), completed_at, now_str, id],
        )?;

        Ok(new_status)
    }

    pub fn set_exclusive_in_progress(&self, id: i64) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let tx = self.conn.unchecked_transaction()?;

        // Reset all InProgress tasks to Todo
        tx.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE status = ?3 AND id != ?4",
            params![
                TaskStatus::Todo.as_str(),
                now,
                TaskStatus::InProgress.as_str(),
                id
            ],
        )?;

        // Set target task to InProgress and clear completed_at
        tx.execute(
            "UPDATE tasks SET status = ?1, completed_at = NULL, updated_at = ?2 WHERE id = ?3",
            params![TaskStatus::InProgress.as_str(), now, id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn get_task_by_id(&self, id: i64) -> Result<Option<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, status, priority, due_date, created_at, updated_at, completed_at, routine_id
             FROM tasks WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get(2)?;
            let status_str: String = row.get(3)?;
            let priority_str: String = row.get(4)?;
            let due_date_str: Option<String> = row.get(5)?;
            let created_at_str: String = row.get(6)?;
            let updated_at_str: String = row.get(7)?;
            let completed_at_str: Option<String> = row.get(8)?;
            let routine_id: Option<i64> = row.get(9)?;

            let status = TaskStatus::from_str_loose(&status_str);
            let priority = Priority::from_str_loose(&priority_str);

            let due_date =
                due_date_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());
            let completed_at = completed_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Local))
                    .ok()
            });

            Ok(Task {
                id,
                title,
                description,
                status,
                priority,
                due_date,
                created_at,
                updated_at,
                completed_at,
                routine_id,
            })
        })?;

        if let Some(task) = rows.next() {
            Ok(Some(task?))
        } else {
            Ok(None)
        }
    }

    pub fn set_parallel_in_progress(&self, id: i64) -> Result<()> {
        let now = Local::now().to_rfc3339();
        self.conn.execute(
            "UPDATE tasks SET status = ?1, completed_at = NULL, updated_at = ?2 WHERE id = ?3",
            params![TaskStatus::InProgress.as_str(), now, id],
        )?;
        Ok(())
    }

    pub fn set_done(&self, id: i64) -> Result<()> {
        let now = Local::now().to_rfc3339();
        self.conn.execute(
            "UPDATE tasks SET status = ?1, completed_at = ?2, updated_at = ?3 WHERE id = ?4",
            params![TaskStatus::Done.as_str(), now, now, id],
        )?;
        Ok(())
    }

    pub fn get_all_routines(&self) -> Result<Vec<Routine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, priority, frequency, due_weekday, last_generated_date, created_at, updated_at
             FROM routines ORDER BY id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get(2)?;
            let priority_str: String = row.get(3)?;
            let freq_str: String = row.get(4)?;
            let due_weekday_str: Option<String> = row.get(5)?;
            let last_gen_str: Option<String> = row.get(6)?;
            let created_at_str: String = row.get(7)?;
            let updated_at_str: String = row.get(8)?;

            let priority = Priority::from_str_loose(&priority_str);
            let frequency = RoutineFrequency::from_str_loose(&freq_str);
            let due_weekday = due_weekday_str.and_then(|s| parse_weekday_loose(&s));
            let last_generated_date =
                last_gen_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());

            Ok(Routine {
                id,
                title,
                description,
                priority,
                frequency,
                due_weekday,
                last_generated_date,
                created_at,
                updated_at,
            })
        })?;

        let mut routines = Vec::new();
        for r in rows {
            routines.push(r?);
        }
        Ok(routines)
    }

    pub fn get_routine_by_id(&self, id: i64) -> Result<Option<Routine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, priority, frequency, due_weekday, last_generated_date, created_at, updated_at
             FROM routines WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get(2)?;
            let priority_str: String = row.get(3)?;
            let freq_str: String = row.get(4)?;
            let due_weekday_str: Option<String> = row.get(5)?;
            let last_gen_str: Option<String> = row.get(6)?;
            let created_at_str: String = row.get(7)?;
            let updated_at_str: String = row.get(8)?;

            let priority = Priority::from_str_loose(&priority_str);
            let frequency = RoutineFrequency::from_str_loose(&freq_str);
            let due_weekday = due_weekday_str.and_then(|s| parse_weekday_loose(&s));
            let last_generated_date =
                last_gen_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());

            Ok(Routine {
                id,
                title,
                description,
                priority,
                frequency,
                due_weekday,
                last_generated_date,
                created_at,
                updated_at,
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn add_routine(
        &self,
        title: &str,
        description: &str,
        priority: Priority,
        frequency: RoutineFrequency,
        due_weekday: Option<Weekday>,
    ) -> Result<i64> {
        self.add_routine_at(
            title,
            description,
            priority,
            frequency,
            due_weekday,
            Local::now().date_naive(),
        )
    }

    pub fn add_routine_at(
        &self,
        title: &str,
        description: &str,
        priority: Priority,
        frequency: RoutineFrequency,
        due_weekday: Option<Weekday>,
        today: NaiveDate,
    ) -> Result<i64> {
        let now = Local::now().to_rfc3339();
        let due_str = weekday_to_str(due_weekday);
        let init_last_gen = Routine::latest_spawn_date(frequency, today)
            .format("%Y-%m-%d")
            .to_string();

        self.conn.execute(
            "INSERT INTO routines (title, description, priority, frequency, due_weekday, last_generated_date, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                title,
                description,
                priority.as_str(),
                frequency.as_str(),
                due_str,
                init_last_gen,
                now,
                now
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_routine(
        &self,
        id: i64,
        title: &str,
        description: &str,
        priority: Priority,
        frequency: RoutineFrequency,
        due_weekday: Option<Weekday>,
    ) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let due_str = weekday_to_str(due_weekday);
        let prev_routine = self.get_routine_by_id(id)?;

        // If frequency changed, reset last_generated_date to current latest_spawn_date
        // so that past cycles don't unexpectedly trigger.
        let new_last_gen = match prev_routine {
            Some(ref r) if r.frequency != frequency => Some(
                Routine::latest_spawn_date(frequency, Local::now().date_naive())
                    .format("%Y-%m-%d")
                    .to_string(),
            ),
            Some(r) => r
                .last_generated_date
                .map(|d| d.format("%Y-%m-%d").to_string()),
            None => None,
        };

        self.conn.execute(
            "UPDATE routines
             SET title = ?1, description = ?2, priority = ?3, frequency = ?4, due_weekday = ?5, last_generated_date = ?6, updated_at = ?7
             WHERE id = ?8",
            params![
                title,
                description,
                priority.as_str(),
                frequency.as_str(),
                due_str,
                new_last_gen,
                now,
                id
            ],
        )?;
        Ok(())
    }

    pub fn delete_routine(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM routines WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn update_routine_last_generated(&self, id: i64, date: NaiveDate) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let date_str = date.format("%Y-%m-%d").to_string();
        self.conn.execute(
            "UPDATE routines SET last_generated_date = ?1, updated_at = ?2 WHERE id = ?3",
            params![date_str, now, id],
        )?;
        Ok(())
    }

    pub fn generate_routine_instances(&self) -> Result<Vec<i64>> {
        self.generate_routine_instances_at(Local::now().date_naive())
    }

    pub fn generate_routine_instances_at(&self, today: NaiveDate) -> Result<Vec<i64>> {
        let routines = self.get_all_routines()?;
        let mut generated_ids = Vec::new();

        for routine in routines {
            let spawn_date = Routine::latest_spawn_date(routine.frequency, today);
            let should_generate = routine
                .last_generated_date
                .is_none_or(|last| last < spawn_date);

            if should_generate {
                let due_date = routine.due_date_from_spawn(spawn_date);

                let task_id = self.add_task_with_routine(
                    &routine.title,
                    &routine.description,
                    TaskStatus::Todo,
                    routine.priority,
                    due_date,
                    Some(routine.id),
                )?;

                self.update_routine_last_generated(routine.id, spawn_date)?;
                generated_ids.push(task_id);
            }
        }

        Ok(generated_ids)
    }

    pub fn generate_single_routine_instance(&self, routine_id: i64) -> Result<Option<i64>> {
        let routine = match self.get_routine_by_id(routine_id)? {
            Some(r) => r,
            None => return Ok(None),
        };

        let today = Local::now().date_naive();
        let due_date = routine.due_date_for_manual_spawn(today);

        let task_id = self.add_task_with_routine(
            &routine.title,
            &routine.description,
            TaskStatus::Todo,
            routine.priority,
            due_date,
            Some(routine.id),
        )?;

        let spawn_date = Routine::latest_spawn_date(routine.frequency, today);
        self.update_routine_last_generated(routine.id, spawn_date)?;
        Ok(Some(task_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_crud_and_status() {
        let db = Db::open_in_memory().unwrap();

        // Create
        let id1 = db
            .add_task(
                "Task 1",
                "Description 1",
                TaskStatus::Todo,
                Priority::Medium,
                Some(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()),
            )
            .unwrap();

        let id2 = db
            .add_task(
                "Task 2",
                "Description 2",
                TaskStatus::InProgress,
                Priority::High,
                None,
            )
            .unwrap();

        let tasks = db.get_all_tasks().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Task 1");
        assert_eq!(tasks[0].status, TaskStatus::Todo);
        assert_eq!(tasks[1].status, TaskStatus::InProgress);

        // Exclusive InProgress
        db.set_exclusive_in_progress(id1).unwrap();
        let tasks = db.get_all_tasks().unwrap();
        assert_eq!(tasks[0].status, TaskStatus::InProgress);
        assert_eq!(tasks[1].status, TaskStatus::Todo); // id2 reset to Todo

        // Toggle completed
        let new_status = db.toggle_completed(id1).unwrap();
        assert_eq!(new_status, TaskStatus::Done);
        let tasks = db.get_all_tasks().unwrap();
        assert_eq!(tasks[0].status, TaskStatus::Done);
        assert!(tasks[0].completed_at.is_some());

        // Toggle back to Todo
        let new_status = db.toggle_completed(id1).unwrap();
        assert_eq!(new_status, TaskStatus::Todo);
        let tasks = db.get_all_tasks().unwrap();
        assert_eq!(tasks[0].status, TaskStatus::Todo);
        assert!(tasks[0].completed_at.is_none());

        // Get by ID
        let task1 = db.get_task_by_id(id1).unwrap().unwrap();
        assert_eq!(task1.title, "Task 1");

        // Set done
        db.set_done(id1).unwrap();
        let task1_done = db.get_task_by_id(id1).unwrap().unwrap();
        assert_eq!(task1_done.status, TaskStatus::Done);
        assert!(task1_done.completed_at.is_some());

        // Delete
        db.delete_task(id2).unwrap();
        let tasks = db.get_all_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, id1);
        assert!(db.get_task_by_id(id2).unwrap().is_none());
    }

    #[test]
    fn test_db_routine_crud_and_generation() {
        let db = Db::open_in_memory().unwrap();

        // Simulate Thursday 2026-10-29
        let thu = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

        // Add a Tuesday routine on Thursday (due on Friday)
        let r_tue = db
            .add_routine_at(
                "Tuesday Task",
                "Spawn Tue, Due Fri",
                Priority::High,
                RoutineFrequency::Weekly(Weekday::Tue),
                Some(Weekday::Fri),
                thu,
            )
            .unwrap();

        // Add a Daily routine on Thursday
        let r_daily = db
            .add_routine_at(
                "Daily Review",
                "Every day",
                Priority::Medium,
                RoutineFrequency::Daily,
                None,
                thu,
            )
            .unwrap();

        // 1. On the day of creation (Thursday), auto-generation should NOT retroactively spawn Tuesday's task
        let gen_thu = db.generate_routine_instances_at(thu).unwrap();
        assert_eq!(gen_thu.len(), 0);

        // 2. Next day (Friday 2026-10-30): Daily routine spawns, Tuesday routine still waits for next Tuesday
        let fri = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let gen_fri = db.generate_routine_instances_at(fri).unwrap();
        assert_eq!(gen_fri.len(), 1);
        let tasks = db.get_all_tasks().unwrap();
        assert_eq!(tasks[0].routine_id, Some(r_daily));
        assert_eq!(tasks[0].due_date, Some(fri));

        // 3. Next Tuesday (2026-11-03): Both Daily and Tuesday routine spawn!
        let next_tue = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
        let next_fri = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        let gen_next_tue = db.generate_routine_instances_at(next_tue).unwrap();
        assert_eq!(gen_next_tue.len(), 2);

        let tasks = db.get_all_tasks().unwrap();
        let tue_task = tasks.iter().find(|t| t.routine_id == Some(r_tue)).unwrap();
        assert_eq!(tue_task.due_date, Some(next_fri));

        // 4. Second call on same Tuesday generates nothing
        assert_eq!(db.generate_routine_instances_at(next_tue).unwrap().len(), 0);

        // 5. Manual generation works anytime
        let manual_id = db.generate_single_routine_instance(r_tue).unwrap();
        assert!(manual_id.is_some());

        // Delete routine
        db.delete_routine(r_tue).unwrap();
        assert_eq!(db.get_all_routines().unwrap().len(), 1);
    }
}
