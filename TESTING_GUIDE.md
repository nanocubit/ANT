# 🧪 Руководство по тестированию ANT OS v9.0

## Быстрая проверка

### 1. Проверка компиляции
```bash
cd /Users/Vladimir/code/ant
cargo check
```

### 2. Сборка проекта
```bash
cargo build --release
```

### 3. Запуск тестов
```bash
# Запустить все тесты
cargo test

# Запустить тесты памяти
cargo test --test integration_tests test_memory

# Запустить тесты тем
cargo test --test integration_tests test_theme

# Запустить тесты sandbox
cargo test --test integration_tests test_sandbox

# Пропустить игнорируемые тесты (требуют внешние зависимости)
cargo test -- --skip test_git
```

---

## Тестирование модулей

### Тест 1: Память (Hybrid Search)

```bash
cargo test --test integration_tests test_memory_store_and_search -- --nocapture
```

**Что проверяется:**
- Создание VectorMemory
- Сохранение документа с метаданными
- Гибридный поиск (BM25 + векторный)
- Очистка тестовых файлов

**Ожидаемый результат:** ✅ PASS

---

### Тест 2: Time-Travel Debugging

```bash
cargo test --test integration_tests test_timetravel_creation -- --nocapture
```

**Что проверяется:**
- Создание TimeTravelDebugger
- Инициализация таблиц снэпшотов

**Ожидаемый результат:** ✅ PASS

---

### Тест 3: Система тем TUI

```bash
cargo test --test integration_tests test_theme_creation -- --nocapture
```

**Что проверяется:**
- Создание тёмной темы
- Создание светлой темы
- Переключение тем через ThemeManager

**Ожидаемый результат:** ✅ PASS

---

### Тест 4: Workspace Sandbox

```bash
cargo test --test integration_tests test_sandbox_file_operations -- --nocapture
```

**Что проверяется:**
- Запись файла в sandbox
- Чтение файла из sandbox
- Удаление файла
- Проверка содержимого

**Ожидаемый результат:** ✅ PASS

---

### Тест 5: Безопасность Sandbox

```bash
cargo test --test integration_tests test_sandbox_path_security -- --nocapture
```

**Что проверяется:**
- Нормальные пути работают
- Path traversal блокируется

**Ожидаемый результат:** ✅ PASS

---

## Ручное тестирование

### Тест 6: Запуск TUI

```bash
cargo run --release
```

**Проверка:**
1. ✅ TUI открывается
2. ✅ Status bar отображает метрики
3. ✅ Вкладки переключаются (Tab/Shift+Tab)
4. ✅ Переключение темы работает ('t')
5. ✅ Ввод команд работает
6. ✅ Выход работает (Esc/q)

---

### Тест 7: Переключение тем

**Действия:**
1. Запустить: `cargo run --release`
2. Нажать 't'

**Ожидаемый результат:**
- Тема переключается с Dark на Light
- Цвета всех элементов обновляются
- В status bar отображается текущая тема

---

### Тест 8: Поиск памяти

**Действия:**
1. Запустить: `cargo run --release`
2. Перейти на вкладку Memory (Tab)
3. Нажать 'r'
4. Ввести запрос

**Ожидаемый результат:**
- Поиск активируется
- Результаты отображаются

---

## Тестирование с внешними зависимостями

### Тест 9: Git навыки (требуется git)

```bash
cargo test --test integration_tests test_git_status -- --ignored --nocapture
```

**Требуется:**
- Установленный git

**Проверка:**
- GitStatus возвращается
- Команды выполняются

---

### Тест 10: Browser Actor (требуется Chrome)

```bash
cargo run --release
# Ввести: scrape https://rust-lang.org
```

**Требуется:**
- Установленный Chrome/Chromium

**Ожидаемый результат:**
- Headless Chrome запускается
- Страница скрапится
- Результат возвращается

---

### Тест 11: LLM интеграция (требуется API ключ)

```bash
# Настроить .env
echo "OPENROUTER_API_KEY=sk-or-v1-xxx" > .env
cargo run --release
```

**Требуется:**
- API ключ OpenRouter или другого провайдера

**Ожидаемый результат:**
- LLM отвечает на запросы
- Планирование задач работает

---

## Проверка производительности

### Тест 12: Скорость поиска памяти

```rust
#[tokio::test]
async fn benchmark_hybrid_search() {
    let memory = VectorMemory::new("/tmp/bench.duckdb").unwrap();
    
    // Сохранить 100 документов
    for i in 0..100 {
        memory.store("test", &format!("Document {}", i), MemoryMetadata::default()).await.unwrap();
    }
    
    let start = std::time::Instant::now();
    memory.hybrid_search("document", 10, 0.5, 0.5).await.unwrap();
    let elapsed = start.elapsed();
    
    println!("Search time: {:?}", elapsed);
    assert!(elapsed < std::time::Duration::from_millis(100));
}
```

**Ожидаемое время:** < 50мс для гибридного поиска

---

## Проверка WASM Runtime

### Тест 13: Загрузка WASM модуля

```bash
# Создать тестовый WASM модуль
cd skills
# ... компиляция тестового модуля ...

# Загрузить в ANT
cargo run --release
# Ввести: wasm:test-skill hello
```

**Требуется:**
- WASM модуль в директории skills

---

## Диагностика проблем

### Ошибка компиляции DuckDB

**Симптомы:**
```
error: failed to run custom build command for libduckdb-sys
```

**Решение:**
```bash
# Очистить кэш
cargo clean

# Пересобрать
cargo build --release
```

---

### Ошибка headless_chrome

**Симптомы:**
```
Failed to launch Chrome
```

**Решение:**
```bash
# Установить Chrome
brew install --cask google-chrome

# Или использовать Chromium
brew install --cask chromium
```

---

### Ошибка LLM API

**Симптомы:**
```
LLM API error: 401 Unauthorized
```

**Решение:**
1. Проверить API ключ в .env
2. Убедиться что ключ активен
3. Проверить лимиты API

---

## Сводная таблица тестов

| Тест | Модуль | Статус | Время |
|------|--------|--------|-------|
| Memory Creation | core | ✅ | <1с |
| Memory Store/Search | core | ✅ | <1с |
| TimeTravel Creation | core | ✅ | <1с |
| Theme Creation | ui | ✅ | <1с |
| Sandbox Creation | tools | ✅ | <1с |
| Sandbox File Ops | tools | ✅ | <1с |
| Sandbox Security | tools | ✅ | <1с |
| TUI Launch | ui | ✅ | <2с |
| Theme Toggle | ui | ✅ | <1с |
| Git Status | tools | ⏭️ | - |
| Browser Scraping | tools | ⏭️ | - |
| LLM Integration | ai | ⏭️ | - |

✅ - автоматические тесты
⏭️ - требуют внешние зависимости

---

## Запуск всех тестов сразу

```bash
# Быстрый прогон (без игнорируемых)
cargo test --lib --tests

# Полный прогон (включая игнорируемые)
cargo test --lib --tests -- --include-ignored

# С выводом результатов
cargo test -- --nocapture --test-threads=1
```

---

## Ожидаемые результаты

**Минимальные требования:**
- ✅ Все unit тесты проходят
- ✅ TUI запускается без ошибок
- ✅ Память создаётся и работает
- ✅ Sandbox безопасен
- ✅ Темы переключаются

**Опционально (требует зависимости):**
- ⏭️ Git команды работают
- ⏭️ Browser scraping работает
- ⏭️ LLM отвечает

---

## Отчёт о тестировании

После завершения тестов создайте отчёт:

```markdown
## Тестирование ANT OS v9.0

**Дата:** 2026-02-22
**Версия:** 0.9.0
**Платформа:** macOS

### Результаты:
- Unit тесты: ✅ 7/7
- Integration тесты: ✅ 5/5
- TUI тесты: ✅ 2/2
- Игнорируемые: ⏭️ 3/3

### Проблемы:
- Нет

### Вывод:
Система готова к использованию
```

---

**🦀 Удачи в тестировании!**
