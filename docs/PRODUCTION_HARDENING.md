# Production Hardening Summary

This project ships three hooks (`pretooluse`, `posttooluse`, `userpromptsubmit`). The binaries are production‑hardened. Dev/test flags are disabled in release builds. This document summarizes what is allowed to change behavior in production and what is gated to debug/test only.

## Allowed in Production

Use these environment variables or the `.env` file next to the binaries:

- OPENAI_API_KEY, ANTHROPIC_API_KEY, GOOGLE_API_KEY, XAI_API_KEY
- OPENAI_BASE_URL, ANTHROPIC_BASE_URL, GOOGLE_BASE_URL, XAI_BASE_URL
- PRETOOL_PROVIDER, POSTTOOL_PROVIDER (openai | anthropic | google | xai)
- PRETOOL_MODEL, POSTTOOL_MODEL (provider models)
- MAX_TOKENS, TEMPERATURE
- REQUEST_TIMEOUT_SECS, CONNECT_TIMEOUT_SECS
- SENSITIVITY (low | medium | high)
- ADDITIONAL_CONTEXT_LIMIT_CHARS (default 100000)
- LOG_JSON or HOOK_LOG_JSON (enable JSON logging via telemetry)

Notes:
- `.env` alongside the `.exe` is always the primary config source.
- `.env.local` is ignored in production (used only in debug/test).

## Debug/Test Only (ignored in production)

- POSTTOOL_DRY_RUN, PRETOOL_AST_ONLY, POSTTOOL_AST_ONLY
- DEBUG_HOOKS, AST_TIMINGS
- AST_DIFF_ONLY, AST_SNIPPETS, AST_MAX_SNIPPETS, AST_SNIPPETS_MAX_CHARS
- AST_SOFT_BUDGET_BYTES, AST_SOFT_BUDGET_LINES
- AST_ANALYSIS_TIMEOUT_SECS, FILE_READ_TIMEOUT
- AST_ENV, AST_ALLOWLIST_VARS, AST_IGNORE_GLOBS
- API_CONTRACT=0 (cannot disable contract report in production)

## Fixed Production Defaults

- AST diff context: 3 lines
- Snippet extraction: enabled with conservative caps
- File read timeout: 10s
- Per‑file AST analysis timeout: 8s
- Soft AST budgets: 500KB / 10,000 lines

## Release Build

- `cargo build --release` (LTO on, panic=abort, strip)
- Artifacts include binaries plus `prompts/` directory.
- Windows installers and helper scripts are in `scripts/`.

## Security Posture

- No dev dumps (post‑context, debug logs) are written in production.
- Path validation prevents traversal, UNC on non‑Windows, and invalid encodings.
- On provider failure or missing keys, `posttooluse` produces an offline AST‑based report (never empty output).

