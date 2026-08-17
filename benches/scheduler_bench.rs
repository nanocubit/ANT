//! Бенчмарки для scheduler и DAG

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;

// Имитация структур для бенчмарков
#[derive(Debug, Clone)]
pub struct TaskNode {
    pub id: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub goal_id: String,
    pub steps: Vec<TaskNode>,
}

/// Бенчмарк топологической сортировки DAG
fn bench_dag_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("dag_validation");

    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("tasks_{}", size)),
            size,
            |b, &s| {
                // Линейная зависимость: t0 -> t1 -> t2 -> ...
                let plan = ExecutionPlan {
                    goal_id: "bench".into(),
                    steps: (0..s)
                        .map(|i| TaskNode {
                            id: format!("t{}", i),
                            depends_on: if i == 0 { vec![] } else { vec![format!("t{}", i - 1)] },
                        })
                        .collect(),
                };

                b.to_async(&rt).iter(|| async {
                    let _ = validate_dag(black_box(&plan));
                });
            },
        );
    }

    group.finish();
}

/// Бенчмарк параллельных задач (без зависимостей)
fn bench_dag_parallel(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("dag_parallel");

    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("tasks_{}", size)),
            size,
            |b, &s| {
                // Все задачи независимы
                let plan = ExecutionPlan {
                    goal_id: "bench".into(),
                    steps: (0..s)
                        .map(|i| TaskNode {
                            id: format!("t{}", i),
                            depends_on: vec![],
                        })
                        .collect(),
                };

                b.to_async(&rt).iter(|| async {
                    let _ = validate_dag(black_box(&plan));
                });
            },
        );
    }

    group.finish();
}

/// Бенчмарк сложного графа зависимостей
fn bench_dag_complex(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("dag_complex");

    for size in [10, 20, 30].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("tasks_{}", size)),
            size,
            |b, &s| {
                // Каждая задача зависит от двух предыдущих
                let plan = ExecutionPlan {
                    goal_id: "bench".into(),
                    steps: (0..s)
                        .map(|i| {
                            let mut deps = vec![];
                            if i > 0 {
                                deps.push(format!("t{}", i - 1));
                            }
                            if i > 1 {
                                deps.push(format!("t{}", i - 2));
                            }
                            TaskNode {
                                id: format!("t{}", i),
                                depends_on: deps,
                            }
                        })
                        .collect(),
                };

                b.to_async(&rt).iter(|| async {
                    let _ = validate_dag(black_box(&plan));
                });
            },
        );
    }

    group.finish();
}

/// Бенчмарк детекции циклов
fn bench_cycle_detection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cycle_detection");

    for size in [5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("cycle_{}", size)),
            size,
            |b, &s| {
                // Создаём цикл: t0 -> t1 -> ... -> t{n-1} -> t0
                let mut steps: Vec<TaskNode> = (0..s)
                    .map(|i| TaskNode {
                        id: format!("t{}", i),
                        depends_on: if i == 0 { vec![format!("t{}", s - 1)] } else { vec![format!("t{}", i - 1)] },
                    })
                    .collect();

                // Добавляем одну задачу без цикла для реалистичности
                steps.push(TaskNode {
                    id: "t_no_cycle".into(),
                    depends_on: vec![],
                });

                let plan = ExecutionPlan {
                    goal_id: "bench_cycle".into(),
                    steps,
                };

                b.to_async(&rt).iter(|| async {
                    let _ = validate_dag(black_box(&plan));
                });
            },
        );
    }

    group.finish();
}

/// Функция валидации DAG (упрощённая версия для бенчмарков)
fn validate_dag(plan: &ExecutionPlan) -> Result<Vec<String>, String> {
    use std::collections::{HashMap, HashSet, VecDeque};

    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();

    // Инициализация
    for step in &plan.steps {
        in_degree.entry(step.id.clone()).or_insert(0);
    }

    // Построение графа
    for step in &plan.steps {
        for dep in &step.depends_on {
            if in_degree.contains_key(dep) {
                adj.entry(dep.clone()).or_default().push(step.id.clone());
                *in_degree.entry(step.id.clone()).or_insert(0) += 1;
            }
        }
    }

    // Топологическая сортировка (Kahn's algorithm)
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut result = Vec::new();
    let mut visited = HashSet::new();

    while let Some(cur) = queue.pop_front() {
        if visited.contains(&cur) {
            continue;
        }
        visited.insert(cur.clone());
        result.push(cur.clone());

        if let Some(neighbors) = adj.get(&cur) {
            for neighbor in neighbors {
                let degree = in_degree.get_mut(neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    // Детекция циклов
    if result.len() != plan.steps.len() {
        let cycle_nodes: Vec<String> = plan
            .steps
            .iter()
            .filter(|s| !visited.contains(&s.id))
            .map(|s| s.id.clone())
            .collect();
        return Err(format!("Cycle detected: {:?}", cycle_nodes));
    }

    Ok(result)
}

criterion_group!(
    benches,
    bench_dag_validation,
    bench_dag_parallel,
    bench_dag_complex,
    bench_cycle_detection,
);

criterion_main!(benches);
