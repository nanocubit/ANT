//! EventBus с поддержкой single-node и distributed режимов

#[cfg(feature = "distributed")]
pub mod distributed;

#[cfg(feature = "distributed")]
pub use distributed::*;

/// Single-node EventBus (режим по умолчанию)
#[cfg(not(feature = "distributed"))]
pub use single_node::*;

#[cfg(not(feature = "distributed"))]
mod single_node {
    use serde::{Deserialize, Serialize};
    use tokio::sync::broadcast;

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
    }

    pub struct EventBus {
        pub tx: broadcast::Sender<SystemEvent>,
    }

    impl EventBus {
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

    impl Default for EventBus {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "distributed")]
mod single_node {
    //! Single-node режим для обратной совместимости
    use serde::{Deserialize, Serialize};
    use tokio::sync::broadcast;

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
    }

    pub struct SingleNodeEventBus {
        pub tx: broadcast::Sender<SystemEvent>,
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
}
