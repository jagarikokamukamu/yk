use chrono::NaiveDate;
use ratatui::style::Color;

use crate::model::{Priority, TaskStatus};

// Semantic Colors (OKLCH L=0.75, C=0.12)
pub const COLOR_CRIMSON_RED: Color = Color::Rgb(0xf4, 0x89, 0x98); // 期限切れ / 高優先 (H=12)
pub const COLOR_AMBER_ORANGE: Color = Color::Rgb(0xe5, 0x9b, 0x5b); // 本日 / 中優先 (H=60)
pub const COLOR_GOLD_YELLOW: Color = Color::Rgb(0xbd, 0xb0, 0x4e); // 進行中 / 明日 (H=102)
pub const COLOR_SAGE_GREEN: Color = Color::Rgb(0x85, 0xc3, 0x88); // 完了 (H=145)
pub const COLOR_CYAN: Color = Color::Rgb(0x5a, 0xbf, 0xdf); // 選択フォーカス (H=210)
pub const COLOR_SLATE_PURPLE: Color = Color::Rgb(0xb0, 0xa8, 0xe0); // 低優先 / 未来 (H=270)

// Neutral Hierarchy (C=0.015, H=240)
pub const COLOR_TEXT_TITLE: Color = Color::Rgb(0xe6, 0xe6, 0xea); // L=0.90
pub const COLOR_TEXT_BODY: Color = Color::Rgb(0xbf, 0xbf, 0xc6); // L=0.75
pub const COLOR_TEXT_MUTED: Color = Color::Rgb(0x8c, 0x8c, 0x99); // L=0.55
pub const COLOR_BORDER: Color = Color::Rgb(0x56, 0x57, 0x66); // L=0.38 (境界・カッコ)

pub fn status_color(status: TaskStatus) -> Color {
    match status {
        TaskStatus::Todo => COLOR_TEXT_MUTED,
        TaskStatus::InProgress => COLOR_GOLD_YELLOW,
        TaskStatus::Done => COLOR_SAGE_GREEN,
    }
}

pub fn priority_color(priority: Priority) -> Color {
    match priority {
        Priority::High => COLOR_CRIMSON_RED,
        Priority::Medium => COLOR_AMBER_ORANGE,
        Priority::Low => COLOR_SLATE_PURPLE,
    }
}

pub fn due_date_display(due_date: Option<NaiveDate>, today: NaiveDate) -> (String, Color) {
    match due_date {
        None => ("-".to_string(), COLOR_TEXT_MUTED),
        Some(date) => {
            let diff = (date - today).num_days();
            let date_str = date.format("%Y-%m-%d").to_string();

            if diff < 0 {
                let days_ago = -diff;
                (
                    format!("{} ({}日前 期限切れ)", date_str, days_ago),
                    COLOR_CRIMSON_RED,
                )
            } else if diff == 0 {
                (format!("{} (今日)", date_str), COLOR_AMBER_ORANGE)
            } else if diff == 1 {
                (format!("{} (明日)", date_str), COLOR_GOLD_YELLOW)
            } else if diff == 2 {
                (format!("{} (明後日)", date_str), COLOR_SLATE_PURPLE)
            } else {
                (format!("{} ({}日後)", date_str, diff), COLOR_SLATE_PURPLE)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_due_date_display() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

        let past = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let (label, color) = due_date_display(Some(past), today);
        assert_eq!(label, "2026-10-27 (2日前 期限切れ)");
        assert_eq!(color, COLOR_CRIMSON_RED);

        let cur = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let (label, color) = due_date_display(Some(cur), today);
        assert_eq!(label, "2026-10-29 (今日)");
        assert_eq!(color, COLOR_AMBER_ORANGE);

        let tomorrow = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let (label, color) = due_date_display(Some(tomorrow), today);
        assert_eq!(label, "2026-10-30 (明日)");
        assert_eq!(color, COLOR_GOLD_YELLOW);

        let none_val = due_date_display(None, today);
        assert_eq!(none_val.0, "-");
        assert_eq!(none_val.1, COLOR_TEXT_MUTED);
    }
}
