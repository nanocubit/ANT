//! Git навык для работы с репозиториями
//! Поддерживаемые операции: clone, commit, push, pull, status, log, branch, checkout

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use std::process::Stdio;

/// Git команды
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum GitCommand {
    Clone {
        url: String,
        path: Option<String>,
    },
    Init {
        path: String,
        bare: Option<bool>,
    },
    Add {
        files: Vec<String>,
    },
    Commit {
        message: String,
        all: Option<bool>,
    },
    Push {
        remote: Option<String>,
        branch: Option<String>,
        force: Option<bool>,
    },
    Pull {
        remote: Option<String>,
        branch: Option<String>,
        rebase: Option<bool>,
    },
    Status {
        short: Option<bool>,
    },
    Log {
        limit: Option<usize>,
        oneline: Option<bool>,
    },
    Branch {
        list: Option<bool>,
        create: Option<String>,
        delete: Option<String>,
        checkout: Option<String>,
    },
    Checkout {
        branch: String,
        create_new: Option<bool>,
    },
    Diff {
        staged: Option<bool>,
    },
    Remote {
        list: Option<bool>,
        add: Option<(String, String)>,
    },
    Fetch {
        remote: Option<String>,
        all: Option<bool>,
    },
    Merge {
        branch: String,
        no_ff: Option<bool>,
    },
    Rebase {
        branch: String,
    },
    Stash {
        save: Option<bool>,
        pop: Option<bool>,
        list: Option<bool>,
    },
    Show {
        reference: String,
    },
}

/// Результат Git операции
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub command: String,
    pub duration_secs: f64,
}

/// Git навык
pub struct GitSkill {
    working_dir: Option<PathBuf>,
    timeout_secs: u64,
}

impl GitSkill {
    /// Создать новый GitSkill
    pub fn new(working_dir: Option<PathBuf>) -> Self {
        Self {
            working_dir,
            timeout_secs: 300,
        }
    }

    /// Создать с таймаутом
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Выполнить Git команду
    pub async fn execute(&self, command: GitCommand) -> Result<GitResult> {
        let start_time = std::time::Instant::now();

        let (args, cmd_str) = self.build_args(&command);

        let mut cmd = Command::new("git");
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Устанавливаем рабочую директорию
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }

        // Для clone и init можно указать путь
        if let GitCommand::Clone { path: Some(path), .. } = &command {
            cmd.current_dir(
                path.split('/').next().unwrap_or("."),
            );
        }

        // Выполняем с таймаутом
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(self.timeout_secs),
            cmd.output()
        ).await;

        let duration = start_time.elapsed().as_secs_f64();

        match result {
            Ok(Ok(output)) => Ok(GitResult {
                success: output.status.success(),
                output: String::from_utf8_lossy(&output.stdout).to_string(),
                error: if output.stderr.is_empty() {
                    None
                } else {
                    Some(String::from_utf8_lossy(&output.stderr).to_string())
                },
                command: cmd_str,
                duration_secs: duration,
            }),
            Ok(Err(e)) => Ok(GitResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                command: cmd_str,
                duration_secs: duration,
            }),
            Err(_) => Ok(GitResult {
                success: false,
                output: String::new(),
                error: Some(format!("Timeout after {} seconds", self.timeout_secs)),
                command: cmd_str,
                duration_secs: duration,
            }),
        }
    }

    /// Построить аргументы для команды
    fn build_args(&self, command: &GitCommand) -> (Vec<String>, String) {
        match command {
            GitCommand::Clone { url, path } => {
                let mut args = vec!["clone".to_string(), url.clone()];
                if let Some(p) = path {
                    args.push(p.clone());
                }
                (args, format!("git clone {}", url))
            }
            GitCommand::Init { path, bare } => {
                let mut args = vec!["init".to_string()];
                if let Some(true) = bare {
                    args.push("--bare".to_string());
                }
                if let Some(p) = path {
                    args.push(p.clone());
                }
                (args, "git init".to_string())
            }
            GitCommand::Add { files } => {
                let mut args = vec!["add".to_string()];
                args.extend(files.clone());
                (args, format!("git add {}", files.join(" ")))
            }
            GitCommand::Commit { message, all } => {
                let mut args = vec!["commit".to_string(), "-m".to_string(), message.clone()];
                if let Some(true) = all {
                    args.push("-a".to_string());
                }
                (args, format!("git commit -m \"{}\"", message))
            }
            GitCommand::Push { remote, branch, force } => {
                let mut args = vec!["push".to_string()];
                if let Some(true) = force {
                    args.push("--force".to_string());
                }
                if let Some(r) = remote {
                    args.push(r.clone());
                }
                if let Some(b) = branch {
                    args.push(b.clone());
                }
                (args, "git push".to_string())
            }
            GitCommand::Pull { remote, branch, rebase } => {
                let mut args = vec!["pull".to_string()];
                if let Some(true) = rebase {
                    args.push("--rebase".to_string());
                }
                if let Some(r) = remote {
                    args.push(r.clone());
                }
                if let Some(b) = branch {
                    args.push(b.clone());
                }
                (args, "git pull".to_string())
            }
            GitCommand::Status { short } => {
                let mut args = vec!["status".to_string()];
                if let Some(true) = short {
                    args.push("-s".to_string());
                }
                (args, "git status".to_string())
            }
            GitCommand::Log { limit, oneline } => {
                let mut args = vec!["log".to_string()];
                if let Some(true) = oneline {
                    args.push("--oneline".to_string());
                }
                if let Some(l) = limit {
                    args.push(format!("-n{}", l));
                }
                (args, "git log".to_string())
            }
            GitCommand::Branch { list, create, delete, checkout } => {
                let mut args = vec!["branch".to_string()];
                if let Some(true) = list {
                    args.push("-l".to_string());
                }
                if let Some(name) = create {
                    args.push(name.clone());
                }
                if let Some(name) = delete {
                    args.push("-d".to_string());
                    args.push(name.clone());
                }
                if let Some(name) = checkout {
                    args.push(name.clone());
                }
                (args, "git branch".to_string())
            }
            GitCommand::Checkout { branch, create_new } => {
                let mut args = vec!["checkout".to_string()];
                if let Some(true) = create_new {
                    args.push("-b".to_string());
                }
                args.push(branch.clone());
                (args, format!("git checkout {}", branch))
            }
            GitCommand::Diff { staged } => {
                let mut args = vec!["diff".to_string()];
                if let Some(true) = staged {
                    args.push("--staged".to_string());
                }
                (args, "git diff".to_string())
            }
            GitCommand::Remote { list, add } => {
                let mut args = vec!["remote".to_string()];
                if let Some(true) = list {
                    args.push("-v".to_string());
                }
                if let Some((name, url)) = add {
                    args.push("add".to_string());
                    args.push(name.clone());
                    args.push(url.clone());
                }
                (args, "git remote".to_string())
            }
            GitCommand::Fetch { remote, all } => {
                let mut args = vec!["fetch".to_string()];
                if let Some(true) = all {
                    args.push("--all".to_string());
                }
                if let Some(r) = remote {
                    args.push(r.clone());
                }
                (args, "git fetch".to_string())
            }
            GitCommand::Merge { branch, no_ff } => {
                let mut args = vec!["merge".to_string()];
                if let Some(true) = no_ff {
                    args.push("--no-ff".to_string());
                }
                args.push(branch.clone());
                (args, format!("git merge {}", branch))
            }
            GitCommand::Rebase { branch } => {
                let mut args = vec!["rebase".to_string()];
                args.push(branch.clone());
                (args, format!("git rebase {}", branch))
            }
            GitCommand::Stash { save, pop, list } => {
                let mut args = vec!["stash".to_string()];
                if let Some(true) = save {
                    // save по умолчанию
                }
                if let Some(true) = pop {
                    args.push("pop".to_string());
                }
                if let Some(true) = list {
                    args.push("list".to_string());
                }
                (args, "git stash".to_string())
            }
            GitCommand::Show { reference } => {
                let mut args = vec!["show".to_string()];
                args.push(reference.clone());
                (args, format!("git show {}", reference))
            }
        }
    }

    /// Быстрая проверка статуса
    pub async fn status(&self) -> Result<GitResult> {
        self.execute(GitCommand::Status { short: Some(true) }).await
    }

    /// Быстрый commit + push
    pub async fn commit_and_push(&self, message: &str) -> Result<GitResult> {
        self.execute(GitCommand::Add { files: vec![".".to_string()] }).await?;
        self.execute(GitCommand::Commit { message: message.to_string(), all: Some(true) }).await?;
        self.execute(GitCommand::Push { remote: None, branch: None, force: Some(false) }).await
    }

    /// Получить текущую ветку
    pub async fn current_branch(&self) -> Result<String> {
        let result = self.execute(GitCommand::Branch {
            list: Some(true),
            create: None,
            delete: None,
            checkout: None,
        }).await?;

        if result.success {
            // Парсим вывод git branch
            for line in result.output.lines() {
                if line.starts_with("* ") {
                    return Ok(line[2..].trim().to_string());
                }
            }
        }

        anyhow::bail!("Failed to get current branch")
    }

    /// Проверить, является ли директория git репозиторием
    pub async fn is_repo(&self) -> bool {
        let result = self.execute(GitCommand::Status { short: Some(true) }).await;
        result.map(|r| r.success).unwrap_or(false)
    }
}

impl Default for GitSkill {
    fn default() -> Self {
        Self::new(None)
    }
}

// Actor для интеграции с EventBus
use crate::bus::{EventBus, SystemEvent};
use std::sync::Arc;

pub struct GitActor;

impl GitActor {
    pub async fn run_daemon(bus: Arc<EventBus>, working_dir: Option<PathBuf>) -> Result<()> {
        let mut rx = bus.subscribe();
        let git = GitSkill::new(working_dir);

        bus.emit(SystemEvent::Log {
            level: "INFO".into(),
            source: "Git".into(),
            message: "Git actor started".into(),
        });

        while let Ok(SystemEvent::TaskDispatched { task_id, tool, input }) = rx.recv().await {
            if tool == "git" || tool.starts_with("git:") {
                let bus_clone = bus.clone();
                let git_ref = &git;
                let task_id_clone = task_id.clone();
                let input_clone = input.clone();

                tokio::spawn(async move {
                    bus_clone.emit(SystemEvent::Log {
                        level: "INFO".into(),
                        source: "Git".into(),
                        message: format!("Executing: {}", input_clone),
                    });

                    // Парсим команду из input
                    let result = match Self::parse_command(&input_clone) {
                        Ok(cmd) => git_ref.execute(cmd).await,
                        Err(e) => Ok(GitResult {
                            success: false,
                            output: String::new(),
                            error: Some(e),
                            command: input_clone,
                            duration_secs: 0.0,
                        }),
                    };

                    match result {
                        Ok(git_result) => {
                            if git_result.success {
                                bus_clone.emit(SystemEvent::TaskCompleted {
                                    task_id: task_id_clone,
                                    result: format!(
                                        "Command: {}\nOutput:\n{}",
                                        git_result.command,
                                        git_result.output
                                    ),
                                });
                            } else {
                                bus_clone.emit(SystemEvent::TaskFailed {
                                    task_id: task_id_clone,
                                    error: git_result.error.unwrap_or("Unknown error".to_string()),
                                });
                            }
                        }
                        Err(e) => {
                            bus_clone.emit(SystemEvent::TaskFailed {
                                task_id: task_id_clone,
                                error: e.to_string(),
                            });
                        }
                    }
                });
            }
        }

        Ok(())
    }

    fn parse_command(input: &str) -> Result<GitCommand, String> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        match parts[0] {
            "clone" => Ok(GitCommand::Clone {
                url: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
                path: parts.get(2).map(|s| s.to_string()),
            }),
            "status" => Ok(GitCommand::Status { short: Some(true) }),
            "commit" => Ok(GitCommand::Commit {
                message: input.replace("commit ", "").trim().to_string(),
                all: Some(true),
            }),
            "push" => Ok(GitCommand::Push {
                remote: None,
                branch: None,
                force: Some(false),
            }),
            "pull" => Ok(GitCommand::Pull {
                remote: None,
                branch: None,
                rebase: Some(false),
            }),
            "log" => Ok(GitCommand::Log {
                limit: Some(10),
                oneline: Some(true),
            }),
            _ => Err(format!("Unknown git command: {}", parts[0])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Требуется git в системе
    async fn test_git_status() {
        let git = GitSkill::new(None);
        let result = git.status().await.unwrap();
        println!("Status: {}", result.output);
    }

    #[tokio::test]
    #[ignore] // Требуется git в системе
    async fn test_git_log() {
        let git = GitSkill::new(None);
        let result = git.execute(GitCommand::Log {
            limit: Some(5),
            oneline: Some(true),
        }).await.unwrap();
        
        assert!(result.success);
        println!("Log: {}", result.output);
    }
}
