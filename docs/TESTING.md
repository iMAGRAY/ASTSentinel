<p align="center">
  <img src="../assets/hero.svg" width="100%" alt="AST Sentinel — Deterministic AST Hooks"/>
</p>

# Testing & Coverage

## Running Tests
- Default (fast path): `cargo test`
- Legacy multi‑pass: `cargo test --no-default-features`

## E2E
- Hook e2e tests live in `tests/` and can run fully offline via:
  - `POSTTOOL_AST_ONLY=1` — structured AST context only, no network
  - `POSTTOOL_DRY_RUN=1` — build prompt/context, do not call providers

## Unit Highlights
- AST single‑pass vs legacy parity
- Safe path handling across platforms
- Dependency parsers (npm/pip/cargo/poetry)
- DuplicateDetector (deterministic ordering, caps, per‑type summary)

## Coverage
- Tarpaulin (Linux): `cargo tarpaulin --features ast_fastpath --timeout 120 --out Html`
- Goal: cover critical AST/validation/duplicates/deps paths; glue covered by e2e.
