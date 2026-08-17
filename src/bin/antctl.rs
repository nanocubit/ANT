//! ANT OS CLI - утилита командной строки для управления ANT

use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "antctl")]
#[command(about = "ANT OS CLI utility", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Endpoint ANT OS API
    #[arg(short, long, default_value = "http://localhost:9090")]
    endpoint: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Отправить новую задачу (goal)
    Submit {
        /// Текст задачи
        task: String,
        /// Приоритет (1-10)
        #[arg(short, long, default_value = "5")]
        priority: u8,
    },

    /// Показать статус системы
    Status,

    /// Показать логи
    Logs {
        /// Количество строк
        #[arg(short, long, default_value = "50")]
        lines: usize,
    },

    /// Показать доступные skills (WASM)
    Skills,

    /// Проверить health endpoint
    Health,

    /// Инициировать shutdown
    Shutdown {
        /// Принудительный shutdown
        #[arg(short, long)]
        force: bool,
    },

    /// Показать метрики (Prometheus format)
    Metrics,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    match cli.command {
        Commands::Submit { task, priority } => {
            submit(&client, &cli.endpoint, &task, priority).await?;
        }
        Commands::Status => {
            status(&client, &cli.endpoint).await?;
        }
        Commands::Logs { lines } => {
            show_logs(lines)?;
        }
        Commands::Skills => {
            list_skills()?;
        }
        Commands::Health => {
            health(&client, &cli.endpoint).await?;
        }
        Commands::Shutdown { force } => {
            shutdown(&client, &cli.endpoint, force).await?;
        }
        Commands::Metrics => {
            show_metrics(&client, &cli.endpoint).await?;
        }
    }

    Ok(())
}

/// POST /api/v1/goals - создать новую задачу
async fn submit(client: &Client, endpoint: &str, task: &str, priority: u8) -> Result<()> {
    let url = format!("{}/api/v1/goals", endpoint);

    let response = client
        .post(&url)
        .json(&json!({
            "task": task,
            "priority": priority
        }))
        .send()
        .await
        .context("Connection failed")?;

    if response.status().is_success() {
        let value: serde_json::Value = response.json().await?;
        let goal_id = value["goal_id"].as_str().unwrap_or("unknown");
        println!("✅ Goal submitted: {}", goal_id);
    } else {
        let error = response.text().await?;
        anyhow::bail!("Failed to submit goal: {}", error);
    }

    Ok(())
}

/// GET /api/v1/status - статус системы
async fn status(client: &Client, endpoint: &str) -> Result<()> {
    let url = format!("{}/api/v1/status", endpoint);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Connection failed")?;

    let value: serde_json::Value = response.json().await?;

    println!("🚀 ANT OS Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Active DAGs:   {}", value.get("active_dags").and_then(|v| v.as_u64()).unwrap_or(0));
    println!("  Active Goals:  {}", value.get("active_goals").and_then(|v| v.as_u64()).unwrap_or(0));
    println!("  Completed:     {}", value.get("completed").and_then(|v| v.as_u64()).unwrap_or(0));
    println!("  Failed:        {}", value.get("failed").and_then(|v| v.as_u64()).unwrap_or(0));
    println!("  Pending:       {}", value.get("pending").and_then(|v| v.as_u64()).unwrap_or(0));

    if let Some(version) = value.get("version").and_then(|v| v.as_str()) {
        println!("  Version:       {}", version);
    }

    Ok(())
}

/// Показать логи из файла
fn show_logs(lines: usize) -> Result<()> {
    let log_path = "logs/ant.log";

    if !std::path::Path::new(log_path).exists() {
        println!("📜 No logs found at {}", log_path);
        return Ok(());
    }

    let content = std::fs::read_to_string(log_path)?;
    let all_lines: Vec<&str> = content.lines().collect();
    let start = all_lines.len().saturating_sub(lines);

    println!("📜 Last {} lines from {}:", lines, log_path);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    for line in &all_lines[start..] {
        println!("{}", line);
    }

    Ok(())
}

/// Показать доступные WASM skills
fn list_skills() -> Result<()> {
    let skills_dir = std::path::Path::new("skills");

    if !skills_dir.exists() {
        println!("🛠️ No skills directory found");
        return Ok(());
    }

    println!("🛠️ Available Skills:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut found = false;
    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|ext| ext == "wasm").unwrap_or(false) {
            found = true;
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?");
            println!("  • {}.wasm", name);
        }
    }

    if !found {
        println!("  (no .wasm files found)");
    }

    Ok(())
}

/// GET /health - проверка здоровья
async fn health(client: &Client, endpoint: &str) -> Result<()> {
    let url = format!("{}/health", endpoint);

    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            println!("✅ Healthy");
            Ok(())
        }
        Ok(response) => {
            println!("⚠️ Status: {}", response.status());
            Ok(())
        }
        Err(e) => {
            println!("❌ Unhealthy: {}", e);
            std::process::exit(1);
        }
    }
}

/// POST /admin/shutdown - shutdown системы
async fn shutdown(client: &Client, endpoint: &str, force: bool) -> Result<()> {
    let url = if force {
        format!("{}/admin/shutdown?force=true", endpoint)
    } else {
        format!("{}/admin/shutdown", endpoint)
    };

    let response = client
        .post(&url)
        .send()
        .await
        .context("Shutdown request failed")?;

    if response.status().is_success() {
        println!("✅ Shutdown initiated");
    } else {
        println!("⚠️ Request failed: {}", response.status());
    }

    Ok(())
}

/// GET /metrics - Prometheus метрики
async fn show_metrics(client: &Client, endpoint: &str) -> Result<()> {
    let url = format!("{}/metrics", endpoint);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Connection failed")?;

    if response.status().is_success() {
        let text = response.text().await?;
        println!("{}", text);
    } else {
        anyhow::bail!("Failed to get metrics: {}", response.status());
    }

    Ok(())
}
