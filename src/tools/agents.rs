//! Модуль интеграции с внешними AI агентами
//! Поддержка: Goose, Codex, AgentAPI (Coder)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use std::process::Stdio as ProcStdio;

/// Типы поддерживаемых агентов
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    Goose,
    Codex,
    ClaudeCode,
    Aider,
    Gemini,
    Custom { name: String, command: String },
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Goose => write!(f, "goose"),
            AgentType::Codex => write!(f, "codex"),
            AgentType::ClaudeCode => write!(f, "claude-code"),
            AgentType::Aider => write!(f, "aider"),
            AgentType::Gemini => write!(f, "gemini"),
            AgentType::Custom { name, .. } => write!(f, "{}", name),
        }
    }
}

/// Конфигурация агента
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env: Vec<(String, String)>,
    pub timeout_secs: u64,
    pub max_output_lines: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent_type: AgentType::Goose,
            command: "goose".to_string(),
            args: vec!["run".to_string()],
            working_dir: None,
            env: vec![],
            timeout_secs: 300,
            max_output_lines: 1000,
        }
    }
}

/// Результат выполнения агента
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_secs: f64,
    pub exit_code: Option<i32>,
    pub session_id: Option<String>,
}

/// Клиент для прямого вызова агентов через CLI
pub struct AgentClient {
    config: AgentConfig,
}

impl AgentClient {
    /// Создать клиента с конфигурацией по умолчанию
    pub fn new(agent_type: AgentType) -> Self {
        let config = match agent_type {
            AgentType::Goose => AgentConfig {
                agent_type: AgentType::Goose,
                command: "goose".to_string(),
                args: vec!["run".to_string()],
                ..Default::default()
            },
            AgentType::Codex => AgentConfig {
                agent_type: AgentType::Codex,
                command: "codex".to_string(),
                args: vec![],
                ..Default::default()
            },
            AgentType::ClaudeCode => AgentConfig {
                agent_type: AgentType::ClaudeCode,
                command: "claude".to_string(),
                args: vec![],
                ..Default::default()
            },
            AgentType::Aider => AgentConfig {
                agent_type: AgentType::Aider,
                command: "aider".to_string(),
                args: vec![],
                ..Default::default()
            },
            AgentType::Gemini => AgentConfig {
                agent_type: AgentType::Gemini,
                command: "gemini".to_string(),
                args: vec![],
                ..Default::default()
            },
            AgentType::Custom { name, command } => AgentConfig {
                agent_type: AgentType::Custom { name, command: command.clone() },
                command,
                args: vec![],
                ..Default::default()
            },
        };

        Self { config }
    }

    /// Создать клиента с кастомной конфигурацией
    pub fn with_config(config: AgentConfig) -> Self {
        Self { config }
    }

    /// Выполнить задачу через агента
    pub async fn execute(&self, prompt: &str) -> Result<AgentResult> {
        let start_time = std::time::Instant::now();

        // Формируем команду
        let mut args = self.config.args.clone();
        args.push(prompt.to_string());

        let mut cmd = Command::new(&self.config.command);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Устанавливаем рабочую директорию
        if let Some(dir) = &self.config.working_dir {
            cmd.current_dir(dir);
        }

        // Устанавливаем переменные окружения
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Запускаем процесс
        let child = cmd.spawn()
            .with_context(|| format!("Failed to spawn agent: {}", self.config.command))?;

        // Ждём завершения с таймаутом
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(self.config.timeout_secs),
            self.wait_for_output(child)
        ).await;

        let duration = start_time.elapsed().as_secs_f64();

        match result {
            Ok(Ok(agent_result)) => Ok(AgentResult {
                duration_secs: duration,
                ..agent_result
            }),
            Ok(Err(e)) => Ok(AgentResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                duration_secs: duration,
                exit_code: None,
                session_id: None,
            }),
            Err(_) => Ok(AgentResult {
                success: false,
                output: String::new(),
                error: Some(format!("Timeout after {} seconds", self.config.timeout_secs)),
                duration_secs: duration,
                exit_code: None,
                session_id: None,
            }),
        }
    }

    async fn wait_for_output(&self, mut child: Child) -> Result<AgentResult> {
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();

        // Читаем stdout и stderr параллельно
        let stdout_task = tokio::spawn(async move {
            let mut lines = Vec::new();
            let mut line = String::new();
            while stdout_reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                lines.push(line.trim().to_string());
                line.clear();
            }
            lines
        });

        let stderr_task = tokio::spawn(async move {
            let mut lines = Vec::new();
            let mut line = String::new();
            while stderr_reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                lines.push(line.trim().to_string());
                line.clear();
            }
            lines
        });

        let (stdout_result, stderr_result) = tokio::join!(stdout_task, stderr_task);

        let stdout_lines = stdout_result.unwrap_or_default();
        let stderr_lines = stderr_result.unwrap_or_default();

        // Ограничиваем количество строк
        let stdout_lines: Vec<String> = stdout_lines
            .into_iter()
            .take(self.config.max_output_lines)
            .collect();
        let stderr_lines: Vec<String> = stderr_lines
            .into_iter()
            .take(self.config.max_output_lines)
            .collect();

        let status = child.wait().await?;
        let exit_code = status.code();

        Ok(AgentResult {
            success: status.success(),
            output: stdout_lines.join("\n"),
            error: if stderr_lines.is_empty() {
                None
            } else {
                Some(stderr_lines.join("\n"))
            },
            duration_secs: 0.0,
            exit_code,
            session_id: None,
        })
    }

    /// Проверить доступность агента
    pub async fn is_available(&self) -> bool {
        Command::new(&self.config.command)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Получить версию агента
    pub async fn get_version(&self) -> Option<String> {
        let output = Command::new(&self.config.command)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }
}

/// AgentAPI клиент от Coder
/// Унифицированный HTTP API для множества агентов
pub struct AgentApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl AgentApiClient {
    /// Создать клиент для подключения к локальному AgentAPI
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Создать клиент по умолчанию (localhost:8080)
    pub fn default_client() -> Self {
        Self::new("http://localhost:8080")
    }

    /// Отправить сообщение агенту
    pub async fn send_message(
        &self,
        agent: &str,
        message: &str,
        session_id: Option<&str>,
    ) -> Result<AgentApiResponse> {
        let url = format!("{}/agents/{}/message", self.base_url, agent);

        let payload = json!({
            "content": message,
            "session_id": session_id.unwrap_or("default")
        });

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .with_context(|| format!("Failed to send message to agent {}", agent))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("AgentAPI error ({}): {}", response.status(), error_text);
        }

        let result: AgentApiResponse = response.json().await
            .context("Failed to parse AgentAPI response")?;

        Ok(result)
    }

    /// Получить историю сообщений
    pub async fn get_history(
        &self,
        agent: &str,
        session_id: Option<&str>,
    ) -> Result<Vec<AgentMessage>> {
        let url = format!("{}/agents/{}/messages", self.base_url, agent);

        let mut req = self.client.get(&url);
        if let Some(sid) = session_id {
            req = req.query(&[("session_id", sid)]);
        }

        let response = req.send().await
            .with_context(|| format!("Failed to get history for agent {}", agent))?;

        if !response.status().is_success() {
            anyhow::bail!("AgentAPI error: {}", response.status());
        }

        let result: Vec<AgentMessage> = response.json().await
            .context("Failed to parse AgentAPI history")?;

        Ok(result)
    }

    /// Получить статус агента
    pub async fn get_status(&self, agent: &str) -> Result<AgentStatus> {
        let url = format!("{}/agents/{}/status", self.base_url, agent);

        let response = self.client.get(&url).send().await
            .with_context(|| format!("Failed to get status for agent {}", agent))?;

        if !response.status().is_success() {
            anyhow::bail!("AgentAPI error: {}", response.status());
        }

        let result: AgentStatus = response.json().await
            .context("Failed to parse AgentAPI status")?;

        Ok(result)
    }

    /// Подключиться к SSE потоку событий
    pub async fn subscribe_events(
        &self,
        agent: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<AgentEvent>> {
        let url = format!("{}/agents/{}/events", self.base_url, agent);

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        let client = self.client.clone();
        tokio::spawn(async move {
            let mut stream = client
                .get(&url)
                .send()
                .await
                .ok()
                .and_then(|r| r.bytes_stream().into_stream());

            while let Some(Ok(bytes)) = stream.as_mut().and_then(|s| futures_util::StreamExt::next(s).await) {
                if let Ok(event) = serde_json::from_slice::<AgentEvent>(&bytes) {
                    tx.send(event).await.ok();
                }
            }
        });

        Ok(rx)
    }

    /// Проверить доступность AgentAPI сервера
    pub async fn is_available(&self) -> bool {
        self.client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

/// Ответ от AgentAPI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApiResponse {
    pub content: String,
    pub session_id: String,
    pub timestamp: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Сообщение агента
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Статус агента
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub state: String,
    pub session_id: String,
    pub last_activity: Option<String>,
}

/// Событие из SSE потока
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: Option<String>,
}

/// Менеджер множественных агентов
pub struct AgentManager {
    clients: std::collections::HashMap<String, AgentClient>,
    agentapi_client: Option<AgentApiClient>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            clients: std::collections::HashMap::new(),
            agentapi_client: None,
        }
    }

    /// Зарегистрировать агента
    pub fn register_agent(&mut self, name: &str, config: AgentConfig) {
        self.clients.insert(name.to_string(), AgentClient::with_config(config));
    }

    /// Зарегистрировать агента по умолчанию
    pub fn register_default(&mut self, name: &str, agent_type: AgentType) {
        self.clients.insert(name.to_string(), AgentClient::new(agent_type));
    }

    /// Настроить AgentAPI клиент
    pub fn with_agentapi(mut self, base_url: &str) -> Self {
        self.agentapi_client = Some(AgentApiClient::new(base_url));
        self
    }

    /// Выполнить задачу через указанного агента
    pub async fn execute(&self, agent_name: &str, prompt: &str) -> Result<AgentResult> {
        if let Some(client) = self.clients.get(agent_name) {
            client.execute(prompt).await
        } else if let Some(api_client) = &self.agentapi_client {
            // Пытаемся выполнить через AgentAPI
            let response = api_client.send_message(agent_name, prompt, None).await?;
            Ok(AgentResult {
                success: true,
                output: response.content,
                error: None,
                duration_secs: 0.0,
                exit_code: None,
                session_id: Some(response.session_id),
            })
        } else {
            anyhow::bail!("Agent '{}' not found", agent_name);
        }
    }

    /// Получить список доступных агентов
    pub fn list_agents(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }

    /// Проверить доступность всех агентов
    pub async fn check_all_agents(&self) -> std::collections::HashMap<String, bool> {
        let mut results = std::collections::HashMap::new();

        for (name, client) in &self.clients {
            results.insert(name.clone(), client.is_available().await);
        }

        if let Some(api_client) = &self.agentapi_client {
            if api_client.is_available().await {
                results.insert("agentapi".to_string(), true);
            }
        }

        results
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Требуется установленный Goose
    async fn test_goose_available() {
        let client = AgentClient::new(AgentType::Goose);
        let available = client.is_available().await;
        println!("Goose available: {}", available);
    }

    #[tokio::test]
    #[ignore] // Требуется запущенный AgentAPI сервер
    async fn test_agentapi_client() {
        let client = AgentApiClient::default_client();
        let available = client.is_available().await;
        println!("AgentAPI available: {}", available);
    }
}
