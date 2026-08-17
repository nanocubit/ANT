//! Time-Travel Debugging Module
//! Воспроизведение состояния системы на любой момент времени

use crate::bus::SystemEvent;
use crate::core::dag::{DagState, TaskStatus, ExecutionPlan};
use chrono::{DateTime, Utc};
use duckdb::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Снэпшот состояния системы
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemSnapshot {
    pub timestamp: DateTime<Utc>,
    pub active_goals: Vec<GoalState>,
    pub completed_goals: Vec<GoalState>,
    pub dag_states: HashMap<String, DagState>,
    pub memory_stats: MemorySnapshot,
}

/// Состояние цели
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalState {
    pub id: String,
    pub task: String,
    pub status: GoalStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<String>,
    pub plan: Option<ExecutionPlan>,
}

/// Статус цели
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Pending,
    Planning,
    Executing,
    Completed,
    Failed,
}

/// Снэпшот памяти
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemorySnapshot {
    pub total_documents: usize,
    pub total_size_bytes: usize,
}

/// Менеджер time-travel debugging
pub struct TimeTravelDebugger {
    conn: Connection,
}

impl TimeTravelDebugger {
    /// Инициализация отладчика
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        
        // Создаём таблицы для снэпшотов и лога событий
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS system_snapshots (
                id VARCHAR PRIMARY KEY,
                timestamp TIMESTAMP DEFAULT current_timestamp,
                snapshot JSON NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON system_snapshots(timestamp);
            
            CREATE TABLE IF NOT EXISTS event_log (
                id VARCHAR PRIMARY KEY,
                ts TIMESTAMP DEFAULT current_timestamp,
                type VARCHAR,
                payload JSON
            );
            CREATE INDEX IF NOT EXISTS idx_event_log_ts ON event_log(ts);"
        )?;

        Ok(Self { conn })
    }

    /// Создать снэпшот текущего состояния
    pub fn create_snapshot(
        &self,
        snapshot: &SystemSnapshot,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let snapshot_json = serde_json::to_string(snapshot)?;

        self.conn.execute(
            "INSERT INTO system_snapshots (id, timestamp, snapshot) VALUES (?, ?, ?)",
            params![id, snapshot.timestamp.to_rfc3339(), snapshot_json],
        )?;

        Ok(id)
    }

    /// Получить состояние на указанный момент времени
    pub fn get_state_at_time(
        &self,
        timestamp: DateTime<Utc>,
    ) -> anyhow::Result<Option<SystemSnapshot>> {
        // Находим ближайший снэпшот до указанного времени
        let mut stmt = self.conn.prepare(
            "SELECT snapshot FROM system_snapshots 
             WHERE timestamp <= ? 
             ORDER BY timestamp DESC 
             LIMIT 1"
        )?;

        let snapshot = stmt.query_row(params![timestamp.to_rfc3339()], |row| {
            let snapshot_json: String = row.get(0)?;
            let snapshot: SystemSnapshot = serde_json::from_str(&snapshot_json)?;
            Ok(snapshot)
        }).optional()?;

        Ok(snapshot)
    }

    /// Получить все снэпшоты в диапазоне времени
    pub fn get_snapshots_in_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<Vec<SystemSnapshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT snapshot FROM system_snapshots 
             WHERE timestamp BETWEEN ? AND ? 
             ORDER BY timestamp ASC"
        )?;

        let snapshots = stmt.query_map(params![from.to_rfc3339(), to.to_rfc3339()], |row| {
            let snapshot_json: String = row.get(0)?;
            let snapshot: SystemSnapshot = serde_json::from_str(&snapshot_json)?;
            Ok(snapshot)
        })?;

        let mut results = Vec::new();
        for snapshot in snapshots {
            results.push(snapshot?);
        }

        Ok(results)
    }

    /// Воспроизвести события из лога для восстановления состояния
    pub fn replay_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<Vec<SystemEvent>> {
        // Предполагаем, что события хранятся в event_log с timestamp
        let mut stmt = self.conn.prepare(
            "SELECT payload FROM event_log 
             WHERE ts BETWEEN ? AND ? 
             ORDER BY ts ASC"
        )?;

        let events = stmt.query_map(params![from.to_rfc3339(), to.to_rfc3339()], |row| {
            let payload: String = row.get(0)?;
            let event: SystemEvent = serde_json::from_str(&payload)?;
            Ok(event)
        })?;

        let mut results = Vec::new();
        for event in events {
            results.push(event?);
        }

        Ok(results)
    }

    /// Получить список всех доступных снэпшотов
    pub fn list_snapshots(&self) -> anyhow::Result<Vec<SnapshotInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, length(snapshot) as size 
             FROM system_snapshots 
             ORDER BY timestamp DESC"
        )?;

        let snapshots = stmt.query_map(params![], |row| {
            let id: String = row.get(0)?;
            let timestamp: String = row.get(1)?;
            let size: i64 = row.get(2)?;

            let ts = DateTime::parse_from_rfc3339(&timestamp)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);

            Ok(SnapshotInfo {
                id,
                timestamp: ts,
                size_bytes: size as usize,
            })
        })?;

        let mut results = Vec::new();
        for snapshot in snapshots {
            results.push(snapshot?);
        }

        Ok(results)
    }

    /// Удалить снэпшот по ID
    pub fn delete_snapshot(&self, id: &str) -> anyhow::Result<usize> {
        let affected = self.conn.execute(
            "DELETE FROM system_snapshots WHERE id = ?",
            params![id],
        )?;
        Ok(affected)
    }

    /// Очистить старые снэпшоты (старше указанного времени)
    pub fn cleanup_old_snapshots(
        &self,
        older_than: DateTime<Utc>,
    ) -> anyhow::Result<usize> {
        let affected = self.conn.execute(
            "DELETE FROM system_snapshots WHERE timestamp < ?",
            params![older_than.to_rfc3339()],
        )?;
        Ok(affected)
    }

    /// Сравнить два снэпшота
    pub fn compare_snapshots(
        &self,
        id1: &str,
        id2: &str,
    ) -> anyhow::Result<SnapshotComparison> {
        let mut stmt = self.conn.prepare(
            "SELECT snapshot FROM system_snapshots WHERE id = ?"
        )?;

        let snapshot1_json: String = stmt.query_row(params![id1], |row| row.get(0))?;
        let snapshot2_json: String = stmt.query_row(params![id2], |row| row.get(0))?;

        let snapshot1: SystemSnapshot = serde_json::from_str(&snapshot1_json)?;
        let snapshot2: SystemSnapshot = serde_json::from_str(&snapshot2_json)?;

        let goals_added = snapshot2.active_goals.len() as i32 - snapshot1.active_goals.len() as i32;
        let goals_completed = snapshot2.completed_goals.len() as i32 - snapshot1.completed_goals.len() as i32;

        Ok(SnapshotComparison {
            snapshot1_id: id1.to_string(),
            snapshot2_id: id2.to_string(),
            time_delta_seconds: (snapshot2.timestamp - snapshot1.timestamp).num_seconds(),
            goals_added,
            goals_completed,
            memory_delta_bytes: snapshot2.memory_stats.total_size_bytes as i64 
                - snapshot1.memory_stats.total_size_bytes as i64,
        })
    }

    /// Экспорт снэпшота в JSON файл
    pub fn export_snapshot(&self, id: &str, output_path: &str) -> anyhow::Result<()> {
        let mut stmt = self.conn.prepare(
            "SELECT snapshot FROM system_snapshots WHERE id = ?"
        )?;

        let snapshot_json: String = stmt.query_row(params![id], |row| row.get(0))?;
        
        // Форматируем JSON для читаемости
        let formatted: serde_json::Value = serde_json::from_str(&snapshot_json)?;
        let pretty_json = serde_json::to_string_pretty(&formatted)?;

        std::fs::write(output_path, pretty_json)?;
        Ok(())
    }

    /// Импорт снэпшота из JSON файла
    pub fn import_snapshot(&self, input_path: &str) -> anyhow::Result<String> {
        let json_content = std::fs::read_to_string(input_path)?;
        let snapshot: SystemSnapshot = serde_json::from_str(&json_content)?;
        
        self.create_snapshot(&snapshot)
    }
}

/// Информация о снэпшоте
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub size_bytes: usize,
}

/// Сравнение двух снэпшотов
#[derive(Debug, Clone)]
pub struct SnapshotComparison {
    pub snapshot1_id: String,
    pub snapshot2_id: String,
    pub time_delta_seconds: i64,
    pub goals_added: i32,
    pub goals_completed: i32,
    pub memory_delta_bytes: i64,
}

/// Автоматическое создание снэпшотов
pub struct SnapshotScheduler {
    debugger: std::sync::Arc<TimeTravelDebugger>,
    interval_secs: u64,
}

impl SnapshotScheduler {
    pub fn new(debugger: std::sync::Arc<TimeTravelDebugger>, interval_secs: u64) -> Self {
        Self {
            debugger,
            interval_secs,
        }
    }

    /// Запустить фоновое создание снэпшотов
    pub async fn run(&self) -> ! {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(self.interval_secs));

        loop {
            interval.tick().await;

            // Здесь должна быть логика сбора текущего состояния системы
            // Для упрощения создаём пустой снэпшот
            let snapshot = SystemSnapshot {
                timestamp: Utc::now(),
                ..Default::default()
            };

            if let Err(e) = self.debugger.create_snapshot(&snapshot) {
                eprintln!("Failed to create snapshot: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let temp_path = "/tmp/test_timetravel.duckdb";
        let debugger = TimeTravelDebugger::new(temp_path).unwrap();

        let snapshot = SystemSnapshot {
            timestamp: Utc::now(),
            active_goals: vec![
                GoalState {
                    id: "G-1".to_string(),
                    task: "Test task".to_string(),
                    status: GoalStatus::Executing,
                    created_at: Utc::now(),
                    completed_at: None,
                    result: None,
                    plan: None,
                },
            ],
            completed_goals: vec![],
            dag_states: HashMap::new(),
            memory_stats: MemorySnapshot {
                total_documents: 5,
                total_size_bytes: 1024,
            },
        };

        let id = debugger.create_snapshot(&snapshot).unwrap();
        assert!(!id.is_empty());

        // Получение снэпшота
        let retrieved = debugger.get_state_at_time(Utc::now()).unwrap();
        assert!(retrieved.is_some());

        // Очистка
        std::fs::remove_file(temp_path).ok();
    }
}
