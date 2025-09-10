# Testing & Coverage

## Running Tests
- Default (fastpath): `cargo test`
- Legacy multi‑pass: `cargo test --no-default-features`

## E2E
- Hooks e2e находятся в `tests/` и не требуют сети в режимах:
  - `POSTTOOL_AST_ONLY=1` (структурный контекст без сетевых вызовов)
  - `POSTTOOL_DRY_RUN=1` (построение промпта/контекста без сетевых вызовов)

## Unit Highlights
- AST single‑pass/legacy, безопасная обработка путей, парсеры deps (npm/pip/cargo/poetry), DuplicateDetector (caps, порядок, сводки)

## Coverage
- Tarpaulin (Linux): `cargo tarpaulin --features ast_fastpath --timeout 120 --out Html`
- Цель: покрытие критичных путей AST/validation/duplicates/deps; остальной клей покрывается e2e.
