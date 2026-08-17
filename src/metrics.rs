//! Prometheus метрики и metrics server

#[cfg(feature = "metrics")]
use prometheus::{Counter, Gauge, Histogram, Registry, TextEncoder};
use std::sync::Arc;
#[cfg(feature = "metrics")]
use tokio::sync::RwLock;
#[cfg(feature = "metrics")]
use std::collections::HashMap;
#[cfg(feature = "metrics")]
use std::time::Instant;
#[cfg(feature = "metrics")]
use crate::bus::SystemEvent;
#[cfg(feature = "metrics")]
use tracing::info;

/// Реестр метрик
#[cfg(feature = "metrics")]
pub struct AntMetrics {
    registry: Registry,
    pub tasks_total: Counter,
    pub tasks_completed: Counter,
    pub tasks_failed: Counter,
    pub task_duration: Histogram,
    pub active_dags: Gauge,
    pub active_goals: Gauge,
}

#[cfg(feature = "metrics")]
impl AntMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let reg = Registry::new();

        let tasks_total = Counter::new("ant_tasks_total", "Total tasks dispatched")?;
        let tasks_completed = Counter::new("ant_tasks_completed_total", "Total tasks completed")?;
        let tasks_failed = Counter::new("ant_tasks_failed_total", "Total tasks failed")?;
        let task_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new("ant_task_duration_seconds", "Task execution duration")
                .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0])
        )?;
        let active_dags = Gauge::new("ant_active_dags", "Active DAGs count")?;
        let active_goals = Gauge::new("ant_active_goals", "Active goals count")?;

        reg.register(Box::new(tasks_total.clone()))?;
        reg.register(Box::new(tasks_completed.clone()))?;
        reg.register(Box::new(tasks_failed.clone()))?;
        reg.register(Box::new(task_duration.clone()))?;
        reg.register(Box::new(active_dags.clone()))?;
        reg.register(Box::new(active_goals.clone()))?;

        Ok(Self {
            registry: reg,
            tasks_total,
            tasks_completed,
            tasks_failed,
            task_duration,
            active_dags,
            active_goals,
        })
    }

    /// Запуск metrics server на указанном порту
    pub async fn serve(&self, port: u16) -> anyhow::Result<()> {
        let registry = self.registry.clone();

        let metrics_route = warp::path("metrics").map(move || {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer).unwrap();
            String::from_utf8(buffer).unwrap()
        });

        let health_route = warp::path("health")
            .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));

        let routes = metrics_route.or(health_route);

        info!(port, "Starting metrics server on 0.0.0.0:{}", port);
        warp::serve(routes).run(([0, 0, 0, 0], port)).await;

        Ok(())
    }

    /// Get registry for custom collectors
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

#[cfg(feature = "metrics")]
impl Default for AntMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics")
    }
}

/// Collector для сбора метрик из событий
#[cfg(feature = "metrics")]
pub struct MetricsCollector {
    metrics: Arc<AntMetrics>,
    starts: Arc<RwLock<HashMap<String, Instant>>>,
}

#[cfg(feature = "metrics")]
impl MetricsCollector {
    pub fn new(metrics: Arc<AntMetrics>) -> Self {
        Self {
            metrics,
            starts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(&self, mut event_rx: tokio::sync::broadcast::Receiver<SystemEvent>) -> anyhow::Result<()> {
        use tokio::sync::broadcast::error::RecvError;

        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    match &event {
                        SystemEvent::TaskDispatched { task_id, .. } => {
                            self.metrics.tasks_total.inc();
                            let mut starts = self.starts.write().await;
                            starts.insert(task_id.clone(), Instant::now());
                        }
                        SystemEvent::TaskCompleted { task_id, .. } => {
                            let start = {
                                let mut starts = self.starts.write().await;
                                starts.remove(task_id)
                            };
                            if let Some(start_time) = start {
                                self.metrics.tasks_completed.inc();
                                self.metrics.task_duration.observe(start_time.elapsed().as_secs_f64());
                            }
                        }
                        SystemEvent::TaskFailed { .. } => {
                            self.metrics.tasks_failed.inc();
                        }
                        SystemEvent::GoalCreated { .. } => {
                            self.metrics.active_goals.inc();
                        }
                        SystemEvent::GoalCompleted { .. } | SystemEvent::GoalFailed { .. } => {
                            self.metrics.active_goals.dec();
                        }
                        _ => {}
                    }
                }
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!("Metrics collector lagged, skipped {} events", n);
                }
                Err(RecvError::Closed) => break,
            }
        }

        Ok(())
    }
}

/// Stub для режима без метрик
#[cfg(not(feature = "metrics"))]
pub struct AntMetrics;

#[cfg(not(feature = "metrics"))]
impl AntMetrics {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Self)
    }

    pub async fn serve(&self, _port: u16) -> anyhow::Result<()> {
        // Metrics disabled
        std::future::pending().await
    }
}

#[cfg(not(feature = "metrics"))]
impl Default for AntMetrics {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(feature = "metrics"))]
pub struct MetricsCollector;

#[cfg(not(feature = "metrics"))]
impl MetricsCollector {
    pub fn new(_metrics: Arc<AntMetrics>) -> Self {
        Self
    }

    pub async fn run(&self, _event_rx: tokio::sync::broadcast::Receiver<SystemEvent>) -> anyhow::Result<()> {
        Ok(())
    }
}
