use crate::core::dag::{ExecutionPlan, TaskNode};
use crate::ai::llm::{LlmClient, PlanStep};
use anyhow::Result;

pub struct PlanningEngine {
    llm: LlmClient,
}

impl PlanningEngine {
    pub fn new() -> Self {
        Self {
            llm: LlmClient::new(),
        }
    }

    pub async fn create_plan(goal_id: &str, task: &str) -> Result<ExecutionPlan> {
        let engine = Self::new();
        engine.generate_plan(goal_id, task).await
    }

    async fn generate_plan(&self, goal_id: &str, task: &str) -> Result<ExecutionPlan> {
        // Если нет API ключа — выдаем демо-граф
        if !self.llm.config().has_api_key() {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            return Ok(ExecutionPlan {
                goal_id: goal_id.to_string(),
                steps: vec![
                    TaskNode {
                        id: "t1".into(),
                        tool: "browser".into(),
                        input: "https://rust.org".into(),
                        depends_on: vec![],
                    },
                    TaskNode {
                        id: "t2".into(),
                        tool: "lsp".into(),
                        input: "Analyze syntax".into(),
                        depends_on: vec![],
                    },
                    TaskNode {
                        id: "t3".into(),
                        tool: "wasm:analyzer".into(),
                        input: "Process t1 & t2".into(),
                        depends_on: vec!["t1".into(), "t2".into()],
                    },
                ],
            });
        }

        // Запрашиваем план у LLM
        match self.llm.plan_task(task).await {
            Ok(steps) => {
                let task_nodes: Vec<TaskNode> = steps
                    .into_iter()
                    .map(|s: PlanStep| TaskNode {
                        id: s.id,
                        tool: s.tool,
                        input: s.input,
                        depends_on: s.depends_on,
                    })
                    .collect();

                Ok(ExecutionPlan {
                    goal_id: goal_id.to_string(),
                    steps: task_nodes,
                })
            }
            Err(e) => {
                // Fallback на демо-план при ошибке
                eprintln!("LLM planning failed: {}, using fallback plan", e);
                Ok(ExecutionPlan {
                    goal_id: goal_id.to_string(),
                    steps: vec![
                        TaskNode {
                            id: "t1".into(),
                            tool: "shell".into(),
                            input: task.to_string(),
                            depends_on: vec![],
                        },
                    ],
                })
            }
        }
    }
}

impl Default for PlanningEngine {
    fn default() -> Self {
        Self::new()
    }
}
