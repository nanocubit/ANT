mod bus;
mod sys;
mod core;
mod ai;
mod tools;
mod ui;
mod metrics;

use bus::{EventBus, SystemEvent};
use std::sync::Arc;
use sys::supervisor::{Supervisor, RestartPolicy};
use metrics::{AntMetrics, MetricsCollector};
use tracing::{info, error, warn};

/// Инициализация tracing
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ant=debug"));
    
    #[cfg(feature = "structured-logs")]
    {
        let file_appender = tracing_appender::rolling::daily("logs", "ant.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .with_writer(non_blocking)
            .init();
    }
    
    #[cfg(not(feature = "structured-logs"))]
    {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .init();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    
    info!(version = env!("CARGO_PKG_VERSION"), "ANT OS starting");
    
    // Создаем необходимые директории
    std::fs::create_dir_all("skills").ok();
    std::fs::create_dir_all("sandbox_data").ok();
    std::fs::create_dir_all("logs").ok();
    std::fs::create_dir_all(".ant/workspace").ok();

    println!("🦀 ANT OS v{} - AI Orchestration System", env!("CARGO_PKG_VERSION"));
    println!("   Initializing components...");

    // Выбор режима: distributed или single-node
    #[cfg(feature = "distributed")]
    let (bus, pod_id) = {
        if let Ok(nats_url) = std::env::var("NATS_URL") {
            info!("Connecting to distributed NATS cluster at {}", nats_url);
            let bus = bus::EventBusMode::connect_distributed(&nats_url).await?;
            let pod_id = bus.pod_id().to_string();
            info!("Connected to NATS as {}", pod_id);
            println!("   ✓ Distributed Event Bus: NATS ({})", pod_id);
            (Arc::new(bus), pod_id)
        } else {
            info!("Running in single-node mode (tokio::broadcast)");
            let bus = bus::EventBusMode::connect_single_node();
            println!("   ✓ Single-node Event Bus: tokio::broadcast");
            (Arc::new(bus), "single-node".to_string())
        }
    };
    
    #[cfg(not(feature = "distributed"))]
    let (bus, pod_id) = {
        let bus = EventBus::new();
        println!("   ✓ Event Bus: tokio::broadcast");
        (Arc::new(bus), "single-node".to_string())
    };

    bus.emit(SystemEvent::SystemBoot(format!("ANT OS v{} on {}-{}", 
        env!("CARGO_PKG_VERSION"), 
        std::env::consts::OS, 
        std::env::consts::ARCH
    )));

    // Инициализация метрик
    #[cfg(feature = "metrics")]
    {
        println!("   Initializing metrics...");
        let metrics = Arc::new(AntMetrics::new()?);
        
        // Metrics collector
        let mc = MetricsCollector::new(metrics.clone());
        let metrics_bus = bus.clone();
        let _mc_handle = tokio::spawn(async move {
            let rx = metrics_bus.subscribe();
            if let Err(e) = mc.run(rx).await {
                error!("Metrics collector error: {}", e);
            }
        });
        
        // Metrics server
        let port: u16 = std::env::var("METRICS_PORT")
            .unwrap_or_else(|_| "9090".to_string())
            .parse()
            .unwrap_or(9090);
        
        let m = metrics.clone();
        let _metrics_server = tokio::spawn(async move {
            if let Err(e) = m.serve(port).await {
                error!("Metrics server error: {}", e);
            }
        });
        
        println!("   ✓ Metrics server on port {}", port);
        info!(port, "Metrics server started");
    }

    // Инициализация памяти
    println!("   Initializing memory system...");
    let memory = Arc::new(core::memory::VectorMemory::new("ant_memory.duckdb")?);
    let memory_stats = memory.get_stats()?;
    println!("   ✓ Memory: {} documents, {:.2} KB",
        memory_stats.total_documents,
        memory_stats.total_size_bytes as f64 / 1024.0);

    // Инициализация time-travel debugger
    println!("   Initializing time-travel debugger...");
    let debugger = core::timetravel::TimeTravelDebugger::new("ant_memory.duckdb")?;
    let snapshots = debugger.list_snapshots()?;
    println!("   ✓ Time-travel: {} snapshots available", snapshots.len());

    // Инициализация менеджера агентов
    println!("   Initializing agent manager...");
    let mut agent_manager = tools::agents::AgentManager::new();
    agent_manager.register_default("goose", tools::agents::AgentType::Goose);
    agent_manager.register_default("codex", tools::agents::AgentType::Codex);

    // Проверка доступности агентов
    let agents_check = agent_manager.check_all_agents().await;
    for (name, available) in &agents_check {
        let status = if *available { "✓" } else { "⚠" };
        println!("   {} Agent '{}': {}", status, name, if *available { "available" } else { "not found" });
    }

    // === Daemons ===
    
    // 1. Системные сервисы
    Supervisor::spawn_daemon("AuditLogger", RestartPolicy::Always, bus.clone(), {
        let b = bus.clone();
        move || crate::sys::audit_logger::AuditLogger::run_daemon(b.clone())
    });

    // 2. Планировщик ядра
    Supervisor::spawn_daemon("CoreScheduler", RestartPolicy::Always, bus.clone(), {
        let b = bus.clone();
        move || crate::core::scheduler::CoreScheduler::run_daemon(b.clone())
    });

    // 3. Оркестратор задач
    Supervisor::spawn_daemon("Orchestrator", RestartPolicy::Always, bus.clone(), {
        let b = bus.clone();
        move || crate::sys::orchestrator::OrchestratorDaemon::run_daemon(b.clone())
    });

    // 4. WASM Sandbox
    #[cfg(feature = "with-wasm")]
    Supervisor::spawn_daemon("WasmSandbox", RestartPolicy::Always, bus.clone(), {
        let b = bus.clone();
        move || crate::tools::wasm_actor::WasmActor::run_daemon(b.clone())
    });

    // 5. Browser Actor (headless_chrome)
    #[cfg(feature = "with-browser")]
    Supervisor::spawn_daemon("BrowserActor", RestartPolicy::OnFailure, bus.clone(), {
        let b = bus.clone();
        move || crate::tools::browser::BrowserActor::run_daemon(b.clone())
    });

    // 6. LSP Actor
    Supervisor::spawn_daemon("LspActor", RestartPolicy::OnFailure, bus.clone(), {
        let b = bus.clone();
        move || crate::tools::dummy_actors::run_lsp_daemon(b.clone())
    });

    println!("   ✓ Daemons started");

    // Проверка конфигурации LLM
    let llm_config = crate::ai::llm::LlmConfig::from_env();
    if llm_config.has_api_key() {
        println!("   ✓ LLM configured: {} ({})", llm_config.provider_name(), llm_config.model);
    } else {
        println!("   ⚠ LLM not configured. Set OPENROUTER_API_KEY for AI features.");
    }

    // Демонстрационная цель через 2 секунды
    let b = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        b.emit(SystemEvent::GoalCreated {
            id: "G-DEMO".into(),
            task: "Analyze Rust project at https://rust-lang.org".into(),
        });
    });

    println!("\n   📋 Controls:");
    println!("   - Tab/Shift+Tab: Switch views");
    println!("   - Enter: Send command");
    println!("   - m: Toggle memory panel");
    println!("   - r: Memory search");
    println!("   - Esc/q: Quit\n");
    
    println!("   📊 Metrics: http://localhost:9090/metrics");
    println!("   🛠️  CLI: antctl status\n");

    // Запуск TUI
    crate::ui::dashboard::run_ui(bus).await?;

    info!("ANT OS shutdown complete");
    println!("\n🦀 ANT OS shutdown complete.");

    Ok(())
}

