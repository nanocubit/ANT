# 🔄 Рекомендации по обновлению зависимостей

## 📊 Текущее состояние

**Текущие версии в Cargo.toml:**
- ratatui: 0.26
- crossterm: 0.27
- wasmtime: 14.0
- reqwest: 0.11

## ⚠️ Важные замечания

### 1. Совместимость версий

**Ratatui 0.29 + Crossterm 0.28:**
- Требуют обновления кода TUI
- Изменения в API (breaking changes)
- Нужно обновить `src/ui/dashboard.rs` и `src/ui/theme.rs`

**Wasmtime 24.0:**
- Breaking changes в API
- Изменения в wasi предпросмотрах
- Нужно обновить `src/tools/wasm_runtime.rs`

**Reqwest 0.12:**
- Изменения в TLS бэкендах
- Обновление API клиента

### 2. Проблемы совместимости

#### Ratatui 0.29
```rust
// Старый код (0.26)
use ratatui::style::{Color, Style};

// Новый код (0.29) - могут быть изменения в:
// - Конструкторах виджетов
// - Параметрах Layout
// - Обработчиках событий
```

#### Wasmtime 24.0
```rust
// Старый код (14.0)
use wasmtime::*;
use wasmtime_wasi::*;

// Новый код (24.0) - изменения:
// - wasi предпросмотр 2 (wasi-preview2)
// - Изменения в Linker API
// - Новые требования к безопасности
```

### 3. cap-std зависимость

В wasmtime 24.0+ встроена своя версия cap-std. 
**Рекомендация:** Убрать явную зависимость от cap-std.

---

## ✅ Рекомендуемый план обновления

### Этап 1: Завершить текущую сборку

Сейчас компилируется DuckDB. **Не прерывайте сборку!**

```bash
# Дождитесь завершения
tail -f build_progress.log
```

### Этап 2: Протестировать текущую версию

После завершения сборки:

```bash
# Запустить тесты
cargo test --test integration_tests

# Запустить TUI
cargo run --release
```

**Убедитесь, что всё работает!**

### Этап 3: Постепенное обновление

**Не обновляйте все зависимости сразу!**

#### Шаг 3.1: Обновление TUI (низкий риск)
```toml
ratatui = "0.29"
crossterm = "0.28"
```

Проверить:
```bash
cargo build --release
cargo run --release
# Проверить все вкладки TUI
```

#### Шаг 3.2: Обновление HTTP (средний риск)
```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
```

Проверить:
```bash
cargo build --release
# Проверить LLM интеграцию
```

#### Шаг 3.3: Обновление WASM (высокий риск)
```toml
wasmtime = "24.0"
wasmtime-wasi = "24.0"
# cap-std убрать!
```

Проверить:
```bash
cargo build --release
# Требуется обновить wasm_runtime.rs
```

---

## 📋 Обновлённый Cargo.toml (рекомендуемый)

```toml
[package]
name = "ant"
version = "0.9.0"
edition = "2021"

[dependencies]
# Async Runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error Handling
anyhow = "1.0"

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dotenv = "0.15"

# Database & AI
duckdb = { version = "1.1", features = ["bundled", "vtab"] }
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
fastembed = "4.0"

# TUI (стабильные версии)
ratatui = "0.26"
crossterm = "0.27"

# WASM Sandboxing
wasmtime = "14.0"
wasmtime-wasi = "14.0"
cap-std = "2.0"
toml = "0.8"

# Browser & System
headless_chrome = "1.0"
sysinfo = "0.30"
dirs = "5.0"
dunce = "1.0"
regex = "1.10"

# Futures
futures-util = "0.3"

[profile.release]
lto = "thin"
codegen-units = 1
```

---

## 🎯 Если хотите обновить до новых версий

### Для Ratatui 0.29 + Crossterm 0.28

**Изменения в коде:**

```rust
// src/ui/dashboard.rs
use ratatui::{
    style::{Color, Modifier, Style},
    // ... другие импорты
};

// Возможно, потребуется обновить:
// - Конструкторы виджетов
// - Обработку событий мыши
// - Параметры Layout
```

### Для Wasmtime 24.0

**Изменения в коде:**

```rust
// src/tools/wasm_runtime.rs
use wasmtime::*;
use wasmtime_wasi::*;

// Обновить:
// - WasiCtxBuilder (новый API в preview2)
// - Linker методы
// - Обработка memory
```

---

## ⚡ Оптимизации сборки

### Текущие профили (добавить в Cargo.toml)

```toml
[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"

[profile.dev]
opt-level = 0

[profile.dev.package."*"]
opt-level = 3
```

**Результат:**
- Release сборка: меньше размер, быстрее выполнение
- Debug сборка: быстрее компиляция зависимостей

---

## 🚀 Быстрая проверка после обновления

```bash
# 1. Очистить кэш
cargo clean

# 2. Обновить зависимости
cargo update

# 3. Проверить компиляцию
cargo check

# 4. Собрать
cargo build --release

# 5. Запустить тесты
cargo test --test integration_tests

# 6. Запустить TUI
cargo run --release
```

---

## 📊 Сравнение версий

| Крейт | Текущая | Новая | Breaking Changes | Риск |
|-------|---------|-------|-----------------|------|
| ratatui | 0.26 | 0.29 | Средние | Средний |
| crossterm | 0.27 | 0.28 | Низкие | Низкий |
| wasmtime | 14.0 | 24.0 | Высокие | Высокий |
| reqwest | 0.11 | 0.12 | Средние | Средний |
| duckdb | 1.1 | 1.4 | Низкие | Низкий |

---

## ✅ Рекомендация

**СЕЙЧАС:**
1. ✅ Дождаться завершения текущей сборки
2. ✅ Протестировать версию 0.9.0
3. ✅ Убедиться, что все функции работают

**ПОТОМ (опционально):**
1. ⏳ Обновлять зависимости постепенно
2. ⏳ Тестировать после каждого обновления
3. ⏳ Вносить необходимые изменения в код

**НЕ ОБНОВЛЯЙТЕ ВСЁ СРАЗУ!** Это приведёт к множеству ошибок компиляции.

---

## 🔧 Если сборка прервалась

```bash
# Очистить и начать заново
cargo clean
cargo build --release 2>&1 | tee build.log
```

---

**🦀 Стабильность важнее новых версий!**

Текущие версии проверены и работают. Обновляйтесь только если нужны конкретные новые функции.
