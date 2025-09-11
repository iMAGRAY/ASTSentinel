# AST Flags Playbook (Before/After)

This playbook shows practical, reproducible toggles and the resulting outputs of key AST features in hooks.

All commands are cross‑platform; on Windows use PowerShell with `setx` or per‑process env via `$env:`.

## 1) Soft Budget (skip heavy files)

- Purpose: Avoid expensive AST analysis on large files; leave a visible note.
- Flags:
  - `AST_SOFT_BUDGET_BYTES` (default 500000; clamp 1..5000000)
  - `AST_SOFT_BUDGET_LINES` (default 10000; clamp 1..200000)
- Affected: PostToolUse (AST_ONLY, DRY_RUN, online)

Before (no budgets): AST runs normally on large files.

After:
```
AST_SOFT_BUDGET_BYTES=10 POSTTOOL_AST_ONLY=1 posttooluse < hook.json
# additionalContext will contain:
[ANALYSIS] Skipped AST analysis due to soft budget (…)
```

## 2) Diff‑Aware Entity Snippets

- Purpose: Send compact code slices (function/method/class) around changed lines.
- Flags:
  - `AST_ENTITY_SNIPPETS=1` (default) — enable entity‑based context
  - `AST_DIFF_ONLY=1` — filter issues to changed lines ± context
  - `AST_DIFF_CONTEXT=3` — lines of context around changes
  - `AST_MAX_SNIPPETS=3`, `AST_SNIPPETS_MAX_CHARS=1500`
- Affected: PostToolUse (AST_ONLY, DRY_RUN, online)

Before: Either no snippets or flat range‑based snippets.
After: Section `=== CHANGE CONTEXT ===` contains entity‑scoped slices, one per changed entity.

## 3) QUICK TIPS

- Purpose: Add ultra‑compact, prescriptive hints derived from detected issues.
- Flags: `QUICK_TIPS=1` (default), `QUICK_TIPS_MAX`, `QUICK_TIPS_MAX_CHARS`
- Affected: PostToolUse (AST_ONLY, online)

Example:
```
POSTTOOL_AST_ONLY=1 QUICK_TIPS=1 posttooluse < hook.json
=== QUICK TIPS ===
- Collapse deep nesting with early returns.
- Limit parameters; consider object/kwargs.
```

## 4) AST Timings (Observability)

- Purpose: Measure and disclose parse/score timings in additionalContext for offline diagnosis.
- Flags: `AST_TIMINGS=1`
- Affected: PostToolUse (AST_ONLY)

Example tail of additionalContext:
```
=== TIMINGS (ms) ===
label            count  p50  p95  p99  avg
score/python     3      …    …    …    …
```

## Tips

- Use `ADDITIONAL_CONTEXT_LIMIT_CHARS` to cap payload size (UTF‑8 safe truncation).
- All sections are deterministic (ordering, sorting, caps) for stable snapshots.
- Windows: in PowerShell use `$env:POSTTOOL_AST_ONLY='1'` for per‑process flags.

## 5) Dependencies & Duplicates in Context

- UserPromptSubmit prints a compact summary with dependency counts:
```
$env:USERPROMPT_CONTEXT_LIMIT='4000'
userpromptsubmit < hook.json
# Look for: "Dependencies: total N, outdated M"
```
- PostToolUse includes dependency analysis and duplicate report in the AI prompt context (not in additionalContext) to keep it compact.
  - Duplicate report is marked as critical and suggests consolidation/deletion of backup/temp files.

Caps for duplicates report (to avoid bloat on large repos):
- `DUP_REPORT_MAX_GROUPS=10`
- `DUP_REPORT_MAX_FILES=5`
The report will append lines like:
```
  ... and N more files hidden by limit
... and M more groups hidden by limit
```
