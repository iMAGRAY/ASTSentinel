<p align="center">
  <img src="../assets/hero.svg" width="100%" alt="AST Sentinel — Deterministic AST Hooks"/>
</p>

# Project Structure

```
ast-sentinel/
├─ src/
│  ├─ analysis/
│  │  ├─ ast/                  # multi‑language AST (tree‑sitter); Rust via syn
│  │  ├─ duplicate_detector.rs  # content/name duplicates (caps, per‑type summary, top dirs)
│  │  ├─ dependencies.rs        # npm/pip/cargo/poetry parsers
│  │  ├─ project.rs             # scan & cache project structure/metrics
│  │  └─ metrics.rs             # complexity metrics
│  ├─ formatting/               # formatters per language + service
│  ├─ providers/                # AI client abstraction (optional online)
│  ├─ cache/                    # project cache
│  ├─ validation/               # diff formatter and helpers
│  └─ bin/                      # pretooluse, posttooluse, userpromptsubmit
├─ tests/                       # unit + e2e (single entry: `cargo test`)
├─ benches/                     # Criterion benchmarks
├─ scripts/                     # perf gate and utilities
├─ reports/benchmarks/baseline/ # Criterion baseline (many estimates.json by design)
├─ dist/                        # release artifacts (gitignored)
├─ hooks/                       # compiled hooks for local use (gitignored)
└─ docs/                        # architecture, testing, structure
```

## Notes
- Binaries are in `src/bin/` and built via `cargo build --release`.
- GitHub Actions build/test workflow lives under `.github/workflows/`.
- Release workflow attaches Windows/Linux archives and `SHA256SUMS` to tags `vX.Y.Z`.

## Testing & Coverage

Running Tests
- Default: `cargo test` (fastpath enabled)
- Legacy multi‑pass: `cargo test --no-default-features`

E2E
- PreToolUse/PostToolUse/UserPromptSubmit e2e live in `tests/`.
- Offline options: `POSTTOOL_AST_ONLY=1` and `POSTTOOL_DRY_RUN=1`.

Unit Highlights
- Single‑pass AST rules and parsers
- Duplicate detector: ordering, caps, summaries
- API contract: golden section ordering for AST_ONLY/DRY_RUN

Coverage
- Tarpaulin (Linux): `cargo tarpaulin --features ast_fastpath --timeout 120 --out Html`
