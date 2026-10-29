use chrono::{Duration, Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::config::Config;
use crate::db::Db;
use crate::model::{
    next_due_weekday, prev_due_weekday, Priority, Routine, RoutineFrequency, Task, TaskStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewList {
    Active,
    Archive,
    Routine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    Title,
    Description,
    Status,
    Priority,
    DueDate,
}

impl EditField {
    pub fn next(&self) -> Self {
        match self {
            EditField::Title => EditField::Description,
            EditField::Description => EditField::Status,
            EditField::Status => EditField::Priority,
            EditField::Priority => EditField::DueDate,
            EditField::DueDate => EditField::Title,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            EditField::Title => EditField::DueDate,
            EditField::Description => EditField::Title,
            EditField::Status => EditField::Description,
            EditField::Priority => EditField::Status,
            EditField::DueDate => EditField::Priority,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditState {
    pub target_task_id: i64,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub due_date_str: String,
    pub active_field: EditField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewTaskState {
    pub task: Task,
    pub scroll_offset: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditRoutineField {
    Title,
    Frequency,
    DueWeekday,
    Priority,
    Description,
}

impl EditRoutineField {
    pub fn next(&self) -> Self {
        match self {
            EditRoutineField::Title => EditRoutineField::Frequency,
            EditRoutineField::Frequency => EditRoutineField::DueWeekday,
            EditRoutineField::DueWeekday => EditRoutineField::Priority,
            EditRoutineField::Priority => EditRoutineField::Description,
            EditRoutineField::Description => EditRoutineField::Title,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            EditRoutineField::Title => EditRoutineField::Description,
            EditRoutineField::Frequency => EditRoutineField::Title,
            EditRoutineField::DueWeekday => EditRoutineField::Frequency,
            EditRoutineField::Priority => EditRoutineField::DueWeekday,
            EditRoutineField::Description => EditRoutineField::Priority,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditRoutineState {
    pub target_routine_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub frequency: RoutineFrequency,
    pub due_weekday: Option<chrono::Weekday>,
    pub active_field: EditRoutineField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewRoutineState {
    pub routine: Routine,
    pub scroll_offset: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    NewTask(String), // input title
    EditTask(EditState),
    ViewTask(ViewTaskState),
    DeleteConfirm(Task),
    EditRoutine(EditRoutineState),
    ViewRoutine(ViewRoutineState),
    DeleteRoutineConfirm(Routine),
    Help,
}

pub struct App {
    pub db: Db,
    pub config: Config,
    pub mode: AppMode,
    pub view_list: ViewList,
    pub selected_index: usize,
    pub list_state: ListState,
    pub should_quit: bool,
}

impl App {
    pub fn new(db: Db, config: Config) -> Self {
        // Auto-generate instances for today/this week on startup
        let _ = db.generate_routine_instances();

        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            db,
            config,
            mode: AppMode::Normal,
            view_list: ViewList::Active,
            selected_index: 0,
            list_state,
            should_quit: false,
        }
    }

    pub fn current_tasks(&self) -> Vec<Task> {
        let all_tasks = self.db.get_all_tasks().unwrap_or_default();
        let now = Local::now();
        all_tasks
            .into_iter()
            .filter(|t| match self.view_list {
                ViewList::Active => !t.is_archived(self.config.archive_after_days, now),
                ViewList::Archive => t.is_archived(self.config.archive_after_days, now),
                ViewList::Routine => false,
            })
            .collect()
    }

    pub fn current_routines(&self) -> Vec<Routine> {
        self.db.get_all_routines().unwrap_or_default()
    }

    pub fn current_item_count(&self) -> usize {
        match self.view_list {
            ViewList::Routine => self.current_routines().len(),
            _ => self.current_tasks().len(),
        }
    }

    pub fn selected_task(&self) -> Option<Task> {
        let tasks = self.current_tasks();
        if tasks.is_empty() {
            None
        } else {
            let idx = self.selected_index.min(tasks.len() - 1);
            Some(tasks[idx].clone())
        }
    }

    pub fn selected_routine(&self) -> Option<Routine> {
        let routines = self.current_routines();
        if routines.is_empty() {
            None
        } else {
            let idx = self.selected_index.min(routines.len() - 1);
            Some(routines[idx].clone())
        }
    }

    pub fn move_down(&mut self) {
        let count = self.current_item_count();
        if count == 0 {
            self.selected_index = 0;
            self.list_state.select(Some(0));
            return;
        }
        if self.selected_index + 1 >= count {
            self.selected_index = 0;
        } else {
            self.selected_index += 1;
        }
        self.list_state.select(Some(self.selected_index));
    }

    pub fn move_up(&mut self) {
        let count = self.current_item_count();
        if count == 0 {
            self.selected_index = 0;
            self.list_state.select(Some(0));
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = count - 1;
        } else {
            self.selected_index -= 1;
        }
        self.list_state.select(Some(self.selected_index));
    }

    pub fn clamp_selection(&mut self) {
        let count = self.current_item_count();
        if count == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= count {
            self.selected_index = count - 1;
        }
        self.list_state.select(Some(self.selected_index));
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match &self.mode {
            AppMode::Normal => self.handle_normal_key(key),
            AppMode::NewTask(_) => self.handle_new_task_key(key),
            AppMode::EditTask(_) => self.handle_edit_key(key),
            AppMode::ViewTask(_) => self.handle_view_key(key),
            AppMode::DeleteConfirm(_) => self.handle_delete_confirm_key(key),
            AppMode::EditRoutine(_) => self.handle_edit_routine_key(key),
            AppMode::ViewRoutine(_) => self.handle_view_routine_key(key),
            AppMode::DeleteRoutineConfirm(_) => self.handle_delete_routine_confirm_key(key),
            AppMode::Help => self.handle_help_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        // Tab cycles through Active -> Archive -> Routine
        if key.code == KeyCode::Tab {
            self.view_list = match self.view_list {
                ViewList::Active => ViewList::Archive,
                ViewList::Archive => ViewList::Routine,
                ViewList::Routine => ViewList::Active,
            };
            self.clamp_selection();
            return;
        }

        // Global keybindings
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.should_quit = true;
                return;
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.move_down();
                return;
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.move_up();
                return;
            }
            (KeyModifiers::SHIFT, KeyCode::Char('A'))
            | (KeyModifiers::NONE, KeyCode::Char('A')) => {
                self.view_list = if self.view_list == ViewList::Archive {
                    ViewList::Active
                } else {
                    ViewList::Archive
                };
                self.clamp_selection();
                return;
            }
            (KeyModifiers::SHIFT, KeyCode::Char('R'))
            | (KeyModifiers::NONE, KeyCode::Char('R')) => {
                self.view_list = if self.view_list == ViewList::Routine {
                    ViewList::Active
                } else {
                    ViewList::Routine
                };
                self.clamp_selection();
                return;
            }
            (KeyModifiers::NONE, KeyCode::Char('?')) => {
                self.mode = AppMode::Help;
                return;
            }
            _ => {}
        }

        // View-specific keybindings
        if self.view_list == ViewList::Routine {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Char('a')) => {
                    self.mode = AppMode::EditRoutine(EditRoutineState {
                        target_routine_id: None,
                        title: String::new(),
                        description: String::new(),
                        priority: Priority::Medium,
                        frequency: RoutineFrequency::Weekly(chrono::Weekday::Mon),
                        due_weekday: Some(chrono::Weekday::Mon),
                        active_field: EditRoutineField::Title,
                    });
                }
                (KeyModifiers::NONE, KeyCode::Char('e')) | (KeyModifiers::NONE, KeyCode::Enter) => {
                    if let Some(r) = self.selected_routine() {
                        self.mode = AppMode::EditRoutine(EditRoutineState {
                            target_routine_id: Some(r.id),
                            title: r.title,
                            description: r.description,
                            priority: r.priority,
                            frequency: r.frequency,
                            due_weekday: r.due_weekday,
                            active_field: EditRoutineField::Title,
                        });
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('v')) => {
                    if let Some(r) = self.selected_routine() {
                        self.mode = AppMode::ViewRoutine(ViewRoutineState {
                            routine: r,
                            scroll_offset: 0,
                        });
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('d')) => {
                    if let Some(r) = self.selected_routine() {
                        self.mode = AppMode::DeleteRoutineConfirm(r);
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('p'))
                | (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                    if let Some(r) = self.selected_routine() {
                        let _ = self.db.generate_single_routine_instance(r.id);
                    }
                }
                _ => {}
            }
        } else {
            // Active / Archive Task operations
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                    if let Some(task) = self.selected_task() {
                        let _ = self.db.toggle_completed(task.id);
                        self.clamp_selection();
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('i')) => {
                    if let Some(task) = self.selected_task() {
                        let _ = self.db.set_exclusive_in_progress(task.id);
                    }
                }
                (KeyModifiers::SHIFT, KeyCode::Char('I'))
                | (KeyModifiers::NONE, KeyCode::Char('I')) => {
                    if let Some(task) = self.selected_task() {
                        let _ = self.db.set_parallel_in_progress(task.id);
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('a')) => {
                    self.mode = AppMode::NewTask(String::new());
                }
                (KeyModifiers::NONE, KeyCode::Char('e')) | (KeyModifiers::NONE, KeyCode::Enter) => {
                    if let Some(task) = self.selected_task() {
                        let due_date_str = task
                            .due_date
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .unwrap_or_default();
                        self.mode = AppMode::EditTask(EditState {
                            target_task_id: task.id,
                            title: task.title,
                            description: task.description,
                            status: task.status,
                            priority: task.priority,
                            due_date_str,
                            active_field: EditField::Title,
                        });
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('v')) => {
                    if let Some(task) = self.selected_task() {
                        self.mode = AppMode::ViewTask(ViewTaskState {
                            task,
                            scroll_offset: 0,
                        });
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('d')) => {
                    if let Some(task) = self.selected_task() {
                        self.mode = AppMode::DeleteConfirm(task);
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_new_task_key(&mut self, key: KeyEvent) {
        if let AppMode::NewTask(ref mut title) = self.mode {
            match key.code {
                KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                }
                KeyCode::Enter => {
                    let trimmed = title.trim();
                    if !trimmed.is_empty() {
                        let _ =
                            self.db
                                .add_task(trimmed, "", TaskStatus::Todo, Priority::Medium, None);
                        self.clamp_selection();
                    }
                    self.mode = AppMode::Normal;
                }
                KeyCode::Backspace => {
                    title.pop();
                }
                KeyCode::Char(c) => {
                    title.push(c);
                }
                _ => {}
            }
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) {
        let is_ctrl_enter =
            key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Enter;
        let is_tab = key.code == KeyCode::Tab;
        let is_backtab = key.code == KeyCode::BackTab;

        if let AppMode::EditTask(ref mut state) = self.mode {
            if is_ctrl_enter
                || (key.code == KeyCode::Enter && state.active_field != EditField::Description)
            {
                let due_date =
                    NaiveDate::parse_from_str(state.due_date_str.trim(), "%Y-%m-%d").ok();
                let _ = self.db.update_task(
                    state.target_task_id,
                    state.title.trim(),
                    state.description.trim(),
                    state.status,
                    state.priority,
                    due_date,
                );
                self.mode = AppMode::Normal;
                self.clamp_selection();
                return;
            }

            if key.code == KeyCode::Esc {
                self.mode = AppMode::Normal;
                return;
            }

            if is_tab {
                state.active_field = state.active_field.next();
                return;
            }
            if is_backtab {
                state.active_field = state.active_field.prev();
                return;
            }

            if state.active_field != EditField::Description {
                match key.code {
                    KeyCode::Up => {
                        state.active_field = state.active_field.prev();
                        return;
                    }
                    KeyCode::Down => {
                        state.active_field = state.active_field.next();
                        return;
                    }
                    _ => {}
                }
            }

            match state.active_field {
                EditField::Title => match key.code {
                    KeyCode::Backspace => {
                        state.title.pop();
                    }
                    KeyCode::Char(c) => {
                        state.title.push(c);
                    }
                    _ => {}
                },
                EditField::Description => match key.code {
                    KeyCode::Enter => {
                        state.description.push('\n');
                    }
                    KeyCode::Backspace => {
                        state.description.pop();
                    }
                    KeyCode::Char(c) => {
                        state.description.push(c);
                    }
                    _ => {}
                },
                EditField::Status => match key.code {
                    KeyCode::Char(' ') | KeyCode::Right => {
                        state.status = state.status.next();
                    }
                    KeyCode::Left => {
                        state.status = state.status.prev();
                    }
                    _ => {}
                },
                EditField::Priority => match key.code {
                    KeyCode::Char(' ') | KeyCode::Right => {
                        state.priority = state.priority.next();
                    }
                    KeyCode::Left => {
                        state.priority = state.priority.prev();
                    }
                    _ => {}
                },
                EditField::DueDate => match key.code {
                    KeyCode::Left => {
                        let cur_date =
                            NaiveDate::parse_from_str(state.due_date_str.trim(), "%Y-%m-%d")
                                .unwrap_or_else(|_| Local::now().date_naive());
                        let prev_date = cur_date - Duration::days(1);
                        state.due_date_str = prev_date.format("%Y-%m-%d").to_string();
                    }
                    KeyCode::Right => {
                        let cur_date =
                            NaiveDate::parse_from_str(state.due_date_str.trim(), "%Y-%m-%d")
                                .unwrap_or_else(|_| Local::now().date_naive());
                        let next_date = cur_date + Duration::days(1);
                        state.due_date_str = next_date.format("%Y-%m-%d").to_string();
                    }
                    KeyCode::Backspace => {
                        state.due_date_str.pop();
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => {
                        state.due_date_str.push(c);
                    }
                    _ => {}
                },
            }
        }
    }

    fn handle_view_key(&mut self, key: KeyEvent) {
        if let AppMode::ViewTask(ref mut state) = self.mode {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    state.scroll_offset = state.scroll_offset.saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    state.scroll_offset = state.scroll_offset.saturating_sub(1);
                }
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('v') => {
                    self.mode = AppMode::Normal;
                }
                _ => {}
            }
        }
    }

    fn handle_delete_confirm_key(&mut self, key: KeyEvent) {
        if let AppMode::DeleteConfirm(ref task) = self.mode {
            let task_id = task.id;
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let _ = self.db.delete_task(task_id);
                    self.clamp_selection();
                    self.mode = AppMode::Normal;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                }
                _ => {}
            }
        }
    }

    fn handle_edit_routine_key(&mut self, key: KeyEvent) {
        let is_ctrl_enter =
            key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Enter;
        let is_tab = key.code == KeyCode::Tab;
        let is_backtab = key.code == KeyCode::BackTab;

        if let AppMode::EditRoutine(ref mut state) = self.mode {
            if is_ctrl_enter
                || (key.code == KeyCode::Enter
                    && state.active_field != EditRoutineField::Description)
            {
                let trimmed_title = state.title.trim();
                if !trimmed_title.is_empty() {
                    let effective_due = match state.frequency {
                        RoutineFrequency::Daily => None,
                        RoutineFrequency::Weekly(_) => state.due_weekday,
                    };
                    if let Some(id) = state.target_routine_id {
                        let _ = self.db.update_routine(
                            id,
                            trimmed_title,
                            state.description.trim(),
                            state.priority,
                            state.frequency,
                            effective_due,
                        );
                    } else {
                        let _ = self.db.add_routine(
                            trimmed_title,
                            state.description.trim(),
                            state.priority,
                            state.frequency,
                            effective_due,
                        );
                    }
                }
                self.mode = AppMode::Normal;
                self.clamp_selection();
                return;
            }

            if key.code == KeyCode::Esc {
                self.mode = AppMode::Normal;
                return;
            }

            if is_tab {
                state.active_field = state.active_field.next();
                return;
            }
            if is_backtab {
                state.active_field = state.active_field.prev();
                return;
            }

            if state.active_field != EditRoutineField::Description {
                match key.code {
                    KeyCode::Up => {
                        state.active_field = state.active_field.prev();
                        return;
                    }
                    KeyCode::Down => {
                        state.active_field = state.active_field.next();
                        return;
                    }
                    _ => {}
                }
            }

            match state.active_field {
                EditRoutineField::Title => match key.code {
                    KeyCode::Backspace => {
                        state.title.pop();
                    }
                    KeyCode::Char(c) => {
                        state.title.push(c);
                    }
                    _ => {}
                },
                EditRoutineField::Frequency => {
                    let old_freq = state.frequency;
                    let changed = match key.code {
                        KeyCode::Char(' ') | KeyCode::Right => {
                            state.frequency = state.frequency.next();
                            true
                        }
                        KeyCode::Left => {
                            state.frequency = state.frequency.prev();
                            true
                        }
                        _ => false,
                    };
                    if changed {
                        if let RoutineFrequency::Weekly(new_w) = state.frequency {
                            let should_sync = match old_freq {
                                RoutineFrequency::Daily => true,
                                RoutineFrequency::Weekly(old_w) => state.due_weekday == Some(old_w),
                            };
                            if should_sync {
                                state.due_weekday = Some(new_w);
                            }
                        }
                    }
                }
                EditRoutineField::DueWeekday => {
                    if matches!(state.frequency, RoutineFrequency::Weekly(_)) {
                        match key.code {
                            KeyCode::Char(' ') | KeyCode::Right => {
                                state.due_weekday = next_due_weekday(state.due_weekday);
                            }
                            KeyCode::Left => {
                                state.due_weekday = prev_due_weekday(state.due_weekday);
                            }
                            _ => {}
                        }
                    }
                }
                EditRoutineField::Priority => match key.code {
                    KeyCode::Char(' ') | KeyCode::Right => {
                        state.priority = state.priority.next();
                    }
                    KeyCode::Left => {
                        state.priority = state.priority.prev();
                    }
                    _ => {}
                },
                EditRoutineField::Description => match key.code {
                    KeyCode::Enter => {
                        state.description.push('\n');
                    }
                    KeyCode::Backspace => {
                        state.description.pop();
                    }
                    KeyCode::Char(c) => {
                        state.description.push(c);
                    }
                    _ => {}
                },
            }
        }
    }

    fn handle_view_routine_key(&mut self, key: KeyEvent) {
        if let AppMode::ViewRoutine(ref mut state) = self.mode {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    state.scroll_offset = state.scroll_offset.saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    state.scroll_offset = state.scroll_offset.saturating_sub(1);
                }
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('v') => {
                    self.mode = AppMode::Normal;
                }
                _ => {}
            }
        }
    }

    fn handle_delete_routine_confirm_key(&mut self, key: KeyEvent) {
        if let AppMode::DeleteRoutineConfirm(ref routine) = self.mode {
            let routine_id = routine.id;
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let _ = self.db.delete_routine(routine_id);
                    self.clamp_selection();
                    self.mode = AppMode::Normal;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                }
                _ => {}
            }
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        let db = Db::open_in_memory().unwrap();
        let config = Config::default();
        let app = App::new(db, config);

        let _ = app.db.add_task(
            "First Task",
            "Details 1",
            TaskStatus::Todo,
            Priority::Low,
            Some(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()),
        );
        let _ = app.db.add_task(
            "Second Task",
            "Details 2",
            TaskStatus::InProgress,
            Priority::High,
            None,
        );
        app
    }

    #[test]
    fn test_navigation_and_cycling() {
        let mut app = create_test_app();
        assert_eq!(app.selected_index, 0);

        app.move_down();
        assert_eq!(app.selected_index, 1);

        app.move_down();
        assert_eq!(app.selected_index, 0);

        app.move_up();
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn test_key_quick_add_task() {
        let mut app = create_test_app();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert!(matches!(app.mode, AppMode::NewTask(_)));

        for c in "Buy milk".chars() {
            app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Normal);
        let tasks = app.current_tasks();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[2].title, "Buy milk");
    }

    #[test]
    fn test_toggle_complete_key() {
        let mut app = create_test_app();
        assert_eq!(app.current_tasks()[0].status, TaskStatus::Todo);

        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.current_tasks()[0].status, TaskStatus::Done);

        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.current_tasks()[0].status, TaskStatus::Todo);
    }

    #[test]
    fn test_exclusive_and_parallel_in_progress() {
        let mut app = create_test_app();
        assert_eq!(app.current_tasks()[0].status, TaskStatus::Todo);
        assert_eq!(app.current_tasks()[1].status, TaskStatus::InProgress);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        let tasks = app.current_tasks();
        assert_eq!(tasks[0].status, TaskStatus::InProgress);
        assert_eq!(tasks[1].status, TaskStatus::Todo);

        app.move_down();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT));
        let tasks = app.current_tasks();
        assert_eq!(tasks[0].status, TaskStatus::InProgress);
        assert_eq!(tasks[1].status, TaskStatus::InProgress);
    }

    #[test]
    fn test_delete_confirm_modal() {
        let mut app = create_test_app();
        assert_eq!(app.current_tasks().len(), 2);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(matches!(app.mode, AppMode::DeleteConfirm(_)));

        app.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.current_tasks().len(), 1);
    }

    #[test]
    fn test_view_modal_scroll() {
        let mut app = create_test_app();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        if let AppMode::ViewTask(ref state) = app.mode {
            assert_eq!(state.scroll_offset, 0);
        } else {
            panic!("Expected ViewTask mode");
        }

        app.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        if let AppMode::ViewTask(ref state) = app.mode {
            assert_eq!(state.scroll_offset, 1);
        } else {
            panic!("Expected ViewTask mode");
        }

        app.handle_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        if let AppMode::ViewTask(ref state) = app.mode {
            assert_eq!(state.scroll_offset, 0);
        } else {
            panic!("Expected ViewTask mode");
        }

        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn test_list_state_sync() {
        let mut app = create_test_app();
        assert_eq!(app.list_state.selected(), Some(0));

        app.move_down();
        assert_eq!(app.list_state.selected(), Some(1));

        app.move_down();
        assert_eq!(app.list_state.selected(), Some(0));

        app.move_up();
        assert_eq!(app.list_state.selected(), Some(1));
    }

    #[test]
    fn test_routine_view_navigation_and_crud() {
        let mut app = create_test_app();
        // Switch to Routine view with 'R'
        app.handle_key_event(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::NONE));
        assert_eq!(app.view_list, ViewList::Routine);

        // Open add routine modal with 'a'
        app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert!(matches!(app.mode, AppMode::EditRoutine(_)));

        // Type title 'Weekly Review'
        for c in "Weekly Review".chars() {
            app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }

        // Tab to Frequency (default Mon -> Right to Tue; due_weekday should sync to Tue)
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

        // Tab to DueWeekday (currently Tue -> Right 3 times to Fri)
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        for _ in 0..3 {
            app.handle_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        }

        // Save with Enter
        app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
        let routines = app.current_routines();
        assert_eq!(routines.len(), 1);
        assert_eq!(routines[0].title, "Weekly Review");
        assert_eq!(
            routines[0].frequency,
            RoutineFrequency::Weekly(chrono::Weekday::Tue)
        );
        assert_eq!(routines[0].due_weekday, Some(chrono::Weekday::Fri));

        // Generate instance manually with 'p'
        app.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

        // Switch back to Active view with 'R'
        app.handle_key_event(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::NONE));
        assert_eq!(app.view_list, ViewList::Active);
        let tasks = app.current_tasks();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[2].title, "Weekly Review");
    }
}
