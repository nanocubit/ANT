#![cfg(feature = "with-wasm")]

//! WASM Actor - демон для выполнения WASM навыков
//! Интегрируется с WasmRuntime для реального выполнения

use crate::bus::{EventBus, SystemEvent};
use crate::tools::wasm_runtime::{WasmRuntime, WasmRuntimeConfig, SkillManifest};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct WasmActor;

impl WasmActor {
    pub async fn run_daemon(bus: Arc<EventBus>) -> anyhow::Result<()> {
        let mut rx = bus.subscribe();

        // Инициализация WASM runtime
        let config = WasmRuntimeConfig::default();
        let runtime = Arc::new(Mutex::new(WasmRuntime::new(config)?));

        bus.emit(SystemEvent::Log {
            level: "INFO".into(),
            source: "WasmActor".into(),
            message: "WASM runtime initialized".into(),
        });

        // Загрузка доступных навыков
        let runtime_clone = runtime.clone();
        let bus_clone = bus.clone();
        tokio::spawn(async move {
            let mut rt = runtime_clone.lock().await;
            
            // Попытка загрузить навыки из директории
            if let Ok(entries) = std::fs::read_dir(&rt.config().skills_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().map_or(false, |ext| ext == "wasm") {
                        if let Some(name) = entry.path().file_stem() {
                            let name = name.to_string_lossy().to_string();
                            match rt.load_skill(&name) {
                                Ok(_) => {
                                    bus_clone.emit(SystemEvent::Log {
                                        level: "INFO".into(),
                                        source: "WasmActor".into(),
                                        message: format!("Loaded skill: {}", name),
                                    });
                                }
                                Err(e) => {
                                    bus_clone.emit(SystemEvent::Log {
                                        level: "WARN".into(),
                                        source: "WasmActor".into(),
                                        message: format!("Failed to load skill {}: {}", name, e),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        });

        // Обработка задач
        while let Ok(SystemEvent::TaskDispatched { task_id, tool, input }) = rx.recv().await {
            if tool.starts_with("wasm:") {
                let skill_name = tool.strip_prefix("wasm:").unwrap_or("unknown").to_string();
                let bus_clone = bus.clone();
                let runtime_clone = runtime.clone();
                let task_id_clone = task_id.clone();
                let input_clone = input.clone();

                tokio::spawn(async move {
                    bus_clone.emit(SystemEvent::Log {
                        level: "INFO".into(),
                        source: "WasmActor".into(),
                        message: format!("Executing WASM skill: {}", skill_name),
                    });

                    let mut rt = runtime_clone.lock().await;

                    // Проверяем загружен ли навык
                    if rt.get_skill_info(&skill_name).is_none() {
                        // Пытаемся загрузить
                        if let Err(e) = rt.load_skill(&skill_name) {
                            bus_clone.emit(SystemEvent::TaskFailed {
                                task_id: task_id_clone,
                                error: format!("Failed to load skill '{}': {}", skill_name, e),
                            });
                            return;
                        }
                    }

                    // Выполняем навык
                    match rt.execute_skill(&skill_name, input_clone.as_bytes()) {
                        Ok(output) => {
                            let output_str = String::from_utf8_lossy(&output).to_string();
                            bus_clone.emit(SystemEvent::TaskCompleted {
                                task_id: task_id_clone,
                                result: output_str,
                            });

                            bus_clone.emit(SystemEvent::Log {
                                level: "INFO".into(),
                                source: "WasmActor".into(),
                                message: format!("Skill {} completed", skill_name),
                            });
                        }
                        Err(e) => {
                            bus_clone.emit(SystemEvent::TaskFailed {
                                task_id: task_id_clone,
                                error: format!("WASM execution error: {}", e),
                            });

                            bus_clone.emit(SystemEvent::Log {
                                level: "ERROR".into(),
                                source: "WasmActor".into(),
                                message: format!("Skill {} failed: {}", skill_name, e),
                            });
                        }
                    }
                });
            }
        }

        Ok(())
    }
}

