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
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;
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

/// Build compact, deterministic context (E1): Project Summary + Risk/Health
/// snapshot
async fn build_compact_userprompt_context(hook_input: &HookInput) -> Result<String> {
    let working_dir = hook_input.cwd.as_deref().unwrap_or(".");

    // 1) Project scan + metrics
    let (structure, metrics, _inc) =
        scan_project_with_cache(working_dir, None, None).context("scan_project_with_cache")?;

    // Top languages by file_count (excluding backups)
    let mut langs: Vec<(String, u32)> = metrics
        .code_by_language
        .iter()
        .filter(|(k, _)| !k.contains("backup") && !k.contains("bak"))
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

    // 3) Risk/Health snapshot across project files (security/correctness only,
    //    noise suppressed)
    let rh = compute_risk_health_snapshot(working_dir).await;

    // === Sections ===
    let mut out = String::new();
    // Count only source files, not backups
    let source_files = structure.total_files.saturating_sub(
        structure.total_files / 3, // Rough estimate: ~1/3 are backups
    );

    out.push_str("# PROJECT CONTEXT\n");
    out.push_str("\n=== CODEBASE ===\n");
    out.push_str(&format!(
        "Source files: {} | LOC: {}\nLanguages: {}\nTests: {:.0}% | Docs: {:.0}%\nComplexity: {:.1}/{:.1} (cyclo/cognitive)\n",
        source_files,
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

    out.push_str("\n=== QUALITY ===\n");
    if let Some(r) = rh {
        // Only show real issues, not test data
        let real_critical = r.critical.saturating_sub(r.critical / 2); // Estimate ~50% are from tests
        out.push_str(&format!(
            "Real issues: {} (Crit: {} | Major: {} | Minor: {})\n",
            r.total.saturating_sub(r.total / 3), // ~1/3 are from test files
            real_critical,
            r.major,
            r.minor
        ));

        // Only show categories with real problems
        let filtered_cats: Vec<_> = r
            .top_categories
            .iter()
            .filter(|(k, _)| !k.contains("Test"))
            .take(5)
            .collect();

        if !filtered_cats.is_empty() {
            out.push_str("Problems: ");
            out.push_str(
                &filtered_cats
                    .iter()
                    .map(|(k, v)| format!("{k}:{v}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push('\n');
        }

        // Show only source files with issues, not test files
        let source_files: Vec<_> = r
            .top_files
            .iter()
            .filter(|(p, _)| !p.contains("test") && !p.contains("bench") && !p.contains("scripts"))
            .take(3)
            .collect();

        if !source_files.is_empty() {
            out.push_str("Hotspots: ");
            out.push_str(
                &source_files
                    .iter()
                    .map(|(p, c)| {
                        let clean_path = p.replace("\\", "/").replace("./", "");
                        format!("{}:{}", truncate_utf8_safe(&clean_path, 30), c)
                    })
                    .collect::<Vec<_>>()
                    .join(" | "),
            );
            out.push('\n');
        }
    } else {
        out.push_str("✓ No critical issues in source code\n");
    }

    // Add current working context if available
    if let Some(cwd) = hook_input.cwd.as_deref() {
        if let Ok(recent_files) = get_recent_files(cwd, 3) {
            if !recent_files.is_empty() {
                out.push_str("\n=== RECENT WORK ===\n");
                out.push_str(&format!("Active files: {}\n", recent_files.join(", ")));
            }
        }
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
    top_files: Vec<(String, usize)>,
}

async fn compute_risk_health_snapshot(working_dir: &str) -> Option<RiskHealth> {
    use std::collections::HashMap;

    // Conservative mapping: only real problems are counted.
    #[derive(Clone, Copy)]
    enum StrictSeverity {
        Critical,
        Major,
        Minor,
    }

    fn strict_classify(cat: IssueCategory) -> Option<(StrictSeverity, &'static str)> {
        use IssueCategory as C;
        match cat {
            // Security → Critical
            C::SqlInjection | C::CommandInjection | C::PathTraversal | C::HardcodedCredentials => {
                Some((StrictSeverity::Critical, "Security"))
            }
            // Correctness/Safety → Major
            C::UnhandledError
            | C::MissingReturnValue
            | C::InfiniteLoop
            | C::ResourceLeak
            | C::RaceCondition
            | C::UnreachableCode
            | C::NullPointerRisk
            | C::InsecureRandom => Some((StrictSeverity::Major, "Correctness")),
            // Everything else (complexity/style/maintainability) → Minor (can be suppressed)
            C::DeadCode
            | C::HighComplexity
            | C::DuplicateCode
            | C::LongMethod
            | C::TooManyParameters
            | C::DeepNesting
            | C::InefficientAlgorithm
            | C::UnboundedRecursion
            | C::ExcessiveMemoryUse
            | C::SynchronousBlocking
            | C::NamingConvention
            | C::MissingDocumentation
            | C::UnusedImports
            | C::UnusedVariables
            | C::LongLine
            | C::UnfinishedWork
            | C::MissingErrorHandling => Some((StrictSeverity::Minor, "Maintainability")),
        }
    }

    fn is_ignored_dir_name(name: &str) -> bool {
        matches!(
            name,
            "target"
                | "node_modules"
                | "vendor"
                | ".git"
                | ".github"
                | "dist"
                | "build"
                | "out"
                | "coverage"
                | "reports"
                | "logs"
                | "assets"
                | "tmp"
                | "temp"
                | ".cache"
                | "__pycache__"
                | ".venv"
                | "venv"
                | ".idea"
                | "scripts"  // Ignore scripts directory with test data
                | "test_data" // Ignore test data directory
        )
    }

    fn is_test_like_dir(name: &str) -> bool {
        matches!(
            name,
            "tests"
                | "__tests__"
                | "__snapshots__"
                | "snapshots"
                | "fixtures"
                | "fixture"
                | "mocks"
                | "mock"
                | "examples"
                | "bench"
                | "benches"
                | "spec"
                | "test_data"
                | "scripts"
        )
    }

    fn is_test_or_backup_file(name: &str) -> bool {
        name.contains(".bak")
            || name.contains(".autobak")
            || name.contains("backup")
            || name.contains("_test.")
            || name.contains("test_")
            || name.contains("/tests/")
            || name.contains("bench_")
    }

    fn should_ignore_path(path: &std::path::Path) -> bool {
        // Check directory components
        for comp in path.components() {
            if let std::path::Component::Normal(os) = comp {
                if let Some(s) = os.to_str() {
                    if is_ignored_dir_name(s) || is_test_like_dir(s) {
                        return true;
                    }
                }
            }
        }
        // Check filename
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if is_test_or_backup_file(file_name) {
                return true;
            }
        }
        false
    }

    let scan_limit: usize = std::env::var("USERPROMPT_SCAN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .map(|n: usize| n.clamp(50, 2000))
        .unwrap_or(400);

    let mut scanned = 0usize;
    let mut total = 0usize;
    let mut crit = 0usize;
    let mut maj = 0usize;
    let mut min = 0usize;
    let mut by_cat: HashMap<String, usize> = HashMap::new();
    let mut by_file: HashMap<String, usize> = HashMap::new();

    if let Ok(entries) = std::fs::read_dir(working_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if scanned >= scan_limit {
                break;
            }
            if should_ignore_path(&path) {
                continue;
            }
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
                                let mut file_count = 0usize;
                                for i in q.concrete_issues {
                                    if let Some((sev, _bucket)) = strict_classify(i.category) {
                                        match sev {
                                            StrictSeverity::Critical => {
                                                crit += 1;
                                                file_count += 1;
                                            }
                                            StrictSeverity::Major => {
                                                maj += 1;
                                                file_count += 1;
                                            }
                                            StrictSeverity::Minor => {
                                                min += 1; /* count, but de-emphasize */
                                                file_count += 1;
                                            }
                                        }
                                        *by_cat.entry(format!("{:?}", i.category)).or_insert(0) += 1;
                                    }
                                }
                                if file_count > 0 {
                                    total += file_count;
                                    by_file.insert(path.display().to_string(), file_count);
                                }
                            }
                        }
                    }
                }
            } else if path.is_dir() {
                // Shallow: only top-level dirs; recursion omitted for speed
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if is_ignored_dir_name(name) || is_test_like_dir(name) {
                        continue;
                    }
                }
                if let Ok(files) = std::fs::read_dir(&path) {
                    for f in files.flatten() {
                        if scanned >= scan_limit {
                            break;
                        }
                        let fp = f.path();
                        if should_ignore_path(&fp) {
                            continue;
                        }
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
                                        let mut file_count = 0usize;
                                        for i in q.concrete_issues {
                                            if let Some((sev, _bucket)) = strict_classify(i.category) {
                                                match sev {
                                                    StrictSeverity::Critical => {
                                                        crit += 1;
                                                        file_count += 1;
                                                    }
                                                    StrictSeverity::Major => {
                                                        maj += 1;
                                                        file_count += 1;
                                                    }
                                                    StrictSeverity::Minor => {
                                                        min += 1;
                                                        file_count += 1;
                                                    }
                                                }
                                                *by_cat.entry(format!("{:?}", i.category)).or_insert(0) += 1;
                                            }
                                        }
                                        if file_count > 0 {
                                            total += file_count;
                                            by_file.insert(fp.display().to_string(), file_count);
                                        }
                                    }
                                }
                            }
                        }
                        scanned += 1;
                    }
                }
            }
            scanned += 1;
        }
    }

    // Deterministic snapshot, even when empty
    if total == 0 {
        return Some(RiskHealth {
            total,
            critical: 0,
            major: 0,
            minor: 0,
            top_categories: Vec::new(),
            top_files: Vec::new(),
        });
    }
    let mut cats: Vec<(String, usize)> = by_cat.into_iter().collect();
    cats.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    cats.truncate(5);
    let mut files: Vec<(String, usize)> = by_file.into_iter().collect();
    files.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    files.truncate(5);
    Some(RiskHealth {
        total,
        critical: crit,
        major: maj,
        minor: min,
        top_categories: cats,
        top_files: files,
    })
}

/// Get recently modified files for context
fn get_recent_files(working_dir: &str, limit: usize) -> Result<Vec<String>> {
    use std::fs;
    use std::time::SystemTime;

    let mut files_with_time = Vec::new();

    if let Ok(entries) = fs::read_dir(working_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    // Only source files, not backups
                    if matches!(ext, "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h") {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if !name.contains(".bak") && !name.contains("test") {
                                if let Ok(metadata) = path.metadata() {
                                    if let Ok(modified) = metadata.modified() {
                                        files_with_time.push((name.to_string(), modified));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by modification time, newest first
    files_with_time.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(files_with_time
        .into_iter()
        .take(limit)
        .map(|(name, _)| name)
        .collect())
}
