# Changelog

Все значимые изменения в проекте ANT OS.

## [9.0.0] - 2026-02-22

### ✨ Новые возможности

#### WASM Runtime
- **WASM выполнение** (`src/tools/wasm_runtime.rs`)
  - `WasmRuntime` для загрузки и выполнения WASM модулей
  - Изоляция навыков через wasmtime sandbox
  - WASI поддержка через wasmtime-wasi
  - Capability-based security через cap-std
- **Ограничение ресурсов**
  - Fuel (CPU единицы) — предотвращение бесконечных циклов
  - Память (MB) — лимит на использование RAM
  - Таймаут (секунды) — максимальное время выполнения
  - Stack frames — ограничение глубины рекурсии
- **Preopened directories** — безопасный доступ к файловой системе
- **Асинхронное выполнение** — через tokio::task::spawn_blocking
- **WasmRuntimeActor** — интеграция с EventBus

#### Манифесты навыков
- **SkillManifest** (TOML формат)
  - name, version, description, author
  - permissions (filesystem, network, execute, env)
  - resources (max_fuel, max_memory_mb, timeout_secs)
  - exports (process, init, cleanup)
  - dependencies
- **SkillPermissions**
  - FileSystemAccess с allowed_paths и mode
  - Network access (boolean)
  - Execute whitelist
  - Environment variables whitelist
- **ResourceLimits**
  - max_fuel (по умолчанию 1M = ~1 секунда CPU)
  - max_memory_mb (по умолчанию 64MB)
  - timeout_secs (по умолчанию 30s)
  - max_stack_frames (по умолчанию 100)
- **Пример манифеста**: `skills/skill_manifest.example.toml`

#### TUI Themes
- **Система тем** (`src/ui/theme.rs`)
  - `ThemeType` enum (Dark, Light)
  - `Theme` с полной конфигурацией цветов
  - `ThemeColors` для всех компонентов
  - `ThemeManager` для управления
- **Тёмная тема** (по умолчанию)
  - Background: RGB(30, 30, 30)
  - Foreground: RGB(220, 220, 220)
  - Primary: RGB(0, 150, 255)
- **Светлая тема**
  - Background: RGB(250, 250, 250)
  - Foreground: RGB(30, 30, 30)
  - Primary: RGB(0, 100, 200)
- **Переключение тем**
  - Клавиша 't' для переключения
  - Отображение в status bar
  - Применение ко всем виджетам

#### TUI Mouse Support
- **Поддержка мыши** (crossterm MouseEvent)
- **Scroll колёсиком** — навигация по спискам
- **Клик для навигации** — базовая поддержка

### 🔧 Технические улучшения

- **wasmtime** — обновлено до 14.0
- **wasmtime-wasi** — обновлено до 14.0
- **cap-std** — добавлено для security
- **toml** — добавлено для манифестов

### 📦 Зависимости

Добавлены:
- `wasmtime = "14.0"` — WASM runtime
- `wasmtime-wasi = "14.0"` — WASI поддержка
- `cap-std = "2.0"` — Capability-based security
- `toml = "0.8"` — TOML сериализация

### 📝 Документация

- `FINAL_IMPROVEMENTS.md` — описание улучшений v9.0
- `skills/skill_manifest.example.toml` — пример манифеста

### 🎯 Реализация плана

- ✅ WASM выполнение (100%)
- ✅ Манифесты для навыков (100%)
- ✅ Тёмная/светлая тема TUI (100%)
- ⏳ Drag-and-drop в DAG редакторе (базовая поддержка мыши)

### ⚠️ Breaking Changes

- Добавлен `theme_manager` в `DashboardState`
- Изменена структура `SystemEvent` для WASM задач

---

## [8.0.0] - 2026-02-22

### ✨ Новые возможности

#### Память (MEMVID-inspired)
- **Гибридный поиск** (`src/core/memory.rs`)
  - BM25 полнотекстовый поиск через DuckDB FTS
  - Векторный поиск через косинусное сходство
  - Комбинированный поиск с настраиваемыми весами
  - Индексация для ускорения поиска
- **Метаданные**
  - Структура `MemoryMetadata` (source, task_id, goal_id, tool, confidence, tags)
  - Типы событий `MemoryEventType`
  - Автоматические timestamp (created_at, updated_at)
  - Счётчик доступа (access_count)
- **Фильтрация**
  - Поиск с фильтрами по метаданным
  - Фильтры по session_id, tags, tool, date range
- **Пагинация**
  - `get_all_paginated()` для постраничного вывода

#### Time-Travel Debugging
- **Снэпшоты** (`src/core/timetravel.rs`)
  - `SystemSnapshot` с состоянием целей и DAG
  - `GoalState`, `GoalStatus` для отслеживания
  - Автоматическое создание снэпшотов
- **Воспроизведение**
  - `get_state_at_time()` — состояние на момент времени
  - `get_snapshots_in_range()` — снэпшоты в диапазоне
  - `replay_events()` — воспроизведение событий
- **Сравнение**
  - `compare_snapshots()` — разница между снэпшотами
  - Экспорт/импорт JSON
  - Очистка старых снэпшотов

#### Внешние агенты (GOOSE/CODEX)
- **Agent Client** (`src/tools/agents.rs`)
  - `AgentClient` для Goose CLI
  - `AgentClient` для Codex CLI
  - Поддержка Claude Code, Aider, Gemini
  - Асинхронный запуск с таймаутом
  - Проверка доступности и версий
- **AgentAPI интеграция**
  - `AgentApiClient` для унифицированного HTTP API
  - POST /agents/{name}/message
  - GET /agents/{name}/messages (история)
  - GET /agents/{name}/status
  - SSE поток событий
- **Agent Manager**
  - Управление множеством агентов
  - Проверка доступности всех агентов
  - Динамическая регистрация

#### Git интеграция
- **Git Skill** (`src/tools/git.rs`)
  - Поддержка команд: clone, init, add, commit, push, pull
  - Status, log, diff, branch, checkout
  - Remote, fetch, merge, rebase, stash, show
  - `GitCommand` enum для типобезопасности
  - `GitResult` с результатом выполнения
  - Быстрые методы: `status()`, `commit_and_push()`, `current_branch()`
- **Git Actor**
  - Интеграция с EventBus
  - Парсинг команд из input
  - Работа в sandbox

#### TUI улучшения
- **Новые вкладки** (`src/ui/dashboard.rs`)
  - Memory — база знаний с поиском
  - Graph — графики ресурсов (Sparkline)
  - Agents — статус AI агентов
  - Git — статус репозитория
- **Поиск памяти**
  - Активация клавишей 'r'
  - Отображение результатов с превью
  - Навигация ↑/↓
- **Графики**
  - Sparkline для RAM истории
  - Sparkline для CPU истории
  - История за 50 секунд
- **Gauges**
  - RAM gauge с процентом
  - CPU gauge с процентом
- **Навигация**
  - Tab/Shift+Tab переключение вкладок
  - Поддержка мыши (scroll)
  - Скролл логов

### 🔧 Технические улучшения

- **DuckDB FTS** — расширение для полнотекстового поиска
- **FTS триггеры** — автоматическая синхронизация с основной таблицей
- **GIN индекс** — для JSON метаданных
- **Session tracking** — отслеживание сессий
- **Resource history** — история метрик в TUI

### 📦 Зависимости

Добавлены:
- `futures-util = "0.3"` — SSE стриминг

### 📝 Документация

- `README.md` — полное руководство v8.0
- `IMPLEMENTATION_SUMMARY.md` — итоги реализации
- `ANT_ROADMAP.md` — стратегический план
- `.env.example` — шаблон конфигурации

### 🐛 Исправления

- Улучшена обработка ошибок LLM
- Корректная очистка памяти
- Fallback на демо-план при недоступности API

### ⚠️ Breaking Changes

- Изменена структура `VectorMemory` — добавлены метаданные
- Обновлён формат хранения документов в DuckDB
- Изменён `DashboardState` — добавлены поля для памяти и графиков

---

## [7.0.0] - 2026-02-22

### ✨ Новые возможности

#### AI/LLM
- **LLM Integration** (`src/ai/llm.rs`)
  - Поддержка провайдеров: OpenRouter, DeepSeek, Ollama
  - Конфигурация через переменные окружения
  - Методы: `chat()`, `chat_with_context()`, `generate_code()`, `plan_task()`
  - Fallback на демо-план при отсутствии API ключа

#### RAG Memory
- **Векторная память** (`src/core/memory.rs`)
  - FastEmbed (MiniLM) для локальных эмбеддингов
  - DuckDB для хранения векторов
  - Семантический поиск через косинусное сходство
  - Методы: `store()`, `search()`, `delete()`, `get_stats()`

#### Browser
- **Headless Chrome** (`src/tools/browser.rs`)
  - Полноценный веб-скрапинг
  - Методы: `scrape()`, `scrape_with_scroll()`, `save_pdf()`
  - Извлечение текста, заголовков, ссылок
  - Скриншоты страниц
  - Выполнение JavaScript
  - Взаимодействие с элементами (клик, ввод текста)

#### Security
- **Workspace Sandbox** (`src/tools/sandbox.rs`)
  - Изоляция файловых операций в `~/.ant/workspace`
  - Защита от Path Traversal атак
  - Кроссплатформенная поддержка (Windows/macOS/Linux)
  - Capability-система прав доступа
  - Таймауты для команд

#### Orchestrator
- **Оркестратор задач** (`src/sys/orchestrator.rs`)
  - Централизованное управление задачами
  - Интеграция с Sandbox и Memory
  - Обработчики: browser, shell, file:read, file:write, memory:search, memory:store
  - LLM fallback для неизвестных задач

#### TUI
- **Улучшенный Dashboard** (`src/ui/dashboard.rs`)
  - Вкладки: Dashboard, Goals, Logs, Help
  - Поле ввода команд с историей
  - Отображение целей (Goals)
  - Системные метрики: RAM, CPU (sysinfo)
  - Скролл логов (↑/↓)
  - Цветовая индикация статусов

### 🔧 Технические улучшения

- **Cross-Platform**: нормализация путей для Windows (`dunce` crate)
- **Task Timeouts**: лимиты времени выполнения задач
- **System Metrics**: мониторинг ресурсов в реальном времени
- **Event Logging**: расширенное логирование с timestamp

### 📦 Зависимости

Добавлены:
- `fastembed = "4.0"` — векторные эмбеддинги
- `headless_chrome = "1.0"` — браузерная автоматизация
- `sysinfo = "0.30"` — системные метрики
- `dirs = "5.0"` — кроссплатформенные директории
- `dunce = "1.0"` — нормализация путей Windows
- `regex = "1.10"` — работа с регулярными выражениями
- `toml = "0.8"` — поддержка TOML конфигурации

Обновлены:
- `duckdb = { version = "1.1", features = ["bundled", "vtab"] }`

### 📝 Документация

- `.env.example` — шаблон конфигурации
- `README_v7.md` — полная документация v7.0
- `CHANGELOG.md` — история изменений

### 🐛 Исправления

- Улучшена обработка ошибок LLM
- Fallback на демо-план при недоступности API
- Корректная обработка путей на Windows
- Автоскролл для длинных веб-страниц

### ⚠️ Breaking Changes

- Изменен формат событий `TaskDispatched` — добавлено поле `input`
- Обновлен формат `DashboardState` — добавлены `goals`, `input_text`, `memory_stats`
- Изменен `PlanningEngine::create_plan()` — теперь использует LLM

---

## [6.0.0] - Историческая версия

### Особенности
- Микроядерная архитектура
- EventBus на tokio::broadcast
- Supervisor с fault tolerance
- DAG планировщик задач
- WASM песочница (заглушка)
- TUI Dashboard (базовый)
- DuckDB аудит

---

## Планы на будущее

### v7.1.0
- [ ] Tauri 2.0 интеграция для GUI браузера
- [ ] MCP (Model Context Protocol) клиент
- [ ] Skills Marketplace (`npx skills`)

### v7.2.0
- [ ] Реальное выполнение WASM модулей
- [ ] Event replay из DuckDB
- [ ] TOML конфигурация

### v8.0.0 (Enterprise)
- [ ] Мультиагентная координация
- [ ] Распределенное выполнение задач
- [ ] PostgreSQL бэкенд
- [ ] REST API для внешнего управления
