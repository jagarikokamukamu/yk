use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use std::io::{self, Write};

use crate::config::Config;
use crate::db::Db;
use crate::model::{Priority, RoutineFrequency, TaskStatus};
use crate::theme::due_date_display;

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
    /// タスク一覧を表示
    #[command(alias = "ls")]
    List {
        /// アーカイブ済みを含むすべてのタスクを表示
        #[arg(short = 'a', long)]
        all: bool,
        /// アーカイブ済みのタスクのみを表示
        #[arg(long)]
        archive: bool,
    },
    /// タスクを進行中状態にする
    #[command(alias = "doing")]
    Start {
        /// 進行中にするタスクID
        id: i64,
        /// 他のタスクを変更せず並行して進行中にする（未指定時は排他モード）
        #[arg(short = 'p', long)]
        parallel: bool,
    },
    /// タスクを完了状態にする
    #[command(alias = "finish", alias = "complete")]
    Done {
        /// 完了にするタスクID（複数指定可）
        #[arg(required = true)]
        ids: Vec<i64>,
    },
    /// タスクを削除する
    #[command(alias = "delete", alias = "del")]
    Rm {
        /// 削除するタスクID（複数指定可）
        #[arg(required = true)]
        ids: Vec<i64>,
        /// 確認プロンプトを表示せずに即時削除する
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// 定期タスク（Routine）の管理
    #[command(alias = "r")]
    Routine {
        #[command(subcommand)]
        subcommand: RoutineCommands,
    },
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum RoutineCommands {
    /// 定期タスク一覧を表示
    #[command(alias = "ls")]
    List,
    /// 新規定期タスクを追加
    #[command(alias = "a")]
    Add {
        /// タスク名
        title: String,
        /// 頻度 (daily, mon, tue, wed, thu, fri, sat, sun)
        #[arg(short = 'f', long, default_value = "daily")]
        frequency: String,
        /// 優先度 (low, medium, high)
        #[arg(short = 'p', long, default_value = "medium")]
        priority: String,
        /// 詳細説明
        #[arg(short = 'm', long, default_value = "")]
        desc: String,
    },
    /// 定期タスクを削除する
    #[command(alias = "delete", alias = "del")]
    Rm {
        /// 削除する定期タスクID（複数指定可）
        #[arg(required = true)]
        ids: Vec<i64>,
        /// 確認プロンプトを表示せずに即時削除する
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// 定期タスクからActiveタスクを即時発行する
    #[command(alias = "gen")]
    Generate {
        /// 特定の定期タスクID（未指定時は全定期タスクが対象）
        id: Option<i64>,
    },
}

pub fn handle_cli(cli: Cli, db: &Db, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
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
        Commands::List { all, archive } => {
            let all_tasks = db.get_all_tasks()?;
            let now = Local::now();
            let today = now.date_naive();

            let tasks: Vec<_> = all_tasks
                .into_iter()
                .filter(|t| {
                    if all {
                        true
                    } else if archive {
                        t.is_archived(config.archive_after_days, now)
                    } else {
                        !t.is_archived(config.archive_after_days, now)
                    }
                })
                .collect();

            if tasks.is_empty() {
                if archive {
                    println!("アーカイブされたタスクはありません。");
                } else {
                    println!(
                        "表示対象のタスクはありません。'yk add \"タスク名\"' で追加できます。"
                    );
                }
                return Ok(());
            }

            println!(
                "{:<5} {:<7} {:<5} {:<24} TITLE",
                "ID", "STATUS", "PRIO", "DUE DATE"
            );
            println!("{}", "-".repeat(60));

            for task in tasks {
                let status_str = format!("[{}]", task.status.indicator());
                let prio_str = task.priority.indicator();
                let (due_str, _) = due_date_display(task.due_date, today);

                println!(
                    "{:<5} {:<7} {:<5} {:<24} {}",
                    task.id, status_str, prio_str, due_str, task.title
                );
            }
        }
        Commands::Start { id, parallel } => match db.get_task_by_id(id)? {
            Some(task) => {
                if parallel {
                    db.set_parallel_in_progress(id)?;
                    println!("タスク #{} を並行進行中にしました: {}", id, task.title);
                } else {
                    db.set_exclusive_in_progress(id)?;
                    println!("タスク #{} を排他進行中にしました: {}", id, task.title);
                }
            }
            None => {
                eprintln!("エラー: タスク #{} は存在しません。", id);
            }
        },
        Commands::Done { ids } => {
            for id in ids {
                match db.get_task_by_id(id)? {
                    Some(task) => {
                        db.set_done(id)?;
                        println!("タスク #{} を完了にしました: {}", id, task.title);
                    }
                    None => {
                        eprintln!("エラー: タスク #{} は存在しません。", id);
                    }
                }
            }
        }
        Commands::Rm { ids, force } => {
            for id in ids {
                match db.get_task_by_id(id)? {
                    Some(task) => {
                        if !force {
                            print!("タスク #{} '{}' を削除しますか？ [y/N]: ", id, task.title);
                            io::stdout().flush()?;
                            let mut answer = String::new();
                            io::stdin().read_line(&mut answer)?;
                            let trimmed = answer.trim();
                            if trimmed != "y" && trimmed != "Y" {
                                println!("削除をキャンセルしました。");
                                continue;
                            }
                        }
                        db.delete_task(id)?;
                        println!("タスク #{} を削除しました: {}", id, task.title);
                    }
                    None => {
                        eprintln!("エラー: タスク #{} は存在しません。", id);
                    }
                }
            }
        }
        Commands::Routine { subcommand } => match subcommand {
            RoutineCommands::List => {
                let routines = db.get_all_routines()?;
                if routines.is_empty() {
                    println!(
                        "登録されている定期タスクはありません。'yk routine add \"タスク名\"' で追加できます。"
                    );
                    return Ok(());
                }

                println!("{:<5} {:<18} {:<5} TITLE", "ID", "FREQUENCY", "PRIO");
                println!("{}", "-".repeat(50));

                for r in routines {
                    println!(
                        "{:<5} {:<18} {:<5} {}",
                        r.id,
                        r.frequency.display_label(),
                        r.priority.indicator(),
                        r.title
                    );
                }
            }
            RoutineCommands::Add {
                title,
                frequency,
                priority,
                desc,
            } => {
                let freq = RoutineFrequency::from_str_loose(&frequency);
                let prio = Priority::from_str_loose(&priority);
                let id = db.add_routine(&title, &desc, prio, freq)?;
                println!(
                    "定期タスク #{} を作成しました [{}]: {}",
                    id,
                    freq.display_label(),
                    title
                );
            }
            RoutineCommands::Rm { ids, force } => {
                for id in ids {
                    match db.get_routine_by_id(id)? {
                        Some(routine) => {
                            if !force {
                                print!(
                                    "定期タスク #{} '{}' を削除しますか？ [y/N]: ",
                                    id, routine.title
                                );
                                io::stdout().flush()?;
                                let mut answer = String::new();
                                io::stdin().read_line(&mut answer)?;
                                let trimmed = answer.trim();
                                if trimmed != "y" && trimmed != "Y" {
                                    println!("削除をキャンセルしました。");
                                    continue;
                                }
                            }
                            db.delete_routine(id)?;
                            println!("定期タスク #{} を削除しました: {}", id, routine.title);
                        }
                        None => {
                            eprintln!("エラー: 定期タスク #{} は存在しません。", id);
                        }
                    }
                }
            }
            RoutineCommands::Generate { id } => match id {
                Some(rid) => match db.get_routine_by_id(rid)? {
                    Some(routine) => {
                        if let Some(task_id) = db.generate_single_routine_instance(rid)? {
                            println!(
                                "定期タスク #{} からタスク #{} を発行しました: {}",
                                rid, task_id, routine.title
                            );
                        } else {
                            println!(
                                "定期タスク #{} は本日の発行条件に該当しないか、既に発行済みです。",
                                rid
                            );
                        }
                    }
                    None => {
                        eprintln!("エラー: 定期タスク #{} は存在しません。", rid);
                    }
                },
                None => {
                    let generated = db.generate_routine_instances()?;
                    println!(
                        "定期タスクから {} 件のタスクを新規発行しました。",
                        generated.len()
                    );
                }
            },
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_rm() {
        let cli = Cli::try_parse_from(["yk", "rm", "3", "-f"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Rm {
                ids: vec![3],
                force: true,
            })
        );
    }

    #[test]
    fn test_cli_parse_done() {
        let cli = Cli::try_parse_from(["yk", "done", "1", "2"]).unwrap();
        assert_eq!(cli.command, Some(Commands::Done { ids: vec![1, 2] }));
    }

    #[test]
    fn test_cli_parse_start() {
        let cli = Cli::try_parse_from(["yk", "start", "1", "-p"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Start {
                id: 1,
                parallel: true
            })
        );
    }

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
    fn test_cli_parse_list() {
        let cli = Cli::try_parse_from(["yk", "ls", "-a"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::List {
                all: true,
                archive: false,
            })
        );
    }

    #[test]
    fn test_cli_parse_routine_commands() {
        // routine list
        let cli = Cli::try_parse_from(["yk", "routine", "ls"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Routine {
                subcommand: RoutineCommands::List
            })
        );

        // routine add
        let cli = Cli::try_parse_from([
            "yk",
            "r",
            "a",
            "Gym workout",
            "-f",
            "mon",
            "-p",
            "high",
            "-m",
            "Leg day",
        ])
        .unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Routine {
                subcommand: RoutineCommands::Add {
                    title: "Gym workout".to_string(),
                    frequency: "mon".to_string(),
                    priority: "high".to_string(),
                    desc: "Leg day".to_string(),
                }
            })
        );

        // routine rm
        let cli = Cli::try_parse_from(["yk", "routine", "rm", "1", "2", "-f"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Routine {
                subcommand: RoutineCommands::Rm {
                    ids: vec![1, 2],
                    force: true,
                }
            })
        );

        // routine generate
        let cli = Cli::try_parse_from(["yk", "routine", "gen", "1"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Routine {
                subcommand: RoutineCommands::Generate { id: Some(1) }
            })
        );
    }

    #[test]
    fn test_cli_parse_none_launches_tui() {
        let cli = Cli::try_parse_from(["yk"]).unwrap();
        assert_eq!(cli.command, None);
    }
}
