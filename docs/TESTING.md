Testing & Coverage

Running Tests
- Default (fastpath enabled): `cargo test --features ast_fastpath`
- Legacy multi-pass: `cargo test --no-default-features`

E2E
- PostToolUse and PreToolUse have e2e tests in `tests/` that run without network when `POSTTOOL_AST_ONLY=1` or using dry-run.

Unit Tests Highlights
- Single-pass engine: module tests in `src/analysis/ast/single_pass.rs` cover Go switch-unreachable fix and TS TooManyParameters.
- Duplicate detector: `tests/test_duplicate_detector.rs` verifies ExactDuplicate and VersionConflict detection and report formatting.
- Path handling (Windows-friendly): `src/bin/posttooluse.rs` contains unit tests for alias normalization, gitignore matching with `\` → `/`, and safe path validation.

Coverage
- Recommended: tarpaulin or grcov.
  - Example (tarpaulin): `cargo tarpaulin --features ast_fastpath --timeout 120 --out Html`
- Goal: cover critical paths 100% (AST analysis, path validation, diff formatter, duplicate detection, dependency parsers). Remaining CLI glue code measured via e2e tests.

