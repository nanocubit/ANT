//! Dummy Actors - базовые реализации для тестирования
//! Могут быть заменены на реальные реализации

use crate::bus::{EventBus, SystemEvent};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Browser Actor - заглушка (реальная реализация в browser.rs)
pub async fn run_browser_daemon(bus: Arc<EventBus>) -> anyhow::Result<()> {
    let mut rx = bus.subscribe();
    
    while let Ok(SystemEvent::TaskDispatched { task_id, tool, input }) = rx.recv().await {
        if tool == "browser" || tool == "scrape" {
            let bus_clone = bus.clone();
            let url = input.clone();
            let task_id_clone = task_id.clone();

            tokio::spawn(async move {
                bus_clone.emit(SystemEvent::Log {
                    level: "WARN".into(),
                    source: "Browser".into(),
                    message: format!("Browser actor not available. Install headless_chrome feature."),
                });

                // Эмуляция ответа
                sleep(Duration::from_millis(500)).await;
                
                bus_clone.emit(SystemEvent::TaskCompleted {
                    task_id: task_id_clone,
                    result: format!(
                        "Browser simulation for: {}\n\n\
                        Note: Real browser requires 'headless_chrome' feature.\n\
                        Build with: cargo build --features with-browser",
                        url
                    ),
                });
            });
        }
    }
    Ok(())
}

/// LSP Actor - базовая реализация для rust-analyzer
pub async fn run_lsp_daemon(bus: Arc<EventBus>) -> anyhow::Result<()> {
    let mut rx = bus.subscribe();

    while let Ok(SystemEvent::TaskDispatched { task_id, tool, input }) = rx.recv().await {
        if tool == "lsp" || tool == "analyze" || tool == "review" {
            let bus_clone = bus.clone();
            let task_id_clone = task_id.clone();
            let code = input.clone();

            tokio::spawn(async move {
                bus_clone.emit(SystemEvent::Log {
                    level: "INFO".into(),
                    source: "LSP".into(),
                    message: "Analyzing code...".into(),
                });

                // Эмуляция анализа
                sleep(Duration::from_millis(100)).await;

                // Простой статический анализ
                let mut diagnostics = Vec::new();
                
                if code.contains("unsafe") {
                    diagnostics.push("⚠️ Warning: unsafe code detected");
                }
                
                if code.contains("unwrap()") {
                    diagnostics.push("⚠️ Suggestion: Consider using Result handling instead of unwrap()");
                }
                
                if !code.contains("fn main()") && !code.contains("pub fn") && !code.contains("fn ") {
                    diagnostics.push("ℹ️ Info: No function definitions found");
                }

                let result = if diagnostics.is_empty() {
                    "✅ No issues found".to_string()
                } else {
                    diagnostics.join("\n")
                };

                bus_clone.emit(SystemEvent::TaskCompleted {
                    task_id: task_id_clone,
                    result: format!("Code Analysis:\n{}\n\nCode preview:\n{}", result, code.chars().take(500).collect::<String>()),
                });
            });
        }
    }
    Ok(())
}

/// Shell Actor - базовая эмуляция (реальная в orchestrator.rs)
pub async fn run_shell_daemon(bus: Arc<EventBus>) -> anyhow::Result<()> {
    let mut rx = bus.subscribe();

    while let Ok(SystemEvent::TaskDispatched { task_id, tool, input }) = rx.recv().await {
        if tool == "shell" || tool == "run" || tool == "exec" {
            let bus_clone = bus.clone();
            let task_id_clone = task_id.clone();
            let cmd = input.clone();

            tokio::spawn(async move {
                bus_clone.emit(SystemEvent::Log {
                    level: "WARN".into(),
                    source: "Shell".into(),
                    message: "Shell execution not available in dummy mode".into(),
                });

                sleep(Duration::from_millis(100)).await;

                bus_clone.emit(SystemEvent::TaskCompleted {
                    task_id: task_id_clone,
                    result: format!(
                        "Shell command simulation: {}\n\n\
                        Note: Real shell execution requires sandbox integration.\n\
                        Use orchestrator for real command execution.",
                        cmd
                    ),
                });
            });
        }
    }
    Ok(())
}
