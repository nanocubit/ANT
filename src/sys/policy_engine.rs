//! Policy Engine - система управления правами доступа
//! Реализует проверку capabilities для операций

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::collections::HashSet;

/// Типы разрешений (Capabilities)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Доступ к сети
    NetworkAccess,
    /// Чтение файлов
    FileSystemRead,
    /// Запись файлов
    FileSystemWrite,
    /// Выполнение бинарников
    ExecuteBinaries,
    /// Доступ к конкретному пути
    AccessPath(PathBuf),
    /// Доступ к переменным окружения
    EnvironmentAccess,
    /// Доступ к WASM
    WasmExecution,
}

/// Политика для инструмента
#[derive(Debug, Clone)]
pub struct ToolPolicy {
    pub name: String,
    pub capabilities: HashSet<Capability>,
    pub max_execution_time_ms: u64,
    pub max_memory_mb: u64,
    pub allowed_paths: Vec<PathBuf>,
}

impl ToolPolicy {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            capabilities: HashSet::new(),
            max_execution_time_ms: 30000, // 30 секунд по умолчанию
            max_memory_mb: 256,
            allowed_paths: vec![],
        }
    }

    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.insert(cap);
        self
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.max_execution_time_ms = ms;
        self
    }

    pub fn with_memory_limit(mut self, mb: u64) -> Self {
        self.max_memory_mb = mb;
        self
    }

    pub fn with_allowed_path(mut self, path: &str) -> Self {
        self.allowed_paths.push(PathBuf::from(path));
        self
    }
}

/// Policy Engine - проверяет права доступа
pub struct PolicyEngine {
    policies: std::collections::HashMap<String, ToolPolicy>,
    workspace_root: PathBuf,
}

impl PolicyEngine {
    /// Создать новый Policy Engine
    pub fn new(workspace_root: &str) -> Result<Self> {
        let workspace = PathBuf::from(workspace_root);
        
        if !workspace.exists() {
            std::fs::create_dir_all(&workspace)?;
        }

        let mut engine = Self {
            policies: std::collections::HashMap::new(),
            workspace_root: workspace,
        };

        // Регистрируем политики по умолчанию
        engine.register_default_policies();

        Ok(engine)
    }

    /// Зарегистрировать политики по умолчанию
    fn register_default_policies(&mut self) {
        // Browser policy
        let browser_policy = ToolPolicy::new("browser")
            .with_capability(Capability::NetworkAccess)
            .with_capability(Capability::FileSystemWrite)
            .with_timeout(60000)
            .with_memory_limit(512)
            .with_allowed_path(&self.workspace_root.to_string_lossy());

        self.policies.insert("browser".to_string(), browser_policy);

        // LSP policy
        let lsp_policy = ToolPolicy::new("lsp")
            .with_capability(Capability::FileSystemRead)
            .with_capability(Capability::ExecuteBinaries)
            .with_timeout(30000)
            .with_memory_limit(1024);

        self.policies.insert("lsp".to_string(), lsp_policy);

        // WASM policy
        let wasm_policy = ToolPolicy::new("wasm")
            .with_capability(Capability::WasmExecution)
            .with_capability(Capability::FileSystemRead)
            .with_timeout(10000)
            .with_memory_limit(128);

        self.policies.insert("wasm".to_string(), wasm_policy);

        // Git policy
        let git_policy = ToolPolicy::new("git")
            .with_capability(Capability::FileSystemRead)
            .with_capability(Capability::FileSystemWrite)
            .with_capability(Capability::ExecuteBinaries)
            .with_timeout(120000)
            .with_allowed_path(&self.workspace_root.to_string_lossy());

        self.policies.insert("git".to_string(), git_policy);

        // Shell policy
        let shell_policy = ToolPolicy::new("shell")
            .with_capability(Capability::ExecuteBinaries)
            .with_capability(Capability::FileSystemRead)
            .with_capability(Capability::FileSystemWrite)
            .with_timeout(60000)
            .with_allowed_path(&self.workspace_root.to_string_lossy());

        self.policies.insert("shell".to_string(), shell_policy);
    }

    /// Проверить доступ для инструмента
    pub fn check_access(&self, tool: &str, operation: &Capability) -> Result<()> {
        let policy = self.policies
            .get(tool)
            .ok_or_else(|| anyhow::anyhow!("Policy not found for tool: {}", tool))?;

        if !policy.capabilities.contains(operation) {
            bail!(
                "Access denied: tool '{}' does not have capability {:?}",
                tool,
                operation
            );
        }

        Ok(())
    }

    /// Проверить путь для операции
    pub fn check_path(&self, tool: &str, path: &Path) -> Result<()> {
        let policy = self.policies
            .get(tool)
            .ok_or_else(|| anyhow::anyhow!("Policy not found for tool: {}", tool))?;

        // Если есть allowed_paths, проверяем что путь внутри
        if !policy.allowed_paths.is_empty() {
            let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            
            let is_allowed = policy.allowed_paths.iter().any(|allowed| {
                canonical_path.starts_with(allowed)
            });

            if !is_allowed {
                bail!(
                    "Access denied: path {:?} is not in allowed paths for tool '{}'",
                    path,
                    tool
                );
            }
        }

        Ok(())
    }

    /// Получить политику для инструмента
    pub fn get_policy(&self, tool: &str) -> Option<&ToolPolicy> {
        self.policies.get(tool)
    }

    /// Зарегистрировать новую политику
    pub fn register_policy(&mut self, name: &str, policy: ToolPolicy) {
        self.policies.insert(name.to_string(), policy);
    }

    /// Получить workspace root
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new(".ant/workspace").expect("Failed to create PolicyEngine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = ToolPolicy::new("test")
            .with_capability(Capability::NetworkAccess)
            .with_timeout(5000);

        assert_eq!(policy.name, "test");
        assert!(policy.capabilities.contains(&Capability::NetworkAccess));
        assert_eq!(policy.max_execution_time_ms, 5000);
    }

    #[test]
    fn test_policy_engine() {
        let engine = PolicyEngine::new("/tmp/test_policy").unwrap();
        
        // Проверка доступа
        let result = engine.check_access("browser", &Capability::NetworkAccess);
        assert!(result.is_ok());

        // Проверка запрета
        let result = engine.check_access("browser", &Capability::WasmExecution);
        assert!(result.is_err());
    }
}
