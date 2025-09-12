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
use std::io::{self, Read};
use std::path::Path;

use rust_validation_hooks::*;
// Use project analysis for full context
use rust_validation_hooks::analysis::project::scan_project_with_cache;
// Use AST analysis for comprehensive error detection
use rust_validation_hooks::analysis::ast::{AstQualityScorer, SupportedLanguage};
// Use duplicate detector for conflict awareness
// Use dependency analysis
use rust_validation_hooks::analysis::dependencies::analyze_project_dependencies;

#[tokio::main]
async fn main() -> Result<()> {
    // Defensive: log any unexpected panic as a structured error
    rust_validation_hooks::telemetry::init();
    std::panic::set_hook(Box::new(|info| {
        tracing::error!(panic=%info, "panic in userpromptsubmit");
    }));
    // Read hook input from stdin
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read stdin")?;

    // Parse the input - handle empty or invalid JSON gracefully
    let hook_input: HookInput = match serde_json::from_str(&buffer) {
        Ok(input) => input,
        Err(_) => {
            // Create default input if JSON parsing fails
            // Silent fallback - no debug output
            HookInput {
                tool_name: "UserPromptSubmit".to_string(),
                tool_input: std::collections::HashMap::new(),
                tool_response: None,
                session_id: Some("default".to_string()),
                transcript_path: None,
                cwd: Some(".".to_string()),
                hook_event_name: Some("UserPromptSubmit".to_string()),
            }
        }
    };

    // Process hook silently without debug output

    // Fast-fail if cwd is provided but does not exist
    if let Some(cwd) = hook_input.cwd.as_deref() {
        if !std::path::Path::new(cwd).exists() {
            println!("Project analysis unavailable");
            return Ok(());
        }
    }

    // Build compact, deterministic context for UserPromptSubmit
    match build_compact_userprompt_context(&hook_input).await {
        Ok(analysis_context) => {
            // Output directly as text for UserPromptSubmit hook
            println!("{}", analysis_context);
        }
        Err(_e) => {
            // Silent failure for UserPromptSubmit
            println!("Project analysis unavailable");
        }
    }

    Ok(())
}

/// Build compact, deterministic context (E1): Project Summary + Risk/Health snapshot
async fn build_compact_userprompt_context(hook_input: &HookInput) -> Result<String> {
    let working_dir = hook_input.cwd.as_deref().unwrap_or(".");

    // 1) Project scan + metrics
    let (structure, metrics, _inc) =
        scan_project_with_cache(working_dir, None, None).context("scan_project_with_cache")?;

    // Top languages by file_count
    let mut langs: Vec<(String, u32)> = metrics
        .code_by_language
        .iter()
        .map(|(k, v)| (k.clone(), v.file_count as u32))
        .collect();
    langs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let top_langs = langs
        .iter()
        .take(3)
        .map(|(k, v)| format!("{k}: {v}"))
        .collect::<Vec<_>>()
        .join(", ");

    // 2) Dependencies (summary only)
    let deps = analyze_project_dependencies(Path::new(working_dir))
        .await
        .unwrap_or_default();

    // 3) Risk/Health snapshot across project files
    let rh = compute_risk_health_snapshot(working_dir).await;

    // === Sections ===
    let mut out = String::new();
    out.push_str("# COMPREHENSIVE PROJECT CONTEXT\n");
    out.push_str("\n=== PROJECT SUMMARY ===\n");
    out.push_str(&format!(
        "Files: {}\nLOC: {}\nTop languages: {}\nTests share: {:.0}%\nDocs share: {:.0}%\nComplexity (avg cyclomatic/cognitive): {:.1}/{:.1}\n",
        structure.total_files,
        metrics.total_lines_of_code,
        if top_langs.is_empty() { "n/a".to_string() } else { top_langs },
        metrics.test_coverage_estimate * 100.0,
        metrics.documentation_ratio * 100.0,
        metrics.average_cyclomatic_complexity,
        metrics.average_cognitive_complexity
    ));
    out.push_str(&format!("Dependencies: total {v0}, outdated {v0}\n", v0 = deps.total_count, v1 = deps.outdated_count));

    out.push_str("\n---\n\n=== RISK/HEALTH SNAPSHOT ===\n");
    if let Some(r) = rh {
        out.push_str(&format!("Issues: total {v0} (Critical {v0} / Major {v0} / Minor {v0})\n", v0 = r.total, v1 = r.critical, v2 = r.major, v3 = r.minor));
        if !r.top_categories.is_empty() {
            out.push_str("Top categories: ");
            out.push_str(
                &r.top_categories
                    .iter()
                    .map(|(k, v)| format!("{k}:{v}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push('\n');
        }
        out.push_str(&format!(
            "High-complexity files: {} (score>7)\n",
            metrics.high_complexity_files
        ));
    } else {
        out.push_str("No issues detected.\n");
    }

    // Truncate deterministically (no code fences), default 4000 chars
    let lim = std::env::var("USERPROMPT_CONTEXT_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .map(|n: usize| n.clamp(1000, 8000))
        .unwrap_or(4000);
    Ok(truncate_utf8_safe(&out, lim))
}

struct RiskHealth {
    total: usize,
    critical: usize,
    major: usize,
    minor: usize,
    top_categories: Vec<(String, usize)>,
}

async fn compute_risk_health_snapshot(working_dir: &str) -> Option<RiskHealth> {
    use rust_validation_hooks::analysis::ast::quality_scorer::IssueSeverity;
    use std::collections::HashMap;

    let mut total = 0usize;
    let mut crit = 0usize;
    let mut maj = 0usize;
    let mut min = 0usize;
    let mut by_cat: HashMap<String, usize> = HashMap::new();

    if let Ok(entries) = std::fs::read_dir(working_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if let Some(lang) = SupportedLanguage::from_extension(ext) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            // Skip huge files to keep latency low
                            if content.len() > 500_000 {
                                continue;
                            }
                            let scorer = AstQualityScorer::new();
                            if let Ok(q) = scorer.analyze(&content, lang) {
                                total += q.concrete_issues.len();
                                for i in q.concrete_issues {
                                    match i.severity {
                                        IssueSeverity::Critical => crit += 1,
                                        IssueSeverity::Major => maj += 1,
                                        IssueSeverity::Minor => min += 1,
                                    }
                                    *by_cat.entry(format!("{:?}", i.category)).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            } else if path.is_dir() {
                // Shallow: only top-level dirs; recursion omitted for speed
                if let Ok(files) = std::fs::read_dir(&path) {
                    for f in files.flatten() {
                        let fp = f.path();
                        if !fp.is_file() {
                            continue;
                        }
                        if let Some(ext) = fp.extension().and_then(|e| e.to_str()) {
                            if let Some(lang) = SupportedLanguage::from_extension(ext) {
                                if let Ok(content) = std::fs::read_to_string(&fp) {
                                    if content.len() > 300_000 {
                                        continue;
                                    }
                                    let scorer = AstQualityScorer::new();
                                    if let Ok(q) = scorer.analyze(&content, lang) {
                                        total += q.concrete_issues.len();
                                        for i in q.concrete_issues {
                                            match i.severity {
                                                IssueSeverity::Critical => crit += 1,
                                                IssueSeverity::Major => maj += 1,
                                                IssueSeverity::Minor => min += 1,
                                            }
                                            *by_cat.entry(format!("{:?}", i.category)).or_insert(0) += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if total == 0 {
        return Some(RiskHealth {
            total,
            critical: 0,
            major: 0,
            minor: 0,
            top_categories: Vec::new(),
        });
    }
    let mut cats: Vec<(String, usize)> = by_cat.into_iter().collect();
    cats.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    cats.truncate(5);
    Some(RiskHealth {
        total,
        critical: crit,
        major: maj,
        minor: min,
        top_categories: cats,
    })
}

// Removed unused project-wide AST analysis helpers


