# TODO Roadmap (детерминированный план)

Легенда: [+] — готово, [-] — не готово. Каждый пункт имеет чёткий критерий успеха.

ID A — Конфигурация и Политики
- [+] A1. Конфиг‑загрузчик: env + .hooks-config.json (sensitivity, environment, ignore_globs, allowlist_vars)
  Критерий: unit‑тесты загрузки; корректное слияние источников; README c примерами.
- [+] A2. Предикаты: is_test_context/should_ignore_path/code_contains_allowlisted_vars
  Критерий: unit‑тесты на пути (tests/, __tests__, *_test.rs, fixtures, snapshots) и по содержимому кода.

ID B — PreToolUse: анти‑обман и здравый смысл (Diff‑aware)
- [+] B1. «Пустышки»: изменения только пробелы/комментарии/без логики → soft‑deny/ask‑explain
  Критерий: e2e deny/ask‑explain для пустых изменений, allow для валидных.
- [+] B2. «Иллюзии реализации»: return констант, print/log вместо логики, заглушки NotImplemented → deny
  Критерий: e2e — deny в prod, soft‑allow в test (смягчение по конфигу).
- [+] B3. «Глушение исключений»: пустые catch/except без действий — блокировка
  Критерий: e2e — deny при замене реал. логики на глушение.
- [+] B4. Contract‑check (упрощённо): упрощение сигнатур/игнор параметров
  Критерий: unit+e2e — ловим упрощение, не блокируем корректные рефакторы.

ID C — PreToolUse: безопасность без «перебора»
- [+] C1. Смягчение тест‑контекста: allowlist_vars (default_, demo_, sample_, mock_, test_, dummy_)
  Критерий: e2e — allow для creds/SQL в тестах с allowlisted; deny в prod.
- [+] C2. AST_IGNORE_GLOBS: игнор снапшотов/фикстур
  Критерий: e2e — изменения в игнорируемых файлах не блокируются.

ID D — PostToolUse: AST Аналитика 2.0 (diff‑aware, структурировано)
- [+] D1. Секции additionalContext: Change Summary, Risk Report (Critical/Major), Code Health, API Contract, Next Steps (готово в AST‑only и обычном потоке)
  Критерий: golden‑snapshot (структура/порядок); cap по символам.
- [+] D2. Diff‑aware AST Slice: только затронутые функции/файлы + микроконтекст
  Критерий: совпадение с diff; нет постороннего шума. (БАЗОВЫЕ сниппеты и фильтр по изменённым строкам — ГОТОВО; сущностные срезы — реализованы для Python/JS/TS)
- [+] D3. Cap‑логика: Critical — все, Major top‑N, Minor top‑K (env) + общий cap; сортировка severity→line→rule_id
  Критерий: unit‑тест детерминизма/капов; e2e — context ≤ лимита (ГОТОВО).

ID E — UserPromptSubmit: компактный AST‑контекст
- [+] E1. Секции «Project Summary» + «Risk/Health snapshot»
  Критерий: snapshot‑тест формата; размер ≤ лимита.

ID F — Языковые правила (точность)
- [-] F1. Rust: углубить DeepNesting (if/while let/loop), Unreachable внутри вложенных блоков
  Критерий: unit‑тесты на конструкции; без ложнопозитивов на good code.
- [-] F2. TS/JS: decorators, optional chaining, сложные комбинации params (readonly/default/rest/optional/деструктуризация)
  Критерий: unit‑тесты good/bad; корректная TooManyParameters и отсутствие шума.

  Прогресс: в контексте API‑контракта (PostToolUse) улучшен парсер сигнатур JS/TS:
  - шортхэнд‑методы объектных литералов (`foo(){}`), поля‑функции классов, arrow/function‑выражения, методы классов — распознаются;
  - computed‑имена уточняются (`[computed: …]` при возможности);
  - параметры извлекаются с учётом `decorator`/`readonly`/`optional`/`rest`/`default` и деструктурирования; типы/модификаторы игнорируются.
  Правила качества (TooManyParameters и др.) пока не менялись.
- [-] F3. Python: raise/continue/break → Unreachable; сложные try/except цепочки
  Критерий: unit‑тесты good/bad; без ложнопозитивов.

ID G — Многословность и полезность сообщений
- [-] G1. Глоссарий кратких сообщений per category + короткие «как исправить»
  Критерий: snapshot‑тесты на длину и отсутствие повторов.

ID H — Детерминизм и капы (БАЗА)
- [+] H1. Сортировка issues, utf‑8 safe truncate, капы, стабильность порядка
  Критерий: unit‑тесты детерминизма (ГОТОВО).

ID I — Наблюдаемость и перф
- [-] I1. Тайминги per file (AST_TIMINGS), краткий отчёт
  Критерий: вывод включается по env, не шумит по умолчанию.
- [-] I2. Soft time budget per file, graceful skip
  Критерий: тяжёлые файлы/патологии не блокируют общий процесс.
- [-] I3. Perf‑гейт без регрессий (>20%)
  Критерий: benches+perf_gate проходят.

ID J — Тестовая матрица и покрытие
- [-] J1. Покрытие ≥85% по критическим модулям (AST/PreTool/PostTool/diff/format/path)
  Критерий: tarpaulin ≥85% (Linux), артефакт coverage.
- [-] J2. E2E Windows/Unix: PreToolUse (deny/allow), PostToolUse (diff‑caps/sections), UserPromptSubmit snapshot
  Критерий: CI матрица зелёная; Windows‑e2e стабильны.

ID K — Документация и примеры
- [-] K1. README_HOOKS: «Политики и конфиг», примеры deny/allow, настройка многословности
  Критерий: примеры .hooks-config.json; разделы по каждому флагу.
- [-] K2. docs/: Playbook PreToolUse/PostToolUse (решения/структуры/лимиты)
  Критерий: пошаговые примеры «до/после», без излишней «воды».

ID Z — База (готово)
- [+] Z1. Windows path validation: backslash/UNC корректны, e2e Windows — ОК
- [+] Z2. JS/TS security: SQL‑строки (Critical), creds в assignment — ОК
- [+] Z3. Rust macros: panic!/todo!/unimplemented! (expr+stmt) — ОК
- [+] Z4. Legacy LongLineRule + лимиты + сортировка — ОК
- [+] Z5. Cargo профили: уменьшение бинарников (lto=fat, strip, abort, opt=z) — ОК

# Итерации (порядок выполнения)
- Итерация 1: A1–A2, C1–C2, J2 — конфиг/смягчение тест‑контекста/игноры, e2e платформы
- Итерация 2: B1–B4 — анти‑обман и diff‑эвристики
- Итерация 3: D1–D3 — структурированный PostToolUse (AST 2.0)
- Итерация 4: F1–F3 — углубление правил языков
- Итерация 5: G1 — краткость и полезность сообщений
- Итерация 6: I1–I3 — наблюдаемость и перф‑гейт
- Итерация 7: J1, K1–K2 — покрытие 85%+, docs/примеры

## Журнал выполнения
- 2025-09-10: F1 — углублены Rust-правила (DeepNesting/Unreachable). Юнит-тесты добавлены; все тесты: PASS.
  - Новое: unreachable после break/continue в циклах; deep nesting учитывает while let/loop.

- 2025-09-10: Проверен интернет‑доступ и базовый веб‑поиск.
  - HEAD https://example.com → 200 OK.
  - GitHub API: поиск репозиториев по запросу "ValidationCodeHook" → 0 результатов.
  - DuckDuckGo Instant Answer: запрос "OpenAI" → получено описание и ссылка (сеть/JSON работают).
  - Статусы Roadmap без изменений.
- 2025-09-10: Повторная проверка интернет‑доступа (CLI + web.run).
  - curl -I https://example.com → 200 OK (Alt-Svc: h3; Date: Wed, 10 Sep 2025 13:11:16 GMT).
  - curl -I https://www.wikipedia.org → 200 OK (cache-control, HSTS включены).
  - curl https://api.ipify.org?format=json → {"ip":"5.44.47.2"} (внешний IP получен).
  - web.run: найден и открыт RFC 9114 (HTTP/3) на rfc-editor.org и datatracker.ietf.org; ссылки доступны.
  - Вывод: Интернет‑доступ рабочий; DNS+TLS+HTTP/2/3 заголовки корректно получены.
  - Добавлен e2e‑скрипт `scripts/net_check.ps1` и тест `tests/e2e/test_net_check.ps1`; локальный прогон: PASS.
  - Реализованы Contract‑check unit+e2e: `tests/e2e_pretooluse_contract.rs`; deny при уменьшении арности (Python), allow при сохранении сигнатуры (JS). Запуск: `cargo test --bin pretooluse`.
  - D2: добавлены unit‑тесты сущностных срезов и фильтра по диффу в `src/bin/posttooluse.rs` (unit_*), а также e2e `tests/e2e_posttooluse_entity_snippets.rs`. Документирован `AST_ENTITY_SNIPPETS`. Все тесты: PASS.
  - E1: реализован компактный контекст для UserPromptSubmit (Project Summary + Risk/Health snapshot) с лимитом (`USERPROMPT_CONTEXT_LIMIT`, по умолчанию 4000). Добавлен юнит‑тест `tests/unit_userpromptsubmit_snapshot.rs`. Все тесты: PASS.


