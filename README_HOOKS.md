# Validation Hooks for Claude Code

## Overview
This project provides security and code quality validation hooks for Claude Code.

## Installation
1. Build the hooks: `cargo build --release`
2. Copy to hooks directory: Already done!
3. Configure Claude Code to use the hooks

## Configuration
Edit `.env` file to configure:
- API providers (OpenAI, Anthropic, Google, xAI)
- Models for validation
- Timeout settings
- Debug options

### AST Analysis Settings
- `AST_MAX_ISSUES` (default: `100`, range `10..500`) — общий cap числа AST‑issues в контексте (детерминированная сортировка: severity → line → rule_id).
- `AST_MAX_MAJOR` (optional) — cap для Major‑issues (по умолчанию берётся из `AST_MAX_ISSUES`).
- `AST_MAX_MINOR` (optional) — cap для Minor‑issues (по умолчанию берётся из `AST_MAX_ISSUES`).
- `ADDITIONAL_CONTEXT_LIMIT_CHARS` (default: `100000`, range `10000..1000000`) — max size of additional context; safely truncated on UTF‑8 boundaries.
- `AST_ANALYSIS_TIMEOUT_SECS` (default: `8`, range `1..30`) — per‑file AST analysis timeout to avoid stalls on pathological inputs.
- `AST_TIMINGS` (set to any value to enable) — include a brief per‑file timings summary (p50/p95/p99/mean) at the end of project‑wide AST analysis.
- `FILE_READ_TIMEOUT` (seconds, default: 10) — timeout for safe file reads inside hooks.

### PostToolUse: Structured Additional Context
- Формат (детерминированный порядок секций):
  - `=== CHANGE SUMMARY ===` — унифицированный дифф изменения.
  - `=== RISK REPORT ===` — Critical (все), Major (top‑N), Minor (top‑K) — капы управляются `AST_MAX_ISSUES`/`AST_MAX_MAJOR`/`AST_MAX_MINOR`.
  - `=== CHANGE CONTEXT ===` — сниппеты кода с нумерацией строк и маркером `>` на строке issue (включено по умолчанию).
  - `=== CODE HEALTH ===` — краткие метрики читаемости/сложности.
  - `=== NEXT STEPS ===` — приоритетные действия.
- Контекст, передаваемый в AI:
  - Структура проекта + метрики (с кэшем), зависимости, отчёт о дубликатах.
  - Унифицированный дифф изменения и краткая сводка транскрипта.
- Все строки безопасно обрезаются по UTF‑8 по `ADDITIONAL_CONTEXT_LIMIT_CHARS`.
  - Примечание: `AST_MAX_ISSUES` имеет нижнюю границу 10 (clamp): значения <10 интерпретируются как 10.

#### JS/TS: сигнатуры и имена сущностей
- Распознаются функции и методы в JS/TS, включая:
  - объявление функций, методы классов, стрелочные и `function`‑выражения, поля‑функции в классах;
  - методы в объектных литералах как в виде `foo: ()=>{}`/`foo: function(){}`, так и шортхэнд `foo(){}`.
- Вычисляемые имена (`[computed]`) теперь уточняются, если синтаксис прозрачен для Tree‑sitter (напр., `[Symbol.iterator]` → `[computed: Symbol.iterator]`, `['name']` → `[computed: name]`).
- Параметры в сигнатурах JS/TS извлекаются аккуратно:
  - игнорируются `decorator`/модификаторы (`public`/`private`/`protected`/`readonly` и т.п.), типы (`: T`, generics) и аннотации;
  - поддерживаются `optional` (`x?`), `rest` (`...args`), `default` (`x=1`) и сложные паттерны деструктурирования (`{a, b:c}`, `[x,y]`).

### Diff‑aware и сниппеты
- `AST_DIFF_ONLY=1` — включить фильтрацию issues по изменённым строкам (с окном контекста `AST_DIFF_CONTEXT`, default `3`).
- `AST_SNIPPETS` — управление секцией `CHANGE CONTEXT`:
  - По умолчанию включено во всех режимах; выключить: `AST_SNIPPETS=0`.
  - `AST_MAX_SNIPPETS` (default: `3`, range `1..50`) — максимум сниппетов.
  - `AST_SNIPPETS_MAX_CHARS` (default: `1500`, range `200..20000`) — предел символов секции.
  - `AST_ENTITY_SNIPPETS` (default: `1`) — включить сущностные срезы (функция/метод/класс) вместо «плоских» окон по строкам. Поддерживаются Python/JS/TS; при невозможности выделить сущность автоматически выполняется fallback к `CHANGE CONTEXT` на основе строк.

### Non‑modifying tools
- Инструменты, которые не изменяют код (`ReadFile`, `Search`, etc.), проходят транзитом: `additionalContext` будет пустым.

### PreToolUse: Контракт и чувствительность
- В офлайн‑режиме (`PRETOOL_AST_ONLY=1`):
  - Ослабление API‑контракта (уменьшение числа параметров/удаление/переименование) ⇒ `deny` с пояснением.
- В обычном режиме (с AI):
  - В промпт добавляется `HEURISTIC SUMMARY` при подозрении на ослабление контракта.
  - При `SENSITIVITY=high` — ранний `deny` при ослаблении контракта.
  - При `SENSITIVITY=medium` — ранний `deny` при сочетании ослабления контракта и security‑рисков (creds/SQL) в новом коде.

### Offline/E2E mode
- Set `POSTTOOL_AST_ONLY=1` to skip AI provider call and still return deterministic AST context in `additionalContext`.
  - Полезно для e2e‑тестов и оффлайн‑прогонов без сетевого доступа/ключей.
- Set `POSTTOOL_DRY_RUN=1` to build full prompt (project+diff+transcript+AST) and skip network call. Полезно для проверки состава промпта (см. post-context.txt при `DEBUG_HOOKS=true`).

### PreToolUse offline mode
- Set `PRETOOL_AST_ONLY=1` — решение allow/deny принимается локально на базе AST/правил безопасности без обращения к AI:
  - deny при критичных находках (напр., hardcoded credentials, SQL‑инъекции, path/command injection);
  - allow при отсутствии критичных находок.
  - Режим предназначен для e2e/оффлайн‑прогонов и не заменяет полноценную AI‑валидацию.

### Performance Feature Flags
- `ast_fastpath` — включен по умолчанию. Однопроходный AST‑движок (Python/JS/TS/Java/C#/Go) даёт прирост производительности и детерминизма.
  - Отключить: `cargo build --release --no-default-features`
  - Включить явно: `cargo build --release --features ast_fastpath`
  - Тесты: `cargo test` (по умолчанию с fastpath) или `cargo test --no-default-features`

### Perf Gate (бенчмарки Criterion)
- Сохранить эталон: `cargo bench --bench ast_perf` затем `python scripts/perf_gate_save.py --out reports/benchmarks/baseline`
- Проверить регрессии (>20% по среднему времени): `python scripts/perf_gate.py --baseline reports/benchmarks/baseline --threshold 0.2`
 - Получить сводку бейзлайна в один файл: `python scripts/perf_baseline_summary.py --baseline reports/benchmarks/baseline > reports/benchmarks/baseline_summary.json`

### Windows: пути и безопасность

Хуки валидируют пути, не блокируя валидные Windows‑пути:
- Поддерживаются backslash‑пути и UNC; проверка «UNC на non‑Windows» срабатывает только вне Windows.
- Больше нет blanket‑запрета `..`, `~`, `$` как подстрок — валидность определяет каноникализация и проверка, что путь остаётся в рамках рабочей директории/разрешённых директорий.
- Для `~` запрещён только опасный префикс `~/` (на non‑Windows).
- Gitignore‑паттерны сопоставляются кроссплатформенно: все разделители пути нормализуются к `/` перед сопоставлением.

Примеры:
- `C:\\proj\\src\\main.rs` — ок, если подпадает под рабочую директорию.
- `\\\\server\\share\\logs.txt` — ок на Windows; на non‑Windows отклоняется как UNC.
- `~/secrets.txt` — отклоняется на non‑Windows (только префикс), обычные имена с `~` внутри разрешены.

## Hook Binaries
- `pretooluse.exe` - Pre-execution validation
- `posttooluse.exe` - Post-execution validation

Both hooks are located in the `hooks/` directory and are ready to use.

## Release Artifacts
- Сборки и контрольные суммы доступны в `dist/`:
  - `dist/linux-x86_64/{pretooluse,posttooluse}` + `SHA256SUMS.txt`
  - `dist/windows-x86_64/{pretooluse.exe,posttooluse.exe}` + `SHA256SUMS.txt`
- Актуальный манифест с git‑ревизией: `dist/RELEASE_MANIFEST.txt`.

### Быстрый старт (Linux)
```
cp dist/linux-x86_64/posttooluse hooks/
cp dist/linux-x86_64/pretooluse hooks/
```

### Быстрый старт (Windows)
```
copy dist\windows-x86_64\posttooluse.exe hooks\
copy dist\windows-x86_64\pretooluse.exe hooks\
```

Для проверки целостности используйте `sha256sum -c SHA256SUMS.txt` внутри соответствующей папки.


