//! DAG Scheduler с детекцией циклов и статистикой

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::RwLock;
use std::sync::Arc;
use anyhow::{Result, bail};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub goal_id: String,
    pub steps: Vec<TaskNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskNode {
    pub id: String,
    pub tool: String,
    pub input: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed(String),
    Failed(String),
    Cancelled,
}

/// Ошибка детекции циклов
#[derive(Debug)]
pub struct CycleError {
    pub cycle: Vec<String>,
}

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cycle detected: {}", self.cycle.join(" -> "))
    }
}

impl std::error::Error for CycleError {}

pub struct DagState {
    pub tasks: Arc<RwLock<HashMap<String, TaskNode>>>,
    pub statuses: Arc<RwLock<HashMap<String, TaskStatus>>>,
    pub goal_id: String,
    pub execution_order: Vec<String>,
}

/// Статистика DAG
#[derive(Debug, Clone)]
pub struct DagStats {
    pub total: usize,
    pub pending: u32,
    pub running: u32,
    pub completed: u32,
    pub failed: u32,
    pub cancelled: u32,
}

impl DagStats {
    pub fn progress_pct(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            ((self.completed + self.failed + self.cancelled) as f64 / self.total as f64) * 100.0
        }
    }
}

impl DagState {
    pub async fn new(plan: ExecutionPlan) -> Result<Self> {
        let mut tasks = HashMap::new();
        let mut statuses = HashMap::new();
        
        for step in &plan.steps {
            statuses.insert(step.id.clone(), TaskStatus::Pending);
            tasks.insert(step.id.clone(), step.clone());
        }
        
        let execution_order = Self::validate_and_sort(&tasks)?;
        
        Ok(Self {
            tasks: Arc::new(RwLock::new(tasks)),
            statuses: Arc::new(RwLock::new(statuses)),
            goal_id: plan.goal_id,
            execution_order,
        })
    }

    /// Валидация DAG и топологическая сортировка
    fn validate_and_sort(tasks: &HashMap<String, TaskNode>) -> Result<Vec<String>, CycleError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        
        // Инициализация
        for id in tasks.keys() {
            in_degree.entry(id.clone()).or_insert(0);
        }
        
        // Построение графа
        for (id, node) in tasks {
            for dep in &node.depends_on {
                if tasks.contains_key(dep) {
                    adj.entry(dep.clone()).or_default().push(id.clone());
                    *in_degree.entry(id.clone()).or_insert(0) += 1;
                }
            }
        }
        
        // Kahn's algorithm
        let mut q: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| id.clone())
            .collect();
        
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        
        while let Some(cur) = q.pop_front() {
            if visited.contains(&cur) {
                continue;
            }
            visited.insert(cur.clone());
            result.push(cur.clone());
            
            if let Some(nei) = adj.get(&cur) {
                for n in nei {
                    let deg = in_degree.get_mut(n).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        q.push_back(n.clone());
                    }
                }
            }
        }
        
        // Детекция циклов
        if result.len() != tasks.len() {
            let cycle: Vec<String> = tasks
                .keys()
                .filter(|k| !visited.contains(*k))
                .cloned()
                .collect();
            return Err(CycleError { cycle });
        }
        
        Ok(result)
    }

    /// Получить готовые к выполнению задачи
    pub async fn get_ready_tasks(&self) -> Vec<TaskNode> {
        let tasks = self.tasks.read().await;
        let statuses = self.statuses.read().await;
        
        tasks
            .values()
            .filter(|n| {
                matches!(statuses.get(&n.id), Some(TaskStatus::Pending))
                    && n.depends_on
                        .iter()
                        .all(|d| matches!(statuses.get(d), Some(TaskStatus::Completed(_))))
            })
            .cloned()
            .collect()
    }

    /// Обновить статус задачи
    pub async fn update_status(&self, id: &str, status: TaskStatus) -> Result<()> {
        let mut st = self.statuses.write().await;
        if !st.contains_key(id) {
            bail!("Task {} not found", id);
        }
        st.insert(id.to_string(), status);
        Ok(())
    }

    /// Проверка завершения всех задач
    pub async fn is_terminal(&self) -> bool {
        let st = self.statuses.read().await;
        st.values()
            .all(|s| {
                matches!(
                    s,
                    TaskStatus::Completed(_) | TaskStatus::Failed(_) | TaskStatus::Cancelled
                )
            })
    }

    /// Получить статистику
    pub async fn stats(&self) -> DagStats {
        let st = self.statuses.read().await;
        let mut p = 0u32;
        let mut r = 0u32;
        let mut c = 0u32;
        let mut f = 0u32;
        let mut x = 0u32;
        
        for s in st.values() {
            match s {
                TaskStatus::Pending => p += 1,
                TaskStatus::Running => r += 1,
                TaskStatus::Completed(_) => c += 1,
                TaskStatus::Failed(_) => f += 1,
                TaskStatus::Cancelled => x += 1,
            }
        }
        
        DagStats {
            total: st.len(),
            pending: p,
            running: r,
            completed: c,
            failed: f,
            cancelled: x,
        }
    }

    /// Отменить все ожидающие задачи
    pub async fn cancel_pending(&self) -> Result<()> {
        let mut st = self.statuses.write().await;
        for (_, s) in st.iter_mut() {
            if *s == TaskStatus::Pending {
                *s = TaskStatus::Cancelled;
            }
        }
        Ok(())
    }
}
            })
            .cloned()
            .collect()
    }
}
