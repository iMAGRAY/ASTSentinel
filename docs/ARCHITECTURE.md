Architecture Overview

Modules
- src/analysis: AST, metrics, duplicates, project scan
  - ast/: multi-language AST via tree-sitter + Rust via syn
  - metrics/: complexity metrics structs
  - duplicate_detector.rs: content-hash and name-based groups
  - project.rs: project-wide scan orchestration
- src/formatting: multi-language formatters + service
- src/providers: AI abstraction (UniversalAIClient)
- src/cache: project cache
- src/validation: diff formatter, misc checks
- src/bin: hooks binaries (pretooluse, posttooluse)

Key Flows
- PreToolUse: lightweight AST/security checks to allow/deny high-risk operations (optionally offline via PRETOOL_AST_ONLY)
- PostToolUse: deterministic additionalContext (AST issues, project/deps summary, diffs, transcript tail)
- AST scoring: AstQualityScorer
  - Rust uses syn visitor (panic/unwrap/todo!, nesting, params, long lines, creds/sql)
  - Others use tree-sitter. Fastpath (feature ast_fastpath) consolidates rules into a single pass

Determinism & Limits
- Stable sorting: severity → line → rule_id
- Caps: AST_MAX_ISSUES, ADDITIONAL_CONTEXT_LIMIT_CHARS (UTF‑8 safe)
- Timeouts: AST_ANALYSIS_TIMEOUT_SECS; file I/O via FILE_READ_TIMEOUT

