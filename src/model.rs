use chrono::{DateTime, Local, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutineFrequency {
    Daily,
    Weekly(Weekday),
}

impl RoutineFrequency {
    pub fn as_str(&self) -> String {
        match self {
            RoutineFrequency::Daily => "DAILY".to_string(),
            RoutineFrequency::Weekly(w) => format!("WEEKLY_{:?}", w).to_uppercase(),
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        let upper = s.trim().to_uppercase();
        if upper == "DAILY" || upper == "DAY" || upper == "D" {
            return RoutineFrequency::Daily;
        }

        let weekday_part = upper.strip_prefix("WEEKLY_").unwrap_or(&upper);
        let weekday = match weekday_part {
            "MON" | "MONDAY" | "月" => Weekday::Mon,
            "TUE" | "TUESDAY" | "火" => Weekday::Tue,
            "WED" | "WEDNESDAY" | "水" => Weekday::Wed,
            "THU" | "THURSDAY" | "木" => Weekday::Thu,
            "FRI" | "FRIDAY" | "金" => Weekday::Fri,
            "SAT" | "SATURDAY" | "土" => Weekday::Sat,
            "SUN" | "SUNDAY" | "日" => Weekday::Sun,
            _ => Weekday::Mon,
        };
        RoutineFrequency::Weekly(weekday)
    }

    pub fn display_label(&self) -> String {
        match self {
            RoutineFrequency::Daily => "毎日 (Daily)".to_string(),
            RoutineFrequency::Weekly(Weekday::Mon) => "毎週月曜 (Mon)".to_string(),
            RoutineFrequency::Weekly(Weekday::Tue) => "毎週火曜 (Tue)".to_string(),
            RoutineFrequency::Weekly(Weekday::Wed) => "毎週水曜 (Wed)".to_string(),
            RoutineFrequency::Weekly(Weekday::Thu) => "毎週木曜 (Thu)".to_string(),
            RoutineFrequency::Weekly(Weekday::Fri) => "毎週金曜 (Fri)".to_string(),
            RoutineFrequency::Weekly(Weekday::Sat) => "毎週土曜 (Sat)".to_string(),
            RoutineFrequency::Weekly(Weekday::Sun) => "毎週日曜 (Sun)".to_string(),
        }
    }

    pub fn next(&self) -> Self {
        match self {
            RoutineFrequency::Daily => RoutineFrequency::Weekly(Weekday::Mon),
            RoutineFrequency::Weekly(Weekday::Mon) => RoutineFrequency::Weekly(Weekday::Tue),
            RoutineFrequency::Weekly(Weekday::Tue) => RoutineFrequency::Weekly(Weekday::Wed),
            RoutineFrequency::Weekly(Weekday::Wed) => RoutineFrequency::Weekly(Weekday::Thu),
            RoutineFrequency::Weekly(Weekday::Thu) => RoutineFrequency::Weekly(Weekday::Fri),
            RoutineFrequency::Weekly(Weekday::Fri) => RoutineFrequency::Weekly(Weekday::Sat),
            RoutineFrequency::Weekly(Weekday::Sat) => RoutineFrequency::Weekly(Weekday::Sun),
            RoutineFrequency::Weekly(Weekday::Sun) => RoutineFrequency::Daily,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            RoutineFrequency::Daily => RoutineFrequency::Weekly(Weekday::Sun),
            RoutineFrequency::Weekly(Weekday::Mon) => RoutineFrequency::Daily,
            RoutineFrequency::Weekly(Weekday::Tue) => RoutineFrequency::Weekly(Weekday::Mon),
            RoutineFrequency::Weekly(Weekday::Wed) => RoutineFrequency::Weekly(Weekday::Tue),
            RoutineFrequency::Weekly(Weekday::Thu) => RoutineFrequency::Weekly(Weekday::Wed),
            RoutineFrequency::Weekly(Weekday::Fri) => RoutineFrequency::Weekly(Weekday::Thu),
            RoutineFrequency::Weekly(Weekday::Sat) => RoutineFrequency::Weekly(Weekday::Fri),
            RoutineFrequency::Weekly(Weekday::Sun) => RoutineFrequency::Weekly(Weekday::Sat),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Routine {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub frequency: RoutineFrequency,
    pub last_generated_date: Option<NaiveDate>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "TODO",
            TaskStatus::InProgress => "IN_PROGRESS",
            TaskStatus::Done => "DONE",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "IN_PROGRESS" | "INPROGRESS" | "DOING" => TaskStatus::InProgress,
            "DONE" | "COMPLETED" => TaskStatus::Done,
            _ => TaskStatus::Todo,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            TaskStatus::Todo => TaskStatus::InProgress,
            TaskStatus::InProgress => TaskStatus::Done,
            TaskStatus::Done => TaskStatus::Todo,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            TaskStatus::Todo => TaskStatus::Done,
            TaskStatus::InProgress => TaskStatus::Todo,
            TaskStatus::Done => TaskStatus::InProgress,
        }
    }

    pub fn indicator(&self) -> &'static str {
        match self {
            TaskStatus::Todo => " ",
            TaskStatus::InProgress => ">",
            TaskStatus::Done => "x",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Low => "LOW",
            Priority::Medium => "MEDIUM",
            Priority::High => "HIGH",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "HIGH" | "H" | "3" => Priority::High,
            "LOW" | "L" | "1" => Priority::Low,
            _ => Priority::Medium,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Priority::Low => Priority::Medium,
            Priority::Medium => Priority::High,
            Priority::High => Priority::Low,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Priority::Low => Priority::High,
            Priority::Medium => Priority::Low,
            Priority::High => Priority::Medium,
        }
    }

    pub fn indicator(&self) -> &'static str {
        match self {
            Priority::Low => "!",
            Priority::Medium => "!!",
            Priority::High => "!!!",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub due_date: Option<NaiveDate>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
    pub completed_at: Option<DateTime<Local>>,
    pub routine_id: Option<i64>,
}

impl Task {
    pub fn is_archived(&self, archive_after_days: i64, now: DateTime<Local>) -> bool {
        if self.status != TaskStatus::Done {
            return false;
        }
        if let Some(completed_at) = self.completed_at {
            let threshold = chrono::Duration::days(archive_after_days);
            (now - completed_at) >= threshold
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    #[test]
    fn test_routine_frequency_cycle() {
        let freq = RoutineFrequency::Daily;
        assert_eq!(freq.next(), RoutineFrequency::Weekly(Weekday::Mon));
        assert_eq!(freq.next().next(), RoutineFrequency::Weekly(Weekday::Tue));
        assert_eq!(freq.prev(), RoutineFrequency::Weekly(Weekday::Sun));

        assert_eq!(
            RoutineFrequency::from_str_loose("daily"),
            RoutineFrequency::Daily
        );
        assert_eq!(
            RoutineFrequency::from_str_loose("weekly_fri"),
            RoutineFrequency::Weekly(Weekday::Fri)
        );
    }

    #[test]
    fn test_status_cycle() {
        assert_eq!(TaskStatus::Todo.next(), TaskStatus::InProgress);
        assert_eq!(TaskStatus::InProgress.next(), TaskStatus::Done);
        assert_eq!(TaskStatus::Done.next(), TaskStatus::Todo);

        assert_eq!(TaskStatus::Todo.prev(), TaskStatus::Done);
        assert_eq!(TaskStatus::InProgress.prev(), TaskStatus::Todo);
        assert_eq!(TaskStatus::Done.prev(), TaskStatus::InProgress);
    }

    #[test]
    fn test_priority_cycle() {
        assert_eq!(Priority::Low.next(), Priority::Medium);
        assert_eq!(Priority::Medium.next(), Priority::High);
        assert_eq!(Priority::High.next(), Priority::Low);

        assert_eq!(Priority::Low.prev(), Priority::High);
        assert_eq!(Priority::Medium.prev(), Priority::Low);
        assert_eq!(Priority::High.prev(), Priority::Medium);
    }

    #[test]
    fn test_is_archived() {
        let now = Local.with_ymd_and_hms(2026, 10, 29, 12, 0, 0).unwrap();
        let mut task = Task {
            id: 1,
            title: "Test".to_string(),
            description: "".to_string(),
            status: TaskStatus::Done,
            priority: Priority::Medium,
            due_date: None,
            created_at: now - Duration::days(5),
            updated_at: now - Duration::days(2),
            completed_at: Some(now - Duration::hours(25)),
            routine_id: None,
        };

        // 1 day elapsed, threshold 1 day -> archived
        assert!(task.is_archived(1, now));

        // threshold 2 days -> not yet archived
        assert!(!task.is_archived(2, now));

        // If status is not Done, never archived even if completed_at was set
        task.status = TaskStatus::Todo;
        assert!(!task.is_archived(1, now));
    }
}
