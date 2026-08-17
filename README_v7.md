# 🦀 ANT OS v7.0 — AI Orchestration System

**Autonomous Neural Tasking** — микроядерная ОС для искусственного интеллекта на Rust.

---

## 🚀 Возможности v7.0

### Реализованные функции

| Компонент | Статус | Описание |
|-----------|--------|----------|
| **LLM Integration** | ✅ | OpenRouter, DeepSeek, Ollama поддержка |
| **RAG Memory** | ✅ | Векторная база знаний на FastEmbed + DuckDB |
| **TUI Dashboard** | ✅ | Интерактивный терминальный UI с вкладками |
| **Headless Browser** | ✅ | Веб-скрапинг на headless_chrome |
| **Workspace Sandbox** | ✅ | Изоляция файловых операций |
| **Capability System** | ✅ | Система прав доступа для инструментов |
| **Task Timeouts** | ✅ | Лимиты времени выполнения задач |
| **Cross-Platform** | ✅ | Windows/macOS/Linux поддержка |
| **System Metrics** | ✅ | Мониторинг RAM/CPU в реальном времени |

### Планируемые улучшения

- [ ] **Tauri 2.0 Browser** — полноценный GUI браузер
- [ ] **WASM Execution** — реальное выполнение WASM модулей
- [ ] **MCP Client** — Model Context Protocol интеграция
- [ ] **Skills Marketplace** — подключение npx skills
- [ ] **Event Replay** — восстановление состояния из DuckDB
- [ ] **Config System** — TOML конфигурация

---

## 📦 Установка

### Требования

- Rust 1.70+
- Chrome/Chromium (для headless браузера)
- API ключ (OpenRouter/DeepSeek) для LLM функций

### Сборка

```bash
# Клонирование и сборка
git clone <repository>
cd ant
cargo build --release

# Копирование примера конфигурации
cp .env.example .env

# Редактирование .env (добавьте API ключ)
nano .env
```

---

## 🎮 Запуск

```bash
cargo run --release
```

### Управление в TUI

| Клавиша | Действие |
|---------|----------|
| `Tab` | Переключение вкладок (Dashboard/Goals/Logs/Help) |
| `Enter` | Отправить команду/цель |
| `Esc` / `q` | Выход |
| `↑` / `↓` | Скролл логов |
| `Backspace` | Удаление символа в input |

---

## 📝 Примеры команд

### Веб-скрапинг
```
scrape https://rust-lang.org
```

### Поиск в RAG базе
```
search токенизация трансформеры
```

### Выполнение команд в sandbox
```
run cargo build
```

### Анализ кода
```
analyze fn main() { println!("Hello"); }
```

### Сохранение в память
```
store source|content text here
```

---

## 🏗️ Архитектура

```
┌─────────────────────────────────────────────────────┐
│              TUI Dashboard (Ratatui)                │
│  [Dashboard] [Goals] [Logs] [Help]                  │
└─────────────────────┬───────────────────────────────┘
                      │ EventBus (broadcast)
┌─────────────────────┼───────────────────────────────┐
│                     │                               │
│  ┌──────────────────▼───────────────────┐           │
│  │      Core Scheduler (DAG)            │           │
│  └──────────────────┬───────────────────┘           │
│                     │                               │
│  ┌──────────────────▼───────────────────┐           │
│  │         Orchestrator                 │           │
│  │  ┌──────────────┐  ┌──────────────┐  │           │
│  │  │   Sandbox    │  │    Memory    │  │           │
│  │  └──────────────┘  └──────────────┘  │           │
│  └───────────────────────────────────────┘           │
│                     │                               │
│  ┌──────────────────┴───────────────────┐           │
│  │         Tool Actors                  │           │
│  │  ┌────────┐ ┌────────┐ ┌──────────┐  │           │
│  │  │Browser │ │  LSP   │ │   WASM   │  │           │
│  │  └────────┘ └────────┘ └──────────┘  │           │
│  └───────────────────────────────────────┘           │
│                                                      │
│  ┌──────────────────────────────────────┐            │
│  │        System Services               │            │
│  │  ┌──────────┐  ┌──────────────────┐  │            │
│  │  │Supervisor│  │  AuditLogger     │  │            │
│  │  │(restart) │  │  (DuckDB)        │  │            │
│  │  └──────────┘  └──────────────────┘  │            │
│  └──────────────────────────────────────┘            │
└──────────────────────────────────────────────────────┘
```

---

## 🔧 Конфигурация

### Переменные окружения

| Переменная | Описание | Пример |
|------------|----------|--------|
| `OPENROUTER_API_KEY` | API ключ для LLM | `sk-or-v1-...` |
| `LLM_PROVIDER` | Провайдер: openrouter/deepseek/ollama | `openrouter` |
| `LLM_MODEL` | Модель для генерации | `qwen/qwen-2.5-coder-32b-instruct` |
| `LLM_BASE_URL` | Кастомный URL (для Ollama) | `http://localhost:11434` |

---

## 🧪 Тесты

```bash
# Запуск тестов
cargo test

# Тесты с игнорированием требующих Chrome
cargo test -- --skip test_browser
```

---

## 📊 Структура проекта

```
ant/
├── Cargo.toml
├── .env.example
├── src/
│   ├── main.rs              # Точка входа
│   ├── bus.rs               # Шина событий
│   ├── ai/
│   │   ├── mod.rs
│   │   ├── llm.rs           # LLM клиент (OpenRouter, DeepSeek)
│   │   └── planner.rs       # Планировщик задач
│   ├── core/
│   │   ├── mod.rs
│   │   ├── dag.rs           # DAG структуры
│   │   ├── scheduler.rs     # Планировщик
│   │   └── memory.rs        # RAG векторная память
│   ├── sys/
│   │   ├── mod.rs
│   │   ├── supervisor.rs    # Fault tolerance
│   │   ├── audit_logger.rs  # DuckDB аудит
│   │   ├── orchestrator.rs  # Оркестратор задач
│   │   └── policy_engine.rs # Policy проверки
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── browser.rs       # Headless Chrome
│   │   ├── sandbox.rs       # Workspace Sandbox
│   │   ├── wasm_actor.rs    # WASM execution
│   │   └── dummy_actors.rs  # Demo акторы
│   └── ui/
│       └── dashboard.rs     # TUI интерфейс
└── target/
```

---

## 🔐 Безопасность

### Workspace Sandbox

Все файловые операции ограничены директорией `~/.ant/workspace`:

```rust
let sandbox = WorkspaceSandbox::new()?;

// Безопасное чтение
let content = sandbox.read_file("config.toml").await?;

// Безопасная запись
sandbox.write_file("output.txt", "data").await?;

// Попытка выхода за пределы sandbox будет заблокирована
sandbox.read_file("../etc/passwd").await?; // Ошибка!
```

### Capability System

Каждый инструмент требует соответствующих прав:

```rust
pub enum Capability {
    NetworkAccess,
    FileSystemRead,
    FileSystemWrite,
    ExecuteBinaries,
}
```

---

## 🤝 Вклад в проект

1. Fork репозитория
2. Создание ветки (`git checkout -b feature/amazing-feature`)
3. Commit изменений (`git commit -m 'Add amazing feature'`)
4. Push (`git push origin feature/amazing-feature`)
5. Pull Request

---

## 📄 Лицензия

MIT License — см. [LICENSE](LICENSE)

---

## 🙏 Благодарности

- **Rust** — язык программирования
- **Tokio** — асинхронная runtime
- **Ratatui** — TUI библиотека
- **DuckDB** — встроенная БД
- **FastEmbed** — векторные эмбеддинги
- **headless_chrome** — браузерная автоматизация

---

## 📞 Контакты

- GitHub Issues: [Сообщить о проблеме](../../issues)
- Discussions: [Обсуждения](../../discussions)

---

**🦀 ANT OS — Building the Future of AI Orchestration**
