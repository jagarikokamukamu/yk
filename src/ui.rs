use chrono::{Local, NaiveDate};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame,
};

use crate::app::{
    App, AppMode, EditField, EditRoutineField, EditRoutineState, EditState, ViewList,
    ViewRoutineState, ViewTaskState,
};
use crate::model::{Priority, Routine, Task, TaskStatus};
use crate::theme::*;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Main List
            Constraint::Length(3), // Footer / Key hints
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_list(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    match &app.mode {
        AppMode::Normal => {}
        AppMode::NewTask(title) => draw_new_task_modal(f, title),
        AppMode::EditTask(edit_state) => draw_edit_modal(f, edit_state),
        AppMode::ViewTask(view_state) => draw_view_modal(f, view_state),
        AppMode::DeleteConfirm(task) => draw_delete_confirm_modal(f, task),
        AppMode::EditRoutine(edit_state) => draw_edit_routine_modal(f, edit_state),
        AppMode::ViewRoutine(view_state) => draw_view_routine_modal(f, view_state),
        AppMode::DeleteRoutineConfirm(routine) => draw_delete_routine_confirm_modal(f, routine),
        AppMode::Help => draw_help_modal(f),
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.view_list {
        ViewList::Active => " ACTIVE ",
        ViewList::Archive => " ARCHIVE ",
        ViewList::Routine => " ROUTINE ",
    };
    let mode_style = match app.view_list {
        ViewList::Active => Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ViewList::Archive => Style::default()
            .fg(COLOR_SLATE_PURPLE)
            .add_modifier(Modifier::BOLD),
        ViewList::Routine => Style::default()
            .fg(COLOR_GOLD_YELLOW)
            .add_modifier(Modifier::BOLD),
    };

    let count_info = match app.view_list {
        ViewList::Routine => format!(" ({} routines)", app.current_routines().len()),
        _ => format!(" ({} tasks)", app.current_tasks().len()),
    };

    let header_line = Line::from(vec![
        Span::styled(
            " yk ",
            Style::default()
                .fg(COLOR_GOLD_YELLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("- yarukoto  ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled("[", Style::default().fg(COLOR_BORDER)),
        Span::styled(mode_str, mode_style),
        Span::styled("]", Style::default().fg(COLOR_BORDER)),
        Span::styled(count_info, Style::default().fg(COLOR_TEXT_MUTED)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER));

    let paragraph = Paragraph::new(header_line)
        .block(block)
        .alignment(Alignment::Left);
    f.render_widget(paragraph, area);
}

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    if app.view_list == ViewList::Routine {
        draw_routine_list(f, app, area);
    } else {
        draw_task_list(f, app, area);
    }
}

fn draw_task_list(f: &mut Frame, app: &mut App, area: Rect) {
    let tasks = app.current_tasks();
    let today = Local::now().date_naive();

    let items: Vec<ListItem> = if tasks.is_empty() {
        let empty_msg = match app.view_list {
            ViewList::Active => "タスクがありません。'a'キーで新規追加してください。",
            ViewList::Archive => "アーカイブされたタスクはありません。",
            ViewList::Routine => "",
        };
        vec![ListItem::new(Line::from(vec![Span::styled(
            format!("  {}", empty_msg),
            Style::default().fg(COLOR_TEXT_MUTED),
        )]))]
    } else {
        tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let is_selected = i == app.selected_index;

                // 1. Cursor indicator
                let cursor_span = if is_selected {
                    Span::styled(
                        "▶ ",
                        Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("  ")
                };

                // 2. Checkbox
                let bracket_open = Span::styled("[", Style::default().fg(COLOR_BORDER));
                let indicator_span = Span::styled(
                    task.status.indicator(),
                    Style::default()
                        .fg(status_color(task.status))
                        .add_modifier(Modifier::BOLD),
                );
                let bracket_close = Span::styled("] ", Style::default().fg(COLOR_BORDER));

                // 3. Priority indicator
                let priority_span = Span::styled(
                    format!("{:<3} ", task.priority.indicator()),
                    Style::default()
                        .fg(priority_color(task.priority))
                        .add_modifier(Modifier::BOLD),
                );

                // 4. Title
                let title_style = if task.status == TaskStatus::Done {
                    Style::default()
                        .fg(COLOR_TEXT_MUTED)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else if is_selected {
                    Style::default()
                        .fg(COLOR_TEXT_TITLE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(COLOR_TEXT_BODY)
                };
                let title_span = Span::styled(&task.title, title_style);

                // 5. Due date with relative badge
                let (due_str, due_col) = due_date_display(task.due_date, today);
                let due_span = Span::styled(format!("  {}", due_str), Style::default().fg(due_col));

                ListItem::new(Line::from(vec![
                    cursor_span,
                    bracket_open,
                    indicator_span,
                    bracket_close,
                    priority_span,
                    title_span,
                    due_span,
                ]))
            })
            .collect()
    };

    let title_text = match app.view_list {
        ViewList::Active => " Active Tasks ",
        ViewList::Archive => " Archived Tasks ",
        ViewList::Routine => " Routine Templates ",
    };

    let list_widget = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title_text)
            .title_style(Style::default().fg(COLOR_TEXT_TITLE))
            .border_style(Style::default().fg(COLOR_BORDER)),
    );

    f.render_stateful_widget(list_widget, area, &mut app.list_state);

    let inner_height = area.height.saturating_sub(2) as usize;
    if tasks.len() > inner_height && inner_height > 0 {
        let mut scrollbar_state = ScrollbarState::new(tasks.len().saturating_sub(inner_height))
            .position(app.list_state.offset());
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .style(Style::default().fg(COLOR_CYAN));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn draw_routine_list(f: &mut Frame, app: &mut App, area: Rect) {
    let routines = app.current_routines();

    let items: Vec<ListItem> = if routines.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  定期タスクがありません。'a'キーで新規登録してください。",
            Style::default().fg(COLOR_TEXT_MUTED),
        )]))]
    } else {
        routines
            .iter()
            .enumerate()
            .map(|(i, routine)| {
                let is_selected = i == app.selected_index;

                // 1. Cursor
                let cursor_span = if is_selected {
                    Span::styled(
                        "▶ ",
                        Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("  ")
                };

                // 2. Frequency badge
                let freq_label = routine.frequency.display_label();
                let freq_span = Span::styled(
                    format!("[ {:<12} ] ", freq_label),
                    Style::default()
                        .fg(COLOR_GOLD_YELLOW)
                        .add_modifier(Modifier::BOLD),
                );

                // 3. Priority indicator
                let priority_span = Span::styled(
                    format!("{:<3} ", routine.priority.indicator()),
                    Style::default()
                        .fg(priority_color(routine.priority))
                        .add_modifier(Modifier::BOLD),
                );

                // 4. Title
                let title_style = if is_selected {
                    Style::default()
                        .fg(COLOR_TEXT_TITLE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(COLOR_TEXT_BODY)
                };
                let title_span = Span::styled(&routine.title, title_style);

                // 5. Last generated info
                let last_gen_str = match routine.last_generated_date {
                    Some(d) => format!("  (最終発行: {})", d.format("%Y-%m-%d")),
                    None => "  (未発行)".to_string(),
                };
                let last_gen_span =
                    Span::styled(last_gen_str, Style::default().fg(COLOR_TEXT_MUTED));

                ListItem::new(Line::from(vec![
                    cursor_span,
                    freq_span,
                    priority_span,
                    title_span,
                    last_gen_span,
                ]))
            })
            .collect()
    };

    let list_widget = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Routine Templates (毎週・毎日の定期タスク雛形) ")
            .title_style(Style::default().fg(COLOR_TEXT_TITLE))
            .border_style(Style::default().fg(COLOR_BORDER)),
    );

    f.render_stateful_widget(list_widget, area, &mut app.list_state);

    let inner_height = area.height.saturating_sub(2) as usize;
    if routines.len() > inner_height && inner_height > 0 {
        let mut scrollbar_state = ScrollbarState::new(routines.len().saturating_sub(inner_height))
            .position(app.list_state.offset());
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .style(Style::default().fg(COLOR_CYAN));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let help_line = if app.view_list == ViewList::Routine {
        Line::from(vec![
            Span::styled(" [a] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Add  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[e] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Edit  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[v] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("View  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[d] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Delete  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[p/Space] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("今すぐ発行  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[R/Tab] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Activeへ  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[?] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Help  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[q] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Quit", Style::default().fg(COLOR_TEXT_MUTED)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" [a] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Add  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[Space] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Done  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[i/I] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("InProg  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[e] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Edit  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[v] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("View  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[d] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Del  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[A] ", Style::default().fg(COLOR_CYAN)),
            Span::styled(
                if app.view_list == ViewList::Active {
                    "Archive  "
                } else {
                    "Active  "
                },
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
            Span::styled("[R] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Routine  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[?] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Help  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("[q] ", Style::default().fg(COLOR_CYAN)),
            Span::styled("Quit", Style::default().fg(COLOR_TEXT_MUTED)),
        ])
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER));

    let paragraph = Paragraph::new(help_line).block(block);
    f.render_widget(paragraph, area);
}

fn draw_new_task_modal(f: &mut Frame, title: &str) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 新規タスク追加 (Quick Add) ")
        .title_style(Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(COLOR_CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

    let prompt =
        Paragraph::new("タスク名を入力してください:").style(Style::default().fg(COLOR_TEXT_TITLE));
    f.render_widget(prompt, chunks[0]);

    let input_line = Line::from(vec![
        Span::styled(title, Style::default().fg(COLOR_TEXT_BODY)),
        Span::styled("█", Style::default().fg(COLOR_CYAN)),
    ]);
    let input_widget = Paragraph::new(input_line).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );
    f.render_widget(input_widget, chunks[1]);

    let footer = Paragraph::new("[Enter] 作成  [Esc] キャンセル")
        .style(Style::default().fg(COLOR_TEXT_MUTED));
    f.render_widget(footer, chunks[2]);
}

fn draw_edit_modal(f: &mut Frame, state: &EditState) {
    let area = centered_rect(75, 70, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" タスク編集 ")
        .title_style(Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(COLOR_CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(5),    // Description
            Constraint::Length(3), // Status
            Constraint::Length(3), // Priority
            Constraint::Length(3), // Due Date
            Constraint::Length(2), // Help / Instructions
        ])
        .split(inner);

    // Title
    let title_border = if state.active_field == EditField::Title {
        COLOR_CYAN
    } else {
        COLOR_BORDER
    };
    let title_content = if state.active_field == EditField::Title {
        Line::from(vec![
            Span::styled(&state.title, Style::default().fg(COLOR_TEXT_TITLE)),
            Span::styled("█", Style::default().fg(COLOR_CYAN)),
        ])
    } else {
        Line::from(vec![Span::styled(
            &state.title,
            Style::default().fg(COLOR_TEXT_TITLE),
        )])
    };
    let title_widget = Paragraph::new(title_content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" タイトル (Title) ")
            .border_style(Style::default().fg(title_border)),
    );
    f.render_widget(title_widget, chunks[0]);

    // Description
    let desc_border = if state.active_field == EditField::Description {
        COLOR_CYAN
    } else {
        COLOR_BORDER
    };
    let mut desc_lines: Vec<Line> = state
        .description
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(COLOR_TEXT_BODY))))
        .collect();
    if desc_lines.is_empty() {
        desc_lines.push(Line::from(vec![]));
    }
    if state.active_field == EditField::Description {
        let last_idx = desc_lines.len() - 1;
        let mut spans = desc_lines[last_idx].spans.clone();
        spans.push(Span::styled("█", Style::default().fg(COLOR_CYAN)));
        desc_lines[last_idx] = Line::from(spans);
    }
    let desc_widget = Paragraph::new(desc_lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 説明 (Description - Enterで改行) ")
            .border_style(Style::default().fg(desc_border)),
    );
    f.render_widget(desc_widget, chunks[1]);

    // Status
    let status_border = if state.active_field == EditField::Status {
        COLOR_CYAN
    } else {
        COLOR_BORDER
    };
    let status_str = match state.status {
        TaskStatus::Todo => "[ ] 未着手 (Todo)",
        TaskStatus::InProgress => "[>] 進行中 (InProgress)",
        TaskStatus::Done => "[x] 完了 (Done)",
    };
    let status_widget = Paragraph::new(Line::from(vec![
        Span::styled(
            status_str,
            Style::default()
                .fg(status_color(state.status))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  (Space / ← / → で変更)",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" ステータス (Status) ")
            .border_style(Style::default().fg(status_border)),
    );
    f.render_widget(status_widget, chunks[2]);

    // Priority
    let priority_border = if state.active_field == EditField::Priority {
        COLOR_CYAN
    } else {
        COLOR_BORDER
    };
    let priority_str = match state.priority {
        Priority::Low => "! 低 (Low)",
        Priority::Medium => "!! 中 (Medium)",
        Priority::High => "!!! 高 (High)",
    };
    let priority_widget = Paragraph::new(Line::from(vec![
        Span::styled(
            priority_str,
            Style::default()
                .fg(priority_color(state.priority))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  (Space / ← / → で変更)",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 優先度 (Priority) ")
            .border_style(Style::default().fg(priority_border)),
    );
    f.render_widget(priority_widget, chunks[3]);

    // Due Date
    let due_border = if state.active_field == EditField::DueDate {
        COLOR_CYAN
    } else {
        COLOR_BORDER
    };
    let today = Local::now().date_naive();
    let parsed_due = NaiveDate::parse_from_str(state.due_date_str.trim(), "%Y-%m-%d").ok();
    let (relative_label, _) = due_date_display(parsed_due, today);

    let due_content = if state.active_field == EditField::DueDate {
        Line::from(vec![
            Span::styled(
                if state.due_date_str.is_empty() {
                    "なし"
                } else {
                    &state.due_date_str
                },
                Style::default().fg(COLOR_TEXT_TITLE),
            ),
            Span::styled("█", Style::default().fg(COLOR_CYAN)),
            Span::styled(
                format!("  [{}]", relative_label),
                Style::default().fg(COLOR_GOLD_YELLOW),
            ),
            Span::styled(
                "  (←/→: ±1日, 直接編集可, 全消去でなし)",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                if state.due_date_str.is_empty() {
                    "なし"
                } else {
                    &state.due_date_str
                },
                Style::default().fg(COLOR_TEXT_TITLE),
            ),
            Span::styled(
                format!("  [{}]", relative_label),
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ])
    };
    let due_widget = Paragraph::new(due_content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 期日 (Due Date - YYYY-MM-DD) ")
            .border_style(Style::default().fg(due_border)),
    );
    f.render_widget(due_widget, chunks[4]);

    // Instructions
    let guide = Paragraph::new(Line::from(vec![
        Span::styled("[Tab/Shift+Tab] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("移動  ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled("[Ctrl+Enter] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("保存  ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled("[Esc] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("キャンセル", Style::default().fg(COLOR_TEXT_MUTED)),
    ]));
    f.render_widget(guide, chunks[5]);
}

fn draw_edit_routine_modal(f: &mut Frame, state: &EditRoutineState) {
    let area = centered_rect(75, 65, f.area());
    f.render_widget(Clear, area);

    let modal_title = if state.target_routine_id.is_some() {
        " 定期タスク編集 "
    } else {
        " 新規定期タスク登録 "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(modal_title)
        .title_style(Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(COLOR_CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Frequency
            Constraint::Length(3), // Priority
            Constraint::Min(4),    // Description
            Constraint::Length(2), // Help / Instructions
        ])
        .split(inner);

    // 1. Title
    let title_border = if state.active_field == EditRoutineField::Title {
        COLOR_CYAN
    } else {
        COLOR_BORDER
    };
    let title_content = if state.active_field == EditRoutineField::Title {
        Line::from(vec![
            Span::styled(&state.title, Style::default().fg(COLOR_TEXT_TITLE)),
            Span::styled("█", Style::default().fg(COLOR_CYAN)),
        ])
    } else {
        Line::from(vec![Span::styled(
            &state.title,
            Style::default().fg(COLOR_TEXT_TITLE),
        )])
    };
    let title_widget = Paragraph::new(title_content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" タイトル (Title) ")
            .border_style(Style::default().fg(title_border)),
    );
    f.render_widget(title_widget, chunks[0]);

    // 2. Frequency
    let freq_border = if state.active_field == EditRoutineField::Frequency {
        COLOR_CYAN
    } else {
        COLOR_BORDER
    };
    let freq_widget = Paragraph::new(Line::from(vec![
        Span::styled(
            state.frequency.display_label(),
            Style::default()
                .fg(COLOR_GOLD_YELLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  (Space / ← / → で変更)",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 頻度 (Frequency) ")
            .border_style(Style::default().fg(freq_border)),
    );
    f.render_widget(freq_widget, chunks[1]);

    // 3. Priority
    let priority_border = if state.active_field == EditRoutineField::Priority {
        COLOR_CYAN
    } else {
        COLOR_BORDER
    };
    let priority_str = match state.priority {
        Priority::Low => "! 低 (Low)",
        Priority::Medium => "!! 中 (Medium)",
        Priority::High => "!!! 高 (High)",
    };
    let priority_widget = Paragraph::new(Line::from(vec![
        Span::styled(
            priority_str,
            Style::default()
                .fg(priority_color(state.priority))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  (Space / ← / → で変更)",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 優先度 (Priority) ")
            .border_style(Style::default().fg(priority_border)),
    );
    f.render_widget(priority_widget, chunks[2]);

    // 4. Description
    let desc_border = if state.active_field == EditRoutineField::Description {
        COLOR_CYAN
    } else {
        COLOR_BORDER
    };
    let mut desc_lines: Vec<Line> = state
        .description
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(COLOR_TEXT_BODY))))
        .collect();
    if desc_lines.is_empty() {
        desc_lines.push(Line::from(vec![]));
    }
    if state.active_field == EditRoutineField::Description {
        let last_idx = desc_lines.len() - 1;
        let mut spans = desc_lines[last_idx].spans.clone();
        spans.push(Span::styled("█", Style::default().fg(COLOR_CYAN)));
        desc_lines[last_idx] = Line::from(spans);
    }
    let desc_widget = Paragraph::new(desc_lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 説明 (Description - Enterで改行) ")
            .border_style(Style::default().fg(desc_border)),
    );
    f.render_widget(desc_widget, chunks[3]);

    // Instructions
    let guide = Paragraph::new(Line::from(vec![
        Span::styled("[Tab/Shift+Tab] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("移動  ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled("[Ctrl+Enter] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("保存  ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled("[Esc] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("キャンセル", Style::default().fg(COLOR_TEXT_MUTED)),
    ]));
    f.render_widget(guide, chunks[4]);
}

fn draw_view_modal(f: &mut Frame, state: &ViewTaskState) {
    let task = &state.task;
    let area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" タスク詳細閲覧 ")
        .title_style(Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(COLOR_CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let today = Local::now().date_naive();
    let (due_str, due_col) = due_date_display(task.due_date, today);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(task.id.to_string(), Style::default().fg(COLOR_TEXT_TITLE)),
        ]),
        Line::from(vec![
            Span::styled("タイトル: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &task.title,
                Style::default()
                    .fg(COLOR_TEXT_TITLE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("状態: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                task.status.as_str(),
                Style::default().fg(status_color(task.status)),
            ),
        ]),
        Line::from(vec![
            Span::styled("優先度: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                format!("{} ({})", task.priority.indicator(), task.priority.as_str()),
                Style::default().fg(priority_color(task.priority)),
            ),
        ]),
        Line::from(vec![
            Span::styled("期日: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(due_str, Style::default().fg(due_col)),
        ]),
        Line::from(vec![
            Span::styled("作成日時: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                task.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                Style::default().fg(COLOR_TEXT_BODY),
            ),
        ]),
        Line::from(vec![
            Span::styled("更新日時: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                task.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                Style::default().fg(COLOR_TEXT_BODY),
            ),
        ]),
    ];

    if let Some(completed_at) = task.completed_at {
        lines.push(Line::from(vec![
            Span::styled("完了日時: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                completed_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                Style::default().fg(COLOR_SAGE_GREEN),
            ),
        ]));
    }

    if let Some(r_id) = task.routine_id {
        lines.push(Line::from(vec![
            Span::styled("定期タスク元: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                format!("Routine #{}", r_id),
                Style::default().fg(COLOR_GOLD_YELLOW),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "--- 説明 ---",
        Style::default().fg(COLOR_BORDER),
    )]));

    if task.description.trim().is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "(説明はありません)",
            Style::default().fg(COLOR_TEXT_MUTED),
        )]));
    } else {
        for l in task.description.lines() {
            lines.push(Line::from(vec![Span::styled(
                l,
                Style::default().fg(COLOR_TEXT_BODY),
            )]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[j/k または ↓/↑] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("スクロール  ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled("[Esc / q / Enter] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("閉じる", Style::default().fg(COLOR_TEXT_MUTED)),
    ]));

    let total_lines = lines.len();
    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset, 0));
    f.render_widget(content, inner);

    let inner_height = inner.height as usize;
    if total_lines > inner_height && inner_height > 0 {
        let max_scroll = (total_lines.saturating_sub(inner_height)) as u16;
        let effective_scroll = state.scroll_offset.min(max_scroll);
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(inner_height))
            .position(effective_scroll as usize);
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .style(Style::default().fg(COLOR_CYAN));
        f.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }
}

fn draw_view_routine_modal(f: &mut Frame, state: &ViewRoutineState) {
    let routine = &state.routine;
    let area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 定期タスク詳細閲覧 ")
        .title_style(Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(COLOR_CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let last_gen_str = match routine.last_generated_date {
        Some(d) => d.format("%Y-%m-%d").to_string(),
        None => "未発行".to_string(),
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                routine.id.to_string(),
                Style::default().fg(COLOR_TEXT_TITLE),
            ),
        ]),
        Line::from(vec![
            Span::styled("タイトル: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &routine.title,
                Style::default()
                    .fg(COLOR_TEXT_TITLE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("頻度: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                routine.frequency.display_label(),
                Style::default()
                    .fg(COLOR_GOLD_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("優先度: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                format!(
                    "{} ({})",
                    routine.priority.indicator(),
                    routine.priority.as_str()
                ),
                Style::default().fg(priority_color(routine.priority)),
            ),
        ]),
        Line::from(vec![
            Span::styled("最終発行日: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(last_gen_str, Style::default().fg(COLOR_TEXT_BODY)),
        ]),
        Line::from(vec![
            Span::styled("作成日時: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                routine.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                Style::default().fg(COLOR_TEXT_BODY),
            ),
        ]),
    ];

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "--- 説明 ---",
        Style::default().fg(COLOR_BORDER),
    )]));

    if routine.description.trim().is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "(説明はありません)",
            Style::default().fg(COLOR_TEXT_MUTED),
        )]));
    } else {
        for l in routine.description.lines() {
            lines.push(Line::from(vec![Span::styled(
                l,
                Style::default().fg(COLOR_TEXT_BODY),
            )]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[j/k または ↓/↑] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("スクロール  ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled("[Esc / q / Enter] ", Style::default().fg(COLOR_CYAN)),
        Span::styled("閉じる", Style::default().fg(COLOR_TEXT_MUTED)),
    ]));

    let total_lines = lines.len();
    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset, 0));
    f.render_widget(content, inner);

    let inner_height = inner.height as usize;
    if total_lines > inner_height && inner_height > 0 {
        let max_scroll = (total_lines.saturating_sub(inner_height)) as u16;
        let effective_scroll = state.scroll_offset.min(max_scroll);
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(inner_height))
            .position(effective_scroll as usize);
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .style(Style::default().fg(COLOR_CYAN));
        f.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }
}

fn draw_delete_confirm_modal(f: &mut Frame, task: &Task) {
    let area = centered_rect(55, 25, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 削除確認 ")
        .title_style(
            Style::default()
                .fg(COLOR_CRIMSON_RED)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(COLOR_CRIMSON_RED));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled(
            "以下のタスクを削除しますか？",
            Style::default().fg(COLOR_TEXT_TITLE),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("タイトル: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &task.title,
                Style::default()
                    .fg(COLOR_CRIMSON_RED)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[y] ",
                Style::default()
                    .fg(COLOR_CRIMSON_RED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("削除する   ", Style::default().fg(COLOR_TEXT_TITLE)),
            Span::styled("[n / Esc] ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("キャンセル", Style::default().fg(COLOR_TEXT_MUTED)),
        ]),
    ];

    let content = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(content, inner);
}

fn draw_delete_routine_confirm_modal(f: &mut Frame, routine: &Routine) {
    let area = centered_rect(55, 25, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 定期タスク削除確認 ")
        .title_style(
            Style::default()
                .fg(COLOR_CRIMSON_RED)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(COLOR_CRIMSON_RED));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled(
            "以下の定期タスク（雛形）を削除しますか？",
            Style::default().fg(COLOR_TEXT_TITLE),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("タイトル: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &routine.title,
                Style::default()
                    .fg(COLOR_CRIMSON_RED)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[y] ",
                Style::default()
                    .fg(COLOR_CRIMSON_RED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("削除する   ", Style::default().fg(COLOR_TEXT_TITLE)),
            Span::styled("[n / Esc] ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled("キャンセル", Style::default().fg(COLOR_TEXT_MUTED)),
        ]),
    ];

    let content = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(content, inner);
}

fn draw_help_modal(f: &mut Frame) {
    let area = centered_rect(65, 80, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ヘルプ (キー操作一覧) ")
        .title_style(Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(COLOR_CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let shortcuts = vec![
        ("メイン一覧 (Active / Archive)", ""),
        ("  j / ↓, k / ↑", "リストの循環移動"),
        ("  Space", "完了トグル (Todo ↔ Done, completed_at自動付与)"),
        (
            "  i",
            "排他進行中 (他をTodoに戻し、選択タスクのみInProgressへ)",
        ),
        ("  I", "並行進行中 (他タスクを変更せずInProgressへ)"),
        ("  a", "新規タスク追加 (簡易タイトル入力ポップアップ)"),
        ("  e / Enter", "詳細編集モーダル"),
        ("  v", "詳細閲覧モーダル"),
        ("  d", "削除確認モーダル (y/n)"),
        ("  Tab", "Active → Archive → Routine のビュー巡回"),
        ("  A", "Active ↔ Archive 表示切り替え"),
        ("  R", "Active ↔ Routine 表示切り替え"),
        ("  ?", "ヘルプモーダル表示"),
        ("  q / Esc", "アプリケーション終了"),
        ("", ""),
        ("定期タスク画面 (Routine)", ""),
        ("  a", "新規定期タスク登録モーダル"),
        ("  e / Enter", "定期タスク編集モーダル"),
        ("  d", "定期タスク削除確認モーダル (y/n)"),
        ("  v", "定期タスク詳細閲覧"),
        ("  p / Space", "今すぐ実体タスクをActiveに発行"),
        ("  R / Esc", "Active画面へ戻る"),
        ("", ""),
        ("編集モーダル", ""),
        ("  Tab / Shift+Tab", "項目間移動 (次 / 前)"),
        ("  ↓ / ↑", "項目間移動 (説明欄以外)"),
        ("  Space, ← / →", "ステータス・頻度・優先度の変更"),
        ("  ← / → (期日欄)", "1日加減 (テキスト直接入力も可能)"),
        ("  Enter (説明欄)", "改行入力"),
        (
            "  Ctrl+Enter",
            "保存して閉じる (タイトル等ではEnterでも保存可)",
        ),
        ("  Esc", "キャンセルして閉じる"),
    ];

    let mut lines = Vec::new();
    for (key, desc) in shortcuts {
        if desc.is_empty() {
            if !key.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("■ {}", key),
                    Style::default()
                        .fg(COLOR_GOLD_YELLOW)
                        .add_modifier(Modifier::BOLD),
                )));
            } else {
                lines.push(Line::from(""));
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<20}", key), Style::default().fg(COLOR_CYAN)),
                Span::styled(desc, Style::default().fg(COLOR_TEXT_BODY)),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Esc / q / ? / Enter] 閉じる",
        Style::default().fg(COLOR_TEXT_MUTED),
    )));

    let content = Paragraph::new(lines);
    f.render_widget(content, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
