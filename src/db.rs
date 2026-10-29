use chrono::{DateTime, Local, NaiveDate};
use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{Priority, Task, TaskStatus};

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
                completed_at TEXT
            );",
            [],
        )?;
        Ok(())
    }

    pub fn get_all_tasks(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, status, priority, due_date, created_at, updated_at, completed_at
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
        let now = Local::now();
        let now_str = now.to_rfc3339();
        let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string());
        let completed_at_str = if status == TaskStatus::Done {
            Some(now_str.clone())
        } else {
            None
        };

        self.conn.execute(
            "INSERT INTO tasks (title, description, status, priority, due_date, created_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                title,
                description,
                status.as_str(),
                priority.as_str(),
                due_date_str,
                now_str,
                now_str,
                completed_at_str
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

    pub fn set_parallel_in_progress(&self, id: i64) -> Result<()> {
        let now = Local::now().to_rfc3339();
        self.conn.execute(
            "UPDATE tasks SET status = ?1, completed_at = NULL, updated_at = ?2 WHERE id = ?3",
            params![TaskStatus::InProgress.as_str(), now, id],
        )?;
        Ok(())
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

        // Delete
        db.delete_task(id2).unwrap();
        let tasks = db.get_all_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, id1);
    }
}
