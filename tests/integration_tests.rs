//! Интеграционные тесты для ANT OS

mod tests {
    use crate::core::memory::{VectorMemory, MemoryMetadata};
    use crate::core::timetravel::TimeTravelDebugger;
    use crate::tools::sandbox::WorkspaceSandbox;
    use crate::tools::git::{GitSkill, GitCommand};
    use crate::ui::theme::{Theme, ThemeManager, ThemeType};

    #[tokio::test]
    async fn test_memory_creation() {
        let temp_path = "/tmp/ant_test_memory.duckdb";
        let memory = VectorMemory::new(temp_path);
        assert!(memory.is_ok(), "Memory should be created");
        
        let memory = memory.unwrap();
        let stats = memory.get_stats();
        assert!(stats.is_ok(), "Stats should be retrieved");
        
        std::fs::remove_file(temp_path).ok();
    }

    #[tokio::test]
    async fn test_memory_store_and_search() {
        let temp_path = "/tmp/ant_test_memory2.duckdb";
        let memory = VectorMemory::new(temp_path).unwrap();

        // Сохранение документа
        let mut metadata = MemoryMetadata::default();
        metadata.source = "test".to_string();
        metadata.tags = vec!["rust".to_string(), "test".to_string()];
        
        let id = memory.store(
            "test_source",
            "Тестовый контент о программировании на Rust",
            metadata
        ).await;
        
        assert!(id.is_ok(), "Document should be stored");
        
        // Поиск
        let results = memory.search("программирование", 5).await;
        assert!(results.is_ok(), "Search should succeed");
        
        let results = results.unwrap();
        assert!(!results.is_empty(), "Should find at least one result");
        
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_timetravel_creation() {
        let temp_path = "/tmp/ant_test_timetravel.duckdb";
        let debugger = TimeTravelDebugger::new(temp_path);
        assert!(debugger.is_ok(), "Debugger should be created");
        
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_theme_creation() {
        let dark_theme = Theme::dark();
        assert_eq!(dark_theme.theme_type, ThemeType::Dark);
        
        let light_theme = Theme::light();
        assert_eq!(light_theme.theme_type, ThemeType::Light);
        
        let mut manager = ThemeManager::new();
        assert_eq!(manager.get_theme().theme_type, ThemeType::Dark);
        
        manager.toggle();
        assert_eq!(manager.get_theme().theme_type, ThemeType::Light);
    }

    #[test]
    fn test_sandbox_creation() {
        let sandbox = WorkspaceSandbox::new();
        assert!(sandbox.is_ok(), "Sandbox should be created");
        
        let sandbox = sandbox.unwrap();
        assert!(sandbox.root().exists(), "Root should exist");
    }

    #[tokio::test]
    async fn test_sandbox_file_operations() {
        let sandbox = WorkspaceSandbox::new().unwrap();
        
        // Запись
        let write_result = sandbox.write_file("test.txt", "Hello, World!").await;
        assert!(write_result.is_ok(), "File should be written");
        
        // Чтение
        let content = sandbox.read_file("test.txt").await;
        assert!(content.is_ok(), "File should be read");
        assert_eq!(content.unwrap(), "Hello, World!");
        
        // Удаление
        let delete_result = sandbox.delete_file("test.txt").await;
        assert!(delete_result.is_ok(), "File should be deleted");
    }

    #[tokio::test]
    async fn test_sandbox_path_security() {
        let sandbox = WorkspaceSandbox::new().unwrap();
        
        // Нормальный путь должен работать
        let path = sandbox.secure_path("test.txt");
        assert!(path.is_ok(), "Normal path should work");
        
        // Попытка выхода должна блокироваться
        let escape_path = sandbox.secure_path("../etc/passwd");
        assert!(escape_path.is_err(), "Path traversal should be blocked");
    }

    // #[tokio::test]
    // #[ignore] // Требуется git в системе
    // async fn test_git_status() {
    //     let git = GitSkill::new(None);
    //     let result = git.status().await;
    //     assert!(result.is_ok(), "Git status should be retrieved");
    // }
}
