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


