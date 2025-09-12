# Changelog

All notable changes to this project will be documented in this file.

## v0.1.0 — Initial public release

- Deterministic validation hooks for Claude Code (Rust):
  - PreToolUse: anti‑cheat (fake implementations), API‑contract protection, structural integrity checks, minimal critical‑risk gate.
  - PostToolUse: unified diff + structured AST context, entity‑aware snippets, Quick Tips, and guaranteed offline fallback (non‑empty) при отсутствии ключей/ошибке провайдера.
  - UserPromptSubmit: компактный снимок проекта (структура, метрики, зависимости).
- Production hardening:
  - Dev/debug флаги (DRY_RUN/AST_ONLY/DEBUG_HOOKS/AST_TIMINGS/AST_*) отключены в релизе, работают только в debug/test.
  - `.env` рядом с бинарём — единственный источник конфигурации в проде; `.env.local` игнорируется.
  - Нельзя отключить секцию API CONTRACT в продакшне.
  - Жёсткие прод‑дефолты: таймауты I/O/AST, soft‑бюджеты и лимиты контекста.
- AST/парсеры: обновлено ядро `tree-sitter` до `0.25`, унифицированы вызовы грамматик, исправлена поддержка Gleam.
- Конфиг: file‑first (`.hooks-config.*`) с `${VAR}`‑подстановкой; `.env` — приоритетная.
- CI: fmt, clippy, tests, audit; perf‑gate; coverage‑badge; релизный workflow для Windows/Linux с SHA256SUMS.
- Артефакты релиза: бинарники (`pretooluse`, `posttooluse`, `userpromptsubmit`) + `prompts/`.
