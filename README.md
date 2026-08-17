# 🚀 ANT OS v0.9.0

**AI Orchestration System** — микрокернел-ОС для AI-агентов с поддержкой distributed-режима

[![CI](https://github.com/antos/ant/actions/workflows/ci.yml/badge.svg)](https://github.com/antos/ant/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75+-orange)](https://rust-lang.org)

---

## 📋 Оглавление

- [Возможности](#-возможности)
- [Быстрый старт](#-быстрый-старт)
- [Архитектура](#-архитектура)
- [Режимы работы](#-режимы-работы)
- [CLI (antctl)](#-cli-antctl)
- [Docker & K8s](#-docker--k8s)
- [Метрики](#-метрики)
- [Разработка](#-разработка)
- [Лицензия](#-лицензия)

---

## ✨ Возможности

### Ядро
- 🔄 **Event-Driven Architecture** — событийная шина для коммуникации компонентов
- 📊 **DAG Scheduler** — планировщик задач с топологической сортировкой и детекцией циклов
- 🛡️ **Supervisor Tree** — отказоустойчивость в стиле Erlang/OTP
- 🧠 **Hybrid Memory** — гибридная память (векторный поиск + DuckDB)
- ⏪ **Time Travel Debugging** — отладка с возможностью «отмотать назад»

### Интеграции
- 🧩 **WASM Sandbox** — безопасное выполнение навыков в WebAssembly
- 🌐 **Browser Automation** — автоматизация браузера (опционально)
- 🔗 **Git Integration** — работа с git-репозиториями
- 🤖 **LLM Integration** — поддержка LLM через OpenRouter API

### Observability
- 📈 **Prometheus Metrics** — метрики производительности
- 🖥️ **TUI Dashboard** — терминальный дашборд (Ratatui)
- 📜 **Structured Logging** — структурированные логи (tracing)
- 🔍 **Audit Log** — аудит событий в DuckDB

### Distributed Mode
- 📡 **NATS JetStream** — распределённая шина событий
- 🗄️ **Redis State Store** — распределённое хранилище состояний
- ☸️ **Kubernetes Native** — манифесты для K8s деплоя
- 🔄 **Horizontal Scaling** — автомасштабирование через HPA

---

## 🚀 Быстрый старт

### Требования

- Rust 1.75+
- (Опционально) Docker & Docker Compose
- (Опционально) Kubernetes cluster

### Сборка из исходников

```bash
# Клонирование репозитория
git clone https://github.com/antos/ant.git
cd ant

# Сборка релизной версии
cargo build --release --features "metrics,structured-logs"

# Запуск
./target/release/ant
```

### Запуск через Docker Compose

```bash
# Клонирование
git clone https://github.com/antos/ant.git
cd ant

# Запуск всего стека (ANT + NATS + Redis + Prometheus)
docker-compose up -d

# Проверка статуса
docker-compose ps

# Просмотр логов
docker-compose logs -f ant
```

### Использование CLI

```bash
# Проверка здоровья
./target/release/antctl health

# Статус системы
./target/release/antctl status

# Отправить задачу
./target/release/antctl submit "Проанализировать рынок акций" --priority 8

# Показать логи
./target/release/antctl logs --lines 100

# Показать метрики
./target/release/antctl metrics
```

---

## 🏗️ Архитектура

```
┌─────────────────────────────────────────────────────────────────┐
│                         ANT OS                                   │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │   TUI       │  │   Metrics   │  │    CLI      │              │
│  │  Dashboard  │  │   Server    │  │  (antctl)   │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
│         │                │                │                      │
│         └────────────────┼────────────────┘                      │
│                          │                                       │
│  ┌───────────────────────▼────────────────────────────┐         │
│  │              Event Bus (NATS / broadcast)          │         │
│  └───────────────────────┬────────────────────────────┘         │
│                          │                                       │
│  ┌───────────┬───────────┼───────────┬───────────┐              │
│  │           │           │           │           │              │
│  │ Scheduler │  Memory   │  Sandbox  │  Tools    │              │
│  │   (DAG)   │ (DuckDB)  │  (WASM)   │ (Git,etc) │              │
│  │           │           │           │           │              │
│  └───────────┴───────────┴───────────┴───────────┘              │
│                                                                  │
│  ┌──────────────────────────────────────────────────┐           │
│  │           Supervisor Tree (Fault Tolerance)      │           │
│  └──────────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

### Компоненты

| Компонент | Описание | Модуль |
|-----------|----------|--------|
| **EventBus** | Шина событий (MPMC) | `src/bus.rs` |
| **Scheduler** | DAG-планировщик задач | `src/core/scheduler.rs` |
| **Memory** | Векторная память + DuckDB | `src/core/memory.rs` |
| **Sandbox** | WASM-песочница | `src/tools/sandbox.rs` |
| **Supervisor** | Демон с restart policy | `src/sys/supervisor.rs` |
| **AuditLogger** | Логирование событий | `src/sys/audit_logger.rs` |
| **Metrics** | Prometheus + warp server | `src/metrics.rs` |
| **TUI** | Terminal dashboard | `src/ui/dashboard.rs` |

---

## 🔧 Режимы работы

### Single-Node (по умолчанию)

```bash
# Запуск в локальном режиме
cargo run

# Event Bus: tokio::broadcast
# State: in-memory + DuckDB
```

### Distributed (Kubernetes)

```bash
# Запуск с distributed-режимом
cargo run --features distributed

# Event Bus: NATS JetStream
# State Store: Redis
```

### Feature Flags

| Feature | Описание | Зависимости |
|---------|----------|-------------|
| `default` | metrics + structured-logs | prometheus, warp, tracing-appender |
| `distributed` | NATS + Redis | async-nats, redis |
| `metrics` | Prometheus server | prometheus, warp |
| `structured-logs` | JSON логи | tracing-appender |
| `with-wasm` | WASM runtime | wasmtime, wasmtime-wasi |
| `with-browser` | Browser automation | headless_chrome, fastembed |
| `with-git` | Git integration | git2 |
| `full` | Все возможности | все выше |

---

## 🖥️ CLI (antctl)

### Команды

| Команда | Описание | Пример |
|---------|----------|--------|
| `submit` | Отправить задачу | `antctl submit "Task" --priority 5` |
| `status` | Статус системы | `antctl status` |
| `logs` | Показать логи | `antctl logs --lines 50` |
| `skills` | Список WASM skills | `antctl skills` |
| `health` | Проверка здоровья | `antctl health` |
| `shutdown` | Shutdown системы | `antctl shutdown --force` |
| `metrics` | Prometheus метрики | `antctl metrics` |

### Примеры использования

```bash
# Отправить задачу с высоким приоритетом
antctl submit "Проанализировать конкурентов" --priority 9

# Проверить статус
antctl status
# 🚀 ANT OS Status
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#   Active DAGs:   3
#   Active Goals:  2
#   Completed:     15
#   Failed:        1
#   Pending:       5
#   Version:       0.9.0

# Посмотреть последние 100 строк логов
antctl logs --lines 100

# Проверить health
antctl health
# ✅ Healthy
```

---

## 🐳 Docker & K8s

### Docker Compose

```yaml
# docker-compose.yml предоставляет:
# - NATS cluster (JetStream)
# - Redis (state store)
# - ANT OS application
# - Prometheus (metrics)
```

Запуск:
```bash
docker-compose up -d
```

### Kubernetes

```bash
# Деплой в K8s
kubectl apply -f k8s-deployment.yaml

# Проверка статуса
kubectl get pods -n antos
kubectl get services -n antos

# Метрики
kubectl port-forward svc/ant 9090:9090 -n antos
curl http://localhost:9090/metrics
```

### Helm Values (пример)

```yaml
replicaCount: 3

image:
  repository: antos/ant
  tag: v0.9.0

env:
  NATS_URL: "nats://nats.antos.svc.cluster.local:4222"
  REDIS_URL: "redis://redis.antos.svc.cluster.local:6379"

autoscaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70
```

---

## 📈 Метрики

### Доступные метрики

| Метрика | Тип | Описание |
|---------|-----|----------|
| `ant_tasks_total` | Counter | Всего задач отправлено |
| `ant_tasks_completed_total` | Counter | Всего задач завершено |
| `ant_tasks_failed_total` | Counter | Всего задач упало |
| `ant_task_duration_seconds` | Histogram | Длительность выполнения задач |
| `ant_active_dags` | Gauge | Активные DAG |
| `ant_active_goals` | Gauge | Активные цели |

### Prometheus scrape config

```yaml
scrape_configs:
  - job_name: 'ant'
    static_configs:
      - targets: ['ant:9090']
    metrics_path: /metrics
    scrape_interval: 5s
```

### Grafana Dashboard

Импортируйте дашборд из `grafana/dashboard.json` (опционально).

---

## 🛠️ Разработка

### Требования для разработки

- Rust 1.75+
- pkg-config
- libssl-dev

```bash
# Ubuntu/Debian
sudo apt-get install pkg-config libssl-dev

# macOS
brew install pkg-config openssl
```

### Тесты

```bash
# Запуск всех тестов
cargo test --all-features

# Запуск конкретных тестов
cargo test event_bus

# Запуск тестов с выводом логов
RUST_LOG=debug cargo test -- --nocapture
```

### Бенчмарки

```bash
# Запуск бенчмарков
cargo bench

# Конкретный бенчмарк
cargo bench --bench scheduler_bench
```

### Форматирование и линтинг

```bash
# Форматирование
cargo fmt

# Clippy
cargo clippy --all-features -- -D warnings
```

### Сборка документации

```bash
cargo doc --open
```

---

## 📁 Структура проекта

```
ant/
├── .github/workflows/    # CI/CD
│   ├── ci.yml
│   └── release.yml
├── benches/              # Бенчмарки
│   └── scheduler_bench.rs
├── src/
│   ├── bin/
│   │   └── antctl.rs     # CLI утилита
│   ├── ai/               # AI/планирование
│   ├── core/             # Ядро (DAG, scheduler, memory)
│   ├── sys/              # Системные компоненты
│   ├── tools/            # Инструменты (WASM, git, browser)
│   ├── ui/               # TUI dashboard
│   ├── bus.rs            # Event Bus
│   ├── metrics.rs        # Prometheus метрики
│   └── main.rs           # Точка входа
├── tests/                # Интеграционные тесты
│   ├── common/
│   ├── event_bus_test.rs
│   └── integration_tests.rs
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── k8s-deployment.yaml   # K8s манифесты
├── prometheus.yml
├── README.md
└── .env.example
```

---

## 🔐 Конфигурация

### Переменные окружения

| Переменная | Описание | По умолчанию |
|------------|----------|--------------|
| `RUST_LOG` | Уровень логирования | `info,ant=debug` |
| `NATS_URL` | URL NATS сервера | (не установлено) |
| `REDIS_URL` | URL Redis сервера | (не установлено) |
| `OPENROUTER_API_KEY` | API ключ для LLM | (не установлено) |
| `WASM_FUEL_LIMIT` | Лимит топлива WASM | `10000000` |
| `WASM_MEMORY_LIMIT` | Лимит памяти WASM | `67108864` |
| `METRICS_PORT` | Порт метрик | `9090` |
| `DUCKDB_PATH` | Путь к DuckDB | `./ant_audit.duckdb` |

---

## 📄 Лицензия

MIT OR Apache-2.0 — см. [LICENSE](LICENSE)

---

## 🤝 Вклад

См. [CONTRIBUTING.md](CONTRIBUTING.md)

---

## 📞 Контакты

- GitHub: https://github.com/antos/ant
- Issues: https://github.com/antos/ant/issues

---

**🦀 Сделано на Rust с любовью**
