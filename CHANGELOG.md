# Changelog

All notable changes to this project will be documented in this file.

## v0.1.0 — Initial public release

- Deterministic validation hooks for Claude Code (Rust):
  - PreToolUse: anti-cheat (fake implementations), API contract protection, structural integrity checks, minimal critical risk gate.
  - PostToolUse: diff + structured AST context, offline fallback without API keys, Quick Tips, entity-aware snippets.
  - UserPromptSubmit: compact project snapshot (structure, metrics, dependencies).
- File-first configuration with `${VAR}` expansion; env/.env fallback preserved.
- Strong CI: fmt, clippy (-D warnings), tests, audit; perf gate; coverage badge.
- Windows-friendly assets: prompts/ included; install scripts for prompts and config.
- Release artifacts include Windows `.exe` and `prompts/` directory.

