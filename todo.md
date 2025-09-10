# TODO (в работе на сегодня)

- [x] Починить компиляцию AST visitor (утечка `_e` → `e` в debug-логах).
- [x] Добавить интеграционный тест проектного AST-анализа в posttooluse (детерминизм + критические находки).
- [x] Актуализировать README_HOOKS по дополнительному контексту PostToolUse (AST + границы).
- [x] Расширить кросс-языковые фикстуры: добавлены fastpath-тесты для JS/TS/Java/C#/Go (unreachable, creds/sql) + «good code» для JS/Java.
  - [x] Добавить «good code» для TypeScript/C#/Go и дополнительные негативные кейсы (unreachable/creds/sql).
  - [x] Углубить TS/C#/Go негативные кейсы (сложные сигнатуры, try/catch/switch, async) и расширенные good/bad:
    - TS: async + try/catch + switch (good), 6 optional typed params (bad: TooManyParameters)
    - C#: async Task + switch (good), generic async с 6 параметрами (bad: TooManyParameters)
    - Go: глубокая вложенность с switch (bad). Примечание: «good switch + return» избегаем из-за fastpath-эвристики unreachable после return; оставлен отдельный «good code» без switch
    - C++: switch (good), try/catch (good)
    - Rust: сложная сигнатура с lifetime/generics/const (bad: TooManyParameters), async + match (good)
  - [x] Исправить ложнопозитив Unreachable в Go fastpath: игнорировать case/default/label/empty_statement и соседние узлы на той же строке после return
  - [x] Rust: включить syn‑анализ в AstQualityScorer (unwrap/panic, unreachable, params, nesting, creds/sql, long lines) + тесты.
- [x] E2E: интеграционный тест PostToolUse (AST-only режим без сети).
- [x] Причесать doctest’ы форматтеров или изолировать их от CI (помечены как no_run; устойчивы без внешних бинарей).
- [x] Полный прогон тестов (unit+e2e+doctest) и сборка release бинарников (pretooluse, posttooluse).
- [x] Упаковать артефакты в dist/ (linux/windows) и сгенерировать SHA256SUMS + RELEASE_MANIFEST.
 - [x] Legacy parity: добавить LongLineRule в multi-pass (no-default-features); e2e cap тест проходит.
 - [x] Ограничить интеграционные тесты fastpath-специфичных правил `cfg(feature=ast_fastpath)`.
- [x] Пройти clippy во всей кодовой базе (0 предупреждений) без изменения поведения.
- [ ] Прогнать perf-бенчмарки (criterion) и обновить baseline — следующий шаг.
 - [x] Прогнать perf-бенчмарки (criterion) и сохранить baseline в reports/benchmarks/baseline; perf_gate без регрессий.

Примечание: все изменения направлены на завершение AST‑системы передачи контекста в PostToolUse для Claude Code (детерминизм, лимиты, ясность).
