//! Core Scheduler с улучшенной обработкой DAG

use crate::bus::{EventBus, SystemEvent};
use crate::core::dag::{DagState, TaskStatus};
use crate::ai::planner::PlanningEngine;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, error};

pub struct CoreScheduler {
    bus: Arc<EventBus>,
    active_dags: Arc<RwLock<HashMap<String, Arc<DagState>>>>,
}

impl CoreScheduler {
    pub fn new(bus: Arc<EventBus>) -> Self {
        Self {
            bus,
            active_dags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run_daemon(bus: Arc<EventBus>) -> Result<()> {
        let mut rx = bus.subscribe();
        let sched = Self::new(bus.clone());
        
        while let Ok(ev) = rx.recv().await {
            if let Err(e) = sched.handle(ev).await {
                error!("Scheduler error: {}", e);
            }
        }
        Ok(())
    }

    async fn handle(&self, ev: SystemEvent) -> Result<()> {
        match ev {
            SystemEvent::GoalCreated { id, task } => self.handle_goal(id, task).await?,
            SystemEvent::TaskCompleted { task_id, result } => self.handle_done(task_id, result).await?,
            SystemEvent::TaskFailed { task_id, error } => self.handle_fail(task_id, error).await?,
            _ => {}
        }
        Ok(())
    }

    async fn handle_goal(&self, gid: String, task: String) -> Result<()> {
        info!(goal = %gid, "Creating goal");
        
        let plan = PlanningEngine::create_plan(&gid, &task).await?;
        let dag = Arc::new(DagState::new(plan).await?);
        
        {
            let mut d = self.active_dags.write().await;
            d.insert(gid.clone(), dag.clone());
        }
        
        self.bus.emit(SystemEvent::PlanCreated {
            goal_id: gid.clone(),
            plan: serde_json::to_value(&dag)?,
        });
        
        self.advance(&gid, &dag).await?;
        Ok(())
    }

    async fn handle_done(&self, tid: String, res: String) -> Result<()> {
        let mut done_gid = None;
        let dags = self.active_dags.read().await;
        
        for (gid, dag) in dags.iter() {
            if {
                let st = dag.statuses.read().await;
                st.contains_key(&tid)
            } {
                dag.update_status(&tid, TaskStatus::Completed(res.clone())).await?;
                
                if dag.is_terminal().await {
                    let stats = dag.stats().await;
                    done_gid = Some((gid.clone(), stats.failed == 0));
                } else {
                    drop(dags);
                    self.advance(gid, dag).await?;
                }
                break;
            }
        }
        
        if let Some((gid, ok)) = done_gid {
            let mut d = self.active_dags.write().await;
            d.remove(&gid);
            
            if ok {
                self.bus.emit(SystemEvent::GoalCompleted {
                    id: gid,
                    result: "Done".into(),
                });
            } else {
                self.bus.emit(SystemEvent::GoalFailed {
                    id: gid,
                    reason: "Tasks failed".into(),
                });
            }
        }
        Ok(())
    }

    async fn handle_fail(&self, tid: String, err: String) -> Result<()> {
        error!(task = %tid, "Task failed: {}", err);
        
        let mut cancel_gid = None;
        let dags = self.active_dags.read().await;
        
        for (gid, dag) in dags.iter() {
            if {
                let st = dag.statuses.read().await;
                st.contains_key(&tid)
            } {
                dag.update_status(&tid, TaskStatus::Failed(err.clone())).await?;
                dag.cancel_pending().await?;
                cancel_gid = Some(gid.clone());
                break;
            }
        }
        
        if let Some(gid) = cancel_gid {
            drop(dags);
            let mut d = self.active_dags.write().await;
            d.remove(&gid);
            
            self.bus.emit(SystemEvent::GoalFailed {
                id: gid,
                reason: format!("Task {} failed: {}", tid, err),
            });
        }
        Ok(())
    }

    async fn advance(&self, gid: &str, dag: &Arc<DagState>) -> Result<()> {
        for t in dag.get_ready_tasks().await {
            dag.update_status(&t.id, TaskStatus::Running).await?;
            self.bus.emit(SystemEvent::TaskDispatched {
                task_id: t.id,
                tool: t.tool,
                input: t.input,
            });
        }
        Ok(())
    }
}
