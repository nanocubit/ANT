# ⚡ БЫСТРАЯ СБОРКА ANT OS v9.0

## 🎯 Решение проблемы долгой сборки

### Проблема
- DuckDB bundled компилируется 20-40 минут
- headless_chrome компилируется 10-15 минут
- Общее время: 40-60 минут

### Решение
Разделили на 2 варианта:

#### 1. Быстрая сборка (5-10 минут)
```bash
cargo build --release --features fast
```

**Что включено:**
- ✅ TUI (ratatui, crossterm)
- ✅ WASM (wasmtime)
- ✅ Базовые зависимости
- ❌ Без headless_chrome (экономия 10-15 мин)
- ❌ Без fastembed (экономия 5-8 мин)

**Функциональность:**
- ✅ TUI Dashboard
- ✅ DAG Scheduler
- ✅ Supervisor
- ✅ Audit Logger
- ✅ WASM runtime
- ❌ Без browser scraping
- ❌ Без RAG/fastembed

#### 2. Полная сборка (40-60 минут)
```bash
cargo build --release
```

**Что включено:**
- ✅ Всё из fast + headless_chrome + fastembed

---

## 🚀 Команды

### Быстрая сборка (рекомендуется для тестирования)
```bash
cargo build --release --features fast
./target/release/ant
```

### Полная сборка (для продакшена)
```bash
cargo build --release
./target/release/ant
```

### Запуск с функционалом
```bash
# Быстрая версия
cargo run --release --features fast

# Полная версия
cargo run --release
```

---

## 📊 Сравнение

| Версия | Время | Размер | Функции |
|--------|-------|--------|---------|
| **fast** | ~5-10 мин | ~8 MB | Базовые |
| **full** | ~40-60 мин | ~20 MB | Все |

---

## ✅ Текущий статус

**Сборка:** ⏳ В процессе

**Прогресс:**
- Компилируются зависимости
- Ожидается: ~5-10 минут

---

## 🎯 После завершения

### Быстрая версия
```bash
# Запуск TUI
./target/release/ant

# Тесты
cargo test --features fast
```

### Полная версия
```bash
# Запуск со всеми функциями
./target/release/ant

# Тесты
cargo test
```

---

## 🔧 Если нужна полная версия

После быстрой сборки можно добрать зависимости:

```bash
# Дособрать headless_chrome и fastembed
cargo build --release
```

---

**🦀 Экономия времени: 80-85%!**
