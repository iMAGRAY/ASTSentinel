use anyhow::{Context, Result};
use std::io::{self, Read};
use std::path::Path;

use rust_validation_hooks::*;
// Use project analysis for full context
use rust_validation_hooks::analysis::project::scan_project_with_cache;
// Use AST analysis for comprehensive error detection
use rust_validation_hooks::analysis::ast::{
    MultiLanguageAnalyzer, AstQualityScorer, SupportedLanguage, IssueSeverity,
};
// Use duplicate detector for conflict awareness
// Use dependency analysis
use rust_validation_hooks::analysis::dependencies::analyze_project_dependencies;

#[tokio::main]
async fn main() -> Result<()> {
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
    let (structure, metrics, _inc) = scan_project_with_cache(working_dir, None, None)
        .context("scan_project_with_cache")?;

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
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join(", ");

    // 2) Dependencies (summary only)
    let deps = analyze_project_dependencies(Path::new(working_dir)).await.unwrap_or_default();

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
    out.push_str(&format!(
        "Dependencies: total {}, outdated {}\n",
        deps.total_count, deps.outdated_count
    ));

    out.push_str("\n---\n\n=== RISK/HEALTH SNAPSHOT ===\n");
    if let Some(r) = rh {
        out.push_str(&format!(
            "Issues: total {} (Critical {} / Major {} / Minor {})\n",
            r.total, r.critical, r.major, r.minor
        ));
        if !r.top_categories.is_empty() {
            out.push_str("Top categories: ");
            out.push_str(
                &r.top_categories
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
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
    use rust_validation_hooks::analysis::ast::quality_scorer::{IssueSeverity};
    use std::collections::HashMap;

    let mut total = 0usize;
    let mut crit = 0usize; let mut maj = 0usize; let mut min = 0usize;
    let mut by_cat: HashMap<String, usize> = HashMap::new();

    if let Ok(entries) = std::fs::read_dir(working_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if let Some(lang) = SupportedLanguage::from_extension(ext) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            // Skip huge files to keep latency low
                            if content.len() > 500_000 { continue; }
                            let scorer = AstQualityScorer::new();
                            if let Ok(q) = scorer.analyze(&content, lang) {
                                total += q.concrete_issues.len();
                                for i in q.concrete_issues {
                                    match i.severity {
                                        IssueSeverity::Critical => crit+=1,
                                        IssueSeverity::Major => maj+=1,
                                        IssueSeverity::Minor => min+=1,
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
                        if !fp.is_file() { continue; }
                        if let Some(ext) = fp.extension().and_then(|e| e.to_str()) {
                            if let Some(lang) = SupportedLanguage::from_extension(ext) {
                                if let Ok(content) = std::fs::read_to_string(&fp) {
                                    if content.len() > 300_000 { continue; }
                                    let scorer = AstQualityScorer::new();
                                    if let Ok(q) = scorer.analyze(&content, lang) {
                                        total += q.concrete_issues.len();
                                        for i in q.concrete_issues {
                                            match i.severity {
                                                IssueSeverity::Critical => crit+=1,
                                                IssueSeverity::Major => maj+=1,
                                                IssueSeverity::Minor => min+=1,
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

    if total == 0 { return Some(RiskHealth { total, critical: 0, major: 0, minor: 0, top_categories: Vec::new() }); }
    let mut cats: Vec<(String, usize)> = by_cat.into_iter().collect();
    cats.sort_by(|a,b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    cats.truncate(5);
    Some(RiskHealth { total, critical: crit, major: maj, minor: min, top_categories: cats })
}

/// Perform AST analysis across all code files in the project
#[allow(dead_code)]
async fn perform_project_wide_ast_analysis(working_dir: &str) -> String {
    let mut results = Vec::new();
    let mut total_issues = 0;
    let mut total_files_analyzed = 0;
    let mut critical_issues = Vec::new();

    // Starting project-wide AST analysis

    // Find all code files
    if let Ok(entries) = std::fs::read_dir(working_dir) {
        for entry in entries.flatten() {
            if let Err(e) = analyze_directory_recursive(&entry.path(), &mut results, &mut total_issues, &mut total_files_analyzed, &mut critical_issues, 0).await {
                // Error analyzing directory/file, continue with other files
                eprintln!("Warning: Failed to analyze path {}: {}", entry.path().display(), e);
            }
        }
    }

    if total_files_analyzed == 0 {
        return String::new();
    }

    // Analyzed files and found issues

    let mut analysis = format!(
        "## AST Analysis Summary\n\
        - Files analyzed: {}\n\
        - Total issues found: {}\n\
        - Critical issues: {}\n\n",
        total_files_analyzed,
        total_issues,
        critical_issues.len()
    );

    // Add critical issues first
    if !critical_issues.is_empty() {
        analysis.push_str("### Critical Issues Requiring Attention:\n");
        for (i, issue) in critical_issues.iter().take(10).enumerate() {
            analysis.push_str(&format!("{}. {}\n", i + 1, issue));
        }
        if critical_issues.len() > 10 {
            analysis.push_str(&format!("... and {} more critical issues\n", critical_issues.len() - 10));
        }
        analysis.push('\n');
    }

    // Add file-by-file summary for files with issues
    if !results.is_empty() {
        analysis.push_str("### Files with Quality Issues:\n");
        for (path, issues_count, sample_issues) in results.iter().take(20) {
            analysis.push_str(&format!("- `{}`: {} issues", path, issues_count));
            if !sample_issues.is_empty() {
                analysis.push_str(&format!(" (e.g., {})", sample_issues.join(", ")));
            }
            analysis.push('\n');
        }
        if results.len() > 20 {
            analysis.push_str(&format!("... and {} more files with issues\n", results.len() - 20));
        }
    }

    analysis
}

/// Recursively analyze directory for code files
#[allow(dead_code)]
async fn analyze_directory_recursive(
    path: &Path,
    results: &mut Vec<(String, usize, Vec<String>)>,
    total_issues: &mut usize,
    total_files: &mut usize,
    critical_issues: &mut Vec<String>,
    depth: usize,
) -> Result<()> {
    if depth > 10 {
        return Ok(()); // Prevent infinite recursion
    }

    if path.is_file() {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            if let Some(language) = SupportedLanguage::from_extension(extension) {
                if let Ok(content) = std::fs::read_to_string(path) {
                    // Skip very large files to prevent performance issues
                    if content.len() > 1_000_000 {
                        return Ok(());
                    }

                    match MultiLanguageAnalyzer::analyze_with_tree_sitter(&content, language) {
                        Ok(_complexity_metrics) => {
                            *total_files += 1;
                            
                            // Use AST quality scorer for detailed analysis
                            let scorer = AstQualityScorer::new();
                            if let Ok(quality_score) = scorer.analyze(&content, language) {
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
                                                path_str,
                                                issue.message,
                                                issue.line
                                            ));
                                        }
                                    }

                                    results.push((path_str, issues_count, sample_issues));
                                }
                            }
                        }
                        Err(_) => {
                            // If AST analysis fails, still count the file
                            *total_files += 1;
                        }
                    }
                }
            }
        }
    } else if path.is_dir() {
        // Skip common non-source directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || 
               name == "node_modules" || 
               name == "target" ||
               name == "dist" ||
               name == "build" {
                return Ok(());
            }
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                Box::pin(analyze_directory_recursive(&entry.path(), results, total_issues, total_files, critical_issues, depth + 1)).await?;
            }
        }
    }

    Ok(())
}
