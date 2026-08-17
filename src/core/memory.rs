use anyhow::{Context, Result};
use duckdb::{Connection, params};
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use chrono::{DateTime, Utc};

/// Типы событий для памяти
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum MemoryEventType {
    TaskResult { task_id: String, tool: String },
    WebScrape { url: String },
    CodeAnalysis { file_path: String },
    UserInput { session_id: String },
    LLMResponse { model: String },
    SystemLog { level: String },
    GitOperation { repo: String, operation: String },
    AgentAction { agent: String, action: String },
}

/// Метаданные воспоминания
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryMetadata {
    pub source: String,
    pub task_id: Option<String>,
    pub goal_id: Option<String>,
    pub tool: Option<String>,
    pub confidence: f32,
    pub tags: Vec<String>,
    pub event_type: Option<MemoryEventType>,
    pub session_id: Option<String>,
    pub parent_id: Option<String>,
}

/// Документ в базе знаний
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDocument {
    pub id: String,
    pub content: String,
    pub title: Option<String>,
    pub embedding: Vec<f32>,
    pub metadata: MemoryMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub access_count: u64,
}

/// Результат гибридного поиска
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    pub document: KnowledgeDocument,
    pub hybrid_score: f32,
    pub bm25_score: f32,
    pub vector_score: f32,
}

/// Статистика памяти
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_documents: usize,
    pub total_sessions: usize,
    pub total_size_bytes: usize,
    pub avg_embedding_dim: usize,
    pub oldest_document: Option<DateTime<Utc>>,
    pub newest_document: Option<DateTime<Utc>>,
}

/// Векторная база знаний для RAG с гибридным поиском
pub struct VectorMemory {
    conn: Connection,
    embedding_model: Arc<TextEmbedding>,
    db_path: String,
    bm25_initialized: bool,
}

impl VectorMemory {
    /// Инициализация базы знаний
    pub fn new(db_path: &str) -> Result<Self> {
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)
            .context("Failed to open DuckDB connection")?;

        // Включаем расширения
        conn.execute_batch("INSTALL fts; LOAD fts; INSTALL vss; LOAD vss;")?;

        // Создаём основную таблицу с расширенными полями
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS knowledge_base (
                id VARCHAR PRIMARY KEY,
                content VARCHAR NOT NULL,
                title VARCHAR,
                embedding FLOAT[],
                metadata JSON,
                created_at TIMESTAMP DEFAULT current_timestamp,
                updated_at TIMESTAMP DEFAULT current_timestamp,
                access_count BIGINT DEFAULT 0,
                session_id VARCHAR,
                parent_id VARCHAR
            );
            CREATE INDEX IF NOT EXISTS idx_created_at ON knowledge_base(created_at);
            CREATE INDEX IF NOT EXISTS idx_session_id ON knowledge_base(session_id);
            CREATE INDEX IF NOT EXISTS idx_parent_id ON knowledge_base(parent_id);
            CREATE INDEX IF NOT EXISTS idx_metadata ON knowledge_base USING GIN(metadata);"
        )?;

        // Инициализируем модель эмбеддингов
        let embedding_model = Arc::new(
            TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true)
            )?
        );

        let mut memory = Self {
            conn,
            embedding_model,
            db_path: db_path.to_string(),
            bm25_initialized: false,
        };

        // Инициализируем BM25 индекс для первой колонки
        memory.init_bm25()?;

        Ok(memory)
    }

    /// Инициализация BM25 полнотекстового поиска
    fn init_bm25(&mut self) -> Result<()> {
        // Проверяем, существует ли уже индекс
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_database_list WHERE name = 'fts')",
            params![],
            |row| row.get(0),
        ).unwrap_or(false);

        if !exists {
            // Создаём FTS индекс для полнотекстового поиска
            self.conn.execute_batch(
                "CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_base_fts USING fts5(
                    content,
                    title,
                    content='knowledge_base',
                    content_rowid='rowid'
                );
                
                -- Триггеры для синхронизации FTS с основной таблицей
                CREATE TRIGGER IF NOT EXISTS knowledge_base_ai AFTER INSERT ON knowledge_base BEGIN
                    INSERT INTO knowledge_base_fts(rowid, content, title) 
                    VALUES (NEW.rowid, NEW.content, COALESCE(NEW.title, ''));
                END;
                
                CREATE TRIGGER IF NOT EXISTS knowledge_base_ad AFTER DELETE ON knowledge_base BEGIN
                    INSERT INTO knowledge_base_fts(knowledge_base_fts, rowid, content, title) 
                    VALUES('delete', OLD.rowid, OLD.content, COALESCE(OLD.title, ''));
                END;
                
                CREATE TRIGGER IF NOT EXISTS knowledge_base_au AFTER UPDATE ON knowledge_base BEGIN
                    INSERT INTO knowledge_base_fts(knowledge_base_fts, rowid, content, title) 
                    VALUES('delete', OLD.rowid, OLD.content, COALESCE(OLD.title, ''));
                    INSERT INTO knowledge_base_fts(rowid, content, title) 
                    VALUES (NEW.rowid, NEW.content, COALESCE(NEW.title, ''));
                END;"
            )?;
            self.bm25_initialized = true;
        }

        Ok(())
    }

    /// Генерация эмбеддинга для текста
    pub fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embedding_model.embed(vec![text.to_string()], None)?;
        let mut iter = embeddings.into_iter();
        Ok(iter.next().unwrap_or_default())
    }

    /// Сохранение документа с метаданными
    pub async fn store(
        &self,
        source: &str,
        content: &str,
        metadata: MemoryMetadata,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let embedding = self.generate_embedding(content)?;
        let now = Utc::now();

        let metadata_json = serde_json::to_string(&metadata)?;
        let embedding_json = serde_json::to_string(&embedding)?;

        self.conn.execute(
            "INSERT INTO knowledge_base 
             (id, content, embedding, metadata, created_at, updated_at, session_id, parent_id) 
             VALUES (?, ?, ?::FLOAT[], ?, ?, ?, ?, ?)",
            params![
                id,
                content,
                embedding_json,
                metadata_json,
                now.to_rfc3339(),
                now.to_rfc3339(),
                metadata.session_id,
                metadata.parent_id
            ],
        )?;

        Ok(id)
    }

    /// Сохранение с заголовком
    pub async fn store_with_title(
        &self,
        source: &str,
        title: &str,
        content: &str,
        metadata: MemoryMetadata,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let embedding = self.generate_embedding(content)?;
        let now = Utc::now();

        let metadata_json = serde_json::to_string(&metadata)?;
        let embedding_json = serde_json::to_string(&embedding)?;

        self.conn.execute(
            "INSERT INTO knowledge_base 
             (id, content, title, embedding, metadata, created_at, updated_at, session_id, parent_id) 
             VALUES (?, ?, ?, ?::FLOAT[], ?, ?, ?, ?, ?)",
            params![
                id,
                content,
                title,
                embedding_json,
                metadata_json,
                now.to_rfc3339(),
                now.to_rfc3339(),
                metadata.session_id,
                metadata.parent_id
            ],
        )?;

        Ok(id)
    }

    /// Гибридный поиск (BM25 + векторный)
    pub async fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
        bm25_weight: f32,
        vector_weight: f32,
    ) -> Result<Vec<HybridSearchResult>> {
        let query_embedding = self.generate_embedding(query)?;
        let query_embedding_str = serde_json::to_string(&query_embedding)?;

        // Нормализуем веса
        let total_weight = bm25_weight + vector_weight;
        let bm25_norm = bm25_weight / total_weight;
        let vector_norm = vector_weight / total_weight;

        // Гибридный запрос с комбинированным скором
        let mut stmt = self.conn.prepare(
            "WITH bm25_scores AS (
                SELECT rowid, 
                       bm25(knowledge_base_fts) AS score
                FROM knowledge_base_fts
                WHERE knowledge_base_fts MATCH ?
            ),
            vector_scores AS (
                SELECT id,
                       array_cosine_similarity(embedding, ?::FLOAT[]) AS score
                FROM knowledge_base
            ),
            combined AS (
                SELECT 
                    kb.id,
                    kb.content,
                    kb.title,
                    kb.embedding,
                    kb.metadata,
                    kb.created_at,
                    kb.updated_at,
                    kb.access_count,
                    COALESCE(b.score, 0) AS bm25_score,
                    COALESCE(v.score, 0) AS vector_score,
                    (? * COALESCE(b.score, 0) + ? * COALESCE(v.score, 0)) AS hybrid_score
                FROM knowledge_base kb
                LEFT JOIN bm25_scores b ON kb.rowid = b.rowid
                LEFT JOIN vector_scores v ON kb.id = v.id
                ORDER BY hybrid_score DESC
                LIMIT ?
            )
            SELECT * FROM combined"
        )?;

        let docs = stmt.query_map(
            params![query, query_embedding_str, bm25_norm, vector_norm, limit as i64],
            |row| {
                let id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let title: Option<String> = row.get(2)?;
                let embedding_str: String = row.get(3)?;
                let metadata_str: String = row.get(4)?;
                let created_at: String = row.get(5)?;
                let updated_at: String = row.get(6)?;
                let access_count: u64 = row.get(7)?;
                let bm25_score: f32 = row.get(8)?;
                let vector_score: f32 = row.get(9)?;
                let hybrid_score: f32 = row.get(10)?;

                let embedding: Vec<f32> = serde_json::from_str(&embedding_str).unwrap_or_default();
                let metadata: MemoryMetadata = serde_json::from_str(&metadata_str).unwrap_or_default();
                let created_at = DateTime::parse_from_rfc3339(&created_at)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc);
                let updated_at = DateTime::parse_from_rfc3339(&updated_at)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc);

                Ok(HybridSearchResult {
                    document: KnowledgeDocument {
                        id,
                        content,
                        title,
                        embedding,
                        metadata,
                        created_at,
                        updated_at,
                        access_count,
                    },
                    hybrid_score,
                    bm25_score,
                    vector_score,
                })
            }
        )?;

        let mut results = Vec::new();
        for doc in docs {
            results.push(doc?);
        }

        // Увеличиваем счётчик доступа для найденных документов
        for result in &results {
            self.conn.execute(
                "UPDATE knowledge_base SET access_count = access_count + 1 WHERE id = ?",
                params![&result.document.id],
            ).ok();
        }

        Ok(results)
    }

    /// Векторный поиск (только косинусное сходство)
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<HybridSearchResult>> {
        self.hybrid_search(query, limit, 0.0, 1.0).await
    }

    /// BM25 поиск (только текст)
    pub async fn bm25_search(&self, query: &str, limit: usize) -> Result<Vec<HybridSearchResult>> {
        self.hybrid_search(query, limit, 1.0, 0.0).await
    }

    /// Поиск с фильтрами по метаданным
    pub async fn search_with_filters(
        &self,
        query: &str,
        limit: usize,
        filters: &MemoryFilters,
    ) -> Result<Vec<HybridSearchResult>> {
        let query_embedding = self.generate_embedding(query)?;
        let query_embedding_str = serde_json::to_string(&query_embedding)?;

        let mut where_clauses = Vec::new();
        let mut params_vec: Vec<duckdb::Value> = Vec::new();

        if let Some(session_id) = &filters.session_id {
            where_clauses.push("session_id = ?");
            params_vec.push(duckdb::Value::from(session_id.clone()));
        }

        if let Some(parent_id) = &filters.parent_id {
            where_clauses.push("parent_id = ?");
            params_vec.push(duckdb::Value::from(parent_id.clone()));
        }

        if let Some(tags) = &filters.tags {
            let tags_json = serde_json::to_string(tags)?;
            where_clauses.push("json_extract(metadata, '$.tags') LIKE ?");
            params_vec.push(duckdb::Value::from(format!("%{}%", tags_json)));
        }

        if let Some(tool) = &filters.tool {
            where_clauses.push("json_extract(metadata, '$.tool') = ?");
            params_vec.push(duckdb::Value::from(tool.clone()));
        }

        if let Some(from_date) = filters.from_date {
            where_clauses.push("created_at >= ?");
            params_vec.push(duckdb::Value::from(from_date.to_rfc3339()));
        }

        if let Some(to_date) = filters.to_date {
            where_clauses.push("created_at <= ?");
            params_vec.push(duckdb::Value::from(to_date.to_rfc3339()));
        }

        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let query_str = format!(
            "WITH vector_scores AS (
                SELECT id,
                       array_cosine_similarity(embedding, ?::FLOAT[]) AS score
                FROM knowledge_base
                {}
            )
            SELECT kb.id, kb.content, kb.title, kb.embedding, kb.metadata,
                   kb.created_at, kb.updated_at, kb.access_count,
                   0.0 AS bm25_score,
                   COALESCE(v.score, 0) AS vector_score,
                   COALESCE(v.score, 0) AS hybrid_score
            FROM knowledge_base kb
            JOIN vector_scores v ON kb.id = v.id
            ORDER BY hybrid_score DESC
            LIMIT ?",
            where_clause
        );

        let mut stmt = self.conn.prepare(&query_str)?;
        
        let mut params: Vec<&dyn duckdb::ToSql> = Vec::new();
        params.push(&query_embedding_str);
        for p in &params_vec {
            params.push(p);
        }
        params.push(&limit as i64);

        let docs = stmt.query_map(params.as_slice(), |row| {
            // Аналогично hybrid_search
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let title: Option<String> = row.get(2)?;
            let embedding_str: String = row.get(3)?;
            let metadata_str: String = row.get(4)?;
            let created_at: String = row.get(5)?;
            let updated_at: String = row.get(6)?;
            let access_count: u64 = row.get(7)?;
            let bm25_score: f32 = row.get(8)?;
            let vector_score: f32 = row.get(9)?;
            let hybrid_score: f32 = row.get(10)?;

            let embedding: Vec<f32> = serde_json::from_str(&embedding_str).unwrap_or_default();
            let metadata: MemoryMetadata = serde_json::from_str(&metadata_str).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);

            Ok(HybridSearchResult {
                document: KnowledgeDocument {
                    id,
                    content,
                    title,
                    embedding,
                    metadata,
                    created_at,
                    updated_at,
                    access_count,
                },
                hybrid_score,
                bm25_score,
                vector_score,
            })
        })?;

        let mut results = Vec::new();
        for doc in docs {
            results.push(doc?);
        }

        Ok(results)
    }

    /// Фильтры для поиска
    pub async fn search_content(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let results = self.search(query, limit).await?;
        Ok(results.into_iter().map(|r| r.document.content).collect())
    }

    /// Получить документ по ID
    pub fn get(&self, id: &str) -> Result<Option<KnowledgeDocument>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, title, embedding, metadata, created_at, updated_at, access_count
             FROM knowledge_base WHERE id = ?"
        )?;

        let doc = stmt.query_row(params![id], |row| {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let title: Option<String> = row.get(2)?;
            let embedding_str: String = row.get(3)?;
            let metadata_str: String = row.get(4)?;
            let created_at: String = row.get(5)?;
            let updated_at: String = row.get(6)?;
            let access_count: u64 = row.get(7)?;

            let embedding: Vec<f32> = serde_json::from_str(&embedding_str).unwrap_or_default();
            let metadata: MemoryMetadata = serde_json::from_str(&metadata_str).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);

            Ok(KnowledgeDocument {
                id,
                content,
                title,
                embedding,
                metadata,
                created_at,
                updated_at,
                access_count,
            })
        }).optional()?;

        Ok(doc)
    }

    /// Удалить документ по ID
    pub fn delete(&self, id: &str) -> Result<usize> {
        let affected = self.conn.execute(
            "DELETE FROM knowledge_base WHERE id = ?",
            params![id],
        )?;
        Ok(affected)
    }

    /// Удалить все документы из источника
    pub fn delete_by_session(&self, session_id: &str) -> Result<usize> {
        let affected = self.conn.execute(
            "DELETE FROM knowledge_base WHERE session_id = ?",
            params![session_id],
        )?;
        Ok(affected)
    }

    /// Получить все документы с пагинацией
    pub fn get_all_paginated(&self, limit: usize, offset: usize) -> Result<Vec<KnowledgeDocument>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, title, embedding, metadata, created_at, updated_at, access_count
             FROM knowledge_base ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )?;

        let docs = stmt.query_map(params![limit as i64, offset as i64], |row| {
            // Аналогично get()
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let title: Option<String> = row.get(2)?;
            let embedding_str: String = row.get(3)?;
            let metadata_str: String = row.get(4)?;
            let created_at: String = row.get(5)?;
            let updated_at: String = row.get(6)?;
            let access_count: u64 = row.get(7)?;

            let embedding: Vec<f32> = serde_json::from_str(&embedding_str).unwrap_or_default();
            let metadata: MemoryMetadata = serde_json::from_str(&metadata_str).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);

            Ok(KnowledgeDocument {
                id,
                content,
                title,
                embedding,
                metadata,
                created_at,
                updated_at,
                access_count,
            })
        })?;

        let mut result = Vec::new();
        for doc in docs {
            result.push(doc?);
        }

        Ok(result)
    }

    /// Получить статистику базы
    pub fn get_stats(&self) -> Result<MemoryStats> {
        let total_docs: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM knowledge_base",
            params![],
            |row| row.get(0),
        )?;

        let total_sessions: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT session_id) FROM knowledge_base WHERE session_id IS NOT NULL",
            params![],
            |row| row.get(0),
        ).unwrap_or(0);

        let total_size: i64 = self.conn.query_row(
            "SELECT SUM(length(content)) FROM knowledge_base",
            params![],
            |row| row.get(0),
        )?;

        let oldest: Option<String> = self.conn.query_row(
            "SELECT MIN(created_at) FROM knowledge_base",
            params![],
            |row| row.get(0),
        ).unwrap_or(None);

        let newest: Option<String> = self.conn.query_row(
            "SELECT MAX(created_at) FROM knowledge_base",
            params![],
            |row| row.get(0),
        ).unwrap_or(None);

        Ok(MemoryStats {
            total_documents: total_docs as usize,
            total_sessions: total_sessions as usize,
            total_size_bytes: total_size as usize,
            avg_embedding_dim: 384, // AllMiniLML6V2 dimension
            oldest_document: oldest.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&Utc)),
            newest_document: newest.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&Utc)),
        })
    }

    /// Получить последние документы
    pub fn get_recent(&self, limit: usize) -> Result<Vec<KnowledgeDocument>> {
        self.get_all_paginated(limit, 0)
    }

    /// Получить документы с высоким access_count (популярные)
    pub fn get_popular(&self, limit: usize) -> Result<Vec<KnowledgeDocument>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, title, embedding, metadata, created_at, updated_at, access_count
             FROM knowledge_base ORDER BY access_count DESC LIMIT ?"
        )?;

        let docs = stmt.query_map(params![limit as i64], |row| {
            // Аналогично get()
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let title: Option<String> = row.get(2)?;
            let embedding_str: String = row.get(3)?;
            let metadata_str: String = row.get(4)?;
            let created_at: String = row.get(5)?;
            let updated_at: String = row.get(6)?;
            let access_count: u64 = row.get(7)?;

            let embedding: Vec<f32> = serde_json::from_str(&embedding_str).unwrap_or_default();
            let metadata: MemoryMetadata = serde_json::from_str(&metadata_str).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);

            Ok(KnowledgeDocument {
                id,
                content,
                title,
                embedding,
                metadata,
                created_at,
                updated_at,
                access_count,
            })
        })?;

        let mut result = Vec::new();
        for doc in docs {
            result.push(doc?);
        }

        Ok(result)
    }

    /// Получить путь к базе данных
    pub fn db_path(&self) -> &str {
        &self.db_path
    }
}

/// Фильтры для поиска
#[derive(Debug, Clone, Default)]
pub struct MemoryFilters {
    pub session_id: Option<String>,
    pub parent_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tool: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hybrid_search() {
        let temp_path = "/tmp/test_ant_hybrid_memory.duckdb";
        let memory = VectorMemory::new(temp_path).unwrap();

        // Сохранение документов
        let mut metadata = MemoryMetadata::default();
        metadata.source = "test".to_string();
        metadata.tags = vec!["test".to_string(), "rust".to_string()];
        
        memory.store("test_source", "Тестовый контент о программировании на Rust", metadata.clone()).await.unwrap();
        memory.store("test_source", "Веб-скрапинг и парсинг HTML", metadata.clone()).await.unwrap();
        memory.store("test_source", "Машинное обучение и нейросети", metadata).await.unwrap();

        // Гибридный поиск
        let results = memory.hybrid_search("программирование", 5, 0.5, 0.5).await.unwrap();
        assert!(!results.is_empty());
        assert!(results[0].hybrid_score > 0.0);

        // BM25 поиск
        let bm25_results = memory.bm25_search("веб скрапинг", 5).await.unwrap();
        assert!(!bm25_results.is_empty());

        // Статистика
        let stats = memory.get_stats().unwrap();
        assert_eq!(stats.total_documents, 3);

        // Очистка
        std::fs::remove_file(temp_path).ok();
    }
}
