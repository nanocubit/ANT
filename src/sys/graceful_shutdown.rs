//! Graceful Shutdown Coordinator

use tokio::sync::broadcast;
use tracing::info;

pub struct ShutdownCoordinator {
    tx: broadcast::Sender<()>,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    pub fn trigger_shutdown(&self) {
        info!("Shutdown triggered");
        let _ = self.tx.send(());
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
