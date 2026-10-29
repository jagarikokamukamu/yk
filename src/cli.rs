use chrono::NaiveDate;
use clap::{Parser, Subcommand};

use crate::config::Config;
use crate::db::Db;
use crate::model::{Priority, TaskStatus};

#[derive(Parser, Debug, PartialEq, Eq)]
#[command(
    name = "yk",
    version,
    about = "A fast and elegant TUI/CLI Todo manager (yarukoto)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Commands {
    /// 新規タスクを追加
    #[command(alias = "a")]
    Add {
        /// タスク名
        title: String,
        /// 期日 (YYYY-MM-DD)
        #[arg(short = 'd', long)]
        due: Option<String>,
        /// 優先度 (low, medium, high)
        #[arg(short = 'p', long, default_value = "medium")]
        priority: String,
        /// 詳細説明
        #[arg(short = 'm', long, default_value = "")]
        desc: String,
    },
}

pub fn handle_cli(cli: Cli, db: &Db, _config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let command = match cli.command {
        Some(cmd) => cmd,
        None => return Ok(()),
    };

    match command {
        Commands::Add {
            title,
            due,
            priority,
            desc,
        } => {
            let due_date = match due {
                Some(ref s) => match NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d") {
                    Ok(d) => Some(d),
                    Err(_) => {
                        eprintln!(
                            "警告: 期日の形式が不正です (YYYY-MM-DD で指定してください): {s}"
                        );
                        None
                    }
                },
                None => None,
            };
            let prio = Priority::from_str_loose(&priority);
            let id = db.add_task(&title, &desc, TaskStatus::Todo, prio, due_date)?;
            println!("タスク #{} を作成しました: {}", id, title);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_add() {
        let cli =
            Cli::try_parse_from(["yk", "add", "Buy coffee", "-d", "2026-10-31", "-p", "high"])
                .unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Add {
                title: "Buy coffee".to_string(),
                due: Some("2026-10-31".to_string()),
                priority: "high".to_string(),
                desc: "".to_string(),
            })
        );
    }

    #[test]
    fn test_cli_parse_none_launches_tui() {
        let cli = Cli::try_parse_from(["yk"]).unwrap();
        assert_eq!(cli.command, None);
    }
}
