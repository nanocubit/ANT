use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::env;
use std::time::Duration;

/// Конфигурация LLM провайдера
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub model: String,
    pub api_key: String,
    pub base_url: String,
    pub max_tokens: u32,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LlmProvider {
    OpenRouter,
    DeepSeek,
    Ollama,
    Custom,
}

impl LlmConfig {
    pub fn from_env() -> Self {
        let provider = env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "openrouter".to_string())
            .to_lowercase();

        let (provider, default_model, default_url) = match provider.as_str() {
            "deepseek" => (
                LlmProvider::DeepSeek,
                "deepseek-chat".to_string(),
                "https://api.deepseek.com/v1/chat/completions".to_string(),
            ),
            "ollama" => (
                LlmProvider::Ollama,
                "qwen2.5-coder:32b".to_string(),
                "http://localhost:11434/v1/chat/completions".to_string(),
            ),
            "openrouter" | _ => (
                LlmProvider::OpenRouter,
                "qwen/qwen-2.5-coder-32b-instruct".to_string(),
                "https://openrouter.ai/api/v1/chat/completions".to_string(),
            ),
        };

        let api_key = env::var("OPENROUTER_API_KEY")
            .or_else(|_| env::var("DEEPSEEK_API_KEY"))
            .or_else(|_| env::var("LLM_API_KEY"))
            .unwrap_or_default();

        Self {
            provider,
            model: env::var("LLM_MODEL").unwrap_or(default_model),
            api_key,
            base_url: env::var("LLM_BASE_URL").unwrap_or(default_url),
            max_tokens: 4096,
            timeout_secs: 120,
        }
    }

    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
    }

    pub fn provider_name(&self) -> &str {
        match self.provider {
            LlmProvider::OpenRouter => "OpenRouter",
            LlmProvider::DeepSeek => "DeepSeek",
            LlmProvider::Ollama => "Ollama",
            LlmProvider::Custom => "Custom",
        }
    }
}

/// Клиент для работы с LLM
pub struct LlmClient {
    client: Client,
    config: LlmConfig,
}

impl LlmClient {
    pub fn new() -> Self {
        let config = LlmConfig::from_env();
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Отправить запрос к LLM и получить ответ
    pub async fn chat(&self, user_message: &str, system_prompt: Option<&str>) -> Result<String> {
        if !self.config.has_api_key() {
            return Ok("⚠️ API ключ не задан. Установите OPENROUTER_API_KEY или LLM_API_KEY.".to_string());
        }

        let mut messages = Vec::new();

        if let Some(system) = system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system
            }));
        }

        messages.push(json!({
            "role": "user",
            "content": user_message
        }));

        let response = self
            .client
            .post(&self.config.base_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&json!({
                "model": self.config.model,
                "messages": messages,
                "max_tokens": self.config.max_tokens,
                "temperature": 0.7,
            }))
            .send()
            .await
            .context("Failed to send request to LLM")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error ({}): {}", response.status(), error_text);
        }

        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse LLM response")?;

        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Пустой ответ от LLM");

        Ok(content.to_string())
    }

    /// Чат с контекстом (для RAG)
    pub async fn chat_with_context(
        &self,
        user_message: &str,
        context: &[String],
    ) -> Result<String> {
        let context_str = if context.is_empty() {
            "Контекст не предоставлен.".to_string()
        } else {
            format!(
                "Контекст из базы знаний:\n{}",
                context
                    .iter()
                    .map(|s| format!("---\n{}", s))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        let system_prompt = format!(
            "Ты ANT — автономный AI-агент на Rust.\
            Используй предоставленный контекст для ответа.\
            Если контекст не относится к вопросу, скажи об этом.\n\n\
            {}",
            context_str
        );

        self.chat(user_message, Some(&system_prompt)).await
    }

    /// Генерация кода (специальный режим для кодинга)
    pub async fn generate_code(&self, task: &str, language: &str) -> Result<String> {
        let prompt = format!(
            "Напиши код на {} для задачи: {}\n\n\
            Требования:\n\
            - Только код, без объяснений\n\
            - Следуй лучшим практикам {}\n\
            - Добавь необходимые импорты\n\
            - Обработай ошибки",
            language, task, language
        );

        self.chat(&prompt, Some("Ты опытный разработчик. Пиши чистый, эффективный код.")).await
    }

    /// Планирование задачи (разбиение на шаги)
    pub async fn plan_task(&self, goal: &str) -> Result<Vec<PlanStep>> {
        let prompt = format!(
            "Разбей задачу '{}' на последовательные шаги.\
            Для каждого шага укажи:\
            1. ID шага (t1, t2, ...)\
            2. Инструмент (browser, lsp, wasm, shell, file)\
            3. Входные данные\
            4. Зависимости от других шагов\n\n\
            Верни ответ в формате JSON массива объектов.",
            goal
        );

        let response = self
            .chat(&prompt, Some("Ты планировщик задач. Отвечай ТОЛЬКО валидным JSON."))
            .await?;

        // Парсим ответ как JSON
        let steps: Vec<PlanStep> = serde_json::from_str(&response).unwrap_or_else(|_| {
            // Fallback: создаем демо-план если парсинг не удался
            vec![
                PlanStep {
                    id: "t1".to_string(),
                    tool: "browser".to_string(),
                    input: format!("Исследовать: {}", goal),
                    depends_on: vec![],
                },
                PlanStep {
                    id: "t2".to_string(),
                    tool: "lsp".to_string(),
                    input: "Анализ кода".to_string(),
                    depends_on: vec![],
                },
            ]
        });

        Ok(steps)
    }
}

/// Шаг плана выполнения
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlanStep {
    pub id: String,
    pub tool: String,
    pub input: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        let config = LlmConfig::from_env();
        assert!(!config.model.is_empty());
    }
}
