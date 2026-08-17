# 🚀 ANT OS Roadmap 2026

Стратегический план развития ANT OS на основе анализа лучших практик из MEMVID, GOOSE/CODEX, TUI и GIT проектов.

---

## 📊 Текущее состояние (v7.0)

### ✅ Реализовано
- [x] Базовая RAG память на DuckDB + FastEmbed
- [x] LLM интеграция (OpenRouter, DeepSeek, Ollama)
- [x] Headless Chrome для скрапинга
- [x] Workspace Sandbox с Capability системой
- [x] TUI Dashboard с вкладками
- [x] EventBus архитектура
- [x] Supervisor с fault tolerance
- [x] DAG планировщик задач
- [x] Кроссплатформенная поддержка

### ⚠️ Ограничения текущей версии
- [ ] Только векторный поиск (нет гибридного BM25 + векторы)
- [ ] Нет time-travel debugging
- [ ] Нет интеграции с внешними AI агентами
- [ ] TUI только для просмотра (нет интерактивности)
- [ ] Нет Git интеграции
- [ ] WASM только заглушка (нет реального выполнения)

---

## 🎯 Стратегические направления

### 1. 🧠 Улучшение памяти (MEMVID-inspired)

**Цель:** Превратить память ANT в систему уровня MEMVID с задержкой <5мс и точностью на 60% выше.

#### Задачи:

| ID | Задача | Приоритет | Сложность |
|----|--------|-----------|-----------|
| 1.1 | Гибридный поиск (BM25 + векторный) | Высокий | Средняя |
| 1.2 | Расширенные метаданные для воспоминаний | Высокий | Низкая |
| 1.3 | Time-travel debugging | Средний | Высокая |
| 1.4 | Визуализация памяти в TUI | Средний | Средняя |
| 1.5 | API для внешних фреймворков (LangChain, AutoGen) | Низкий | Высокая |

#### Технические детали:

**1.1 Гибридный поиск**
```rust
// Расширенная схема памяти
CREATE TABLE memories (
    id VARCHAR PRIMARY KEY,
    text VARCHAR,
    embedding FLOAT[],
    metadata JSON,
    timestamp TIMESTAMP,
    session_id VARCHAR,
    event_type VARCHAR
);

// Создание индексов
INSTALL fts;
LOAD fts;
PRAGMA create_fts_index('memories', 'id', 'text');

// Гибридный запрос
SELECT text, metadata,
       (fts_score * 0.3 + vector_similarity * 0.7) AS score
FROM memories
WHERE fts_match('memories', ?)
ORDER BY score DESC
LIMIT 10;
```

**1.2 Метаданные**
```rust
pub struct MemoryMetadata {
    pub source: String,        // "scrape", "task_result", "user_input"
    pub task_id: Option<String>,
    pub goal_id: Option<String>,
    pub tool: Option<String>,
    pub confidence: f32,
    pub tags: Vec<String>,
}
```

**1.3 Time-travel debugging**
```rust
pub fn get_state_at_time(timestamp: DateTime<Utc>) -> Result<AgentState> {
    // Воспроизвести все события до timestamp
    let events = conn.query(
        "SELECT * FROM event_log WHERE ts <= ? ORDER BY ts",
        params![timestamp]
    )?;
    
    let mut state = AgentState::default();
    for event in events {
        state.apply(event);
    }
    Ok(state)
}
```

---

### 2. 🤖 Интеграция внешних агентов (GOOSE/CODEX)

**Цель:** Сделать ANT мета-оркестратором, объединяющим лучших AI агентов.

#### Задачи:

| ID | Задача | Приоритет | Сложность |
|----|--------|-----------|-----------|
| 2.1 | Навык-обёртка для Goose CLI | Высокий | Низкая |
| 2.2 | Навык-обёртка для Codex CLI | Высокий | Низкая |
| 2.3 | Интеграция AgentAPI от Coder | Высокий | Средняя |
| 2.4 | Поддержка множественных агентов | Средний | Высокая |
| 2.5 | Переключение между агентами динамически | Низкий | Высокая |

#### Архитектура интеграции:

```
┌─────────────────────────────────────────────────────┐
│                    ANT Orchestrator                 │
│  (планирование, DAG, супервизоры, память)           │
└─────────────────────┬───────────────────────────────┘
                      │ EventBus
┌─────────────────────┼───────────────────────────────┐
│                     │                               │
│  ┌──────────────────▼───────────────────┐           │
│  │      AgentAPI Client (навык)         │           │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  │           │
│  │  │ Goose  │  │ Codex  │  │ Claude │  │           │
│  │  │        │  │        │  │ Code   │  │           │
│  │  └────────┘  └────────┘  └────────┘  │           │
│  └───────────────────────────────────────┘           │
│                     │ HTTP API                        │
│  ┌──────────────────▼───────────────────┐           │
│  │         AgentAPI Server              │           │
│  │  (унифицированный интерфейс)         │           │
│  └───────────────────────────────────────┘           │
└───────────────────────────────────────────────────────┘
```

**2.3 AgentAPI интеграция**
```rust
// skills/agentapi_client.rs
pub struct AgentApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl AgentApiClient {
    pub async fn send_message(&self, agent: &str, message: &str) -> Result<String> {
        let response = self.client
            .post(format!("{}/agents/{}/message", self.base_url, agent))
            .json(&serde_json::json!({
                "content": message,
                "session_id": "default"
            }))
            .send()
            .await?;
        
        let result: serde_json::Value = response.json().await?;
        Ok(result["content"].as_str().unwrap().to_string())
    }
}
```

---

### 3. 🖥️ Развитие TUI до профессионального уровня

**Цель:** Превратить TUI из "смотрелки" в полноценную консоль управления.

#### Задачи:

| ID | Задача | Приоритет | Сложность |
|----|--------|-----------|-----------|
| 3.1 | Интерактивное редактирование DAG | Высокий | Высокая |
| 3.2 | Панель памяти с поиском | Высокий | Средняя |
| 3.3 | Графики ресурсов в реальном времени | Средний | Средняя |
| 3.4 | Фильтрация и поиск логов | Средний | Низкая |
| 3.5 | Панель управления навыками (WASM) | Низкий | Высокая |
| 3.6 | Тёмная/светлая тема | Низкий | Низкая |

#### Компоненты TUI:

```
┌──────────────────────────────────────────────────────────────┐
│  Status Bar: RAM 125MB │ CPU 12% │ Tasks: 5 │ Agents: 2     │
├──────────────────────────────────────────────────────────────┤
│  [Dashboard] [Memory] [Graph] [Logs] [Skills] [Agents] [?]  │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────────────┐  ┌─────────────────────────────────┐ │
│  │   DAG Graph        │  │   Memory Panel                  │ │
│  │   [t1]───►[t2]     │  │   🔍 [Search...]                │ │
│  │    │      │        │  │   ─────────────────────────────  │ │
│  │   [t3]◄──[t4]      │  │   [14:32] Task completed: ...   │ │
│  │                    │  │   [14:31] Scraped 15KB from...  │ │
│  │   ← Drag & Drop →  │  │   [14:30] LLM response: ...     │ │
│  └────────────────────┘  └─────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │   Resource Graphs                                      │  │
│  │   CPU: ▃▄▅▆▇▁▂▃ (12%)                                 │  │
│  │   RAM: ▃▃▄▄▅▅▆▆ (125MB)                               │  │
│  │   Tasks: ▆▆▆▇▇▇▇▇ (8 active)                          │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│  ❯ Analyze code and fix bugs                                │
└──────────────────────────────────────────────────────────────┘
```

**3.2 Интерактивный DAG**
```rust
// ui/components/dag_editor.rs
pub struct DagEditor {
    nodes: HashMap<String, DagNode>,
    selected_node: Option<String>,
    drag_state: Option<DragState>,
}

impl DagEditor {
    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Drag(node_id, new_pos) => {
                self.nodes.get_mut(&node_id).unwrap().position = new_pos;
            }
            MouseEvent::Click(node_id) => {
                self.selected_node = Some(node_id);
            }
            _ => {}
        }
    }
    
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.nodes.get_mut(to).unwrap().depends_on.push(from.to_string());
    }
}
```

---

### 4. 🔗 Git интеграция

**Цель:** Полная автоматизация Git операций через DAG пайплайны.

#### Задачи:

| ID | Задача | Приоритет | Сложность |
|----|--------|-----------|-----------|
| 4.1 | WASM навык git (clone, commit, push) | Высокий | Средняя |
| 4.2 | Визуализация git статуса в TUI | Средний | Низкая |
| 4.3 | CI/CD пайплайны через DAG | Высокий | Высокая |
| 4.4 | Автоматические PR через агентов | Низкий | Высокая |

**4.1 Git навык**
```rust
// skills/git.rs
pub struct GitSkill {
    sandbox: WorkspaceSandbox,
}

impl GitSkill {
    pub async fn execute(&self, command: GitCommand) -> Result<String> {
        match command {
            GitCommand::Clone { url, path } => {
                self.sandbox.run_cmd("git", &["clone", &url, &path]).await
            }
            GitCommand::Commit { message } => {
                self.sandbox.run_cmd("git", &["commit", "-m", &message]).await
            }
            GitCommand::Push { remote, branch } => {
                self.sandbox.run_cmd("git", &["push", &remote, &branch]).await
            }
            // ... другие команды
        }
    }
}

pub enum GitCommand {
    Clone { url: String, path: String },
    Commit { message: String },
    Push { remote: String, branch: String },
    Pull { remote: String, branch: String },
    Status,
    Log { limit: usize },
}
```

**4.3 CI/CD пайплайн пример**
```yaml
# ant-ci.yaml
goal: "Run CI/CD pipeline"
dag:
  - id: git_checkout
    tool: git
    command: clone https://github.com/user/repo.git
    
  - id: install_deps
    tool: shell
    command: cargo fetch
    depends_on: [git_checkout]
    
  - id: build
    tool: shell
    command: cargo build --release
    depends_on: [install_deps]
    
  - id: test
    tool: shell
    command: cargo test
    depends_on: [build]
    
  - id: goose_review
    tool: agentapi
    agent: goose
    task: "Review code for bugs and style issues"
    depends_on: [test]
    
  - id: commit_changes
    tool: git
    command: commit -m "auto: apply fixes"
    depends_on: [goose_review]
    
  - id: publish
    tool: shell
    command: cargo publish
    depends_on: [commit_changes]
```

---

### 5. 🧩 Реальное WASM выполнение

**Цель:** Полноценная поддержка WASM навыков с изоляцией и ограничением ресурсов.

#### Задачи:

| ID | Задача | Приоритет | Сложность |
|----|--------|-----------|-----------|
| 5.1 | Загрузка и выполнение WASM модулей | Высокий | Высокая |
| 5.2 | Система манифестов для навыков | Высокий | Средняя |
| 5.3 | Изоляция WASM с ограничением ресурсов | Высокий | Высокая |
| 5.4 | Маркетплейс WASM навыков | Средний | Высокая |
| 5.5 | Hot-reload навыков | Низкий | Высокая |

**5.1 WASM выполнение**
```rust
// tools/wasm_runtime.rs
use wasmtime::*;
use wasmtime_wasi::*;

pub struct WasmRuntime {
    engine: Engine,
    linker: Linker<WasiState>,
}

pub struct WasmSkill {
    instance: Instance,
    memory: Memory,
    max_fuel: u64,
}

impl WasmRuntime {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        
        let engine = Engine::new(&config)?;
        let mut linker = Linker::<WasiState>::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        
        Ok(Self { engine, linker })
    }
    
    pub fn load_skill(&self, wasm_bytes: &[u8], manifest: &SkillManifest) -> Result<WasmSkill> {
        let mut store = Store::new(
            &self.engine,
            WasiState::new(WasiCtxBuilder::new()
                .inherit_stdio()
                .build()?)?,
        );
        
        // Устанавливаем лимит топлива (CPU)
        store.set_fuel(manifest.max_fuel)?;
        
        let module = Module::new(&self.engine, wasm_bytes)?;
        let instance = self.linker.instantiate(&mut store, &module)?;
        
        Ok(WasmSkill {
            instance,
            memory: instance.get_memory(&mut store, "memory").unwrap(),
            max_fuel: manifest.max_fuel,
        })
    }
    
    pub fn execute(&self, skill: &mut WasmSkill, input: &[u8]) -> Result<Vec<u8>> {
        // Вызов экспортированной функции process
        let process = skill.instance.get_typed_func::<(u32, u32), u64>(
            &mut store,
            "process"
        )?;
        
        // Выделение памяти для ввода
        let input_ptr = skill.memory.data_ptr(&mut store) as u32;
        let input_len = input.len() as u32;
        
        // Копирование ввода в память WASM
        skill.memory.write(&mut store, input_ptr as usize, input)?;
        
        // Выполнение
        let result_ptr = process.call(&mut store, (input_ptr, input_len))?;
        
        // Чтение результата
        let output = skill.memory.data(&store)[result_ptr as usize..].to_vec();
        Ok(output)
    }
}
```

**5.2 Манифест навыка**
```toml
# skill_manifest.toml
name = "git-skill"
version = "1.0.0"
description = "Git operations skill"
author = "ANT Team"

[permissions]
filesystem = ["read", "write"]
network = false
execute = ["git"]

[resources]
max_fuel = 1_000_000  # ~1 секунда CPU
max_memory_mb = 64
timeout_secs = 30

[exports]
process = "main"
```

---

## 📅 План реализации по кварталам

### Q1 2026 (Январь - Март)
- [x] v7.0: Базовая LLM + RAG память + TUI
- [ ] v7.1: Гибридный поиск + метаданные
- [ ] v7.2: AgentAPI интеграция

### Q2 2026 (Апрель - Июнь)
- [ ] v8.0: Git интеграция + интерактивный TUI
- [ ] v8.1: Time-travel debugging
- [ ] v8.2: Графики ресурсов в TUI

### Q3 2026 (Июль - Сентябрь)
- [ ] v9.0: Реальное WASM выполнение
- [ ] v9.1: Маркетплейс навыков
- [ ] v9.2: CI/CD пайплайны

### Q4 2026 (Октябрь - Декабрь)
- [ ] v10.0: Мультиагентная координация
- [ ] v10.1: API для внешних фреймворков
- [ ] v10.2: Enterprise функции

---

## 🎯 Критерии успеха

### Технические метрики
| Метрика | Текущее | Цель Q2 | Цель Q4 |
|---------|---------|---------|---------|
| Задержка поиска (RAG) | ~50мс | <10мс | <5мс |
| Время запуска | ~2с | <1с | <0.5с |
| Потребление RAM (idle) | ~25MB | ~20MB | ~15MB |
| Макс. параллельных задач | 10 | 50 | 100 |
| Поддержка агентов | 0 | 2 (Goose+Codex) | 5+ |

### Пользовательские метрики
- Количество WASM навыков в маркетплейсе: 0 → 10 → 50+
- Время на построение сложного DAG: 5 мин → 1 мин → 30 сек
- Удовлетворённость TUI (опрос): N/A → 7/10 → 9/10

---

## 🔥 Пример сценария использования (Q4 2026)

**Пользователь ставит цель:**
> "Добавить тесты для модуля parser, исправить баги и запушить изменения"

**ANT строит DAG:**
```
1. git clone репозитория
2. goose --task "написать тесты для parser"
3. cargo test
4. IF тесты упали:
   └─> codex --task "исправить ошибки"
   └─> cargo test (повтор)
5. git add . && git commit -m "add tests"
6. git push
```

**В TUI оператор видит:**
- Визуальный граф с прогрессом каждой задачи
- Логи выполнения в реальном времени
- Графики CPU/RAM
- Предложения из памяти ("похожая задача решалась 2 часа назад")

**При сбое:**
- Supervisor автоматически перезапускает задачу
- Time-travel позволяет откатиться к состоянию до сбоя
- LLM предлагает решение на основе исторических данных

---

## 📚 Ресурсы и референсы

### Проекты для изучения
- **MEMVID** — продвинутая RAG память
- **Goose** — AI агент для кодинга
- **Codex CLI** — OpenAI Codex интерфейс
- **AgentAPI** (Coder) — унифицированный API для агентов
- **Claude Code** — AI агент от Anthropic
- **Aider** — AI pair programming в терминале

### Ключевые технологии
- **DuckDB** — встроенная аналитическая БД
- **FastEmbed** — локальные эмбеддинги
- **headless_chrome** — браузерная автоматизация
- **Wasmtime** — WASM runtime
- **Ratatui** — TUI библиотека
- **Tokio** — асинхронная runtime

---

## 🚀 Следующие шаги

1. **Неделя 1-2:** Гибридный поиск (1.1) + метаданные (1.2)
2. **Неделя 3-4:** AgentAPI интеграция (2.3)
3. **Неделя 5-6:** Git навык (4.1) + TUI визуализация (3.2)
4. **Неделя 7-8:** WASM выполнение (5.1) + манифесты (5.2)

---

**🦀 ANT OS — Building the Future of AI Orchestration**
