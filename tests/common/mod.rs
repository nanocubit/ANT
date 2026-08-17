//! Общие утилиты для тестов

use std::sync::Once;

static INIT: Once = Once::new();

/// Инициализация tracing для тестов
pub fn setup_tracing() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    });
}

/// Временная директория для тестов
pub fn temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ant_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
    dir
}

/// Очистка временной директории
pub fn cleanup_dir(path: &std::path::Path) {
    let _ = std::fs::remove_dir_all(path);
}
