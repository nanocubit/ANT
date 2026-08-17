#![cfg(feature = "with-wasm")]

//! WASM Runtime для выполнения навыков
//! Изолированное выполнение с ограничением ресурсов (fuel, memory, time)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use wasmtime::*;
use wasmtime_wasi::*;

/// Манифест WASM навыка
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Имя навыка
    pub name: String,
    /// Версия (SemVer)
    pub version: String,
    /// Описание
    pub description: Option<String>,
    /// Автор
    pub author: Option<String>,
    /// Разрешения
    pub permissions: SkillPermissions,
    /// Ограничения ресурсов
    pub resources: ResourceLimits,
    /// Экспортированные функции
    pub exports: SkillExports,
    /// Зависимости (другие навыки)
    pub dependencies: Option<Vec<String>>,
}

/// Разрешения для навыка
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillPermissions {
    /// Доступ к файловой системе
    pub filesystem: Option<FileSystemAccess>,
    /// Доступ к сети
    pub network: Option<bool>,
    /// Разрешённые команды для выполнения
    pub execute: Option<Vec<String>>,
    /// Доступ к переменным окружения
    pub env: Option<Vec<String>>,
}

/// Доступ к файловой системе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemAccess {
    /// Разрешённые пути
    pub allowed_paths: Vec<String>,
    /// Режим доступа
    pub mode: FsMode,
}

/// Режим доступа к ФС
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FsMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

/// Ограничения ресурсов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Максимум fuel (CPU единицы)
    pub max_fuel: u64,
    /// Максимум памяти (MB)
    pub max_memory_mb: Option<u64>,
    /// Таймаут выполнения (секунды)
    pub timeout_secs: Option<u64>,
    /// Максимум stack frames
    pub max_stack_frames: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_fuel: 1_000_000, // ~1 секунда CPU
            max_memory_mb: Some(64),
            timeout_secs: Some(30),
            max_stack_frames: Some(100),
        }
    }
}

/// Экспортированные функции навыка
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillExports {
    /// Основная функция обработки
    pub process: Option<String>,
    /// Функция инициализации
    pub init: Option<String>,
    /// Функция очистки
    pub cleanup: Option<String>,
}

/// Конфигурация WASM runtime
#[derive(Debug, Clone)]
pub struct WasmRuntimeConfig {
    /// Директория с навыками
    pub skills_dir: PathBuf,
    /// Включить logging
    pub enable_logging: bool,
    /// Разрешить network
    pub allow_network: bool,
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            skills_dir: PathBuf::from("skills"),
            enable_logging: true,
            allow_network: false,
        }
    }
}

/// Контекст выполнения навыка
pub struct SkillContext {
    pub manifest: SkillManifest,
    pub wasm_bytes: Vec<u8>,
    pub loaded_at: Instant,
    pub execution_count: u64,
    pub last_error: Option<String>,
}

/// WASM Runtime
pub struct WasmRuntime {
    engine: Engine,
    linker: Linker<WasiState>,
    config: WasmRuntimeConfig,
    skills: HashMap<String, SkillContext>,
}

impl WasmRuntime {
    /// Создать новый WASM runtime
    pub fn new(config: WasmRuntimeConfig) -> Result<Self> {
        // Создаём директорию навыков если не существует
        if !config.skills_dir.exists() {
            std::fs::create_dir_all(&config.skills_dir)?;
        }

        // Конфигурация движка
        let mut engine_config = Config::new();
        engine_config.consume_fuel(true);
        engine_config.cranelift_opt_level(OptLevel::Speed);
        
        // Ограничение stack frames
        engine_config.max_wasm_stack(1024 * 1024); // 1MB

        let engine = Engine::new(&engine_config)?;

        // Создаём linker для WASI
        let mut linker = Linker::<WasiState>::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

        Ok(Self {
            engine,
            linker,
            config,
            skills: HashMap::new(),
        })
    }

    /// Загрузить навык из WASM файла
    pub fn load_skill(&mut self, skill_name: &str) -> Result<()> {
        let wasm_path = self.config.skills_dir.join(format!("{}.wasm", skill_name));
        let manifest_path = self.config.skills_dir.join(format!("{}.toml", skill_name));

        if !wasm_path.exists() {
            anyhow::bail!("WASM file not found: {}", wasm_path.display());
        }

        // Читаем манифест
        let manifest: SkillManifest = if manifest_path.exists() {
            let manifest_content = std::fs::read_to_string(&manifest_path)?;
            toml::from_str(&manifest_content)
                .with_context(|| format!("Failed to parse manifest: {}", manifest_path.display()))?
        } else {
            // Манифест по умолчанию
            SkillManifest {
                name: skill_name.to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                permissions: SkillPermissions::default(),
                resources: ResourceLimits::default(),
                exports: SkillExports::default(),
                dependencies: None,
            }
        };

        // Читаем WASM байты
        let wasm_bytes = std::fs::read(&wasm_path)
            .with_context(|| format!("Failed to read WASM file: {}", wasm_path.display()))?;

        // Валидация модуля
        Module::new(&self.engine, &wasm_bytes)
            .with_context(|| format!("Invalid WASM module: {}", wasm_path.display()))?;

        // Сохраняем контекст навыка
        self.skills.insert(
            skill_name.to_string(),
            SkillContext {
                manifest,
                wasm_bytes,
                loaded_at: Instant::now(),
                execution_count: 0,
                last_error: None,
            },
        );

        Ok(())
    }

    /// Загрузить навык из байтов
    pub fn load_skill_from_bytes(
        &mut self,
        skill_name: &str,
        wasm_bytes: &[u8],
        manifest: Option<SkillManifest>,
    ) -> Result<()> {
        // Валидация
        Module::new(&self.engine, wasm_bytes)
            .with_context(|| "Invalid WASM module")?;

        let manifest = manifest.unwrap_or_else(|| SkillManifest {
            name: skill_name.to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            permissions: SkillPermissions::default(),
            resources: ResourceLimits::default(),
            exports: SkillExports::default(),
            dependencies: None,
        });

        self.skills.insert(
            skill_name.to_string(),
            SkillContext {
                manifest,
                wasm_bytes: wasm_bytes.to_vec(),
                loaded_at: Instant::now(),
                execution_count: 0,
                last_error: None,
            },
        );

        Ok(())
    }

    /// Выполнить навык
    pub fn execute_skill(
        &mut self,
        skill_name: &str,
        input: &[u8],
    ) -> Result<Vec<u8>> {
        let skill_ctx = self.skills
            .get(skill_name)
            .with_context(|| format!("Skill '{}' not found", skill_name))?
            .clone();

        let start_time = Instant::now();

        // Создаём store с fuel
        let mut store = self.create_store(&skill_ctx.manifest)?;

        // Загружаем модуль
        let module = Module::new(&self.engine, &skill_ctx.wasm_bytes)?;
        let instance = self.linker.instantiate(&mut store, &module)?;

        // Получаем функцию process
        let process_func = instance
            .get_typed_func::<(u32, u32), u64>(&mut store, "process")
            .or_else(|_| {
                instance.get_typed_func::<(u32, u32), u32>(&mut store, "process")
                    .map(|f| {
                        // Адаптируем u32 -> u64
                        move |mut store, (ptr, len)| {
                            let result = f.call(&mut store, (ptr, len))?;
                            Ok(result as u64)
                        }
                    })
            })
            .with_context(|| "Skill must export 'process' function")?;

        // Получаем память
        let memory = instance
            .get_memory(&mut store, "memory")
            .context("Skill must export 'memory'")?;

        // Выделяем память для ввода
        let input_len = input.len() as u32;
        let input_ptr = memory.data_size(&mut store) as u32;

        // Копируем ввод в память WASM
        memory
            .write(&mut store, input_ptr as usize, input)
            .context("Failed to write input to WASM memory")?;

        // Выполняем с таймаутом
        let result_ptr = if let Some(timeout) = skill_ctx.manifest.resources.timeout_secs {
            let result = tokio::time::timeout(
                Duration::from_secs(timeout),
                tokio::task::spawn_blocking(move || {
                    process_func.call(&mut store, (input_ptr, input_len))
                })
            ).await;

            match result {
                Ok(Ok(Ok(ptr))) => ptr,
                Ok(Err(e)) => anyhow::bail!("WASM execution failed: {}", e),
                Err(_) => anyhow::bail!("WASM execution timeout after {} seconds", timeout),
            }
        } else {
            process_func.call(&mut store, (input_ptr, input_len))?
        };

        // Читаем результат из памяти
        // Предполагаем что результат имеет формат: [ptr: u32][len: u32] упакованные в u64
        let ptr = result_ptr;
        let result_ptr = (ptr >> 32) as u32;
        let result_len = (ptr & 0xFFFFFFFF) as u32;

        let result_data = memory
            .data(&store)[result_ptr as usize..(result_ptr + result_len) as usize]
            .to_vec();

        // Обновляем статистику
        if let Some(ctx) = self.skills.get_mut(skill_name) {
            ctx.execution_count += 1;
        }

        Ok(result_data)
    }

    /// Создать store с ограничениями
    fn create_store(&self, manifest: &SkillManifest) -> Result<Store<WasiState>> {
        // Создаём WASI контекст с использованием нового API
        let mut wasi_builder = WasiCtxBuilder::new();
        
        // Наследуем stdio
        wasi_builder.inherit_stdio();

        // Добавляем разрешённые пути (используем std::fs для простоты)
        if let Some(fs_access) = &manifest.permissions.filesystem {
            for path in &fs_access.allowed_paths {
                if let Ok(dir) = std::fs::read_dir(path) {
                    // Для wasmtime 14.0 используем простой метод
                    wasi_builder.preopen_dir(
                        std::fs::File::open(path)?,
                        path
                    )?;
                }
            }
        }

        // Добавляем переменные окружения
        if let Some(env_vars) = &manifest.permissions.env {
            for var in env_vars {
                if let Ok(value) = std::env::var(var) {
                    wasi_builder.env(var, &value)?;
                }
            }
        }

        let wasi = wasi_builder.build()?;
        let mut store = Store::new(&self.engine, wasi);

        // Устанавливаем лимит fuel
        store.set_fuel(manifest.resources.max_fuel)?;

        Ok(store)
    }

    /// Получить список загруженных навыков
    pub fn list_skills(&self) -> Vec<SkillInfo> {
        self.skills
            .iter()
            .map(|(name, ctx)| SkillInfo {
                name: name.clone(),
                version: ctx.manifest.version.clone(),
                description: ctx.manifest.description.clone(),
                execution_count: ctx.execution_count,
                loaded_at: ctx.loaded_at.elapsed().as_secs(),
            })
            .collect()
    }

    /// Получить информацию о навыке
    pub fn get_skill_info(&self, skill_name: &str) -> Option<SkillInfo> {
        self.skills.get(skill_name).map(|ctx| SkillInfo {
            name: skill_name.to_string(),
            version: ctx.manifest.version.clone(),
            description: ctx.manifest.description.clone(),
            execution_count: ctx.execution_count,
            loaded_at: ctx.loaded_at.elapsed().as_secs(),
        })
    }

    /// Выгрузить навык
    pub fn unload_skill(&mut self, skill_name: &str) -> bool {
        self.skills.remove(skill_name).is_some()
    }

    /// Перезагрузить навык
    pub fn reload_skill(&mut self, skill_name: &str) -> Result<()> {
        self.unload_skill(skill_name);
        self.load_skill(skill_name)
    }

    /// Получить статистику runtime
    pub fn get_stats(&self) -> WasmRuntimeStats {
        let total_skills = self.skills.len();
        let total_executions: u64 = self.skills.values().map(|s| s.execution_count).sum();
        let total_memory: usize = self.skills.values().map(|s| s.wasm_bytes.len()).sum();

        WasmRuntimeStats {
            total_skills,
            total_executions,
            total_memory_bytes: total_memory,
        }
    }
}

/// Информация о навыке
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub execution_count: u64,
    pub loaded_at: u64, // секунд назад
}

/// Статистика WASM runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRuntimeStats {
    pub total_skills: usize,
    pub total_executions: u64,
    pub total_memory_bytes: usize,
}

/// Actor для интеграции с EventBus
use crate::bus::{EventBus, SystemEvent};
use std::sync::Arc as StdArc;
use tokio::sync::Mutex;

pub struct WasmRuntimeActor {
    runtime: StdArc<Mutex<WasmRuntime>>,
}

impl WasmRuntimeActor {
    pub fn new(runtime: WasmRuntime) -> Self {
        Self {
            runtime: StdArc::new(Mutex::new(runtime)),
        }
    }

    pub async fn run_daemon(bus: Arc<EventBus>) -> Result<()> {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmRuntime::new(config)?;
        let actor = Self::new(runtime);

        bus.emit(SystemEvent::Log {
            level: "INFO".into(),
            source: "WasmRuntime".into(),
            message: "WASM runtime initialized".into(),
        });

        let mut rx = bus.subscribe();
        let runtime_arc = actor.runtime.clone();

        while let Ok(SystemEvent::TaskDispatched { task_id, tool, input }) = rx.recv().await {
            if tool.starts_with("wasm:") {
                let skill_name = tool.strip_prefix("wasm:").unwrap_or("unknown").to_string();
                let bus_clone = bus.clone();
                let runtime_clone = runtime_arc.clone();
                let task_id_clone = task_id.clone();
                let input_clone = input.clone();

                tokio::spawn(async move {
                    bus_clone.emit(SystemEvent::Log {
                        level: "INFO".into(),
                        source: "WasmRuntime".into(),
                        message: format!("Executing WASM skill: {}", skill_name),
                    });

                    let mut runtime = runtime_clone.lock().await;
                    
                    // Пытаемся загрузить навык если ещё не загружен
                    if runtime.get_skill_info(&skill_name).is_none() {
                        if let Err(e) = runtime.load_skill(&skill_name) {
                            bus_clone.emit(SystemEvent::Log {
                                level: "WARN".into(),
                                source: "WasmRuntime".into(),
                                message: format!("Failed to load skill '{}': {}", skill_name, e),
                            });
                        }
                    }

                    // Выполняем навык
                    match runtime.execute_skill(&skill_name, input_clone.as_bytes()) {
                        Ok(output) => {
                            let output_str = String::from_utf8_lossy(&output).to_string();
                            bus_clone.emit(SystemEvent::TaskCompleted {
                                task_id: task_id_clone,
                                result: output_str,
                            });
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let manifest = SkillManifest {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test skill".to_string()),
            author: Some("Test Author".to_string()),
            permissions: SkillPermissions::default(),
            resources: ResourceLimits::default(),
            exports: SkillExports::default(),
            dependencies: None,
        };

        let toml_str = toml::to_string(&manifest).unwrap();
        let parsed: SkillManifest = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.name, "test-skill");
        assert_eq!(parsed.version, "1.0.0");
    }
}

