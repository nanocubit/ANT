//! Supervisor Tree с exponential backoff

use crate::bus::{EventBus, SystemEvent};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, error, warn};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestartPolicy {
    Always,
    OnFailure,
    Never,
}

pub struct Supervisor;

impl Supervisor {
    pub fn spawn_daemon<F, Fut>(
        name: &'static str,
        policy: RestartPolicy,
        bus: Arc<EventBus>,
        mut factory: F,
    ) where
        F: FnMut() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        tokio::spawn(async move {
            let mut backoff = 1u64;

            loop {
                info!(daemon = %name, "Starting");
                bus.emit(SystemEvent::DaemonStatus {
                    daemon: name.into(),
                    status: "STARTING".into(),
                });

                let start = Instant::now();
                let handle = tokio::spawn(factory());
                let res = handle.await;

                // Reset backoff if daemon ran for more than 5 seconds
                if start.elapsed() > Duration::from_secs(5) {
                    backoff = 1;
                }

                let restart = match res {
                    Ok(Ok(_)) => {
                        info!(daemon = %name, "Stopped");
                        bus.emit(SystemEvent::DaemonStatus {
                            daemon: name.into(),
                            status: "STOPPED".into(),
                        });
                        policy == RestartPolicy::Always
                    }
                    Ok(Err(e)) => {
                        error!(daemon = %name, err = %e, "Crashed");
                        bus.emit(SystemEvent::Log {
                            level: "ERROR".into(),
                            source: name.into(),
                            message: e.to_string(),
                        });
                        bus.emit(SystemEvent::DaemonStatus {
                            daemon: name.into(),
                            status: "CRASHED".into(),
                        });
                        policy != RestartPolicy::Never
                    }
                    Err(_) => {
                        error!(daemon = %name, "Panicked");
                        bus.emit(SystemEvent::Log {
                            level: "CRIT".into(),
                            source: name.into(),
                            message: "PANIC".into(),
                        });
                        bus.emit(SystemEvent::DaemonStatus {
                            daemon: name.into(),
                            status: "PANICKED".into(),
                        });
                        policy != RestartPolicy::Never
                    }
                };

                if !restart {
                    break;
                }

                warn!(daemon = %name, backoff, "Restarting");
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = std::cmp::min(backoff * 2, 60);
            }
        });
    }
}
