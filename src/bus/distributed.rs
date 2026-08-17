//! Distributed Event Bus на базе NATS JetStream

#[cfg(feature = "distributed")]
use async_nats::{jetstream, ConnectOptions, Client};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{info, warn};

/// Системные события
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SystemEvent {
    SystemBoot(String),
    GoalCreated { id: String, task: String },
    PlanCreated { goal_id: String, plan: serde_json::Value },
    TaskDispatched { task_id: String, tool: String, input: String },
    TaskCompleted { task_id: String, result: String },
    TaskFailed { task_id: String, error: String },
    GoalCompleted { id: String, result: String },
    GoalFailed { id: String, reason: String },
    Log { level: String, source: String, message: String },
    DaemonStatus { daemon: String, status: String },
}

impl SystemEvent {
    pub fn name(&self) -> &'static str {
        match self {
            SystemEvent::SystemBoot(_) => "SystemBoot",
            SystemEvent::GoalCreated { .. } => "GoalCreated",
            SystemEvent::PlanCreated { .. } => "PlanCreated",
            SystemEvent::TaskDispatched { .. } => "TaskDispatched",
            SystemEvent::TaskCompleted { .. } => "TaskCompleted",
            SystemEvent::TaskFailed { .. } => "TaskFailed",
            SystemEvent::GoalCompleted { .. } => "GoalCompleted",
            SystemEvent::GoalFailed { .. } => "GoalFailed",
            SystemEvent::Log { .. } => "Log",
            SystemEvent::DaemonStatus { .. } => "DaemonStatus",
        }
    }

    fn to_subject(&self) -> String {
        format!("antos.events.{}", self.name().to_lowercase())
    }
}

/// Подписчик на события
pub struct EventSubscriber {
    #[cfg(feature = "distributed")]
    subscription: Option<async_nats::Subscriber>,
    #[cfg(not(feature = "distributed"))]
    subscription: broadcast::Receiver<SystemEvent>,
}

impl EventSubscriber {
    #[cfg(feature = "distributed")]
    pub async fn next(&mut self) -> Option<SystemEvent> {
        use futures::StreamExt;
        if let Some(sub) = &mut self.subscription {
            while let Some(msg) = sub.next().await {
                match serde_json::from_slice::<SystemEvent>(&msg.payload) {
                    Ok(event) => return Some(event),
                    Err(e) => {
                        warn!("Failed to deserialize event: {}", e);
                        continue;
                    }
                }
            }
        }
        None
    }

    #[cfg(not(feature = "distributed"))]
    pub async fn next(&mut self) -> Option<SystemEvent> {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match self.subscription.recv().await {
                Ok(event) => return Some(event),
                Err(RecvError::Lagged(n)) => {
                    warn!("Subscriber lagged, skipped {} events", n);
                    continue;
                }
                Err(RecvError::Closed) => return None,
            }
        }
    }
}

/// Distributed EventBus на базе NATS
#[cfg(feature = "distributed")]
pub struct EventBus {
    client: Client,
    subject: String,
    pod_id: String,
}

#[cfg(feature = "distributed")]
impl EventBus {
    /// Подключение к NATS кластеру
    pub async fn connect(nats_url: &str) -> anyhow::Result<Self> {
        let pod_id = std::env::var("POD_NAME")
            .unwrap_or_else(|_| format!("antos-{}", uuid::Uuid::new_v4()));

        info!("Connecting to NATS at {} as {}", nats_url, pod_id);

        let client = ConnectOptions::new()
            .name(pod_id.clone())
            .connect(nats_url)
            .await?;

        // JetStream для персистентности
        let _jetstream = jetstream::new(client.clone());

        Ok(Self {
            client,
            subject: "antos.events".to_string(),
            pod_id,
        })
    }

    /// Публикация события
    pub async fn emit(&self, event: SystemEvent) -> anyhow::Result<()> {
        let subject = format!("{}.{}", self.subject, event.name().to_lowercase());
        let payload = serde_json::to_vec(&event)?;

        self.client.publish(subject, payload.into()).await?;
        Ok(())
    }

    /// Подписка на все события
    pub async fn subscribe(&self) -> anyhow::Result<EventSubscriber> {
        let subscription = self.client.subscribe(format!("{}.>", self.subject)).await?;
        Ok(EventSubscriber {
            subscription: Some(subscription),
        })
    }

    /// Подписка на конкретный тип событий
    pub async fn subscribe_to(&self, event_type: &str) -> anyhow::Result<EventSubscriber> {
        let subject = format!("{}.{}", self.subject, event_type.to_lowercase());
        let subscription = self.client.subscribe(subject).await?;
        Ok(EventSubscriber {
            subscription: Some(subscription),
        })
    }

    /// Get pod ID
    pub fn pod_id(&self) -> &str {
        &self.pod_id
    }

    /// Health check
    pub async fn health(&self) -> bool {
        self.client.health().await.is_ok()
    }
}

/// Single-node EventBus (fallback)
pub struct SingleNodeEventBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl SingleNodeEventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.tx.subscribe()
    }

    pub fn emit(&self, event: SystemEvent) {
        let _ = self.tx.send(event);
    }
}

impl Default for SingleNodeEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// EventBus с поддержкой обоих режимов
#[cfg(feature = "distributed")]
pub enum EventBusMode {
    Distributed(EventBus),
    SingleNode(SingleNodeEventBus),
}

#[cfg(not(feature = "distributed"))]
pub type EventBus = SingleNodeEventBus;

#[cfg(feature = "distributed")]
impl EventBusMode {
    pub async fn connect_distributed(nats_url: &str) -> anyhow::Result<Self> {
        let bus = EventBus::connect(nats_url).await?;
        Ok(EventBusMode::Distributed(bus))
    }

    pub fn connect_single_node() -> Self {
        EventBusMode::SingleNode(SingleNodeEventBus::new())
    }

    pub async fn emit(&self, event: SystemEvent) -> anyhow::Result<()> {
        match self {
            EventBusMode::Distributed(bus) => bus.emit(event).await,
            EventBusMode::SingleNode(bus) => {
                bus.emit(event);
                Ok(())
            }
        }
    }

    pub async fn subscribe(&self) -> anyhow::Result<EventSubscriber> {
        match self {
            EventBusMode::Distributed(bus) => bus.subscribe().await,
            EventBusMode::SingleNode(bus) => Ok(EventSubscriber {
                subscription: Some(bus.tx.subscribe().into()),
            }),
        }
    }

    pub fn pod_id(&self) -> &str {
        match self {
            EventBusMode::Distributed(bus) => bus.pod_id(),
            EventBusMode::SingleNode(_) => "single-node",
        }
    }

    pub async fn health(&self) -> bool {
        match self {
            EventBusMode::Distributed(bus) => bus.health().await,
            EventBusMode::SingleNode(_) => true,
        }
    }
}
