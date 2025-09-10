Project Structure

- src/
  - analysis/
    - ast/: multi-language AST analysis (tree-sitter) + Rust via syn
    - duplicate_detector.rs: detect content/name duplicates
    - project.rs: scan and cache project structure/metrics
    - metrics.rs: complexity metrics
  - formatting/: formatters per language + service
  - providers/: AI client abstraction
  - cache/: project cache
  - validation/: diff formatter and helpers
  - bin/: hook binaries (pretooluse, posttooluse)
- tests/: integration and e2e tests
- benches/: Criterion benchmarks
- scripts/: perf gate and utilities
- reports/benchmarks/baseline/: saved Criterion baseline (many estimates.json by design)
- dist/: release artifacts
- hooks/: compiled hooks for local use
- docs/: architecture, testing, structure

