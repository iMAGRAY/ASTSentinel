<p align="center">
  <img src="assets/hero.svg" width="100%" alt="AST Sentinel — Deterministic AST Hooks"/>
</p>

# AST Sentinel

Детерминированный набор AST‑проверок и хуков для Claude Code:
- PreToolUse — ранняя защита (безопасность и анти‑чит),
- PostToolUse — дифф, структурированный AST‑контекст и рекомендации,
- UserPromptSubmit — компактный снимок проекта (зависимости/дубликаты/здоровье).

Проект ориентирован на стабильные, воспроизводимые выводы, строгие лимиты производительности и отличную разработческую ergonomics (документация, тесты, релизы).

<p>
  <img src="assets/workflow.svg" alt="AST Sentinel — Workflow" width="100%"/>
</p>

## Возможности

- <img src="assets/icons/shield.svg" width="18" alt=""> Безопасность и анти‑чит: SQL/командные инъекции, подмена логики (стабы, иллюзии реализации), глушение исключений.
- <img src="assets/icons/diff.svg" width="18" alt=""> Разумный дифф и контекст: детерминированные секции (Change Summary, Risk Report, Change Context, Code Health, API Contract, Next Steps).
- <img src="assets/icons/tree.svg" width="18" alt=""> AST‑анализ по языкам: JS/TS, Python, Rust, и др.; проверка контрактов API по сигнатурам.
- <img src="assets/icons/graph.svg" width="18" alt=""> Наблюдаемость: тайминги p50/p95/p99/avg (`AST_TIMINGS`), мягкие бюджеты (`AST_SOFT_BUDGET_*`).
- <img src="assets/icons/package.svg" width="18" alt=""> Проектный контекст: зависимости (npm/pip/cargo/poetry), дубликаты/конфликты файлов (детерминированные отчёты с капами).
- <img src="assets/icons/cpu.svg" width="18" alt=""> Производительность и воркфлоу: оффлайн‑режимы AST_ONLY/DRY_RUN, строгий perf‑gate в CI.

<p>
  <img src="assets/architecture.svg" alt="AST Sentinel — Architecture" width="100%"/>
</p>

## Состав и бинари

- `src/bin/pretooluse.rs` — ранняя валидация изменений (безопасность, здравый смысл, contract‑check).
- `src/bin/posttooluse.rs` — сводка изменений, структурированный AST‑контекст, советы, тайминги/бюджеты.
- `src/bin/userpromptsubmit.rs` — снимок проекта (структура/риск/здоровье, deps/duplicates).

## Быстрый старт (Windows)

```
git clone https://github.com/your-org/ast-sentinel.git
cd ast-sentinel
cargo build --release

# Установите хуки (пример: каталог hooks в вашем окружении)
copy target\release\pretooluse.exe hooks\
copy target\release\posttooluse.exe hooks\
copy target\release\userpromptsubmit.exe hooks\

# (опционально) провайдеры для онлайн‑режима
copy hooks\.env.example hooks\.env
# заполните ключи моделей (OpenAI / Anthropic / xAI / Google)

# Оффлайн‑режим (пример запуска PostToolUse без сети)
set POSTTOOL_AST_ONLY=1
set POSTTOOL_DRY_RUN=1
```

## Быстрый старт (Linux/macOS)

```
git clone https://github.com/your-org/ast-sentinel.git
cd ast-sentinel
cargo build --release

cp target/release/pretooluse hooks/
cp target/release/posttooluse hooks/
cp target/release/userpromptsubmit hooks/

# Оффлайн‑режим
POSTTOOL_AST_ONLY=1 POSTTOOL_DRY_RUN=1 hooks/posttooluse
```

## Конфигурация (основные флаги)

```
# Лимиты и контекст
AST_MAX_ISSUES=100               # общий cap; сортировка: severity→line→rule_id
AST_MAX_MAJOR=60                 # cap для Major (по умолчанию = AST_MAX_ISSUES)
AST_MAX_MINOR=40                 # cap для Minor (по умолчанию = AST_MAX_ISSUES)
ADDITIONAL_CONTEXT_LIMIT_CHARS=100000  # безопасное обрезание по UTF‑8

# Наблюдаемость и производительность
AST_TIMINGS=1                    # печать p50/p95/p99/avg по файлам
AST_SOFT_BUDGET_BYTES=250000     # мягкий бюджет по размеру
AST_SOFT_BUDGET_LINES=8000       # мягкий бюджет по строкам
AST_ANALYSIS_TIMEOUT_SECS=8      # таймаут анализа файла (защита от пат. входов)

# Режимы PostToolUse
POSTTOOL_AST_ONLY=1              # строгий AST‑контекст, без сети
POSTTOOL_DRY_RUN=1               # сборка промпта/контекста без обращения к API
API_CONTRACT=1                   # включить вывод секции API CONTRACT

# Перф‑гейт
PERF_GATE_STRICT=1               # падать при регрессиях сверх порога в CI

# Контекст проекта
DUP_REPORT_MAX_GROUPS=20         # максимум групп в отчёте дубликатов
DUP_REPORT_MAX_FILES=10          # максимум файлов в группе
```

Подробности флагов и разделов см. в `README_HOOKS.md` и `docs/PLAYBOOK_AST_FLAGS.md`.

## Режимы работы

- AST_ONLY — выдаёт только структурированный AST‑контекст. Полезно для оффлайна и воспроизводимых тестов.
- DRY_RUN — формирует промпт/контекст, но не обращается к внешним API.
- Soft Budget — при превышении `AST_SOFT_BUDGET_*` добавляется единообразная «примечание‑скип», контекст остаётся детерминированным.
- Timings — при `AST_TIMINGS=1` в конце выводится краткая сводка p50/p95/p99/avg.

## CI/CD и релизы

- CI собирает и тестирует на Windows/Linux матрице, опционально включает perf‑gate (строгий режим).
- Теги `vX.Y.Z` запускают сборку бинарников для Windows/Linux и публикацию в Releases (с `SHA256SUMS`).

## Структура проекта

```
ast-sentinel/
├─ src/
│  ├─ bin/                 # pretooluse, posttooluse, userpromptsubmit
│  └─ analysis/            # ast/*, duplicates.rs, dependencies.rs, metrics.rs
├─ tests/                  # unit + e2e (goldens)
├─ docs/                   # ARCHITECTURE.md, PROJECT_STRUCTURE.md, TESTING.md, ...
├─ assets/                 # hero.svg, icons/*.svg, architecture.svg, workflow.svg
└─ .github/workflows/      # ci.yml, release.yml
```

## Тесты

```
cargo test --all --release

# Быстрые проверки отдельных областей
cargo test e2e_posttooluse -- --nocapture
cargo test unit_duplicate_detector -- --nocapture
```

## Руководства

- README_HOOKS.md — справочник флагов, секций и режимов.
- docs/PROJECT_STRUCTURE.md — ориентирование по коду и архитектуре.
- docs/TESTING.md — как запускать unit/e2e, оффлайн‑флаги.

## Лицензия

MIT. Вклады приветствуются через обычный GitHub workflow (PR + проверка CI).
