//! Audit Logger - вечный лог событий на DuckDB
//! Сохраняет все события системы для последующего анализа

use crate::bus::{EventBus, SystemEvent};
use duckdb::Connection;
use std::sync::Arc;

pub struct AuditLogger;

impl AuditLogger {
    pub async fn run_daemon(bus: Arc<EventBus>) -> anyhow::Result<()> {
        // Инициализация DuckDB
        let conn = Connection::open("ant_audit.duckdb")?;
        
        // Создание таблицы событий
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS event_log (
                id VARCHAR PRIMARY KEY,
                ts TIMESTAMP DEFAULT current_timestamp,
                type VARCHAR NOT NULL,
                source VARCHAR,
                payload JSON
            );
            CREATE INDEX IF NOT EXISTS idx_event_type ON event_log(type);
            CREATE INDEX IF NOT EXISTS idx_event_ts ON event_log(ts);
            CREATE INDEX IF NOT EXISTS idx_event_source ON event_log(source);"
        )?;

        let mut rx = bus.subscribe();
        
        while let Ok(event) = rx.recv().await {
            let id = uuid::Uuid::new_v4().to_string();
            let event_type = event.name();
            let payload = serde_json::to_string(&event)?;
            
            // Извлекаем источник из события
            let source = match &event {
                SystemEvent::Log { source, .. } => source.clone(),
                SystemEvent::DaemonStatus { daemon, .. } => daemon.clone(),
                _ => "system".to_string(),
            };

            // Сохраняем в базу
            conn.execute(
                "INSERT INTO event_log (id, type, source, payload) VALUES (?, ?, ?, ?)",
                duckdb::params![id, event_type, source, payload],
            ).ok();
        }
        
        Ok(())
    }
}
