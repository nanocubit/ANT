# 🛠️ Исправление ошибок компиляции

## Найденные ошибки

1. **sysinfo API changes** - `global_cpu_usage()` → `cpus()[0].cpu_usage()`
2. **ratatui Frame API** - `f.area()` → `f.size()`
3. **Sparkline data** - нужен slice вместо iterator
4. **Cursor position** - старый API

## Решение

Нужно либо:
1. ✅ Использовать старые версии (ratatui 0.26, sysinfo 0.30)
2. ⏳ Исправить код под новые версии

## Быстрое решение

Вернуть Cargo.toml к рабочим версиям:

```toml
ratatui = "0.26"
crossterm = "0.27"
sysinfo = "0.30"
```

И использовать полную версию main.rs вместо main_fast.rs.
