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
- `AST_MAX_ISSUES` (default: `100`, range `10..500`) — cap number of AST issues included in context (deterministic top‑K by severity → line → rule_id).
- `ADDITIONAL_CONTEXT_LIMIT_CHARS` (default: `100000`, range `10000..1000000`) — max size of additional context; safely truncated on UTF‑8 boundaries.
- `AST_ANALYSIS_TIMEOUT_SECS` (default: `8`, range `1..30`) — per‑file AST analysis timeout to avoid stalls on pathological inputs.
- `AST_TIMINGS` (set to any value to enable) — include a brief per‑file timings summary (p50/p95/p99/mean) at the end of project‑wide AST analysis.

### PostToolUse Additional Context
- Hook attaches deterministic AST insights to `additionalContext`:
  - File-level: sorted concrete issues (Critical → Major → Minor), capped by `AST_MAX_ISSUES`.
  - Project-level: summary block "PROJECT-WIDE AST ANALYSIS" with counts and top criticals.
- Prompt sent to the AI provider also includes:
  - Project structure + metrics (with cache), dependencies overview, duplicate files report.
  - Unified diff of the change and a short transcript summary (last messages).
- All strings are UTF‑8 safe truncated by `ADDITIONAL_CONTEXT_LIMIT_CHARS` to bound payload size.
  - Note: `AST_MAX_ISSUES` имеет нижнюю границу 10 (clamp): значения <10 будут интерпретированы как 10.

### Non‑modifying tools
- Инструменты, которые не изменяют код (`ReadFile`, `Search`, etc.), проходят транзитом: `additionalContext` будет пустым.

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


