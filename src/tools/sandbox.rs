use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use std::process::Stdio;

/// Workspace Sandbox - изолированное окружение для выполнения операций
pub struct WorkspaceSandbox {
    root_dir: PathBuf,
}

/// Capability (право доступа) для операций
#[derive(Debug, Clone, PartialEq)]
pub enum Capability {
    NetworkAccess,
    FileSystemRead,
    FileSystemWrite,
    ExecuteBinaries,
    AccessPath(PathBuf),
}

/// Конфигурация песочницы
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub root_path: PathBuf,
    pub allow_network: bool,
    pub allow_execute: bool,
    pub max_file_size_bytes: u64,
    pub allowed_extensions: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            root_path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ant")
                .join("workspace"),
            allow_network: true,
            allow_execute: true,
            max_file_size_bytes: 10 * 1024 * 1024, // 10 MB
            allowed_extensions: vec![
                ".rs".to_string(),
                ".toml".to_string(),
                ".json".to_string(),
                ".md".to_string(),
                ".txt".to_string(),
                ".py".to_string(),
                ".js".to_string(),
                ".ts".to_string(),
                ".html".to_string(),
                ".css".to_string(),
            ],
        }
    }
}

impl WorkspaceSandbox {
    /// Создание новой песочницы
    pub fn new() -> Result<Self> {
        Self::with_config(SandboxConfig::default())
    }

    /// Создание с кастомной конфигурацией
    pub fn with_config(config: SandboxConfig) -> Result<Self> {
        let root = config.root_path;

        if !root.exists() {
            std::fs::create_dir_all(&root)?;
        }

        // Нормализуем путь (особенно важно для Windows)
        let canonical_root = dunce::canonicalize(&root)?;

        Ok(Self {
            root_dir: canonical_root,
        })
    }

    /// Проверка: находится ли путь строго внутри песочницы?
    pub fn secure_path(&self, relative_path: &str) -> Result<PathBuf> {
        let target = self.root_dir.join(relative_path);

        // Нормализуем путь
        let canonical_target = if target.exists() {
            dunce::canonicalize(&target)?
        } else {
            // Если файл не существует, проверяем родительскую директорию
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
                dunce::canonicalize(parent)?.join(target.file_name().unwrap())
            } else {
                target
            }
        };

        // Проверяем что путь начинается с root_dir
        if !canonical_target.starts_with(&self.root_dir) {
            bail!(
                "⛔ ОШИБКА БЕЗОПАСНОСТИ: Попытка выхода из песочницы (Path Traversal)! \
                Путь: {:?}, Root: {:?}",
                canonical_target,
                self.root_dir
            );
        }

        Ok(canonical_target)
    }

    /// Проверка capability для пути
    pub fn check_capability(&self, path: &str, capability: &Capability) -> Result<()> {
        match capability {
            Capability::FileSystemRead | Capability::FileSystemWrite => {
                let secure_path = self.secure_path(path)?;
                if !secure_path.starts_with(&self.root_dir) {
                    bail!("Доступ запрещен: путь вне песочницы");
                }
            }
            Capability::AccessPath(allowed_path) => {
                let secure_path = self.secure_path(path)?;
                if !secure_path.starts_with(allowed_path) {
                    bail!("Доступ к пути запрещен");
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Кроссплатформенный запуск команд внутри песочницы
    pub async fn run_cmd(&self, program: &str, args: &[&str]) -> Result<String> {
        // Решение проблемы Windows: npx -> npx.cmd
        #[cfg(target_os = "windows")]
        let program = if program == "npx" {
            "npx.cmd"
        } else if program == "npm" {
            "npm.cmd"
        } else if program == "node" {
            "node.exe"
        } else {
            program
        };

        #[cfg(target_os = "windows")]
        let program = if program == "cargo" {
            "cargo.exe"
        } else {
            program
        };

        let output = Command::new(program)
            .current_dir(&self.root_dir) // Жестко привязываем к песочнице
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            bail!("Command failed: {}", stderr);
        }
    }

    /// Запуск команды с таймаутом
    pub async fn run_cmd_with_timeout(
        &self,
        program: &str,
        args: &[&str],
        timeout_secs: u64,
    ) -> Result<String> {
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(timeout_secs),
            self.run_cmd(program, args),
        )
        .await;

        match result {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(e)) => Err(e),
            Err(_) => bail!("⏳ Timeout: команда превысила лимит {} секунд", timeout_secs),
        }
    }

    /// Чтение файла из песочницы
    pub async fn read_file(&self, relative_path: &str) -> Result<String> {
        self.check_capability(relative_path, &Capability::FileSystemRead)?;
        let secure_path = self.secure_path(relative_path)?;

        let content = tokio::fs::read_to_string(&secure_path).await?;
        Ok(content)
    }

    /// Запись файла в песочницу
    pub async fn write_file(&self, relative_path: &str, content: &str) -> Result<()> {
        self.check_capability(relative_path, &Capability::FileSystemWrite)?;
        let secure_path = self.secure_path(relative_path)?;

        // Проверка расширения
        if let Some(ext) = Path::new(relative_path).extension() {
            let ext_str = format!(".{}", ext.to_string_lossy().to_lowercase());
            let config = SandboxConfig::default();
            if !config.allowed_extensions.is_empty()
                && !config.allowed_extensions.contains(&ext_str)
            {
                bail!("Недопустимое расширение файла: {}", ext_str);
            }
        }

        // Проверка размера
        if content.len() as u64 > SandboxConfig::default().max_file_size_bytes {
            bail!("Файл превышает максимальный размер");
        }

        tokio::fs::write(&secure_path, content).await?;
        Ok(())
    }

    /// Удаление файла из песочницы
    pub async fn delete_file(&self, relative_path: &str) -> Result<()> {
        let secure_path = self.secure_path(relative_path)?;
        if secure_path.exists() {
            tokio::fs::remove_file(&secure_path).await?;
        }
        Ok(())
    }

    /// Список файлов в директории
    pub async fn list_dir(&self, relative_path: &str) -> Result<Vec<String>> {
        let secure_path = self.secure_path(relative_path)?;
        let mut entries = Vec::new();

        let mut dir = tokio::fs::read_dir(&secure_path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(name);
        }

        Ok(entries)
    }

    /// Получить путь к корню песочницы
    pub fn root(&self) -> &Path {
        &self.root_dir
    }

    /// Получить абсолютный путь для относительного пути
    pub fn resolve(&self, relative_path: &str) -> Result<PathBuf> {
        self.secure_path(relative_path)
    }
}

impl Default for WorkspaceSandbox {
    fn default() -> Self {
        Self::new().expect("Failed to create default sandbox")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_creation() {
        let sandbox = WorkspaceSandbox::new().unwrap();
        assert!(sandbox.root().exists());
    }

    #[test]
    fn test_secure_path() {
        let sandbox = WorkspaceSandbox::new().unwrap();

        // Нормальный путь должен работать
        let path = sandbox.secure_path("test.txt").unwrap();
        assert!(path.starts_with(sandbox.root()));

        // Попытка выхода должна блокироваться
        let result = sandbox.secure_path("../etc/passwd");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_operations() {
        let sandbox = WorkspaceSandbox::new().unwrap();

        // Запись
        sandbox.write_file("test.txt", "Hello, World!").await.unwrap();

        // Чтение
        let content = sandbox.read_file("test.txt").await.unwrap();
        assert_eq!(content, "Hello, World!");

        // Удаление
        sandbox.delete_file("test.txt").await.unwrap();
    }
}
