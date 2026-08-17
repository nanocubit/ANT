use crate::bus::{EventBus, SystemEvent};
use crate::tools::sandbox::{WorkspaceSandbox, Capability};
use crate::core::memory::VectorMemory;
use std::sync::Arc;

/// Orchestrator - управляет выполнением задач
pub struct Orchestrator {
    sandbox: WorkspaceSandbox,
    memory: Arc<VectorMemory>,
}

impl Orchestrator {
    pub fn new() -> anyhow::Result<Self> {
        let sandbox = WorkspaceSandbox::new()?;
        let memory = Arc::new(VectorMemory::new("ant_memory.duckdb")?);

        Ok(Self { sandbox, memory })
    }

    /// Обработка задачи
    pub async fn handle_task(
        &self,
        task_id: String,
        tool: String,
        input: String,
        bus: Arc<EventBus>,
    ) -> anyhow::Result<()> {
        match tool.as_str() {
            "browser" | "scrape" => {
                self.handle_browser(task_id, input, bus).await
            }
            "shell" | "run" => {
                self.handle_shell(task_id, input, bus).await
            }
            "file:read" => {
                self.handle_file_read(task_id, input, bus).await
            }
            "file:write" => {
                self.handle_file_write(task_id, input, bus).await
            }
            "memory:search" => {
                self.handle_memory_search(task_id, input, bus).await
            }
            "memory:store" => {
                self.handle_memory_store(task_id, input, bus).await
            }
            _ => {
                // Попытка обработать как LLM запрос
                self.handle_llm(task_id, tool, input, bus).await
            }
        }
    }

    async fn handle_browser(
        &self,
        task_id: String,
        input: String,
        bus: Arc<EventBus>,
    ) -> anyhow::Result<()> {
        #[cfg(feature = "with-browser")]
        {
            use crate::tools::browser::HeadlessBrowser;

            let browser = HeadlessBrowser::new()?;

            bus.emit(SystemEvent::Log {
                level: "INFO".into(),
                source: "Orchestrator".into(),
                message: format!("Scraping URL: {}", input),
            });

            match browser.scrape(&input) {
                Ok(content) => {
                    // Сохраняем в RAG память
                    let source = format!("scrape:{}", input);
                    self.memory.store(&source, &content.text).await.ok();

                    bus.emit(SystemEvent::TaskCompleted {
                        task_id,
                        result: format!(
                            "Title: {}

Preview:
{}",
                            content.title,
                            content.text.chars().take(1000).collect::<String>()
                        ),
                    });
                }
                Err(e) => {
                    bus.emit(SystemEvent::TaskFailed {
                        task_id,
                        error: e.to_string(),
                    });
                }
            }
        }

        #[cfg(not(feature = "with-browser"))]
        {
            bus.emit(SystemEvent::TaskFailed {
                task_id,
                error: "Browser feature not enabled. Compile with --features with-browser".to_string(),
            });
            return Ok(());
        }

        Ok(())
    }

    async fn handle_shell(
        &self,
        task_id: String,
        input: String,
        bus: Arc<EventBus>,
    ) -> anyhow::Result<()> {
        bus.emit(SystemEvent::Log {
            level: "INFO".into(),
            source: "Sandbox".into(),
            message: format!("Executing: {}", input),
        });

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            bus.emit(SystemEvent::TaskFailed {
                task_id,
                error: "Empty command".to_string(),
            });
            return Ok(());
        }

        let cmd = parts[0];
        let args: Vec<&str> = parts[1..].to_vec();

        match self.sandbox.run_cmd_with_timeout(cmd, &args, 60).await {
            Ok(output) => {
                bus.emit(SystemEvent::TaskCompleted {
                    task_id,
                    result: output,
                });
            }
            Err(e) => {
                bus.emit(SystemEvent::TaskFailed {
                    task_id,
                    error: e.to_string(),
                });
            }
        }

        Ok(())
    }

    async fn handle_file_read(
        &self,
        task_id: String,
        input: String,
        bus: Arc<EventBus>,
    ) -> anyhow::Result<()> {
        match self.sandbox.read_file(&input).await {
            Ok(content) => {
                bus.emit(SystemEvent::TaskCompleted {
                    task_id,
                    result: content,
                });
            }
            Err(e) => {
                bus.emit(SystemEvent::TaskFailed {
                    task_id,
                    error: e.to_string(),
                });
            }
        }
        Ok(())
    }

    async fn handle_file_write(
        &self,
        task_id: String,
        input: String,
        bus: Arc<EventBus>,
    ) -> anyhow::Result<()> {
        // Ожидаем формат: "path|content"
        let parts: Vec<&str> = input.splitn(2, '|').collect();
        if parts.len() != 2 {
            bus.emit(SystemEvent::TaskFailed {
                task_id,
                error: "Format: path|content".to_string(),
            });
            return Ok(());
        }

        match self.sandbox.write_file(parts[0], parts[1]).await {
            Ok(_) => {
                bus.emit(SystemEvent::TaskCompleted {
                    task_id,
                    result: format!("File written: {}", parts[0]),
                });
            }
            Err(e) => {
                bus.emit(SystemEvent::TaskFailed {
                    task_id,
                    error: e.to_string(),
                });
            }
        }
        Ok(())
    }

    async fn handle_memory_search(
        &self,
        task_id: String,
        input: String,
        bus: Arc<EventBus>,
    ) -> anyhow::Result<()> {
        bus.emit(SystemEvent::Log {
            level: "INFO".into(),
            source: "RAG".into(),
            message: format!("Searching memory for: {}", input),
        });

        match self.memory.search_content(&input, 5).await {
            Ok(results) => {
                let formatted = results
                    .iter()
                    .map(|r| format!("---
{}", r))
                    .collect::<Vec<_>>()
                    .join("
");

                bus.emit(SystemEvent::TaskCompleted {
                    task_id,
                    result: if formatted.is_empty() {
                        "Ничего не найдено в базе знаний.".to_string()
                    } else {
                        format!("Найдено документов: {}

{}", results.len(), formatted)
                    },
                });
            }
            Err(e) => {
                bus.emit(SystemEvent::TaskFailed {
                    task_id,
                    error: e.to_string(),
                });
            }
        }
        Ok(())
    }

    async fn handle_memory_store(
        &self,
        task_id: String,
        input: String,
        bus: Arc<EventBus>,
    ) -> anyhow::Result<()> {
        // Формат: "source|content"
        let parts: Vec<&str> = input.splitn(2, '|').collect();
        if parts.len() != 2 {
            bus.emit(SystemEvent::TaskFailed {
                task_id,
                error: "Format: source|content".to_string(),
            });
            return Ok(());
        }

        match self.memory.store(parts[0], parts[1]).await {
            Ok(id) => {
                bus.emit(SystemEvent::TaskCompleted {
                    task_id,
                    result: format!("Document stored with ID: {}", id),
                });
            }
            Err(e) => {
                bus.emit(SystemEvent::TaskFailed {
                    task_id,
                    error: e.to_string(),
                });
            }
        }
        Ok(())
    }

    async fn handle_llm(
        &self,
        task_id: String,
        tool: String,
        input: String,
        bus: Arc<EventBus>,
    ) -> anyhow::Result<()> {
        use crate::ai::llm::LlmClient;

        let llm = LlmClient::new();

        // Получаем контекст из RAG
        let context = self.memory.search_content(&input, 3).await.unwrap_or_default();

        bus.emit(SystemEvent::Log {
            level: "INFO".into(),
            source: "LLM".into(),
            message: format!("Processing with LLM: {}", tool),
        });

        match llm.chat_with_context(&input, &context).await {
            Ok(response) => {
                bus.emit(SystemEvent::TaskCompleted {
                    task_id,
                    result: response,
                });
            }
            Err(e) => {
                bus.emit(SystemEvent::TaskFailed {
                    task_id,
                    error: e.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Получить статистику памяти
    pub fn get_memory_stats(&self) -> Option<String> {
        self.memory.get_stats().ok().map(|s| s.to_string())
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new().expect("Failed to create orchestrator")
    }
}

/// Daemon для оркестратора
pub struct OrchestratorDaemon;

impl OrchestratorDaemon {
    pub async fn run_daemon(bus: Arc<EventBus>) -> anyhow::Result<()> {
        let orchestrator = Orchestrator::new()?;
        let orchestrator = Arc::new(orchestrator);

        let mut rx = bus.subscribe();

        bus.emit(SystemEvent::Log {
            level: "INFO".into(),
            source: "Orchestrator".into(),
            message: "Orchestrator daemon started".into(),
        });

        while let Ok(SystemEvent::TaskDispatched { task_id, tool, input }) = rx.recv().await {
            let orch = Arc::clone(&orchestrator);
            let bus = Arc::clone(&bus);
            let task_id = task_id.clone();
            let tool = tool.clone();
            let input = input.clone();

            tokio::spawn(async move {
                if let Err(e) = orch.handle_task(task_id, tool, input, bus).await {
                    eprintln!("Orchestrator error: {}", e);
                }
            });
        }

        Ok(())
    }
}

