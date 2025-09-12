#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::dbg_macro
    )
)]
#![allow(clippy::items_after_test_module)]
use anyhow::{Context, Result};
use std::io::{self};

use rust_validation_hooks::truncate_utf8_safe;
use rust_validation_hooks::*;
// Use universal AI client for multi-provider support
use rust_validation_hooks::providers::ai::UniversalAIClient;
// Use project context for better AI understanding
use rust_validation_hooks::analysis::project::{
    format_project_structure_for_ai_with_metrics, scan_project_with_cache,
};
// Use dependency analysis for better project understanding
use rust_validation_hooks::analysis::dependencies::analyze_project_dependencies;
use std::path::PathBuf;
// Use diff formatter for better AI context - unified diff for clear change visibility
use rust_validation_hooks::validation::diff_formatter::{format_code_diff, format_multi_edit_full_context};
// Use AST-based quality scorer for deterministic code analysis
use rust_validation_hooks::analysis::ast::languages::LanguageCache;
use rust_validation_hooks::analysis::ast::{
    AstQualityScorer, IssueSeverity, QualityScore, SupportedLanguage,
};
// Use duplicate detector for finding conflicting files
use rust_validation_hooks::analysis::duplicate_detector::DuplicateDetector;
// Use code formatting service for automatic code formatting
use rust_validation_hooks::formatting::FormattingService;
use rust_validation_hooks::messages::glossary::build_quick_tips;
use rust_validation_hooks::providers::AIProvider;

#[inline]
fn dev_flag_enabled(name: &str) -> bool {
    #[cfg(debug_assertions)]
    {
        match std::env::var(name) {
            Ok(v) => v != "0" && !v.is_empty(),
            Err(_) => false,
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = name;
        false
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolKind {
    Write,
    Edit,
    MultiEdit,
    Other,
}

fn normalize_tool_name(name: &str) -> (ToolKind, bool /* is_append */) {
    let n = name.to_ascii_lowercase();
    let is_append = n.contains("append");
    if matches!(
        n.as_str(),
        "write" | "writefile" | "createfile" | "create" | "savefile" | "save" | "appendtofile" | "append"
    ) {
        return (ToolKind::Write, is_append);
    }
    if matches!(
        n.as_str(),
        "edit" | "replace" | "editfile" | "replaceinfile" | "modify"
    ) {
        return (ToolKind::Edit, false);
    }
    if matches!(
        n.as_str(),
        "multiedit" | "applyedits" | "multireplace" | "batchedit"
    ) {
        return (ToolKind::MultiEdit, false);
    }
    (ToolKind::Other, false)
}

// =============================
// Structured additionalContext builders
// =============================
async fn render_offline_posttooluse(
    hook_input: &rust_validation_hooks::HookInput,
    content: &str,
    display_path: &str,
    ast_score: &Option<QualityScore>,
    formatting_changed: bool,
) -> Result<()> {
    let mut final_response = String::new();
    let change = build_change_summary(hook_input, display_path).await;
    if !change.is_empty() {
        final_response.push_str(&change);
        final_response.push('\n');
    }
    if let Some(note) = soft_budget_note(content, display_path) {
        final_response.push_str(&note);
        final_response.push('\n');
    }
    if let Some(ast_score) = ast_score {
        let (filtered, change_snippets) = if let Ok(diff) = generate_diff_context(hook_input, display_path).await {
            let ctxn = if cfg!(debug_assertions) { std::env::var("AST_DIFF_CONTEXT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3) } else { 3 };
            let changed = extract_changed_lines(&diff, ctxn);
            let filtered = if dev_flag_enabled("AST_DIFF_ONLY") {
                filter_issues_to_diff(ast_score, &diff, ctxn)
            } else {
                ast_score.clone()
            };
            let snips_enabled = if cfg!(debug_assertions) { std::env::var("AST_SNIPPETS").map(|v| v != "0").unwrap_or(true) } else { true };
            let snips = if snips_enabled {
                let max_snips = if cfg!(debug_assertions) { std::env::var("AST_MAX_SNIPPETS").ok().and_then(|v| v.parse().ok()).unwrap_or(3) } else { 3 }.clamp(1, 50);
                let max_chars = if cfg!(debug_assertions) { std::env::var("AST_SNIPPETS_MAX_CHARS").ok().and_then(|v| v.parse().ok()).unwrap_or(1500) } else { 1500 }.clamp(200, 20_000);
                let lang = SupportedLanguage::from_extension(display_path.split('.').next_back().unwrap_or(""))
                    .unwrap_or(SupportedLanguage::Python);
                let use_entity = if cfg!(debug_assertions) { std::env::var("AST_ENTITY_SNIPPETS").map(|v| v != "0").unwrap_or(true) } else { true };
                if use_entity {
                    let ent = build_entity_context_snippets(
                        lang, content, &filtered, &changed, ctxn, max_snips, max_chars,
                    );
                    if !ent.is_empty() { ent } else { build_change_context_snippets(content, &filtered, &changed, ctxn, max_snips, max_chars) }
                } else {
                    build_change_context_snippets(content, &filtered, &changed, ctxn, max_snips, max_chars)
                }
            } else { String::new() };
            (filtered, snips)
        } else {
            (ast_score.clone(), String::new())
        };
        final_response.push_str(&build_risk_report(&filtered));
        final_response.push('\n');
        let unfinished = build_unfinished_work_section(&filtered, 6, 120);
        if !unfinished.is_empty() {
            final_response.push_str(&unfinished);
            final_response.push('\n');
        }
        let tips = build_quick_tips_section(&filtered);
        if !tips.is_empty() {
            final_response.push_str(&tips);
            final_response.push('\n');
        }
        if !change_snippets.is_empty() {
            final_response.push_str(&change_snippets);
            final_response.push('\n');
        }
        final_response.push_str(&build_code_health(&filtered));
        final_response.push('\n');
        let lang = SupportedLanguage::from_extension(display_path.split('.').next_back().unwrap_or(""))
            .unwrap_or(SupportedLanguage::Python);
        let api = build_api_contract_report(lang, hook_input, content);
        if !api.is_empty() {
            final_response.push_str(&api);
            final_response.push('\n');
        }
        final_response.push_str(&build_next_steps(&filtered));
        final_response.push('\n');
    }
    if formatting_changed {
        final_response.push_str("[FORMAT] Auto-format applied.\n\n");
    }
    // Always provide agent JSON for downstream agents (fallback from AST if needed)
    if let Some(ast) = ast_score {
        let agent = build_agent_json_from_score(ast);
        final_response.push_str("AGENT_JSON_START\n");
        final_response.push_str(&agent);
        final_response.push_str("\nAGENT_JSON_END\n");
    }
    if crate::analysis::timings::enabled() {
        let sum = crate::analysis::timings::summary();
        if !sum.is_empty() {
            final_response.push_str(&sum);
            final_response.push('\n');
        }
    }
    let output = PostToolUseOutput {
        hook_specific_output: PostToolUseHookOutput {
            hook_event_name: "PostToolUse".to_string(),
            additional_context: {
                let lim = if cfg!(debug_assertions) { std::env::var("ADDITIONAL_CONTEXT_LIMIT_CHARS").ok().and_then(|v| v.parse().ok()).unwrap_or(100_000) } else { 100_000 }.clamp(10_000, 1_000_000);
                truncate_utf8_safe(&final_response, lim)
            },
        },
    };
    println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
    Ok(())
}
fn build_risk_report(score: &QualityScore) -> String {
    use crate::analysis::ast::quality_scorer::{ConcreteIssue, IssueSeverity};
    // Deterministic order base
    let sev_key = |s: IssueSeverity| match s {
        IssueSeverity::Critical => 0,
        IssueSeverity::Major => 1,
        IssueSeverity::Minor => 2,
    };

    // Read caps
    let global_cap: usize = if cfg!(debug_assertions) { std::env::var("AST_MAX_ISSUES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100) } else { 100 }
        .clamp(10, 500);
    let cap_major: usize = if cfg!(debug_assertions) { std::env::var("AST_MAX_MAJOR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(global_cap) } else { global_cap }
        .clamp(5, 500);
    let cap_minor: usize = if cfg!(debug_assertions) { std::env::var("AST_MAX_MINOR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(global_cap) } else { global_cap }
        .clamp(5, 500);

    // Group by severity
    let mut crit: Vec<_> = score
        .concrete_issues
        .iter()
        .filter(|i| matches!(i.severity, IssueSeverity::Critical))
        .cloned()
        .collect();
    let mut major: Vec<_> = score
        .concrete_issues
        .iter()
        .filter(|i| matches!(i.severity, IssueSeverity::Major))
        .cloned()
        .collect();
    let mut minor: Vec<_> = score
        .concrete_issues
        .iter()
        .filter(|i| matches!(i.severity, IssueSeverity::Minor))
        .cloned()
        .collect();

    // Sort deterministically inside groups
    let cmp = |a: &ConcreteIssue, b: &ConcreteIssue| {
        sev_key(a.severity)
            .cmp(&sev_key(b.severity))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    };
    crit.sort_by(cmp);
    major.sort_by(cmp);
    minor.sort_by(cmp);

    // Apply per-severity caps (Critical = all)
    if major.len() > cap_major {
        major.truncate(cap_major);
    }
    if minor.len() > cap_minor {
        minor.truncate(cap_minor);
    }

    // Combine and globally cap for determinism + compactness
    let total_all = score.concrete_issues.len();
    let mut combined = Vec::with_capacity(crit.len() + major.len() + minor.len());
    combined.extend(crit);
    combined.extend(major);
    combined.extend(minor);
    combined.sort_by(cmp);
    if combined.len() > global_cap {
        combined.truncate(global_cap);
    }

    // Build output
    let mut s = String::new();
    s.push_str("=== RISK REPORT ===\n");
    if combined.is_empty() {
        s.push_str("No issues detected.\n");
        return s;
    }
    for i in &combined {
        s.push_str(&format!(
            "- [{:?}] Line {}: {} [{}]\n",
            i.severity, i.line, i.message, i.rule_id
        ));
    }
    if total_all > combined.len() {
        s.push_str(&format!(
            "… truncated: showing {} of {} issues\n",
            combined.len(),
            total_all
        ));
    }
    s
}

fn build_unfinished_work_section(score: &QualityScore, max_items: usize, max_chars: usize) -> String {
    use crate::analysis::ast::quality_scorer::IssueCategory;
    let mut items: Vec<_> = score
        .concrete_issues
        .iter()
        .filter(|i| matches!(i.category, IssueCategory::UnfinishedWork))
        .collect();
    if items.is_empty() {
        return String::new();
    }
    // Sort deterministically by line then message
    items.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.message.cmp(&b.message)));
    let mut out = String::new();
    out.push_str("=== UNFINISHED WORK ===\n");
    for i in items.into_iter().take(max_items) {
        let mut msg = i.message.clone();
        if msg.len() > max_chars {
            msg.truncate(max_chars.saturating_sub(1));
            msg.push('…');
        }
        out.push_str(&format!("- Line {}: {}\n", i.line, msg));
    }
    out
}

fn build_quick_tips_section(score: &QualityScore) -> String {
    let enabled = if cfg!(debug_assertions) { std::env::var("QUICK_TIPS").map(|v| v != "0").unwrap_or(true) } else { true };
    if !enabled {
        return String::new();
    }
    let max_tips = if cfg!(debug_assertions) { std::env::var("QUICK_TIPS_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6) } else { 6 }
        .clamp(1, 20);
    let max_line = if cfg!(debug_assertions) { std::env::var("QUICK_TIPS_MAX_CHARS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120) } else { 120 }
        .clamp(60, 180);
    let tips = build_quick_tips(score, max_tips, max_line);
    if tips.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    s.push_str("=== QUICK TIPS ===\n");
    for t in tips {
        s.push_str("- ");
        s.push_str(&t);
        s.push('\n');
    }
    s
}

fn build_agent_json_from_score(score: &QualityScore) -> String {
    // Map severities
    let sev = |s: IssueSeverity| match s { IssueSeverity::Critical => "critical", IssueSeverity::Major => "major", IssueSeverity::Minor => "minor" };
    // risk_summary top 10
    let mut issues = score.concrete_issues.clone();
    issues.sort_by(|a,b| a.line.cmp(&b.line));
    issues.truncate(10);
    let mut items = Vec::new();
    for i in issues {
        items.push(serde_json::json!({
            "severity": sev(i.severity),
            "line": i.line,
            "rule": i.rule_id,
            "msg": i.message
        }));
    }
    // Minimal actions (empty edits/tests/refactors)
    let j = serde_json::json!({
        "schema_version": "1.1",
        "quality": { "overall": score.total_score, "confidence": 0.85 },
        "risk_summary": items,
        "api_contract": { "removed_functions": [], "param_changes": [] },
        "actions": { "edits": [], "tests": [], "refactors": [] },
        "followup_tools": [],
        "notes": []
    });
    serde_json::to_string(&j).unwrap_or_else(|_| "{}".to_string())
}

fn filter_issues_to_diff(score: &QualityScore, diff_text: &str, ctx: usize) -> QualityScore {
    use std::collections::HashSet;
    let mut changed: HashSet<usize> = HashSet::with_capacity(64);
    for line in diff_text.lines() {
        let t = line.trim_start();
        // Expect leading line number followed by space and a sign (+/-/ )
        let mut num: usize = 0;
        let mut saw_digit = false;
        let mut end_idx = 0usize;
        for (pos, ch) in t.char_indices() {
            if ch.is_ascii_digit() {
                saw_digit = true;
                num = num.saturating_mul(10).saturating_add((ch as u8 - b'0') as usize);
                end_idx = pos + ch.len_utf8();
            } else {
                break;
            }
        }
        if !saw_digit {
            continue;
        }
        // After number, find sign
        let rest = &t[end_idx..];
        // Normalize spaces
        let rest_trim = rest.trim_start();
        if rest_trim.starts_with('+') {
            // Mark changed line with context window
            let start = num.saturating_sub(ctx);
            let end = num.saturating_add(ctx);
            for ln in start..=end {
                if ln > 0 {
                    changed.insert(ln);
                }
            }
        }
    }
    if changed.is_empty() {
        return score.clone();
    }
    let mut filtered = score.clone();
    filtered.concrete_issues = score
        .concrete_issues
        .iter()
        .filter(|i| changed.contains(&i.line))
        .cloned()
        .collect();
    filtered
}

fn build_code_health(score: &QualityScore) -> String {
    use crate::analysis::ast::quality_scorer::IssueCategory;
    let mut params = 0;
    let mut nesting = 0;
    let mut complexity = 0;
    let mut long_method = 0;
    for i in &score.concrete_issues {
        match i.category {
            IssueCategory::TooManyParameters => params += 1,
            IssueCategory::DeepNesting => nesting += 1,
            IssueCategory::HighComplexity => complexity += 1,
            IssueCategory::LongMethod => long_method += 1,
            _ => {}
        }
    }
    let mut s = String::new();
    s.push_str("=== CODE HEALTH ===\n");
    s.push_str(&format!(
        "Too many params: {}\nDeep nesting: {}\nHigh complexity: {}\nLong methods: {}\n",
        params, nesting, complexity, long_method
    ));
    s
}

fn extract_changed_lines(diff_text: &str, ctx: usize) -> std::collections::HashSet<usize> {
    use std::collections::HashSet;
    let mut changed: HashSet<usize> = HashSet::with_capacity(64);
    for line in diff_text.lines() {
        let t = line.trim_start();
        // Leading number
        let mut num: usize = 0;
        let mut saw_digit = false;
        let mut end_idx = 0usize;
        for (pos, ch) in t.char_indices() {
            if ch.is_ascii_digit() {
                saw_digit = true;
                num = num.saturating_mul(10).saturating_add((ch as u8 - b'0') as usize);
                end_idx = pos + ch.len_utf8();
            } else {
                break;
            }
        }
        if !saw_digit {
            continue;
        }
        let rest_trim = t[end_idx..].trim_start();
        if rest_trim.starts_with('+') {
            let start = num.saturating_sub(ctx);
            let end = num.saturating_add(ctx);
            for ln in start..=end {
                if ln > 0 {
                    changed.insert(ln);
                }
            }
        }
    }
    changed
}

fn build_change_context_snippets(
    content: &str,
    score: &QualityScore,
    changed: &std::collections::HashSet<usize>,
    ctx_lines: usize,
    max_snippets: usize,
    max_chars: usize,
) -> String {
    use crate::analysis::ast::quality_scorer::{ConcreteIssue, IssueSeverity};
    if changed.is_empty() || score.concrete_issues.is_empty() {
        let mut out = String::new();
        out.push_str("=== CHANGE CONTEXT ===\n");
        out.push_str("No localized snippets available.\n");
        return out;
    }

    let mut issues = score.concrete_issues.clone();
    let sev_key = |s: IssueSeverity| match s {
        IssueSeverity::Critical => 0,
        IssueSeverity::Major => 1,
        IssueSeverity::Minor => 2,
    };
    issues.sort_by(|a: &ConcreteIssue, b: &ConcreteIssue| {
        sev_key(a.severity)
            .cmp(&sev_key(b.severity))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });

    let lines: Vec<&str> = content.lines().collect();
    let mut out = String::new();
    out.push_str("=== CHANGE CONTEXT ===\n");
    let mut used = 0usize;
    for i in issues {
        if used >= max_snippets {
            break;
        }
        let ln = i.line;
        if !changed.contains(&ln) {
            continue;
        }
        let start = ln.saturating_sub(ctx_lines).max(1) - 1;
        let end = (ln + ctx_lines).min(lines.len());
        // Header for this issue
        let header = format!(
            "- [{:?}] Line {}: {} [{}]\n",
            i.severity, i.line, i.message, i.rule_id
        );
        if out.len() + header.len() > max_chars {
            break;
        }
        out.push_str(&header);
        for idx in start..end {
            let mark = if idx + 1 == ln { '>' } else { ' ' };
            let row = format!(
                "{} {:4} | {}\n",
                mark,
                idx + 1,
                lines.get(idx).copied().unwrap_or("")
            );
            if out.len() + row.len() > max_chars {
                return out;
            }
            out.push_str(&row);
        }
        used += 1;
    }
    out
}

fn compute_line_offsets(s: &str) -> Vec<usize> {
    let mut offs = Vec::with_capacity(256);
    offs.push(0);
    for (i, b) in s.as_bytes().iter().enumerate() {
        if *b == b'\n' {
            offs.push(i + 1);
        }
    }
    offs
}

fn build_entity_context_snippets(
    language: SupportedLanguage,
    content: &str,
    score: &QualityScore,
    changed: &std::collections::HashSet<usize>,
    ctx_lines: usize,
    max_snippets: usize,
    max_chars: usize,
) -> String {
    use crate::analysis::ast::quality_scorer::{ConcreteIssue, IssueSeverity};
    if score.concrete_issues.is_empty() {
        return String::new();
    }
    let lang_supported = matches!(
        language,
        SupportedLanguage::Python | SupportedLanguage::JavaScript | SupportedLanguage::TypeScript
    );
    if !lang_supported {
        return String::new();
    }

    let mut parser = match LanguageCache::create_parser_with_language(language) {
        Ok(p) => p,
        Err(_) => return String::new(),
    };
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return String::new(),
    };
    let root = tree.root_node();
    let lines: Vec<&str> = content.lines().collect();
    let line_offs = compute_line_offsets(content);

    let sev_key = |s: IssueSeverity| match s {
        IssueSeverity::Critical => 0,
        IssueSeverity::Major => 1,
        IssueSeverity::Minor => 2,
    };
    let mut issues = score.concrete_issues.clone();
    issues.sort_by(|a: &ConcreteIssue, b: &ConcreteIssue| {
        sev_key(a.severity)
            .cmp(&sev_key(b.severity))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });

    let mut out = String::new();
    out.push_str("=== CHANGE CONTEXT ===\n");
    let mut used = 0usize;
    for i in issues {
        if used >= max_snippets {
            break;
        }
        let ln = i.line;
        if !changed.is_empty() && !changed.contains(&ln) {
            continue;
        }
        let idx = ln.saturating_sub(1);
        let off = *line_offs.get(idx).unwrap_or(&0);
        let node = root.named_descendant_for_byte_range(off, off).unwrap_or(root);
        let mut cur = node;
        let mut found = None;
        for _ in 0..20 {
            let k = cur.kind();
            let is_entity = match language {
                SupportedLanguage::Python => k == "function_definition" || k == "class_definition",
                SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                    if k == "function_declaration"
                        || k == "function_expression"
                        || k == "arrow_function"
                        || k == "method_definition"
                        || k == "class_declaration"
                    {
                        true
                    } else if k == "pair" || k == "property_assignment" {
                        // Object literal shorthand or function-valued property
                        let mut has_params_or_func = false;
                        for i in 0..cur.child_count() {
                            if let Some(ch) = cur.child(i) {
                                let ck = ch.kind();
                                if ck == "formal_parameters"
                                    || ck == "function"
                                    || ck == "function_expression"
                                    || ck == "arrow_function"
                                {
                                    has_params_or_func = true;
                                    break;
                                }
                            }
                        }
                        has_params_or_func
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if is_entity {
                found = Some(cur);
                break;
            }
            if let Some(p) = cur.parent() {
                cur = p;
            } else {
                break;
            }
        }
        let (start_row, end_row, entity_name, entity_kind) = if let Some(ent) = found {
            let sr = ent.start_position().row + 1;
            let er = ent.end_position().row + 1;
            let mut name = String::new();
            if matches!(language, SupportedLanguage::Python) {
                let mut c = ent.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if n.kind() == "identifier" {
                            if let Ok(txt) = n.utf8_text(content.as_bytes()) {
                                name = txt.to_string();
                                break;
                            }
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
            }
            (sr, er, name, ent.kind().to_string())
        } else {
            (ln, ln, String::new(), String::from("context"))
        };
        let start = start_row
            .saturating_sub(1)
            .max(ln.saturating_sub(ctx_lines * 2).saturating_sub(1));
        let end = end_row.min(ln + ctx_lines * 2);
        let header = if entity_name.is_empty() {
            format!(
                "- [{:?}] Line {} in {}: {} [{}]\n",
                i.severity, i.line, entity_kind, i.message, i.rule_id
            )
        } else {
            format!(
                "- [{:?}] Line {} in {} {}: {} [{}]\n",
                i.severity, i.line, entity_kind, entity_name, i.message, i.rule_id
            )
        };
        if out.len() + header.len() > max_chars {
            break;
        }
        out.push_str(&header);
        for idx in start..end {
            let mark = if idx + 1 == ln { '>' } else { ' ' };
            let row = format!(
                "{} {:4} | {}\n",
                mark,
                idx + 1,
                lines.get(idx).copied().unwrap_or("")
            );
            if out.len() + row.len() > max_chars {
                return out;
            }
            out.push_str(&row);
        }
        used += 1;
    }
    if used == 0 {
        out.push_str("No localized snippets available.\n");
    }
    out
}

// Function signatures via AST (Python/JS/TS)
#[derive(Clone, Debug)]
struct FuncSignature {
    params: Vec<String>,
    start_byte: usize,
    end_byte: usize,
}

fn extract_signatures_ast(
    language: SupportedLanguage,
    content: &str,
) -> std::collections::HashMap<String, FuncSignature> {
    use std::collections::HashMap;
    let mut res: HashMap<String, FuncSignature> = HashMap::new();
    if !matches!(
        language,
        SupportedLanguage::Python | SupportedLanguage::JavaScript | SupportedLanguage::TypeScript
    ) {
        return res;
    }
    let mut parser = match LanguageCache::create_parser_with_language(language) {
        Ok(p) => p,
        Err(_) => return res,
    };
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return res,
    };
    let root = tree.root_node();
    let bytes = content.as_bytes();

    // Extract param names from a Python parameters node (simple: collect identifiers)
    let collect_params_py = |node: tree_sitter::Node| -> Vec<String> {
        let mut out = Vec::new();
        let mut st = vec![node];
        while let Some(n) = st.pop() {
            if n.kind() == "identifier" {
                if let Ok(t) = n.utf8_text(bytes) {
                    out.push(t.to_string());
                }
            }
            for i in 0..n.child_count() {
                if let Some(ch) = n.child(i) {
                    st.push(ch);
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        out.into_iter().filter(|s| seen.insert(s.clone())).collect()
    };

    // Extract param names from JS/TS formal_parameters with careful filtering:
    // - skip type_annotation/type parameters/arguments and decorators/modifiers
    // - support optional/rest/assignment/object/array patterns
    // - collect only binding identifiers (ignore property identifiers and type names)
    let collect_params_js_ts = |node: tree_sitter::Node| -> Vec<String> {
        let mut out = Vec::new();
        let mut st = vec![node];
        while let Some(n) = st.pop() {
            let k = n.kind();
            // Prune subtrees that can't contain bindings we want
            if matches!(
                k,
                "type_annotation"
                    | "type_parameters"
                    | "type_arguments"
                    | "flow_type"
                    | "decorator"
                    | "accessibility_modifier"
                    | "public"
                    | "private"
                    | "protected"
                    | "readonly"
                    | "declare"
                    | "abstract"
                    | "override"
            ) {
                continue;
            }
            // Default value in parameters: only care about the left-hand binding
            if k == "assignment_pattern" {
                if let Some(lhs) = n.child(0) {
                    st.push(lhs);
                }
                continue;
            }
            // Binding identifiers to collect
            if k == "binding_identifier" || k == "shorthand_property_identifier" || k == "identifier" {
                if let Ok(t) = n.utf8_text(bytes) {
                    // Ignore special TS 'this' parameter (used only for typing)
                    if t != "this" {
                        out.push(t.to_string());
                    }
                }
                continue;
            }
            for i in 0..n.child_count() {
                if let Some(ch) = n.child(i) {
                    st.push(ch);
                }
            }
        }
        // Deduplicate but preserve first-seen order
        let mut seen = std::collections::HashSet::new();
        out.into_iter().filter(|s| seen.insert(s.clone())).collect()
    };

    // Helper to render computed property names more specifically when possible
    fn render_computed_name(bytes: &[u8], node: tree_sitter::Node) -> String {
        if let Ok(full) = node.utf8_text(bytes) {
            // full usually like: [expr]
            let trimmed = full.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let inner = &trimmed[1..trimmed.len() - 1].trim();
                // If inner looks reasonably short, surface it; else fallback
                if inner.len() <= 40 && !inner.is_empty() && !inner.contains('\n') {
                    // Strip simple quotes/backticks around literals
                    let inner_clean = inner
                        .trim_matches('"')
                        .trim_matches('\'')
                        .trim_matches('`')
                        .to_string();
                    return format!("[computed: {}]", inner_clean);
                }
            }
        }
        "[computed]".to_string()
    }

    // Helper: from a member/subscript expression, try to extract the property name
    fn extract_member_property_name(bytes: &[u8], node: tree_sitter::Node) -> Option<String> {
        let k = node.kind();
        if k == "member_expression" {
            for i in 0..node.child_count() {
                if let Some(ch) = node.child(i) {
                    let ck = ch.kind();
                    if ck == "property_identifier"
                        || ck == "identifier"
                        || ck == "private_property_identifier"
                    {
                        return ch.utf8_text(bytes).ok().map(|s| s.to_string());
                    }
                }
            }
        } else if k == "subscript_expression" {
            // subscript_expression: object '[' index ']'
            for i in 0..node.child_count() {
                if let Some(ch) = node.child(i) {
                    // Heuristic: pick the first non-bracket child that isn't the object
                    let ck = ch.kind();
                    if ck == "string" || ck == "number" || ck == "identifier" || ck == "member_expression" {
                        let text = ch.utf8_text(bytes).unwrap_or("");
                        if !text.is_empty() {
                            return Some(format!(
                                "[computed: {}]",
                                text.trim_matches('"').trim_matches('\'').trim_matches('`')
                            ));
                        }
                    }
                }
            }
            return Some("[computed]".to_string());
        }
        None
    }

    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for i in 0..n.child_count() {
            if let Some(ch) = n.child(i) {
                st.push(ch);
            }
        }
        let k = n.kind();
        match language {
            SupportedLanguage::Python => {
                if k == "function_definition" || k == "async_function_definition" {
                    let mut name = String::new();
                    let mut params_node = None;
                    for i in 0..n.child_count() {
                        if let Some(ch) = n.child(i) {
                            if name.is_empty() && ch.kind() == "identifier" {
                                if let Ok(t) = ch.utf8_text(bytes) {
                                    name = t.to_string();
                                }
                            }
                            if ch.kind() == "parameters" {
                                params_node = Some(ch);
                            }
                        }
                    }
                    let params = params_node.map(collect_params_py).unwrap_or_default();
                    if !name.is_empty() {
                        res.insert(
                            name.clone(),
                            FuncSignature {
                                params,
                                start_byte: n.start_byte(),
                                end_byte: n.end_byte(),
                            },
                        );
                    }
                }
            }
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                if k == "function_declaration" {
                    let mut name = String::new();
                    let mut params_node = None;
                    for i in 0..n.child_count() {
                        if let Some(ch) = n.child(i) {
                            if name.is_empty()
                                && (ch.kind() == "identifier" || ch.kind() == "binding_identifier")
                            {
                                if let Ok(t) = ch.utf8_text(bytes) {
                                    name = t.to_string();
                                }
                            }
                            if ch.kind() == "formal_parameters" {
                                params_node = Some(ch);
                            }
                        }
                    }
                    let params = params_node.map(collect_params_js_ts).unwrap_or_default();
                    if !name.is_empty() {
                        res.insert(
                            name.clone(),
                            FuncSignature {
                                params,
                                start_byte: n.start_byte(),
                                end_byte: n.end_byte(),
                            },
                        );
                    }
                } else if k == "method_definition" {
                    let mut name = String::new();
                    let mut params_node = None;
                    for i in 0..n.child_count() {
                        if let Some(ch) = n.child(i) {
                            let ck = ch.kind();
                            // Accessor keywords get/set are allowed but not used further here
                            if ck == "computed_property_name" && name.is_empty() {
                                name = render_computed_name(bytes, ch);
                            }
                            if name.is_empty()
                                && (ck == "property_identifier"
                                    || ck == "identifier"
                                    || ck == "private_property_identifier")
                            {
                                if let Ok(t) = ch.utf8_text(bytes) {
                                    name = t.to_string();
                                }
                            }
                            if ck == "formal_parameters" {
                                params_node = Some(ch);
                            }
                        }
                    }
                    let params = params_node.map(collect_params_js_ts).unwrap_or_default();
                    if !name.is_empty() {
                        res.insert(
                            name.clone(),
                            FuncSignature {
                                params,
                                start_byte: n.start_byte(),
                                end_byte: n.end_byte(),
                            },
                        );
                    }
                } else if k == "arrow_function" || k == "function_expression" {
                    let mut cur = n;
                    let mut name = String::new();
                    for _ in 0..6 {
                        if let Some(p) = cur.parent() {
                            let pk = p.kind();
                            if pk == "variable_declarator" {
                                if let Some(lhs) = p.child(0) {
                                    if lhs.kind() == "identifier" || lhs.kind() == "binding_identifier" {
                                        if let Ok(t) = lhs.utf8_text(bytes) {
                                            name = t.to_string();
                                        }
                                    }
                                }
                            } else if pk == "assignment_expression" {
                                if let Some(lhs) = p.child(0) {
                                    let lk = lhs.kind();
                                    if lk == "identifier"
                                        || lk == "binding_identifier"
                                        || lk == "property_identifier"
                                    {
                                        if let Ok(t) = lhs.utf8_text(bytes) {
                                            name = t.to_string();
                                        }
                                    } else if lk == "member_expression" || lk == "subscript_expression" {
                                        if let Some(pname) = extract_member_property_name(bytes, lhs) {
                                            name = pname;
                                        }
                                    }
                                }
                            }
                            cur = p;
                        } else {
                            break;
                        }
                        if !name.is_empty() {
                            break;
                        }
                    }
                    let mut params_node = None;
                    for i in 0..n.child_count() {
                        if let Some(ch) = n.child(i) {
                            if ch.kind() == "formal_parameters" {
                                params_node = Some(ch);
                                break;
                            }
                        }
                    }
                    let mut params = params_node.map(collect_params_js_ts).unwrap_or_default();
                    // Fallback: single identifier parameter for concise arrow functions (x => ...)
                    if params.is_empty() {
                        for i in 0..n.child_count() {
                            if let Some(ch) = n.child(i) {
                                let ck = ch.kind();
                                if ck == "identifier" || ck == "binding_identifier" {
                                    if let Ok(t) = ch.utf8_text(bytes) {
                                        params.push(t.to_string());
                                    }
                                }
                            }
                        }
                    }
                    if !name.is_empty() {
                        res.insert(
                            name.clone(),
                            FuncSignature {
                                params,
                                start_byte: n.start_byte(),
                                end_byte: n.end_byte(),
                            },
                        );
                    }
                } else if k == "pair" || k == "property_assignment" {
                    // Object literal method/property with function value: { foo(){} } or { foo: function(){} } or { foo: ()=>{} }
                    if let Some(key) = n.child(0) {
                        let key_k = key.kind();
                        if key_k == "property_identifier"
                            || key_k == "identifier"
                            || key_k == "private_property_identifier"
                            || key_k == "computed_property_name"
                        {
                            let kname = if key_k == "computed_property_name" {
                                render_computed_name(bytes, key)
                            } else {
                                key.utf8_text(bytes).unwrap_or("").to_string()
                            };
                            if !kname.is_empty() {
                                // Value likely at child(1) or later
                                let mut params_node = None;
                                let mut func_like: Option<tree_sitter::Node> = None;
                                // First, direct function value
                                for i in 1..n.child_count() {
                                    if let Some(ch) = n.child(i) {
                                        let ck = ch.kind();
                                        if ck == "function"
                                            || ck == "function_expression"
                                            || ck == "arrow_function"
                                        {
                                            func_like = Some(ch);
                                            break;
                                        }
                                    }
                                }
                                // Shorthand object method: look for a descendant 'formal_parameters'
                                if func_like.is_none() {
                                    let mut t = Vec::new();
                                    for i in 1..n.child_count() {
                                        if let Some(ch) = n.child(i) {
                                            t.push(ch);
                                        }
                                    }
                                    while let Some(m) = t.pop() {
                                        if m.kind() == "formal_parameters" {
                                            params_node = Some(m);
                                            break;
                                        }
                                        for i in 0..m.child_count() {
                                            if let Some(ch) = m.child(i) {
                                                t.push(ch);
                                            }
                                        }
                                    }
                                }
                                if params_node.is_none() {
                                    if let Some(func) = func_like {
                                        for i in 0..func.child_count() {
                                            if let Some(ch) = func.child(i) {
                                                if ch.kind() == "formal_parameters" {
                                                    params_node = Some(ch);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(pn) = params_node {
                                    let params = collect_params_js_ts(pn);
                                    res.insert(
                                        kname.to_string(),
                                        FuncSignature {
                                            params,
                                            start_byte: n.start_byte(),
                                            end_byte: n.end_byte(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                } else if k == "field_definition"
                    || k == "public_field_definition"
                    || k == "private_field_definition"
                {
                    // Class fields with function values: foo = () => {} or foo = function() {}
                    let mut name = String::new();
                    let mut value_func: Option<tree_sitter::Node> = None;
                    for i in 0..n.child_count() {
                        if let Some(ch) = n.child(i) {
                            let ck = ch.kind();
                            if name.is_empty()
                                && (ck == "property_identifier"
                                    || ck == "identifier"
                                    || ck == "private_property_identifier"
                                    || ck == "computed_property_name")
                            {
                                name = if ck == "computed_property_name" {
                                    render_computed_name(bytes, ch)
                                } else {
                                    ch.utf8_text(bytes).unwrap_or("").to_string()
                                };
                            }
                            if ck == "arrow_function" || ck == "function" || ck == "function_expression" {
                                value_func = Some(ch);
                            }
                        }
                    }
                    if let (false, Some(func)) = (name.is_empty(), value_func) {
                        let mut params_node = None;
                        for i in 0..func.child_count() {
                            if let Some(ch) = func.child(i) {
                                if ch.kind() == "formal_parameters" {
                                    params_node = Some(ch);
                                    break;
                                }
                            }
                        }
                        let params = params_node.map(collect_params_js_ts).unwrap_or_default();
                        res.insert(
                            name.clone(),
                            FuncSignature {
                                params,
                                start_byte: n.start_byte(),
                                end_byte: n.end_byte(),
                            },
                        );
                    }
                }
            }
            _ => {}
        }
    }
    res
}

fn ascend_to_function_node(
    language: SupportedLanguage,
    mut node: tree_sitter::Node,
) -> Option<tree_sitter::Node> {
    for _ in 0..50 {
        let k = node.kind();
        let is_func = match language {
            SupportedLanguage::Python => k == "function_definition" || k == "async_function_definition",
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                k == "function_declaration"
                    || k == "function_expression"
                    || k == "arrow_function"
                    || k == "method_definition"
            }
            _ => false,
        };
        if is_func {
            return Some(node);
        }
        if let Some(p) = node.parent() {
            node = p;
        } else {
            break;
        }
    }
    None
}

fn is_param_used_in_function(
    language: SupportedLanguage,
    content: &str,
    start_byte: usize,
    end_byte: usize,
    param: &str,
) -> bool {
    let mut parser = match LanguageCache::create_parser_with_language(language) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return false,
    };
    let root = tree.root_node();
    let at = root
        .named_descendant_for_byte_range(start_byte, start_byte)
        .unwrap_or(root);
    let func = if let Some(f) = ascend_to_function_node(language, at) {
        f
    } else {
        return false;
    };
    let bytes = content.as_bytes();
    let mut params_range: Option<(usize, usize)> = None;
    for i in 0..func.child_count() {
        if let Some(ch) = func.child(i) {
            let k = ch.kind();
            let is_params = match language {
                SupportedLanguage::Python => k == "parameters",
                SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => k == "formal_parameters",
                _ => false,
            };
            if is_params {
                params_range = Some((ch.start_byte(), ch.end_byte()));
                break;
            }
        }
    }
    let mut stack = vec![func];
    while let Some(n) = stack.pop() {
        if n.start_byte() < start_byte || n.end_byte() > end_byte {
            continue;
        }
        if let Some((ps, pe)) = params_range {
            if n.start_byte() >= ps && n.end_byte() <= pe {
                continue;
            }
        }
        let k = n.kind();
        let is_id = match language {
            SupportedLanguage::Python => k == "identifier",
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                k == "identifier" || k == "shorthand_property_identifier" || k == "binding_identifier"
            }
            _ => false,
        };
        if is_id {
            if let Ok(t) = n.utf8_text(bytes) {
                if t == param {
                    return true;
                }
            }
        }
        for i in 0..n.child_count() {
            if let Some(ch) = n.child(i) {
                stack.push(ch);
            }
        }
    }
    false
}

async fn build_change_summary(hook_input: &HookInput, display_path: &str) -> String {
    match generate_diff_context(hook_input, display_path).await {
        Ok(diff) => {
            let mut s = String::new();
            s.push_str("=== CHANGE SUMMARY ===\n");
            s.push_str(&diff);
            s
        }
        Err(_) => String::new(),
    }
}

// =============================
// API Contract checking (simple heuristics)
// =============================
fn extract_functions_signatures(
    language: SupportedLanguage,
    code: &str,
) -> std::collections::HashMap<String, Vec<String>> {
    use regex::Regex;
    let mut map = std::collections::HashMap::new();
    match language {
        SupportedLanguage::Python => {
            let re = Regex::new(r"(?m)^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)").ok();
            if let Some(re) = re {
                for cap in re.captures_iter(code) {
                    let name = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    let list = params
                        .split(',')
                        .map(|p| p.trim())
                        .filter(|p| !p.is_empty())
                        .map(|p| {
                            let base = p
                                .split(':')
                                .next()
                                .unwrap_or("")
                                .split('=')
                                .next()
                                .unwrap_or("")
                                .trim();
                            let base = base.trim_start_matches('*'); // *args/**kwargs normalization
                            base.to_string()
                        })
                        .filter(|p| !matches!(p.as_str(), "self" | "cls" | "args" | "kwargs"))
                        .collect::<Vec<_>>();
                    if !name.is_empty() {
                        map.insert(name, list);
                    }
                }
            }
        }
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            let re_fn = Regex::new(r"(?m)function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)").ok();
            if let Some(re) = re_fn {
                for cap in re.captures_iter(code) {
                    let name = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    let list = params
                        .split(',')
                        .map(|p| p.trim())
                        .filter(|p| !p.is_empty())
                        .map(|p| {
                            p.trim_start_matches("...")
                                .split(':')
                                .next()
                                .unwrap_or("")
                                .trim_end_matches('?')
                                .to_string()
                        })
                        .collect::<Vec<_>>();
                    if !name.is_empty() {
                        map.insert(name, list);
                    }
                }
            }
            // Class methods: name(param){ ... }
            let re_meth = Regex::new(r"(?m)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*\{").ok();
            if let Some(re) = re_meth {
                for cap in re.captures_iter(code) {
                    let name = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    let list = params
                        .split(',')
                        .map(|p| p.trim())
                        .filter(|p| !p.is_empty())
                        .map(|p| {
                            p.trim_start_matches("...")
                                .split(':')
                                .next()
                                .unwrap_or("")
                                .trim_end_matches('?')
                                .to_string()
                        })
                        .collect::<Vec<_>>();
                    if !name.is_empty() {
                        map.entry(name).or_insert(list.clone());
                    }
                }
            }
        }
        _ => {}
    }
    map
}

fn build_api_contract_report(language: SupportedLanguage, hook_input: &HookInput, content: &str) -> String {
    // API contract reporting cannot be disabled in release builds;
    // in debug/test, allow API_CONTRACT=0 to suppress for local experiments.
    if cfg!(debug_assertions) && std::env::var("API_CONTRACT").map(|v| v == "0").unwrap_or(false) {
        return String::new();
    }

    let mut before_code = String::new();
    let mut after_code = String::new();

    match normalize_tool_name(&hook_input.tool_name).0 {
        ToolKind::Edit => {
            if let Some(s) = hook_input.tool_input.get("old_string").and_then(|v| v.as_str()) {
                before_code = s.to_string();
            }
            if let Some(s) = hook_input.tool_input.get("new_string").and_then(|v| v.as_str()) {
                after_code = s.to_string();
            }
        }
        ToolKind::MultiEdit => {
            if let Some(edits) = hook_input.tool_input.get("edits").and_then(|v| v.as_array()) {
                for e in edits.iter().take(1000) {
                    if let Some(s) = e.get("old_string").and_then(|v| v.as_str()) {
                        before_code.push_str(s);
                        before_code.push('\n');
                    }
                    if let Some(s) = e.get("new_string").and_then(|v| v.as_str()) {
                        after_code.push_str(s);
                        after_code.push('\n');
                    }
                }
            }
        }
        _ => { /* Write/Other: no before */ }
    }

    if before_code.is_empty() && after_code.is_empty() {
        return String::new();
    }

    // Prefer AST-based signatures; fallback to regex
    let mut before_ast = extract_signatures_ast(language, &before_code);
    let mut after_ast = extract_signatures_ast(
        language,
        if after_code.is_empty() {
            content
        } else {
            &after_code
        },
    );
    if before_ast.is_empty() && !before_code.is_empty() {
        let b = extract_functions_signatures(language, &before_code);
        for (k, v) in b {
            before_ast.insert(
                k.clone(),
                FuncSignature {
                    params: v,
                    start_byte: 0,
                    end_byte: content.len(),
                },
            );
        }
    }
    if after_ast.is_empty() {
        let a = extract_functions_signatures(
            language,
            if after_code.is_empty() {
                content
            } else {
                &after_code
            },
        );
        for (k, v) in a {
            after_ast.insert(
                k.clone(),
                FuncSignature {
                    params: v,
                    start_byte: 0,
                    end_byte: content.len(),
                },
            );
        }
    }

    if before_ast.is_empty() && after_ast.is_empty() {
        return String::new();
    }

    let mut s = String::new();
    s.push_str("=== API CONTRACT ===\n");

    // Removed parameters / removed functions
    for (name, bsig) in before_ast.iter() {
        if let Some(asig) = after_ast.get(name) {
            if asig.params.len() < bsig.params.len() {
                s.push_str(&format!(
                    "- Function `{}`: parameter count reduced ({} -> {})\n",
                    name,
                    bsig.params.len(),
                    asig.params.len()
                ));
            } else {
                for bp in &bsig.params {
                    // Skip Python boilerplate params
                    if matches!(language, SupportedLanguage::Python)
                        && (bp == "self" || bp == "cls" || bp == "args" || bp == "kwargs")
                    {
                        continue;
                    }
                    if !bp.is_empty() && !asig.params.iter().any(|x| x == bp) {
                        s.push_str(&format!(
                            "- Function `{}`: parameter `{}` removed or renamed\n",
                            name, bp
                        ));
                    }
                }
            }
        } else {
            s.push_str(&format!(
                "- Function `{}`: removed from module (possible breaking change)\n",
                name
            ));
        }
    }

    // Unused parameters: prefer AST usage check; fallback to regex on the function slice
    use regex::Regex;
    let word = |p: &str| Regex::new(&format!(r"(?m)\b{}\b", regex::escape(p))).ok();
    let is_simple_ident = |v: &str| -> bool {
        if v.is_empty() {
            return false;
        }
        let mut chars = v.chars();
        match chars.next() {
            Some(first) if first.is_ascii_alphabetic() || first == '_' || first == '$' => {
                chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
            }
            _ => false,
        }
    };
    for (name, asig) in after_ast.iter() {
        for p in &asig.params {
            if p.is_empty() {
                continue;
            }
            // Ignore typical receiver params in Python
            if matches!(language, SupportedLanguage::Python) && (p == "self" || p == "cls") {
                continue;
            }
            // Only check simple identifiers for JS/TS; complex patterns are skipped to avoid false positives
            if matches!(
                language,
                SupportedLanguage::JavaScript | SupportedLanguage::TypeScript
            ) && !is_simple_ident(p)
            {
                continue;
            }
            let used = if asig.end_byte > asig.start_byte && asig.end_byte <= content.len() {
                is_param_used_in_function(language, content, asig.start_byte, asig.end_byte, p)
            } else {
                false
            };
            if !used {
                let hay = if asig.end_byte > asig.start_byte && asig.end_byte <= content.len() {
                    &content[asig.start_byte..asig.end_byte]
                } else {
                    content
                };
                let used_re = word(p).map(|re| re.is_match(hay)).unwrap_or(false);
                if !used_re {
                    s.push_str(&format!(
                        "- Function `{}`: parameter `{}` appears unused\n",
                        name, p
                    ));
                }
            }
        }
    }

    if s.trim_end() == "=== API CONTRACT ===" {
        String::new()
    } else {
        s
    }
}

fn build_next_steps(score: &QualityScore) -> String {
    use crate::analysis::ast::quality_scorer::{IssueCategory, IssueSeverity};
    let mut s = String::new();
    s.push_str("=== NEXT STEPS ===\n");
    let has_crit = score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.severity, IssueSeverity::Critical));
    if has_crit {
        s.push_str("- Fix security-critical issues first (creds/SQL/path/command).\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::HardcodedCredentials))
    {
        s.push_str("- Move secrets to env/secret manager; avoid hardcoding credentials.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::SqlInjection))
    {
        s.push_str("- Use parameterized queries; avoid concatenating SQL with inputs.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::CommandInjection))
    {
        s.push_str("- Avoid shell concatenation; use exec APIs with args arrays and validation.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::PathTraversal))
    {
        s.push_str("- Normalize and validate paths; restrict to allowed roots.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::TooManyParameters))
    {
        s.push_str("- Reduce function parameters (>5). Consider grouping.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::DeepNesting))
    {
        s.push_str("- Flatten deep nesting (>4). Extract helpers/early returns.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::HighComplexity))
    {
        s.push_str("- Reduce cyclomatic complexity. Split large functions.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::LongLine))
    {
        s.push_str(
            "- Wrap lines >120 chars; split expressions/format strings; adjust formatter width if needed.\n",
        );
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::UnreachableCode))
    {
        s.push_str("- Remove dead/unreachable code after return/raise/break; keep control flow linear.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::UnusedImports))
    {
        s.push_str("- Remove unused imports to reduce noise and speed up builds.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::UnusedVariables))
    {
        s.push_str("- Remove or underscore unused variables to clarify intent.\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::NamingConvention))
    {
        s.push_str("- Align names with project conventions (snake_case for Python, camelCase for JS/TS).\n");
    }
    if score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::MissingDocumentation))
    {
        s.push_str("- Add short docstrings/comments for public APIs (what, params, returns).\n");
    }
    // Always recommend tests for changed areas
    s.push_str("- Add/Update unit tests covering changed functions and edge cases.\n");
    if s.trim_end() == "=== NEXT STEPS ===" {
        s.push_str("- Looks good. Proceed with implementation and tests.\n");
    }
    s
}
// Removed GrokAnalysisClient - now using UniversalAIClient from ai_providers module

/// Check if a path should be ignored based on gitignore patterns
/// Implements proper glob-style pattern matching instead of simple string contains
fn should_ignore_path(path: &std::path::Path, gitignore_patterns: &[String]) -> bool {
    // Normalize separators for cross-platform matching (Windows \\ -> /)
    let raw = path.to_string_lossy();
    let normalized = raw.replace('\\', "/");
    let path_str = normalized.as_str();
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    for pattern in gitignore_patterns {
        // Handle different gitignore pattern types
        if pattern.is_empty() || pattern.starts_with('#') {
            continue;
        }

        let pattern = pattern.trim();

        // Exact file name match
        if pattern == file_name {
            return true;
        }

        // Directory name match (ends with /)
        if let Some(dir_pattern) = pattern.strip_suffix('/') {
            if path.is_dir() && (file_name == dir_pattern || path_str.contains(&format!("/{}/", dir_pattern)))
            {
                return true;
            }
        }

        // Extension match (*.ext)
        if let Some(ext_pattern) = pattern.strip_prefix("*.") {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == ext_pattern {
                    return true;
                }
            }
        }

        // Path contains pattern (simple substring match for now)
        if path_str.contains(pattern) {
            return true;
        }

        // Pattern at start of path
        if pattern.starts_with('/') && path_str.starts_with(&pattern[1..]) {
            return true;
        }

        // Pattern anywhere in path segments
        if path_str.split('/').any(|segment| segment == pattern) {
            return true;
        }
    }

    false
}

/// Validate path for security and ensure it's a directory
fn validate_prompts_path(path: &PathBuf) -> Option<PathBuf> {
    // Canonicalize handles path traversal, symlinks, and normalization
    // It may fail if path doesn't exist or due to permissions
    match std::fs::canonicalize(path) {
        Ok(canonical) => {
            // Ensure it's a directory
            if canonical.is_dir() {
                Some(canonical)
            } else {
                None
            }
        }
        Err(e) => {
            tracing::warn!(error=%e, "Cannot validate prompts path");
            None
        }
    }
}

/// Get the prompts directory path - always next to executable
fn get_prompts_dir() -> PathBuf {
    // Always look for prompts directory next to executable
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            tracing::error!(error=%e, "Cannot determine executable path; falling back to ./prompts");
            return PathBuf::from("prompts");
        }
    };

    let parent = match exe_path.parent() {
        Some(parent) => parent,
        None => {
            tracing::error!("Cannot get parent directory of executable; falling back to ./prompts");
            return PathBuf::from("prompts");
        }
    };

    // Production scenario: prompts directory next to executable
    let prompts_path = parent.join("prompts");

    if let Some(validated) = validate_prompts_path(&prompts_path) {
        tracing::info!(dir=?validated, "Using prompts directory");
        return validated;
    }

    // Final fallback
    tracing::warn!("Prompts directory not found next to executable; using current directory");
    PathBuf::from("prompts")
}

/// Load prompt content from file in prompts directory
async fn load_prompt_file(filename: &str) -> Result<String> {
    use tokio::time::{timeout, Duration};

    let prompt_path = get_prompts_dir().join(filename);

    // Add 5 second timeout for file reads
    timeout(Duration::from_secs(5), tokio::fs::read_to_string(prompt_path))
        .await
        .context("Timeout reading prompt file")?
        .or_else(|e| {
            // In DRY_RUN mode, tolerate missing prompts and continue with empty content
            if dev_flag_enabled("POSTTOOL_DRY_RUN") {
                tracing::warn!(error=%e, "Continuing with empty prompt due to DRY_RUN");
                Ok(String::new())
            } else {
                Err(e)
            }
        })
        .with_context(|| format!("Failed to load prompt file: {filename}"))
}

// Constants for formatting instructions
const CRITICAL_INSTRUCTION: &str = "\n\nOUTPUT EXACTLY AS SHOWN IN THE TEMPLATE BELOW.\n\n";

const TOKEN_LIMIT: &str = "TOKEN LIMIT: 4500\n\n";

const TEMPLATE_HEADER: &str = "=== REQUIRED OUTPUT FORMAT ===\n";
const TEMPLATE_FOOTER: &str = "\n=== END FORMAT ===\n";

// This will be dynamically constructed with language
const FINAL_INSTRUCTION_PREFIX: &str =
    "\n\nOUTPUT EXACTLY AS TEMPLATE. ANY FORMAT ALLOWED IF TEMPLATE SHOWS IT.\nRESPOND IN ";

// Removed legacy wrapper `format_analysis_prompt` (use `format_analysis_prompt_with_ast` directly)

// Tests live in the larger tests module further below

/// Format the analysis prompt with instructions, project context, conversation, and AST analysis
async fn format_analysis_prompt_with_ast(
    prompt: &str,
    project_context: Option<&str>,
    diff_context: Option<&str>,
    transcript_context: Option<&str>,
    ast_context: Option<&str>,
) -> Result<String> {
    // Load output template from file
    let output_template = load_prompt_file("output_template.txt").await?;

    // Load anti-patterns for comprehensive validation
    let anti_patterns = load_prompt_file("anti_patterns.txt")
        .await
        .unwrap_or_else(|_| String::new());

    // Load context7 documentation recommendation engine
    let context7_docs = load_prompt_file("context7_docs.txt")
        .await
        .unwrap_or_else(|_| String::new());

    // Load language preference with fallback to RUSSIAN
    let language = load_prompt_file("language.txt")
        .await
        .unwrap_or_else(|_| "RUSSIAN".to_string())
        .trim()
        .to_string();

    let context_section = if let Some(context) = project_context {
        format!("\n\nPROJECT CONTEXT:\n{}\n", context)
    } else {
        String::new()
    };

    let diff_section = if let Some(diff) = diff_context {
        format!("\n\nCODE CHANGES (diff format):\n{}\n", diff)
    } else {
        String::new()
    };

    let transcript_section = if let Some(transcript) = transcript_context {
        format!("\n\nCONVERSATION CONTEXT:\n{}\n", transcript)
    } else {
        String::new()
    };

    let context7_section = if !context7_docs.is_empty() {
        format!(
            "\n\nDOCUMENTATION RECOMMENDATION GUIDELINES:\n{}\n",
            context7_docs
        )
    } else {
        String::new()
    };

    let ast_section = if let Some(ast) = ast_context {
        format!("\n{}\n", ast)
    } else {
        String::new()
    };

    // Build prompt with pre-allocated capacity for better performance
    let estimated_capacity = prompt.len()
        + output_template.len()
        + anti_patterns.len()
        + transcript_section.len()
        + context_section.len()
        + diff_section.len()
        + context7_section.len()
        + ast_section.len()
        + CRITICAL_INSTRUCTION.len()
        + TOKEN_LIMIT.len()
        + TEMPLATE_HEADER.len()
        + TEMPLATE_FOOTER.len()
        + FINAL_INSTRUCTION_PREFIX.len()
        + language.len()
        + 50; // buffer for separators, anti-patterns section and " LANGUAGE."

    let mut result = String::with_capacity(estimated_capacity);

    // Main prompt
    result.push_str(prompt);
    result.push_str("\n\n");

    // Context sections
    if !transcript_section.is_empty() {
        result.push_str(&transcript_section);
    }
    if !context_section.is_empty() {
        result.push_str(&context_section);
    }
    if !diff_section.is_empty() {
        result.push_str(&diff_section);
    }

    // Add AST analysis results BEFORE AI analysis for deterministic context
    if !ast_section.is_empty() {
        result.push_str(&ast_section);
    }

    if !context7_section.is_empty() {
        result.push_str(&context7_section);
    }

    // Add anti-patterns reference if loaded
    if !anti_patterns.is_empty() {
        result.push_str("\n\nANTI-PATTERNS REFERENCE:\n");
        result.push_str(&anti_patterns);
    }

    // Critical formatting instruction
    result.push_str(CRITICAL_INSTRUCTION);

    // Token limit warning
    result.push_str(TOKEN_LIMIT);

    // Output template
    result.push_str(TEMPLATE_HEADER);
    result.push_str(&output_template);
    result.push_str(TEMPLATE_FOOTER);

    // Final instructions with language
    result.push_str(FINAL_INSTRUCTION_PREFIX);
    result.push_str(&language);
    result.push_str(" LANGUAGE.");

    Ok(result)
}

// Use analysis structures from lib.rs

/// Simple file path validation - AI will handle security checks
fn validate_file_path(path: &str) -> Result<PathBuf> {
    use std::path::Path;

    // Check for empty path
    if path.is_empty() {
        anyhow::bail!("Invalid file path: empty path");
    }

    // Check for null bytes which are always invalid
    if path.contains('\0') {
        anyhow::bail!("Invalid file path: contains null byte");
    }

    // Check for URL encoding attempts that could bypass validation
    if path.contains('%') {
        const SUSPICIOUS_ENCODINGS: &[&str] = &[
            "%2e", "%2E", // encoded dots
            "%2f", "%2F", // encoded slashes
            "%5c", "%5C", // encoded backslashes
            "%00", // null byte
            "%252e", "%252E", // double encoded dots
        ];

        for encoding in SUSPICIOUS_ENCODINGS {
            if path.contains(encoding) {
                anyhow::bail!(
                    "Invalid file path: contains suspicious URL encoding: {}",
                    encoding
                );
            }
        }
    }

    // Do not pre-reject ".."/"~"/"$" substrings blindly — rely on canonicalization and scope checks below.
    // Special-case only unsafe home-expansion prefix on non-Windows.
    if !cfg!(windows) && (path.starts_with("~/") || path.starts_with("~\\")) {
        anyhow::bail!("Invalid file path: leading ~ home shortcut not allowed");
    }

    let path_obj = Path::new(path);

    // If path exists, validate it's within allowed directories
    if path_obj.exists() {
        // Get current working directory as the base allowed directory
        let cwd =
            std::env::current_dir().map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?;

        // Canonicalize both cwd and the target path to resolve symlinks, relative parts and Windows UNC/\\?\ prefixes
        let cwd_canon = cwd.canonicalize().unwrap_or(cwd.clone());

        // Canonicalize to resolve symlinks and relative paths
        let canonical = path_obj
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("Failed to canonicalize path: {}", e))?;

        // Ensure the canonical path is within the current working directory
        if !canonical.starts_with(&cwd_canon) {
            anyhow::bail!(
                "Invalid file path: path is outside working directory. Path: {:?}, CWD: {:?}",
                canonical,
                cwd_canon
            );
        }
    }

    Ok(PathBuf::from(path))
}

/// Safely read file content with proper error handling, size limits and timeout
async fn read_file_content_safe(path: &str) -> Result<Option<String>> {
    use tokio::time::{timeout, Duration};

    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB limit

    let validated_path = match validate_file_path(path) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(path=%path, error=%e, "Failed to validate file path");
            return Ok(None);
        }
    };

    // Add configurable timeout for file reads (debug-only override)
    let timeout_secs = if cfg!(debug_assertions) {
        std::env::var("FILE_READ_TIMEOUT")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u64>()
            .unwrap_or(10)
    } else { 10 };

    // Use streaming read to prevent TOCTOU race condition
    match timeout(Duration::from_secs(timeout_secs), async {
        use tokio::fs::File;
        use tokio::io::{AsyncReadExt, BufReader};

        let file = File::open(&validated_path).await?;
        let mut reader = BufReader::new(file);
        let mut content = String::new();
        let mut buffer = [0; 8192]; // 8KB chunks
        let mut total_size = 0u64;

        loop {
            match reader.read(&mut buffer).await? {
                0 => break, // EOF
                n => {
                    total_size += n as u64;
                    if total_size > MAX_FILE_SIZE {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "File exceeds {}MB limit during read",
                                MAX_FILE_SIZE / (1024 * 1024)
                            ),
                        ));
                    }
                    content.push_str(&String::from_utf8_lossy(&buffer[..n]));
                }
            }
        }

        Ok::<String, std::io::Error>(content)
    })
    .await
    {
        Ok(Ok(content)) => Ok(Some(content)),
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            // File doesn't exist yet - this is normal for new files
            Ok(None)
        }
        Ok(Err(e)) => {
            tracing::warn!(path=%validated_path.display(), error=%e, "Failed to read file");
            Ok(None)
        }
        Err(_) => {
            tracing::warn!(path=%validated_path.display(), timeout=%timeout_secs, "Timeout reading file");
            Ok(None)
        }
    }
}

/// Generate diff context for tool operations with FULL file content
async fn generate_diff_context(hook_input: &HookInput, display_path: &str) -> Result<String> {
    // Extract the actual file path from tool_input for file operations
    let actual_file_path = hook_input
        .tool_input
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or(display_path);

    // Read file content using actual path
    let file_content = read_file_content_safe(actual_file_path).await?;

    match normalize_tool_name(&hook_input.tool_name).0 {
        ToolKind::Edit => {
            // Extract and validate required fields for Edit operation
            let old_string = hook_input
                .tool_input
                .get("old_string")
                .and_then(|v| v.as_str())
                .context("Edit operation missing required 'old_string' field")?;

            let new_string = hook_input
                .tool_input
                .get("new_string")
                .and_then(|v| v.as_str())
                .context("Edit operation missing required 'new_string' field")?;

            // Use numbered unified diff for better downstream parsing
            Ok(crate::validation::diff_formatter::format_edit_as_unified_diff(
                display_path,
                file_content.as_deref(),
                old_string,
                new_string,
            ))
        }

        ToolKind::MultiEdit => {
            // Extract and validate edits array
            let edits_array = hook_input
                .tool_input
                .get("edits")
                .and_then(|v| v.as_array())
                .context("MultiEdit operation missing required 'edits' array")?;

            // Validate edits array is not empty
            if edits_array.is_empty() {
                anyhow::bail!("MultiEdit operation has empty 'edits' array");
            }

            // Parse edits with validation
            let mut edits = Vec::new();
            for (idx, edit) in edits_array.iter().enumerate() {
                let old_string = edit
                    .get("old_string")
                    .and_then(|v| v.as_str())
                    .with_context(|| format!("Edit {} missing 'old_string'", idx))?;

                let new_string = edit
                    .get("new_string")
                    .and_then(|v| v.as_str())
                    .with_context(|| format!("Edit {} missing 'new_string'", idx))?;

                // Validate strings are not empty
                if old_string.is_empty() {
                    anyhow::bail!("Edit {} has empty 'old_string'", idx);
                }

                edits.push((old_string.to_string(), new_string.to_string()));
            }

            // Use the new format_multi_edit_full_context function for better diff output
            Ok(format_multi_edit_full_context(
                display_path,
                file_content.as_deref(),
                &edits,
            ))
        }

        ToolKind::Write => {
            // Extract and validate content field
            let new_content = hook_input
                .tool_input
                .get("content")
                .and_then(|v| v.as_str())
                .context("Write operation missing required 'content' field")?;

            // If this is an append-like tool and we have current file content, build full new content
            let combined_new = if normalize_tool_name(&hook_input.tool_name).1 {
                if let Some(existing) = file_content.as_deref() {
                    let mut s = String::with_capacity(existing.len() + new_content.len() + 1);
                    s.push_str(existing);
                    if !existing.ends_with('\n') && !new_content.is_empty() {
                        s.push('\n');
                    }
                    s.push_str(new_content);
                    Some(s)
                } else {
                    Some(new_content.to_string())
                }
            } else {
                Some(new_content.to_string())
            };

            // Return unified diff for clear change visibility with +/- markers
            Ok(format_code_diff(
                display_path,
                file_content.as_deref(),
                combined_new.as_deref(),
                3, // context lines
            ))
        }

        ToolKind::Other => {
            // Not a code modification operation
            Ok(String::new())
        }
    }
}

/// Validate transcript path for security with strict directory restrictions
fn validate_transcript_path(path: &str) -> Result<()> {
    use std::path::Path;

    // Check for null bytes which are always invalid in paths
    if path.contains('\0') {
        anyhow::bail!("Path contains null bytes");
    }

    // Check for various URL encoding attempts that could bypass validation
    const SUSPICIOUS_ENCODINGS: &[&str] = &[
        "%2e", "%2E", // encoded dots
        "%2f", "%2F", // encoded slashes
        "%5c", "%5C", // encoded backslashes
        "%00", // null byte
        "%252e", "%252E", // double encoded dots
    ];

    if path.contains('%') {
        for encoding in SUSPICIOUS_ENCODINGS {
            if path.contains(encoding) {
                anyhow::bail!("Path contains suspicious URL encoding: {}", encoding);
            }
        }
    }

    // Check for various path traversal patterns
    const TRAVERSAL_PATTERNS: &[&str] = &[
        "..", // parent directory
        "~",  // home directory expansion
        "$",  // variable expansion
        "./", ".\\", // current directory traversal
        "../",
        "..\\", // parent directory traversal
                // UNC handled below conditionally by OS
    ];

    for pattern in TRAVERSAL_PATTERNS {
        if path.contains(pattern) {
            anyhow::bail!("Path contains potential traversal pattern: {}", pattern);
        }
    }

    // UNC pre-check: only disallow on non-Windows
    if path.contains("\\\\") && !cfg!(windows) {
        anyhow::bail!("Path contains UNC pattern on non-Windows system");
    }

    let path_obj = Path::new(path);

    // Get allowed base directories for transcript files
    // These are typically temp directories and the current working directory
    let allowed_dirs = get_allowed_transcript_directories()?;

    // If path exists, perform strict canonicalization and directory checks
    if path_obj.exists() {
        // Canonicalize to resolve all symlinks and relative paths
        let canonical = path_obj
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("Failed to canonicalize path: {}", e))?;

        // Verify the canonical path is within allowed directories
        let mut is_allowed = false;
        for allowed_dir in &allowed_dirs {
            if let Ok(allowed_canonical) = allowed_dir.canonicalize() {
                if canonical.starts_with(&allowed_canonical) {
                    is_allowed = true;
                    break;
                }
            }
        }

        if !is_allowed {
            anyhow::bail!(
                "Path is outside allowed directories. Path: {:?}, Allowed: {:?}",
                canonical,
                allowed_dirs
            );
        }

        // Ensure the canonical path doesn't contain any remaining suspicious patterns
        let canonical_str = canonical
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Path contains invalid UTF-8"))?;

        // Final sanity checks on the canonical path
        if canonical_str.contains("\\\\") && !cfg!(windows) {
            anyhow::bail!("Path contains UNC pattern on non-Windows system");
        }
    } else {
        // For non-existent paths (like in tests), ensure they would be within allowed directories
        // Check if the parent directory exists and is allowed
        if let Some(parent) = path_obj.parent() {
            if parent.exists() {
                let parent_canonical = parent
                    .canonicalize()
                    .map_err(|e| anyhow::anyhow!("Failed to canonicalize parent path: {}", e))?;

                let mut is_allowed = false;
                for allowed_dir in &allowed_dirs {
                    if let Ok(allowed_canonical) = allowed_dir.canonicalize() {
                        if parent_canonical.starts_with(&allowed_canonical) {
                            is_allowed = true;
                            break;
                        }
                    }
                }

                if !is_allowed {
                    anyhow::bail!(
                        "Parent directory is outside allowed directories: {:?}",
                        parent_canonical
                    );
                }
            }
        }
    }

    Ok(())
}

/// Get allowed base directories for transcript files
fn get_allowed_transcript_directories() -> Result<Vec<PathBuf>> {
    use std::env;
    use std::path::PathBuf;

    let mut allowed = Vec::new();

    // Allow system temp directory
    allowed.push(env::temp_dir());

    // Allow current working directory (for development/testing)
    if let Ok(cwd) = env::current_dir() {
        allowed.push(cwd);
    }

    // Allow user's temp directory variations
    if let Ok(temp) = env::var("TEMP") {
        allowed.push(PathBuf::from(temp));
    }
    if let Ok(tmp) = env::var("TMP") {
        allowed.push(PathBuf::from(tmp));
    }
    if let Ok(tmpdir) = env::var("TMPDIR") {
        allowed.push(PathBuf::from(tmpdir));
    }

    // For tests, also allow cargo's target directory
    if cfg!(test) {
        if let Ok(cargo_target) = env::var("CARGO_TARGET_DIR") {
            allowed.push(PathBuf::from(cargo_target));
        } else {
            // Default target directory
            if let Ok(cwd) = env::current_dir() {
                allowed.push(cwd.join("target"));
            }
        }
    }

    Ok(allowed)
}

/// Read and format transcript for AI context (without tool content)
async fn read_transcript_summary(path: &str, max_messages: usize, max_chars: usize) -> Result<String> {
    // Security check with improved validation
    validate_transcript_path(path)?;

    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, BufReader};

    // For large files, we'll read only the last portion
    const MAX_READ_BYTES: u64 = 1024 * 1024; // 1MB should be enough for recent messages

    let file = File::open(path).await.context("Failed to open transcript file")?;
    let metadata = file.metadata().await?;
    let file_size = metadata.len();

    // Position to read from (read last 1MB or entire file if smaller)
    let start_pos = if file_size > MAX_READ_BYTES {
        file_size.saturating_sub(MAX_READ_BYTES)
    } else {
        0
    };

    // Seek to starting position if needed
    use tokio::io::AsyncSeekExt;
    let mut file = file;
    if start_pos > 0 {
        file.seek(std::io::SeekFrom::Start(start_pos)).await?;
    }

    let reader = BufReader::new(file);
    let mut lines_buffer = Vec::new();
    let mut lines = reader.lines();

    // Collect lines into buffer
    let mut skipped_first = false;
    while let Some(line) = lines.next_line().await? {
        if start_pos > 0 && !skipped_first {
            // Skip potentially partial first line when reading from middle
            skipped_first = true;
            continue;
        }
        lines_buffer.push(line);
    }

    let mut messages = Vec::new();
    let mut total_chars = 0;
    let mut most_recent_user_message = String::new();
    let mut found_first_user_message = false;

    // Parse JSONL format - each line is a separate JSON object
    // We iterate in reverse to get most recent messages first
    for line in lines_buffer.iter().rev() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            // Extract message from the entry - handle both nested and simple formats
            let msg = if let Some(nested_msg) = entry.get("message") {
                // Format: {"message": {"role": "...", "content": "..."}}
                nested_msg
            } else {
                // Format: {"role": "...", "content": "..."} - simple format
                &entry
            };

            // Handle different message formats
            if let Some(role) = msg.get("role").and_then(|v| v.as_str()) {
                let content = if let Some(content_arr) = msg.get("content").and_then(|v| v.as_array()) {
                    // Handle content array (assistant messages)
                    let mut text_parts = Vec::new();

                    for c in content_arr {
                        if let Some(text) = c.get("text").and_then(|v| v.as_str()) {
                            text_parts.push(text.to_string());
                        } else if let Some(tool_name) = c.get("name").and_then(|v| v.as_str()) {
                            // Get tool name and file if available
                            if let Some(input) = c.get("input") {
                                if let Some(file_path) = input.get("file_path").and_then(|v| v.as_str()) {
                                    text_parts.push(format!("{tool_name} tool file: {file_path}"));
                                } else {
                                    text_parts.push(format!("{} tool", tool_name));
                                }
                            }
                        }
                    }

                    if !text_parts.is_empty() {
                        Some(text_parts.join(" "))
                    } else {
                        None
                    }
                } else {
                    // Handle simple string content (user messages)
                    msg.get("content")
                        .and_then(|v| v.as_str())
                        .map(|text| text.to_string())
                };

                if let Some(content) = content {
                    // Format message
                    let formatted_msg = if role == "user" {
                        // Save the FIRST user message we encounter (which is the most recent due to reverse iteration)
                        if !found_first_user_message {
                            most_recent_user_message = content.clone();
                            found_first_user_message = true;
                        }
                        format!("user: {}", truncate_utf8_safe(&content, 150))
                    } else if role == "assistant" {
                        format!("assistant: {}", truncate_utf8_safe(&content, 150))
                    } else {
                        continue;
                    };

                    total_chars += formatted_msg.len();
                    messages.push(formatted_msg);

                    if messages.len() >= max_messages || total_chars >= max_chars {
                        break;
                    }
                }
            }
        }
    }

    // Reverse to get chronological order
    messages.reverse();

    // Format final output
    let conversation = messages.join("\n");

    // Extract current task from most recent user message
    let current_task = if !most_recent_user_message.is_empty() {
        format!(
            "Current user task: {}\n\n",
            truncate_utf8_safe(&most_recent_user_message, 200)
        )
    } else {
        String::new()
    };

    let result = format!("{current_task}conversation:\n{conversation}");

    // Ensure we respect the max_chars limit for the entire output
    if result.len() > max_chars {
        // Truncate to fit within limit
        let truncated = truncate_utf8_safe(&result, max_chars);
        Ok(truncated)
    } else {
        Ok(result)
    }
}

/// Perform AST-based quality analysis on code with a hard timeout
async fn perform_ast_analysis(content: &str, file_path: &str) -> Option<QualityScore> {
    // Detect language from file extension
    let extension = file_path.split('.').next_back().unwrap_or("");
    let language = match SupportedLanguage::from_extension(extension) {
        Some(lang) => lang,
        None => {
            tracing::warn!(%extension, "AST analysis: unsupported file type");
            return None;
        }
    };

    // Timeout configuration (env overrideable)
    let timeout_secs: u64 = std::env::var("AST_ANALYSIS_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8)
        .clamp(1, 30);

    // Run CPU-bound analysis off the async runtime and enforce timeout
    let code = content.to_string();
    let handle = tokio::task::spawn_blocking(move || {
        let scorer = AstQualityScorer::new();
        let t0 = std::time::Instant::now();
        let res = scorer.analyze(&code, language);
        if res.is_ok() {
            crate::analysis::timings::record(&format!("score/{language}"), t0.elapsed().as_millis());
        }
        res
    });

    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), handle).await {
        Ok(join_res) => match join_res {
            Ok(Ok(score)) => Some(score),
            Ok(Err(e)) => {
                tracing::warn!(error=%e, "AST analysis error");
                None
            }
            Err(join_err) => {
                tracing::warn!(error=%join_err, "AST analysis join error");
                None
            }
        },
        Err(_) => {
            tracing::warn!(timeout=%timeout_secs, file=%file_path, "AST analysis timeout");
            None
        }
    }
}

fn soft_budget_note(content: &str, file_path: &str) -> Option<String> {
    let bytes = content.len();
    let lines = content.lines().count();
    // Allow very small budgets for tests; keep a sane upper bound for safety.
    // Previously we clamped to 50_000 bytes/1_000 lines which broke e2e expectations.
    // New policy: lower bound = 1 (caller decides), upper bound remains protective.
    let max_bytes: usize = if cfg!(debug_assertions) {
        std::env::var("AST_SOFT_BUDGET_BYTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500_000)
    } else { 500_000 }
    .clamp(1, 5_000_000);
    let max_lines: usize = if cfg!(debug_assertions) {
        std::env::var("AST_SOFT_BUDGET_LINES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10_000)
    } else { 10_000 }
    .clamp(1, 200_000);
    if bytes > max_bytes || lines > max_lines {
        return Some(format!(
            "[ANALYSIS] Skipped AST analysis due to soft budget ({} bytes, {} lines) for {} (limits: {} bytes, {} lines)",
            bytes, lines, file_path, max_bytes, max_lines
        ));
    }
    None
}

/// Format AST analysis results for AI context (without scores to avoid duplication)
fn format_ast_results(score: &QualityScore) -> String {
    let mut result = String::with_capacity(2000);

    // Only pass concrete issues to AI, not scores (to avoid duplication)
    if score.concrete_issues.is_empty() {
        return String::new();
    }

    // Determine limit for issues
    let max_issues: usize = std::env::var("AST_MAX_ISSUES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
        .clamp(10, 500);

    // Copy and sort deterministically: severity -> line -> rule_id
    let mut issues = score.concrete_issues.clone();
    let sev_key = |s: IssueSeverity| match s {
        IssueSeverity::Critical => 0,
        IssueSeverity::Major => 1,
        IssueSeverity::Minor => 2,
    };
    issues.sort_by(|a, b| {
        sev_key(a.severity)
            .cmp(&sev_key(b.severity))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });

    // Take top-K
    let total = issues.len();
    issues.truncate(max_issues);

    result.push_str("\n\nAST DETECTED ISSUES (Automated, top sorted):\n");

    // Grouped printing preserving sorted order
    let mut print_group = |title: &str, sev: IssueSeverity| {
        let group: Vec<_> = issues.iter().filter(|i| i.severity == sev).collect();
        if !group.is_empty() {
            result.push_str(title);
            result.push('\n');
            for issue in group {
                result.push_str(&format!(
                    "  Line {}: {} [{}] (-{} points)\n",
                    issue.line, issue.message, issue.rule_id, issue.points_deducted
                ));
            }
        }
    };

    print_group("\n🔴 CRITICAL (P1 - Fix immediately):", IssueSeverity::Critical);
    print_group("\n🟡 MAJOR (P2 - Fix soon):", IssueSeverity::Major);
    print_group("\n🟢 MINOR (P3 - Nice to fix):", IssueSeverity::Minor);

    if total > issues.len() {
        result.push_str(&format!(
            "\n… truncated: showing {} of {} issues (AST_MAX_ISSUES).\n",
            issues.len(),
            total
        ));
    }

    result.push_str("\nNote: Use AST issues as baseline. Add context-aware insights.\n");

    result
}

/// Perform project-wide AST analysis excluding non-code files and .gitignore entries
async fn perform_project_ast_analysis(working_dir: &str) -> String {
    let mut results = Vec::new();
    let mut total_issues = 0;
    let mut total_files_analyzed = 0;
    let mut critical_issues = Vec::new();
    let mut skipped_large_files = 0;
    let mut skipped_error_files = 0;

    // Optional timings collection
    let timings_enabled = std::env::var("AST_TIMINGS").is_ok();
    let mut durations_ms: Vec<u128> = Vec::new();

    // Read .gitignore patterns if available
    let gitignore_path = std::path::Path::new(working_dir).join(".gitignore");
    let gitignore_patterns = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path)
            .ok()
            .map(|content| {
                content
                    .lines()
                    .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
                    .map(|line| line.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // Analyze all files in the project
    if let Ok(entries) = std::fs::read_dir(working_dir) {
        for entry in entries.flatten() {
            if let Err(e) = analyze_directory_recursive(
                &entry.path(),
                &mut results,
                &mut total_issues,
                &mut total_files_analyzed,
                &mut critical_issues,
                &gitignore_patterns,
                0,
                &mut skipped_large_files,
                &mut skipped_error_files,
                timings_enabled,
                &mut durations_ms,
            )
            .await
            {
                if dev_flag_enabled("DEBUG_HOOKS") {
                    tracing::debug!(path=%entry.path().display(), error=%e, "Failed to analyze path");
                }
            }
        }
    }

    if total_files_analyzed == 0 && skipped_large_files == 0 && skipped_error_files == 0 {
        return String::new();
    }

    let mut analysis = format!(
        "\n## PROJECT-WIDE AST ANALYSIS\n\
        - Files analyzed: {}\n\
        - Total issues found: {}\n\
        - Critical issues: {}\n\
        - Skipped (too large): {}\n\
        - Skipped (errors): {}\n",
        total_files_analyzed,
        total_issues,
        critical_issues.len(),
        skipped_large_files,
        skipped_error_files
    );

    if !critical_issues.is_empty() {
        // Deterministic ordering for critical issues list
        critical_issues.sort();
        analysis.push_str("\n### Critical Issues in Project:\n");
        for (i, issue) in critical_issues.iter().take(5).enumerate() {
            analysis.push_str(&format!("{}. {}\n", i + 1, issue));
        }
        if critical_issues.len() > 5 {
            analysis.push_str(&format!(
                "... and {} more critical issues\n",
                critical_issues.len() - 5
            ));
        }
    }

    if !results.is_empty() {
        // Deterministic ordering for files with issues: by count desc, then path asc
        results.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        analysis.push_str("\n### Files with Issues:\n");
        for (path, issues_count, _) in results.iter().take(10) {
            analysis.push_str(&format!("- `{}`: {} issues\n", path, issues_count));
        }
        if results.len() > 10 {
            analysis.push_str(&format!(
                "... and {} more files with issues\n",
                results.len() - 10
            ));
        }
    }

    // Timings summary (optional)
    if timings_enabled && !durations_ms.is_empty() {
        let mut v = durations_ms;
        v.sort_unstable();
        let idx = |pct: f64| -> usize {
            let n = v.len();
            let pos = (pct * (n.saturating_sub(1)) as f64).round() as usize;
            pos.min(n - 1)
        };
        let p50 = v[idx(0.50)];
        let p95 = v[idx(0.95)];
        let p99 = v[idx(0.99)];
        let mean: f64 = v.iter().copied().map(|x| x as f64).sum::<f64>() / v.len() as f64;
        analysis.push_str(&format!(
            "\nTimings (per-file AST analysis): p50={}ms, p95={}ms, p99={}ms, mean={:.1}ms, n={}\n",
            p50,
            p95,
            p99,
            mean,
            v.len()
        ));
    }

    analysis
}

/// Recursively analyze directory for code files  
#[allow(clippy::too_many_arguments)]
async fn analyze_directory_recursive(
    path: &std::path::Path,
    results: &mut Vec<(String, usize, Vec<String>)>,
    total_issues: &mut usize,
    total_files: &mut usize,
    critical_issues: &mut Vec<String>,
    gitignore_patterns: &[String],
    depth: usize,
    skipped_large_files: &mut usize,
    skipped_error_files: &mut usize,
    timings_enabled: bool,
    timings_ms: &mut Vec<u128>,
) -> Result<()> {
    // Depth limit to prevent infinite recursion - properly enforced
    const MAX_DEPTH: usize = 10;
    if depth >= MAX_DEPTH {
        if dev_flag_enabled("DEBUG_HOOKS") {
            tracing::debug!(max_depth=MAX_DEPTH, path=%path.display(), "Max depth reached");
        }
        return Ok(());
    }

    // Check if path should be ignored using proper gitignore pattern matching
    if should_ignore_path(path, gitignore_patterns) {
        return Ok(());
    }

    if path.is_file() {
        // Skip only truly non-code files (images, binaries, etc.)
        // Keep configuration files as they may contain security issues
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(
            extension,
            "md" | "txt"
                | "lock"  // lock files are auto-generated
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "svg"
                | "ico"
                | "pdf"
                | "zip"
                | "tar"
                | "gz"
                | "exe"
                | "dll"
                | "so"
        ) {
            return Ok(());
        }

        // Try to analyze the file
        if let Some(language) = SupportedLanguage::from_extension(extension) {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    // Skip very large files but track them
                    if content.len() > 500_000 {
                        *skipped_large_files += 1;
                        if dev_flag_enabled("DEBUG_HOOKS") {
                            tracing::debug!(bytes=%content.len(), path=%path.display(), "Skipped large file");
                        }
                        return Ok(());
                    }

                    // Enforce per-file AST analysis timeout
                    let timeout_secs: u64 = if cfg!(debug_assertions) { std::env::var("AST_ANALYSIS_TIMEOUT_SECS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(8) } else { 8 }
                        .clamp(1, 30);

                    let start = std::time::Instant::now();
                    let analysis = tokio::time::timeout(
                        std::time::Duration::from_secs(timeout_secs),
                        tokio::task::spawn_blocking({
                            let content = content.clone();
                            move || {
                                let scorer = AstQualityScorer::new();
                                scorer.analyze(&content, language)
                            }
                        }),
                    )
                    .await;

                    if let Ok(Ok(Ok(quality_score))) = analysis {
                        if timings_enabled {
                            let elapsed = start.elapsed().as_millis();
                            timings_ms.push(elapsed);
                        }
                        *total_files += 1;
                        let issues_count = quality_score.concrete_issues.len();
                        *total_issues += issues_count;

                        if issues_count > 0 {
                            let path_str = path.display().to_string();
                            let sample_issues: Vec<String> = quality_score
                                .concrete_issues
                                .iter()
                                .take(3)
                                .map(|issue| format!("{:?}", issue.category))
                                .collect();

                            // Collect critical issues
                            for issue in &quality_score.concrete_issues {
                                if issue.severity == IssueSeverity::Critical {
                                    critical_issues.push(format!(
                                        "{}: {} (line {})",
                                        path_str, issue.message, issue.line
                                    ));
                                }
                            }

                            results.push((path_str, issues_count, sample_issues));
                        }
                    } else if let Ok(Err(join_err)) = analysis {
                        *skipped_error_files += 1;
                        if dev_flag_enabled("DEBUG_HOOKS") {
                            tracing::debug!(path=%path.display(), error=%join_err, "AST analysis join error");
                        }
                    } else if analysis.is_err() && dev_flag_enabled("DEBUG_HOOKS") {
                        tracing::debug!(timeout=%timeout_secs, path=%path.display(), "AST analysis timeout");
                    }
                }
                Err(e) => {
                    *skipped_error_files += 1;
                    if dev_flag_enabled("DEBUG_HOOKS") {
                        tracing::debug!(path=%path.display(), error=%e, "Error reading file");
                    }
                }
            }
        }
    } else if path.is_dir() {
        // Skip common non-source directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.')
                || name == "node_modules"
                || name == "target"
                || name == "dist"
                || name == "build"
                || name == "vendor"
                || name == "__pycache__"
                || name == ".git"
            {
                return Ok(());
            }
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                Box::pin(analyze_directory_recursive(
                    &entry.path(),
                    results,
                    total_issues,
                    total_files,
                    critical_issues,
                    gitignore_patterns,
                    depth + 1,
                    skipped_large_files,
                    skipped_error_files,
                    timings_enabled,
                    timings_ms,
                ))
                .await?;
            }
        }
    }

    Ok(())
}

/// Main function for the PostToolUse hook
#[tokio::main]
async fn main() -> Result<()> {
    // Defensive: log any unexpected panic as a structured error
    std::panic::set_hook(Box::new(|info| {
        rust_validation_hooks::telemetry::init();
        tracing::error!(panic=%info, "panic in posttooluse");
    }));
    // Initialize structured logging (stderr). Safe to call multiple times.
    rust_validation_hooks::telemetry::init();
    // Limit stdin input size to prevent DoS attacks
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB limit

    // Read hook input from stdin with size limit
    let mut buffer = String::new();
    let stdin = io::stdin();
    let handle = stdin.lock();

    // Use take() to limit the amount of data read
    use std::io::Read;
    let mut limited_reader = handle.take(MAX_INPUT_SIZE as u64);
    limited_reader
        .read_to_string(&mut buffer)
        .context("Failed to read stdin")?;

    // Check if we hit the size limit
    if buffer.len() >= MAX_INPUT_SIZE {
        anyhow::bail!("Input exceeds maximum size of {}MB", MAX_INPUT_SIZE / 1024 / 1024);
    }

    // Parse the input
    let hook_input: HookInput = serde_json::from_str(&buffer).context("Failed to parse input JSON")?;

    // DEBUG: Write the exact hook input to a file for inspection
    if let Ok(mut debug_file) = tokio::fs::File::create("hook-input-debug.json").await {
        use tokio::io::AsyncWriteExt;
        if let Err(e) = debug_file.write_all(buffer.as_bytes()).await {
            tracing::debug!(error=%e, "Failed to write hook input");
        }
        tracing::debug!("Full hook input written to hook-input-debug.json");
    }

    // Only analyze Write, Edit, and MultiEdit operations
    if !matches!(hook_input.tool_name.as_str(), "Write" | "Edit" | "MultiEdit") {
        // Pass through - not a code modification
        let output = PostToolUseOutput {
            hook_specific_output: PostToolUseHookOutput {
                hook_event_name: "PostToolUse".to_string(),
                additional_context: String::new(),
            },
        };
        println!(
            "{}",
            serde_json::to_string(&output).context("Failed to serialize output")?
        );
        return Ok(());
    }

    // Get the file path and new content from tool input
    let file_path = hook_input
        .tool_input
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Skip non-code files
    if file_path.ends_with(".md")
        || file_path.ends_with(".txt")
        || file_path.ends_with(".json")
        || file_path.ends_with(".toml")
        || file_path.ends_with(".yaml")
        || file_path.ends_with(".yml")
    {
        // Pass through - not a code file
        let output = PostToolUseOutput {
            hook_specific_output: PostToolUseHookOutput {
                hook_event_name: "PostToolUse".to_string(),
                additional_context: String::new(),
            },
        };
        println!(
            "{}",
            serde_json::to_string(&output).context("Failed to serialize output")?
        );
        return Ok(());
    }

    // For AST analysis, we need the COMPLETE file content after the operation
    // Since PostToolUse runs AFTER the operation, read the actual file from disk
    let content = match read_file_content_safe(file_path).await? {
        Some(file_content) => {
        if dev_flag_enabled("DEBUG_HOOKS") {
            tracing::debug!(bytes=%file_content.len(), file=%file_path, "Read file");
        }
            file_content
        }
        None => {
        if dev_flag_enabled("DEBUG_HOOKS") {
            tracing::debug!(%file_path, "Could not read file content");
        }
            // Fallback to extracting partial content from tool_input if file read fails
            match normalize_tool_name(&hook_input.tool_name).0 {
                ToolKind::Write => hook_input
                    .tool_input
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                ToolKind::Edit => hook_input
                    .tool_input
                    .get("new_string")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                ToolKind::MultiEdit => {
                    // For MultiEdit, try to aggregate all new_strings for partial analysis
                    // This gives us at least some content to analyze, even if not the full file
                    // Note: We preserve order of edits and limit memory usage
                    if let Some(edits) = hook_input.tool_input.get("edits").and_then(|v| v.as_array()) {
                        // Pre-calculate capacity to avoid multiple allocations
                        let estimated_capacity: usize = edits
                            .iter()
                            .filter_map(|edit| {
                                edit.get("new_string").and_then(|v| v.as_str()).map(|s| s.len())
                            })
                            .sum::<usize>()
                            + (edits.len() * 2); // Add space for separators

                        let mut aggregated = String::with_capacity(estimated_capacity.min(1024 * 1024)); // Cap at 1MB
                        let mut valid_edits = 0;

                        for edit in edits.iter().take(1000) {
                            // Limit to prevent DoS
                            if let Some(new_string) = edit.get("new_string").and_then(|v| v.as_str()) {
                                if !new_string.is_empty() {
                                    if valid_edits > 0 {
                                        aggregated.push('\n');
                                    }
                                    aggregated.push_str(new_string);
                                    valid_edits += 1;
                                }
                            }
                        }

                if dev_flag_enabled("DEBUG_HOOKS") {
                    tracing::debug!(bytes=%aggregated.len(), valid_edits=%valid_edits, total_edits=%edits.len(), "MultiEdit fallback aggregation");
                }
                        aggregated
                    } else {
                        String::new()
                    }
                }
                ToolKind::Other => String::new(),
            }
        }
    };

    // Skip if no content to analyze
    if content.trim().is_empty() {
        let output = PostToolUseOutput {
            hook_specific_output: PostToolUseHookOutput {
                hook_event_name: "PostToolUse".to_string(),
                additional_context: String::new(),
            },
        };
        println!(
            "{}",
            serde_json::to_string(&output).context("Failed to serialize output")?
        );
        return Ok(());
    }

    // Perform AST-based quality analysis for deterministic scoring
    if dev_flag_enabled("DEBUG_HOOKS") {
        tracing::debug!(%file_path, "Starting AST analysis for file");
    }
    let ast_analysis = perform_ast_analysis(&content, file_path).await;
    if dev_flag_enabled("DEBUG_HOOKS") {
        tracing::debug!(has_result=%ast_analysis.is_some(), "AST analysis result");
    }

    // Perform code formatting after AST analysis and ACTUALLY APPLY IT TO THE FILE
    let formatting_result = match validate_file_path(file_path) {
        Ok(validated_path) => {
            match FormattingService::new() {
                Ok(formatting_service) => {
                    // Format and write file atomically if changes are needed
                    match formatting_service.format_and_write_file(&validated_path) {
                        Ok(format_result) => {
                            // Log success without exposing file contents
                            if format_result.changed {
                                tracing::info!("Code formatting applied successfully - file updated");
                            } else {
                                tracing::info!("No formatting changes needed");
                            }
                            Some(format_result)
                        }
                        Err(_) => {
                            // Don't expose formatting errors - they may contain sensitive paths/content
                            tracing::info!("Code formatting skipped due to formatter limitations");
                            None
                        }
                    }
                }
                Err(_) => {
                    tracing::warn!("Formatting service initialization failed");
                    None
                }
            }
        }
        Err(_) => {
            // Path validation failed - skip formatting for security
            tracing::warn!("Formatting skipped - invalid file path");
            None
        }
    };

    // In offline/e2e tests we may want to skip network and prompt loading entirely
    if dev_flag_enabled("POSTTOOL_AST_ONLY") {
        let mut final_response = String::new();
        let display_path = hook_input
            .tool_input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let change = build_change_summary(&hook_input, display_path).await;
        if !change.is_empty() {
            final_response.push_str(&change);
            final_response.push('\n');
        }
        if let Some(note) = soft_budget_note(&content, display_path) {
            final_response.push_str(&note);
            final_response.push('\n');
        }
        if let Some(ast_score) = &ast_analysis {
            let (filtered, change_snippets) =
                if let Ok(diff) = generate_diff_context(&hook_input, display_path).await {
                    let ctxn = if cfg!(debug_assertions) { std::env::var("AST_DIFF_CONTEXT")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(3) } else { 3 };
                    let changed = extract_changed_lines(&diff, ctxn);
                    let filtered = if dev_flag_enabled("AST_DIFF_ONLY") {
                        filter_issues_to_diff(ast_score, &diff, ctxn)
                    } else {
                        ast_score.clone()
                    };
                    let snips_enabled = if cfg!(debug_assertions) { std::env::var("AST_SNIPPETS").map(|v| v != "0").unwrap_or(true) } else { true };
                    let snips = if snips_enabled {
                        let max_snips = if cfg!(debug_assertions) { std::env::var("AST_MAX_SNIPPETS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(3) } else { 3 }
                            .clamp(1, 50);
                        let max_chars = if cfg!(debug_assertions) { std::env::var("AST_SNIPPETS_MAX_CHARS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(1500) } else { 1500 }
                            .clamp(200, 20_000);
                        let lang =
                            SupportedLanguage::from_extension(file_path.split('.').next_back().unwrap_or(""))
                                .unwrap_or(SupportedLanguage::Python);
                        let use_entity = if cfg!(debug_assertions) { std::env::var("AST_ENTITY_SNIPPETS").map(|v| v != "0").unwrap_or(true) } else { true };
                        if use_entity {
                            let ent = build_entity_context_snippets(
                                lang, &content, &filtered, &changed, ctxn, max_snips, max_chars,
                            );
                            if !ent.is_empty() {
                                ent
                            } else {
                                build_change_context_snippets(
                                    &content, &filtered, &changed, ctxn, max_snips, max_chars,
                                )
                            }
                        } else {
                            build_change_context_snippets(
                                &content, &filtered, &changed, ctxn, max_snips, max_chars,
                            )
                        }
                    } else {
                        String::new()
                    };
                    (filtered, snips)
                } else {
                    (ast_score.clone(), String::new())
                };
            final_response.push_str(&build_risk_report(&filtered));
            final_response.push('\n');
            let unfinished = build_unfinished_work_section(&filtered, 6, 120);
            if !unfinished.is_empty() {
                final_response.push_str(&unfinished);
                final_response.push('\n');
            }
            let tips = build_quick_tips_section(&filtered);
            if !tips.is_empty() {
                final_response.push_str(&tips);
                final_response.push('\n');
            }
            if !change_snippets.is_empty() {
                final_response.push_str(&change_snippets);
                final_response.push('\n');
            }
            final_response.push_str(&build_code_health(&filtered));
            final_response.push('\n');
            // API Contract (simple heuristics)
            let lang = SupportedLanguage::from_extension(file_path.split('.').next_back().unwrap_or(""))
                .unwrap_or(SupportedLanguage::Python);
            let api = build_api_contract_report(lang, &hook_input, &content);
            if !api.is_empty() {
                final_response.push_str(&api);
                final_response.push('\n');
            }
            final_response.push_str(&build_next_steps(&filtered));
            final_response.push('\n');
        }
        if let Some(format_result) = &formatting_result {
            if format_result.changed {
                final_response.push_str("[FORMAT] Auto-format applied.\n\n");
            }
        }
        // Append timings if enabled
        if dev_flag_enabled("AST_TIMINGS") && crate::analysis::timings::enabled() {
            let sum = crate::analysis::timings::summary();
            if !sum.is_empty() {
                final_response.push_str(&sum);
                final_response.push('\n');
            }
        }
        let output = PostToolUseOutput {
            hook_specific_output: PostToolUseHookOutput {
                hook_event_name: "PostToolUse".to_string(),
                additional_context: {
            let lim = if cfg!(debug_assertions) { std::env::var("ADDITIONAL_CONTEXT_LIMIT_CHARS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100_000) } else { 100_000 }
                .clamp(10_000, 1_000_000);
                    truncate_utf8_safe(&final_response, lim)
                },
            },
        };
        println!(
            "{}",
            serde_json::to_string(&output).context("Failed to serialize output")?
        );
        return Ok(());
    }

    // Load configuration from environment with graceful degradation (.env next to executable)
    let config = Config::from_env_graceful().context("Failed to load configuration")?;

    // If missing API key for selected provider, fall back to offline rendering (symmetry with PreToolUse)
    if config
        .get_api_key_for_provider(&config.posttool_provider)
        .is_empty()
        && !dev_flag_enabled("POSTTOOL_DRY_RUN")
    {
        tracing::warn!(provider=%config.posttool_provider, "No API key; falling back to AST-only rendering");
        let display_path = hook_input
            .tool_input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        return render_offline_posttooluse(
            &hook_input,
            &content,
            display_path,
            &ast_analysis,
            formatting_result.as_ref().map(|f| f.changed).unwrap_or(false),
        ).await;
    }

    // Load the analysis prompt
    let prompt = load_prompt_file("post_edit_validation.txt")
        .await
        .context("Failed to load prompt")?;

    // Get project structure with caching and metrics
    let cache_path = PathBuf::from(".claude_project_cache.json");
    let project_context = match scan_project_with_cache(".", Some(&cache_path), None) {
        Ok((structure, metrics, incremental)) => {
            // NEVER compress - always pass full structure to AI validator
            // Compression was causing truncated structure in context
            let use_compression = false; // Changed: Always pass full structure
            let mut formatted =
                format_project_structure_for_ai_with_metrics(&structure, Some(&metrics), use_compression);

            // Add incremental update info if available
            if let Some(inc) = incremental {
                formatted.push_str(&format!("\n{inc}"));
            }

            // Log metrics for debugging (stderr)
            tracing::debug!(loc=%metrics.total_lines_of_code, files=%structure.files.len(), complexity=%metrics.project_complexity_score, "Project metrics");

            Some(formatted)
        }
        Err(e) => {
            tracing::warn!(error=%e, "Failed to scan project structure");
            None
        }
    };

    // Detect duplicate and conflicting files
    let duplicate_report = {
        let mut detector = DuplicateDetector::new();
        match detector.scan_directory(std::path::Path::new(".")) {
            Ok(_) => {
                let duplicates = detector.find_duplicates();
                if !duplicates.is_empty() {
                    tracing::info!(groups=%duplicates.len(), "Found duplicate/conflict groups");
                    Some(detector.format_report(&duplicates))
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::warn!(error=%e, "Failed to scan for duplicates");
                None
            }
        }
    };

    // Analyze project dependencies for AI context
    let dependencies_context = match analyze_project_dependencies(std::path::Path::new(".")).await {
        Ok(deps) => {
            tracing::info!(total=%deps.total_count, "Dependencies analysis summary");
            if deps.outdated_count > 0 {
                tracing::info!(outdated=%deps.outdated_count, "Potentially outdated dependencies found");
            }
            Some(deps.format_for_ai())
        }
        Err(e) => {
            tracing::warn!(error=%e, "Failed to analyze dependencies");
            None
        }
    };

    // Construct full path for display in diff context
    let display_path = if let Some(cwd) = &hook_input.cwd {
        if file_path.starts_with('/') || file_path.starts_with('\\') || file_path.contains(':') {
            // Already an absolute path
            file_path.to_string()
        } else {
            // Relative path - combine with cwd
            format!("{}/{}", cwd.trim_end_matches(&['/', '\\'][..]), file_path)
        }
    } else {
        file_path.to_string()
    };

    // Generate diff context for the code changes
    let diff_context = match generate_diff_context(&hook_input, &display_path).await {
        Ok(diff) => diff,
        Err(e) => {
            // Log error but continue with analysis without diff
            tracing::warn!(error=%e, "Failed to generate diff context");
            String::new()
        }
    };

    // Read conversation transcript for context (10 messages, max 2000 chars)
    let transcript_context = if let Some(transcript_path) = &hook_input.transcript_path {
        match read_transcript_summary(transcript_path, 10, 2000).await {
            Ok(summary) => {
                if dev_flag_enabled("DEBUG_HOOKS") {
                    tracing::debug!("Transcript summary read successfully");
                    // Safe UTF-8 truncation using standard library
                    let truncated: String = summary.chars().take(500).collect();
                    tracing::debug!(first500=%truncated, "Transcript summary first chars");
                }
                Some(summary)
            }
            Err(e) => {
                tracing::warn!(error=%e, "Failed to read transcript");
                None
            }
        }
    } else {
        if dev_flag_enabled("DEBUG_HOOKS") {
            tracing::debug!("No transcript_path provided");
        }
        None
    };

    // Include AST analysis results in context if available
    let ast_context = ast_analysis.as_ref().map(format_ast_results);

    // Perform project-wide AST analysis for comprehensive context
    let project_ast_analysis = perform_project_ast_analysis(".").await;

    // Combine project context with dependencies analysis and duplicate report
    let combined_project_context = {
        let mut context_parts = Vec::new();

        if let Some(project) = project_context.as_deref() {
            context_parts.push(project.to_string());
        }

        if let Some(deps) = dependencies_context.as_deref() {
            context_parts.push(deps.to_string());
        }

        // Add duplicate report as critical context if found
        if let Some(duplicates) = duplicate_report.as_deref() {
            context_parts.push(duplicates.to_string());
        }

        // Add project-wide AST analysis if available
        if !project_ast_analysis.is_empty() {
            context_parts.push(project_ast_analysis);
        }

        if !context_parts.is_empty() {
            Some(context_parts.join("\n"))
        } else {
            None
        }
    };

    // Format the prompt with context, diff, conversation, and AST analysis
    let formatted_prompt = format_analysis_prompt_with_ast(
        &prompt,
        combined_project_context.as_deref(),
        Some(&diff_context),
        transcript_context.as_deref(),
        ast_context.as_deref(),
    )
    .await?;

    // DEBUG: Write the exact prompt to a file for inspection
    if dev_flag_enabled("DEBUG_HOOKS") {
        if let Ok(mut debug_file) = tokio::fs::File::create("post-context.txt").await {
            use tokio::io::AsyncWriteExt;
            let _ = debug_file.write_all(formatted_prompt.as_bytes()).await;
            let _ = debug_file.write_all(b"\n\n=== END OF PROMPT ===\n").await;
            tracing::debug!("Full prompt written to post-context.txt");
        }
    }

    // Dry-run mode: build prompt and contexts, skip network call; return structured AST details in additionalContext
    if dev_flag_enabled("POSTTOOL_DRY_RUN") {
        let mut final_response = String::new();
        // Include change summary
        let change = build_change_summary(&hook_input, &display_path).await;
        if !change.is_empty() {
            final_response.push_str(&change);
            final_response.push('\n');
        }
        // Uniformly include soft-budget note in DRY_RUN as well
        if let Some(note) = soft_budget_note(&content, &display_path) {
            final_response.push_str(&note);
            final_response.push('\n');
        }
        // Include AST structured sections
        if let Some(ast_score) = &ast_analysis {
            let (filtered, change_snippets) =
                if let Ok(diff) = generate_diff_context(&hook_input, &display_path).await {
                    let ctxn = std::env::var("AST_DIFF_CONTEXT")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(3);
                    let changed = extract_changed_lines(&diff, ctxn);
                    let filtered = if std::env::var("AST_DIFF_ONLY").is_ok() {
                        filter_issues_to_diff(ast_score, &diff, ctxn)
                    } else {
                        ast_score.clone()
                    };
                    let snips_enabled = std::env::var("AST_SNIPPETS").map(|v| v != "0").unwrap_or(true);
                    let snips = if snips_enabled {
                        let max_snips = std::env::var("AST_MAX_SNIPPETS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(3)
                            .clamp(1, 50);
                        let max_chars = std::env::var("AST_SNIPPETS_MAX_CHARS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(1500)
                            .clamp(200, 20_000);
                        let lang =
                            SupportedLanguage::from_extension(file_path.split('.').next_back().unwrap_or(""))
                                .unwrap_or(SupportedLanguage::Python);
                        let use_entity = std::env::var("AST_ENTITY_SNIPPETS")
                            .map(|v| v != "0")
                            .unwrap_or(true);
                        if use_entity {
                            let ent = build_entity_context_snippets(
                                lang, &content, &filtered, &changed, ctxn, max_snips, max_chars,
                            );
                            if !ent.is_empty() {
                                ent
                            } else {
                                build_change_context_snippets(
                                    &content, &filtered, &changed, ctxn, max_snips, max_chars,
                                )
                            }
                        } else {
                            build_change_context_snippets(
                                &content, &filtered, &changed, ctxn, max_snips, max_chars,
                            )
                        }
                    } else {
                        String::new()
                    };
                    (filtered, snips)
                } else {
                    (ast_score.clone(), String::new())
                };
            final_response.push_str(&build_risk_report(&filtered));
            final_response.push('\n');
            if !change_snippets.is_empty() {
                final_response.push_str(&change_snippets);
                final_response.push('\n');
            }
            final_response.push_str(&build_code_health(&filtered));
            final_response.push('\n');
            // API Contract (simple heuristics)
            let lang = SupportedLanguage::from_extension(file_path.split('.').next_back().unwrap_or(""))
                .unwrap_or(SupportedLanguage::Python);
            let api = build_api_contract_report(lang, &hook_input, &content);
            if !api.is_empty() {
                final_response.push_str(&api);
                final_response.push('\n');
            }
            final_response.push_str(&build_next_steps(&filtered));
            final_response.push('\n');
        }
        // Include formatting outcome
        if let Some(format_result) = &formatting_result {
            if format_result.changed {
                final_response.push_str("[АВТОФОРМАТИРОВАНИЕ ПРИМЕНЕНО]\n\n");
            } else if !format_result.messages.is_empty() {
                final_response.push_str("[ФОРМАТИРОВАНИЕ] ");
                for message in &format_result.messages {
                    final_response.push_str(&format!("{} ", message));
                }
                final_response.push_str("\n\n");
            }
        }

        let output = PostToolUseOutput {
            hook_specific_output: PostToolUseHookOutput {
                hook_event_name: "PostToolUse".to_string(),
                additional_context: {
                    let lim = std::env::var("ADDITIONAL_CONTEXT_LIMIT_CHARS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(100_000)
                        .clamp(10_000, 1_000_000);
                    truncate_utf8_safe(&final_response, lim)
                },
            },
        };
        println!(
            "{}",
            serde_json::to_string(&output).context("Failed to serialize output")?
        );
        return Ok(());
    }

    // Create AI client and perform analysis
    let client = UniversalAIClient::new(config.clone()).context("Failed to create AI client")?;

    // Analyze code using the configured provider - returns raw response
    match client.analyze_code_posttool(&content, &formatted_prompt).await {
        Ok(ai_response) => {
            // Combine structured sections and AI response
            let mut final_response = String::new();
            // Change Summary
            let change = build_change_summary(&hook_input, &display_path).await;
            if !change.is_empty() {
                final_response.push_str(&change);
                final_response.push('\n');
            }
            if let Some(note) = soft_budget_note(&content, &display_path) {
                final_response.push_str(&note);
                final_response.push('\n');
            }
            // Risk + Health + Change Context + API Contract + Next Steps
            if let Some(ast_score) = &ast_analysis {
                let (filtered, change_snippets) = if let Ok(diff) =
                    generate_diff_context(&hook_input, &display_path).await
                {
                    let ctxn = std::env::var("AST_DIFF_CONTEXT")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(3);
                    let changed = extract_changed_lines(&diff, ctxn);
                    let filtered = if std::env::var("AST_DIFF_ONLY").is_ok() {
                        filter_issues_to_diff(ast_score, &diff, ctxn)
                    } else {
                        ast_score.clone()
                    };
                    let snips_enabled = std::env::var("AST_SNIPPETS").map(|v| v != "0").unwrap_or(true);
                    let snips = if snips_enabled {
                        let max_snips = std::env::var("AST_MAX_SNIPPETS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(3)
                            .clamp(1, 50);
                        let max_chars = std::env::var("AST_SNIPPETS_MAX_CHARS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(1500)
                            .clamp(200, 20_000);
                        let lang =
                            SupportedLanguage::from_extension(file_path.split('.').next_back().unwrap_or(""))
                                .unwrap_or(SupportedLanguage::Python);
                        let use_entity = std::env::var("AST_ENTITY_SNIPPETS")
                            .map(|v| v != "0")
                            .unwrap_or(true);
                        if use_entity {
                            let ent = build_entity_context_snippets(
                                lang, &content, &filtered, &changed, ctxn, max_snips, max_chars,
                            );
                            if !ent.is_empty() {
                                ent
                            } else {
                                build_change_context_snippets(
                                    &content, &filtered, &changed, ctxn, max_snips, max_chars,
                                )
                            }
                        } else {
                            build_change_context_snippets(
                                &content, &filtered, &changed, ctxn, max_snips, max_chars,
                            )
                        }
                    } else {
                        String::new()
                    };
                    (filtered, snips)
                } else {
                    (ast_score.clone(), String::new())
                };
                final_response.push_str(&build_risk_report(&filtered));
                final_response.push('\n');
                let tips = build_quick_tips_section(&filtered);
                if !tips.is_empty() {
                    final_response.push_str(&tips);
                    final_response.push('\n');
                }
                if !change_snippets.is_empty() {
                    final_response.push_str(&change_snippets);
                    final_response.push('\n');
                }
                final_response.push_str(&build_code_health(&filtered));
                final_response.push('\n');
                let lang = SupportedLanguage::from_extension(file_path.split('.').next_back().unwrap_or(""))
                    .unwrap_or(SupportedLanguage::Python);
                let api = build_api_contract_report(lang, &hook_input, &content);
                if !api.is_empty() {
                    final_response.push_str(&api);
                    final_response.push('\n');
                }
                final_response.push_str(&build_next_steps(&filtered));
                final_response.push('\n');
            }
            // Add formatting results if available
            if let Some(format_result) = &formatting_result {
                if format_result.changed {
                    final_response.push_str("[АВТОФОРМАТИРОВАНИЕ ПРИМЕНЕНО]\n\n");
                } else if !format_result.messages.is_empty() {
                    final_response.push_str("[ФОРМАТИРОВАНИЕ] ");
                    for message in &format_result.messages {
                        final_response.push_str(&format!("{} ", message));
                    }
                    final_response.push_str("\n\n");
                }
            }

            // Add AI response (with OpenAI JSON wrapper or fallback JSON from AST)
            let wrapped = if config.posttool_provider == AIProvider::OpenAI {
                let content = if ai_response.trim().is_empty() {
                    if let Some(ast_score) = &ast_analysis { build_agent_json_from_score(ast_score) } else { String::from("{}") }
                } else { ai_response };
                format!("AGENT_JSON_START\n{}\nAGENT_JSON_END\n", content)
            } else {
                ai_response
            };
            final_response.push_str(&wrapped);

            let output = PostToolUseOutput {
                hook_specific_output: PostToolUseHookOutput {
                    hook_event_name: "PostToolUse".to_string(),
                    additional_context: {
                        let lim = std::env::var("ADDITIONAL_CONTEXT_LIMIT_CHARS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(100_000)
                            .clamp(10_000, 1_000_000);
                        truncate_utf8_safe(&final_response, lim)
                    },
                },
            };

            println!(
                "{}",
                serde_json::to_string(&output).context("Failed to serialize output")?
            );
        }
        Err(e) => {
            // Log error and gracefully fall back to offline AST renderer
            tracing::error!(error=%e, "PostToolUse analysis error");

            let display_path = hook_input
                .tool_input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            return render_offline_posttooluse(
                &hook_input,
                &content,
                display_path,
                &ast_analysis,
                formatting_result.as_ref().map(|f| f.changed).unwrap_or(false),
            )
            .await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // std::fs already imported elsewhere in this tests module
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_transcript_summary_with_user_messages() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Help me write a function"}}"#).unwrap();
        writeln!(
            file,
            r#"{{"role":"assistant","content":"I'll help you write a function"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"Make it handle errors properly"}}"#
        )
        .unwrap();
        drop(file); // Ensure file is closed

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        assert!(summary.contains("Current user task: Make it handle errors properly"));
        assert!(summary.contains("user: Help me write a function"));
        assert!(summary.contains("assistant: I'll help you write a function"));
    }

    #[tokio::test]
    async fn test_read_transcript_summary_truncation() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        // Create a very long message
        let long_message = "x".repeat(3000);
        writeln!(file, r#"{{"role":"user","content":"{}"}}"#, long_message).unwrap();
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        // Should be truncated to around 2000 chars
        assert!(summary.len() < 2500);
        assert!(summary.contains("Current user task:"));
    }

    #[tokio::test]
    async fn test_read_transcript_summary_empty_file() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        // Create empty file
        fs::File::create(&transcript_path).unwrap();

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        // Empty file returns just the header
        assert_eq!(summary.trim(), "conversation:");
    }

    #[tokio::test]
    async fn test_read_transcript_summary_nonexistent_file() {
        let result = read_transcript_summary("/nonexistent/path/transcript.jsonl", 20, 2000).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_transcript_summary_invalid_json() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        writeln!(file, "not valid json").unwrap();
        writeln!(file, r#"{{"role":"user","content":"Valid message"}}"#).unwrap();
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        // Should skip invalid lines and process valid ones
        assert!(summary.contains("user: Valid message"));
    }

    #[tokio::test]
    async fn test_read_transcript_summary_complex_content() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        // Message with content array (assistant format)
        writeln!(file, r#"{{"message":{{"role":"assistant","content":[{{"text":"I'll help"}},{{"name":"Edit","input":{{"file_path":"test.rs"}}}}]}},"timestamp":"2024-01-01T00:00:00Z"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Thanks"}}"#).unwrap();
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        assert!(summary.contains("assistant: I'll help Edit tool file: test.rs"));
        assert!(summary.contains("Current user task: Thanks"));
    }

    #[tokio::test]
    async fn test_read_transcript_summary_message_order() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        writeln!(file, r#"{{"role":"user","content":"First message"}}"#).unwrap();
        writeln!(file, r#"{{"role":"assistant","content":"Response"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Second message"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Most recent message"}}"#).unwrap();
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        // Should identify the most recent user message
        assert!(summary.contains("Current user task: Most recent message"));
        // Messages should appear in order in conversation
        // Note: "Most recent message" appears in header, so we check just First and Second in conversation
        if let (Some(first_pos), Some(second_pos)) = (
            summary.find("user: First message"),
            summary.find("user: Second message"),
        ) {
            assert!(first_pos < second_pos, "Messages not in chronological order");
        } else {
            panic!("Expected messages not found in summary");
        }
        // The most recent message should be in the header
        assert!(summary.starts_with("Current user task: Most recent message"));
    }

    #[tokio::test]
    async fn test_validate_transcript_path() {
        // These should fail validation
        assert!(validate_transcript_path("../../etc/passwd").is_err());
        assert!(validate_transcript_path("~/secrets").is_err());
        // UNC: на Windows допустим, на non-Windows — запрещён
        if cfg!(windows) {
            assert!(validate_transcript_path(r"\\\\server\\share").is_ok());
        } else {
            assert!(validate_transcript_path("\\\\server\\share").is_err());
        }
        assert!(validate_transcript_path("file%2e%2e/secrets").is_err());
        assert!(validate_transcript_path("file\0with\0nulls").is_err());

        // These paths would need to exist and be in allowed directories to pass
        // For test purposes, we test with temp directory paths
        let temp_dir = std::env::temp_dir();
        let valid_path = temp_dir.join("transcript.jsonl");

        // Create a test file in temp directory
        if let Ok(mut file) = std::fs::File::create(&valid_path) {
            use std::io::Write;
            let _ = writeln!(file, "{{}}");

            // This should pass validation as it's in temp directory
            assert!(validate_transcript_path(valid_path.to_str().unwrap()).is_ok());

            // Clean up
            let _ = std::fs::remove_file(&valid_path);
        }
    }

    #[tokio::test]
    async fn test_read_transcript_summary_message_limit_reached_before_char_limit() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        // Create more messages than max_messages
        for i in 0..10 {
            writeln!(file, r#"{{"role":"user","content":"Message {}"}}"#, i).unwrap();
        }
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 5, 10000)
            .await
            .unwrap();

        // Should only contain 5 messages even though char limit not reached
        let message_count = summary.matches("user:").count() + summary.matches("assistant:").count();
        assert_eq!(message_count, 5);
    }

    #[test]
    fn normalize_tool_name_aliases() {
        let cases = [
            ("Write", ToolKind::Write, false),
            ("WriteFile", ToolKind::Write, false),
            ("CreateFile", ToolKind::Write, false),
            ("AppendToFile", ToolKind::Write, true),
            ("Append", ToolKind::Write, true),
            ("Edit", ToolKind::Edit, false),
            ("ReplaceInFile", ToolKind::Edit, false),
            ("MultiEdit", ToolKind::MultiEdit, false),
            ("BatchEdit", ToolKind::MultiEdit, false),
            ("SomethingElse", ToolKind::Other, false),
        ];
        for (name, kind, append) in cases {
            let (k, a) = normalize_tool_name(name);
            assert_eq!(k, kind, "kind mismatch for {}", name);
            assert_eq!(a, append, "append mismatch for {}", name);
        }
    }

    #[test]
    fn should_ignore_path_normalizes_windows_separators() {
        let path = std::path::Path::new("dir\\node_modules\\file.js");
        let patterns = vec!["node_modules/".to_string()];
        assert!(should_ignore_path(path, &patterns));
    }

    #[test]
    fn validate_file_path_rejects_home_prefix_on_unix() {
        if !cfg!(windows) {
            let res = validate_file_path("~/secrets.txt");
            assert!(res.is_err(), "expected ~/ to be rejected on non-Windows");
        }
    }

    #[test]
    fn validate_file_path_allows_existing_under_cwd() {
        let cwd = std::env::current_dir().unwrap();
        let test_path = cwd.join(".tmp_validate_path_test.txt");
        fs::write(&test_path, "ok").unwrap();
        let res = validate_file_path(test_path.to_str().unwrap());
        fs::remove_file(&test_path).unwrap();
        assert!(res.is_ok());
    }

    #[test]
    fn validate_file_path_rejects_outside_cwd() {
        // Create two temp dirs and place file in other_dir, then set CWD to dir
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let file_outside = other.path().join("outside.txt");
        fs::write(&file_outside, "x").unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let res = validate_file_path(file_outside.to_str().unwrap());
        // Restore CWD
        std::env::set_current_dir(prev).unwrap();
        assert!(res.is_err(), "expected path outside working dir to be rejected");
    }

    #[tokio::test]
    async fn test_read_transcript_summary_char_limit_reached_before_message_limit() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        // Create messages with very long content
        let long_content = "x".repeat(500);
        for i in 0..10 {
            writeln!(
                file,
                r#"{{"role":"user","content":"Message {}: {}"}}"#,
                i, long_content
            )
            .unwrap();
        }
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 500)
            .await
            .unwrap();

        // Should stop before reaching message limit due to char limit
        // The limit is 500 chars, but we need to be precise
        assert!(summary.len() <= 550); // Allow small buffer for headers/formatting
        assert!(summary.len() >= 450); // Should be close to the limit
        let message_count = summary.matches("user:").count();
        assert!(message_count < 10);
        assert!(message_count > 0); // Should have at least one message
    }

    // ===== Diff-aware AST Slice: unit tests =====
    use crate::analysis::ast::quality_scorer::{ConcreteIssue, IssueCategory, IssueSeverity, QualityScore};

    fn mk_quality_score(lines: &[usize]) -> QualityScore {
        let mut s = QualityScore {
            total_score: 1000,
            functionality_score: 300,
            reliability_score: 200,
            maintainability_score: 200,
            performance_score: 150,
            security_score: 100,
            standards_score: 50,
            concrete_issues: Vec::new(),
        };
        for &ln in lines {
            s.concrete_issues.push(ConcreteIssue {
                severity: IssueSeverity::Major,
                category: IssueCategory::LongMethod,
                message: format!("issue at {ln}"),
                file: String::new(),
                line: ln,
                column: 1,
                rule_id: "TST001".to_string(),
                points_deducted: 0,
            });
        }
        s
    }

    #[test]
    fn unit_extract_changed_lines_and_filter_match_plus_lines() {
        let diff = "--- a/file\n+++ b/file\n@@ -4,3 +4,3 @@\n  4   same\n  5 - old\n  5 + new\n  6   same\n";
        let changed = extract_changed_lines(diff, 0);
        assert!(changed.contains(&5));
        let score = mk_quality_score(&[2, 5, 10]);
        let filtered = filter_issues_to_diff(&score, diff, 0);
        assert_eq!(filtered.concrete_issues.len(), 1);
        assert_eq!(filtered.concrete_issues[0].line, 5);
    }

    fn has_marked_line_for(out: &str, target: usize) -> bool {
        for l in out.lines() {
            let lt = l.trim_start();
            if !lt.starts_with('>') {
                continue;
            }
            if let Some(pipe_idx) = lt.find('|') {
                let num_part = &lt[1..pipe_idx];
                let num = num_part.trim();
                if let Ok(n) = num.parse::<usize>() {
                    if n == target {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[test]
    fn unit_entity_snippet_python_captures_function_scope() {
        let lang = SupportedLanguage::Python;
        let content = "\n\
def add(a, b):\n\
    s = a + b\n\
    return s\n\
\n";
        let score = mk_quality_score(&[3]);
        let mut changed = std::collections::HashSet::new();
        changed.insert(3usize);
        let out = build_entity_context_snippets(lang, content, &score, &changed, 1, 3, 2000);
        assert!(out.contains("=== CHANGE CONTEXT ==="));
        assert!(has_marked_line_for(&out, 3));
    }

    #[test]
    fn unit_entity_snippet_js_captures_function_scope() {
        let lang = SupportedLanguage::JavaScript;
        let content = "\nfunction sum(a, b){\n  const c = a + b;\n  return c;\n}\n";
        let score = mk_quality_score(&[3]);
        let mut changed = std::collections::HashSet::new();
        changed.insert(3usize);
        let out = build_entity_context_snippets(lang, content, &score, &changed, 1, 3, 2000);
        assert!(out.contains("=== CHANGE CONTEXT ==="));
        assert!(has_marked_line_for(&out, 3));
    }

    #[test]
    fn unit_entity_snippet_respects_max_snippets_cap() {
        let lang = SupportedLanguage::Python;
        let content = "\n\
def f1():\n  return 1\n\n\
def f2():\n  return 2\n\n\
def f3():\n  return 3\n";
        let score = mk_quality_score(&[2, 5, 8]);
        let mut changed = std::collections::HashSet::new();
        for &ln in &[2usize, 5, 8] {
            changed.insert(ln);
        }
        let out = build_entity_context_snippets(lang, content, &score, &changed, 1, 2, 10_000);
        let headers = out.lines().filter(|l| l.starts_with("- [")).count();
        assert_eq!(headers, 2);
    }

    #[test]
    fn unit_next_steps_recommendations_cover_common_categories() {
        use crate::analysis::ast::quality_scorer::{
            ConcreteIssue, IssueCategory, IssueSeverity, QualityScore,
        };
        let mut score = QualityScore {
            total_score: 900,
            functionality_score: 300,
            reliability_score: 200,
            maintainability_score: 200,
            performance_score: 100,
            security_score: 50,
            standards_score: 50,
            concrete_issues: Vec::new(),
        };
        let mut push = |cat: IssueCategory| {
            score.concrete_issues.push(ConcreteIssue {
                severity: IssueSeverity::Major,
                category: cat,
                message: String::new(),
                file: String::new(),
                line: 1,
                column: 1,
                rule_id: "TST".into(),
                points_deducted: 0,
            });
        };
        push(IssueCategory::UnreachableCode);
        push(IssueCategory::LongLine);
        push(IssueCategory::UnusedImports);
        push(IssueCategory::MissingDocumentation);
        let steps = build_next_steps(&score);
        assert!(steps.contains("Unreachable") || steps.contains("dead/unreachable"));
        assert!(steps.contains("Wrap lines") || steps.contains(">120"));
        assert!(steps.contains("unused imports"));
        assert!(steps.contains("docstrings") || steps.contains("public APIs"));
        assert!(steps.contains("Add/Update unit tests"));
    }

    #[tokio::test]
    async fn test_read_transcript_summary_various_malformed_json() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        // Various malformed JSON scenarios
        writeln!(file, r#"{{"role": "user", "content": "Valid message 1"}}"#).unwrap();
        writeln!(file, "{{{{invalid json}}}}").unwrap(); // Missing quotes
        writeln!(file, r#"{{"role":"user","content":"Valid message 2"}}"#).unwrap();
        file.write_all(b"{\"role\":\"user\",\"content\":\"Unclosed string\n")
            .unwrap(); // Unclosed
        writeln!(file, r#"{{"role":"user","content":"Valid message 3"}}"#).unwrap();
        writeln!(file).unwrap(); // Empty line
        writeln!(file, "null").unwrap(); // Just null
        writeln!(file, r#"{{"role":"user"}}"#).unwrap(); // Missing content field
        writeln!(file, r#"{{"content":"Missing role"}}"#).unwrap(); // Missing role field
        writeln!(file, r#"{{"role":"user","content":"Valid message 4"}}"#).unwrap();
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        // Should only process valid messages
        assert!(summary.contains("Valid message 1"));
        assert!(summary.contains("Valid message 2"));
        assert!(summary.contains("Valid message 3"));
        assert!(summary.contains("Valid message 4"));
        assert!(summary.contains("Current user task: Valid message 4"));
    }

    #[tokio::test]
    async fn test_read_transcript_summary_utf8_edge_cases() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        // Various UTF-8 edge cases
        writeln!(file, r#"{{"role":"user","content":"ASCII only message"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Emoji 🎉🚀💻 message"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Cyrillic текст сообщение"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"CJK 中文 日本語 한국어"}}"#).unwrap();
        writeln!(
            file,
            "{{\"role\":\"user\",\"content\":\"Zero-width \u{200B}\u{200C}\u{200D} chars\"}}"
        )
        .unwrap();
        writeln!(file, r#"{{"role":"user","content":"RTL text مرحبا עברית"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Combined 👨‍👩‍👧‍👦 emoji"}}"#).unwrap();
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        // Should handle all UTF-8 properly
        assert!(summary.contains("ASCII only"));
        assert!(summary.contains("🎉"));
        assert!(summary.contains("текст"));
        assert!(summary.contains("中文"));
        assert!(summary.contains("مرحبا"));
    }

    #[test]
    #[cfg(not(windows))]
    fn validate_transcript_path_rejects_unc_on_non_windows() {
        let unc = r"\\server\share\file.jsonl";
        let err = validate_transcript_path(unc).unwrap_err();
        assert!(format!("{err}").to_lowercase().contains("unc"));
    }

    #[test]
    #[cfg(windows)]
    fn validate_transcript_path_allows_backslash_under_temp_on_windows() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("transcript.jsonl");
        // Create file
        {
            let mut f = std::fs::File::create(&file).unwrap();
            let _ = writeln!(f, "{{}}\n");
        }
        // Build backslash path string
        let p = file.to_string_lossy().replace('/', "\\");
        // Should be allowed by validation on Windows
        assert!(validate_transcript_path(&p).is_ok());
    }

    #[tokio::test]
    async fn test_read_transcript_summary_large_file_with_seek() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        // Create a file larger than 1MB
        for i in 0..50000 {
            writeln!(file, r#"{{"role":"user","content":"Old message {}"}}"#, i).unwrap();
        }
        // Add recent messages at the end
        writeln!(file, r#"{{"role":"user","content":"Recent message 1"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Recent message 2"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Most recent message"}}"#).unwrap();
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        // Debug output to understand what we're getting
        println!("Summary for large file: {}", summary);

        // Should contain recent messages, not old ones from beginning
        assert!(summary.contains("Recent message") || summary.contains("user: Recent message"));
        assert!(summary.contains("Most recent message"));
        assert!(summary.contains("Current user task: Most recent message"));
        // Should NOT contain very old messages
        assert!(!summary.contains("Old message 0"));
        assert!(!summary.contains("Old message 1"));
    }

    #[tokio::test]
    async fn test_read_transcript_summary_nested_json_content() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        // Message with nested JSON in content - using write! to avoid formatting issues
        file.write_all(b"{\"role\":\"user\",\"content\":\"Here is JSON: {\\\"key\\\": \\\"value\\\"}\"}\n")
            .unwrap();
        // Message with escaped characters
        writeln!(
            file,
            r#"{{"role":"user","content":"Path: C:\\Users\\Test\\file.txt"}}"#
        )
        .unwrap();
        // Message with newlines
        writeln!(file, r#"{{"role":"user","content":"Line 1\nLine 2\nLine 3"}}"#).unwrap();
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        // Should handle nested/escaped content properly
        assert!(summary.contains("JSON"));
        assert!(summary.contains("Path"));
        assert!(summary.contains("Line"));
    }

    #[tokio::test]
    async fn test_read_transcript_summary_empty_content_messages() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        writeln!(file, r#"{{"role":"user","content":""}}"#).unwrap();
        writeln!(file, r#"{{"role":"assistant","content":""}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Non-empty message"}}"#).unwrap();
        writeln!(file, r#"{{"role":"assistant","content":[]}}"#).unwrap(); // Empty array
        writeln!(file, r#"{{"role":"assistant","content":[{{"text":""}}]}}"#).unwrap(); // Empty text in array
        drop(file);

        let summary = read_transcript_summary(transcript_path.to_str().unwrap(), 20, 2000)
            .await
            .unwrap();

        // Should handle empty content gracefully
        assert!(summary.contains("Non-empty message"));
        assert!(summary.contains("Current user task: Non-empty message"));
    }

    #[tokio::test]
    async fn test_format_multi_edit_diff_edge_cases() {
        use crate::validation::diff_formatter::format_multi_edit_diff;

        // Test with empty edits
        let result = format_multi_edit_diff("test.rs", Some("content"), &[]);
        assert!(result.contains("0 edit operations"));

        // Test with empty old_string (should handle gracefully)
        let edits = vec![("".to_string(), "new content".to_string())];
        let result = format_multi_edit_diff("test.rs", Some("file content"), &edits);
        assert!(result.contains("Edit #1 failed"));

        // Test with overlapping edits
        let edits = vec![
            ("hello".to_string(), "hi".to_string()),
            ("hello world".to_string(), "goodbye".to_string()),
        ];
        let result = format_multi_edit_diff("test.rs", Some("hello world"), &edits);
        assert!(result.contains("Applied"));

        // Test with no file content
        let edits = vec![("old".to_string(), "new".to_string())];
        let result = format_multi_edit_diff("test.rs", None, &edits);
        assert!(result.contains("File content not available"));
    }

    #[tokio::test]
    async fn test_truncate_for_display_with_special_chars() {
        use crate::validation::diff_formatter::truncate_for_display;

        // Test with control characters
        let text_with_control = "Hello\x00\x01\x02World";
        let result = truncate_for_display(text_with_control, 10);
        assert_eq!(result.len(), 10);

        // Test with combining characters
        let combining = "e\u{0301}"; // é as e + combining acute
        let result = truncate_for_display(combining, 5);
        assert!(result.len() <= 5);

        // Test with surrogate pairs edge case
        let text = "𝄞𝄞𝄞𝄞𝄞"; // Musical symbols (4 bytes each)
        let result = truncate_for_display(text, 10);
        // 𝄞 is 4 bytes, so with 10 byte limit: 4 bytes (𝄞) + 3 bytes (...) = 7 bytes, fits one symbol
        assert_eq!(result, "𝄞...");
    }

    #[tokio::test]
    async fn test_project_ast_analysis_includes_critical_and_is_deterministic() {
        // Prepare a temporary project with a Python file that triggers a critical issue
        let dir = tempfile::tempdir().unwrap();
        let py_path = dir.path().join("bad.py");
        // Hardcoded credential assignment should trigger HardcodedCredentials (Critical)
        let code = "password = 'supersecret'\nprint('ok')\n";
        tokio::fs::write(&py_path, code).await.unwrap();

        // Run project-wide AST analysis twice to verify determinism of output
        let wd = dir.path().to_str().unwrap();
        let out1 = perform_project_ast_analysis(wd).await;
        let out2 = perform_project_ast_analysis(wd).await;

        // Smoke checks: section header and file counters
        assert!(out1.contains("PROJECT-WIDE AST ANALYSIS"));
        assert!(out1.contains("Files analyzed:"));
        // Should reference the created file somewhere in the summary
        assert!(out1.contains("bad.py"));
        // Should indicate at least one critical issue found
        assert!(out1.contains("Critical issues:"));

        // Deterministic output across runs given same inputs
        assert_eq!(out1, out2);
    }
}



