use anyhow::{Context, Result};
use serde_json;
use std::io::{self, Read, Write};
use tokio;
use std::path::Path;

use rust_validation_hooks::*;
// Use project analysis for full context
use rust_validation_hooks::analysis::project::{
    format_project_structure_for_ai_with_metrics, scan_project_with_cache,
};
// Use AST analysis for comprehensive error detection
use rust_validation_hooks::analysis::ast::{
    MultiLanguageAnalyzer, AstQualityScorer, QualityScore, SupportedLanguage, IssueSeverity,
};
// Use duplicate detector for conflict awareness
use rust_validation_hooks::analysis::duplicate_detector::DuplicateDetector;
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

    // Perform comprehensive project analysis
    match analyze_project_for_ai_context(&hook_input).await {
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

/// Perform comprehensive project analysis for AI context
async fn analyze_project_for_ai_context(hook_input: &HookInput) -> Result<String> {
    let working_dir = hook_input.cwd.as_deref().unwrap_or(".");
    let mut context_parts = Vec::new();

    // Starting comprehensive project analysis

    // 1. Project structure with metrics
    match scan_project_with_cache(working_dir, None, None) {
        Ok((structure, metrics, incremental)) => {
            // Found files in project
            
            let formatted_structure = format_project_structure_for_ai_with_metrics(
                &structure,
                Some(&metrics),
                structure.files.len() > 100, // Use compression for large projects
            );

            let mut project_info = format!("# PROJECT ANALYSIS\n{}", formatted_structure);
            
            if let Some(inc) = incremental {
                project_info.push_str(&format!("\n{}", inc));
            }
            
            context_parts.push(project_info);
        }
        Err(e) => {
            // Failed to scan project
        }
    }

    // 2. Dependencies analysis
    match analyze_project_dependencies(Path::new(working_dir)).await {
        Ok(deps) => {
            // Analyzed dependencies
            context_parts.push(format!("\n# DEPENDENCIES\n{}", deps.format_for_ai()));
        }
        Err(e) => {
            // Dependencies analysis failed
        }
    }

    // 3. Comprehensive AST analysis for all code files
    let ast_analysis = perform_project_wide_ast_analysis(working_dir).await;
    if !ast_analysis.is_empty() {
        context_parts.push(format!("\n# CODE QUALITY ANALYSIS\n{}", ast_analysis));
    }

    // 4. Duplicate file detection
    let mut detector = DuplicateDetector::new();
    match detector.scan_directory(Path::new(working_dir)) {
        Ok(_) => {
            let duplicates = detector.find_duplicates();
            if !duplicates.is_empty() {
                // Found duplicate groups
                let duplicate_report = detector.format_report(&duplicates);
                context_parts.push(format!("\n# FILE CONFLICTS\n{}", duplicate_report));
            }
        }
        Err(e) => {
            // Duplicate detection failed
        }
    }

    let final_context = if !context_parts.is_empty() {
        format!(
            "# COMPREHENSIVE PROJECT CONTEXT\n\
            This context provides a complete view of the project for better AI understanding.\n\n{}",
            context_parts.join("\n")
        )
    } else {
        String::new()
    };

    // Generated context
    Ok(final_context)
}

/// Perform AST analysis across all code files in the project
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
                // Error analyzing file
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
                        Ok(complexity_metrics) => {
                            *total_files += 1;
                            
                            // Use AST quality scorer for detailed analysis
                            let mut scorer = AstQualityScorer::new();
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