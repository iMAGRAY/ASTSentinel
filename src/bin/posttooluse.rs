use anyhow::{Context, Result};
use serde_json;
use std::io::{self, Read};
use tokio;

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
use rust_validation_hooks::validation::diff_formatter::{
    format_code_diff, format_edit_as_unified_diff, format_edit_full_context, format_full_file_with_changes, format_multi_edit_full_context,
};
// Use AST-based quality scorer for deterministic code analysis
use rust_validation_hooks::analysis::ast::{
    AstQualityScorer, QualityScore, SupportedLanguage, IssueSeverity,
};
// Use duplicate detector for finding conflicting files
use rust_validation_hooks::analysis::duplicate_detector::{
    DuplicateDetector, DuplicateGroup, ConflictType,
};

// Removed GrokAnalysisClient - now using UniversalAIClient from ai_providers module

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
            // Log error for debugging but don't expose details
            eprintln!("Warning: Cannot validate prompts path: {}", e);
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
            eprintln!("Error: Cannot determine executable path: {}", e);
            eprintln!("Falling back to current directory + prompts");
            return PathBuf::from("prompts");
        }
    };

    let parent = match exe_path.parent() {
        Some(parent) => parent,
        None => {
            eprintln!("Error: Cannot get parent directory of executable");
            eprintln!("Falling back to current directory + prompts");
            return PathBuf::from("prompts");
        }
    };

    // Production scenario: prompts directory next to executable
    let prompts_path = parent.join("prompts");

    if let Some(validated) = validate_prompts_path(&prompts_path) {
        eprintln!("PostTool using prompts directory: {:?}", validated);
        return validated;
    }

    // Final fallback
    eprintln!("Warning: prompts directory not found next to executable, using current directory");
    PathBuf::from("prompts")
}

/// Load prompt content from file in prompts directory
async fn load_prompt_file(filename: &str) -> Result<String> {
    let prompt_path = get_prompts_dir().join(filename);
    tokio::fs::read_to_string(prompt_path)
        .await
        .with_context(|| format!("Failed to load prompt file: {}", filename))
}

// Constants for formatting instructions
const CRITICAL_INSTRUCTION: &str = "\n\nOUTPUT EXACTLY AS SHOWN IN THE TEMPLATE BELOW.\n\n";

const TOKEN_LIMIT: &str = "TOKEN LIMIT: 4500\n\n";

const TEMPLATE_HEADER: &str = "=== REQUIRED OUTPUT FORMAT ===\n";
const TEMPLATE_FOOTER: &str = "\n=== END FORMAT ===\n";

// This will be dynamically constructed with language
const FINAL_INSTRUCTION_PREFIX: &str = "\n\nOUTPUT EXACTLY AS TEMPLATE. ANY FORMAT ALLOWED IF TEMPLATE SHOWS IT.\nRESPOND IN ";

/// Format the analysis prompt with instructions, project context and conversation
async fn format_analysis_prompt(
    prompt: &str,
    project_context: Option<&str>,
    diff_context: Option<&str>,
    transcript_context: Option<&str>,
) -> Result<String> {
    format_analysis_prompt_with_ast(prompt, project_context, diff_context, transcript_context, None).await
}

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
        format!("\n\nDOCUMENTATION RECOMMENDATION GUIDELINES:\n{}\n", context7_docs)
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
        + 20; // buffer for separators and " LANGUAGE."

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
    // Just basic sanity checks, AI handles real validation
    if path.is_empty() {
        anyhow::bail!("Invalid file path: empty path");
    }

    // Check for null bytes
    if path.contains('\0') {
        anyhow::bail!("Invalid file path: contains null byte");
    }

    // Convert to PathBuf and return - Claude Code already validates paths
    Ok(PathBuf::from(path))
}

/// Safely read file content with proper error handling
async fn read_file_content_safe(path: &str) -> Result<Option<String>> {
    let validated_path = match validate_file_path(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Warning: Failed to validate file path '{}': {}", path, e);
            return Ok(None);
        }
    };

    match tokio::fs::read_to_string(&validated_path).await {
        Ok(content) => Ok(Some(content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // File doesn't exist yet - this is normal for new files
            Ok(None)
        }
        Err(e) => {
            eprintln!(
                "Warning: Failed to read file '{}': {}",
                validated_path.display(),
                e
            );
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

    match hook_input.tool_name.as_str() {
        "Edit" => {
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

            // Return unified diff for clear change visibility
            // Since posttooluse runs AFTER edit, reconstruct diff from old_string -> new_string
            let mut result = String::new();
            result.push_str(&format!("--- a/{}\n", display_path));
            result.push_str(&format!("+++ b/{}\n", display_path));
            
            let old_lines = old_string.lines().count().max(1);
            let new_lines = new_string.lines().count().max(1);
            
            result.push_str(&format!("@@ -1,{} +1,{} @@\n", old_lines, new_lines));
            
            // Show removed lines
            for line in old_string.lines() {
                result.push_str(&format!("-{}\n", line));
            }
            
            // Show added lines  
            for line in new_string.lines() {
                result.push_str(&format!("+{}\n", line));
            }
            
            Ok(result)
        }

        "MultiEdit" => {
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

        "Write" => {
            // Extract and validate content field
            let new_content = hook_input
                .tool_input
                .get("content")
                .and_then(|v| v.as_str())
                .context("Write operation missing required 'content' field")?;

            // Return unified diff for clear change visibility with +/- markers
            Ok(format_code_diff(
                display_path,
                file_content.as_deref(),
                Some(new_content),
                3, // context lines
            ))
        }

        _ => {
            // Not a code modification operation
            Ok(String::new())
        }
    }
}

/// Validate transcript path for security
fn validate_transcript_path(path: &str) -> bool {
    // Check for path traversal attempts
    if path.contains("..") || path.contains("~") || path.contains("\\\\") {
        return false;
    }

    // Check for suspicious patterns
    if path.contains("%") || path.contains('\0') {
        return false;
    }

    true
}

/// Read and format transcript for AI context (without tool content)
async fn read_transcript_summary(
    path: &str,
    max_messages: usize,
    max_chars: usize,
) -> Result<String> {
    // Security check
    if !validate_transcript_path(path) {
        anyhow::bail!("Invalid transcript path: potential security risk");
    }

    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, BufReader};

    // For large files, we'll read only the last portion
    const MAX_READ_BYTES: u64 = 1024 * 1024; // 1MB should be enough for recent messages

    let file = File::open(path)
        .await
        .context("Failed to open transcript file")?;
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
    while let Some(line) = lines.next_line().await? {
        if start_pos > 0 && lines_buffer.is_empty() {
            // Skip potentially partial first line when reading from middle
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
                let content = if let Some(content_arr) =
                    msg.get("content").and_then(|v| v.as_array())
                {
                    // Handle content array (assistant messages)
                    let mut text_parts = Vec::new();

                    for c in content_arr {
                        if let Some(text) = c.get("text").and_then(|v| v.as_str()) {
                            text_parts.push(text.to_string());
                        } else if let Some(tool_name) = c.get("name").and_then(|v| v.as_str()) {
                            // Get tool name and file if available
                            if let Some(input) = c.get("input") {
                                if let Some(file_path) =
                                    input.get("file_path").and_then(|v| v.as_str())
                                {
                                    text_parts
                                        .push(format!("{} tool file: {}", tool_name, file_path));
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
                } else if let Some(text) = msg.get("content").and_then(|v| v.as_str()) {
                    // Handle simple string content (user messages)
                    Some(text.to_string())
                } else {
                    None
                };

                if let Some(content) = content {
                    // Format message
                    let formatted_msg = if role == "user" {
                        // Save the FIRST user message we encounter (which is the most recent due to reverse iteration)
                        if !found_first_user_message {
                            most_recent_user_message = content.clone();
                            found_first_user_message = true;
                        }
                        format!("[user]: {}", truncate_utf8_safe(&content, 150))
                    } else if role == "assistant" {
                        format!("[ai-assistant]: {}", truncate_utf8_safe(&content, 150))
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

    Ok(format!("{}conversation:\n{}", current_task, conversation))
}

/// Perform AST-based quality analysis on code
async fn perform_ast_analysis(content: &str, file_path: &str) -> Option<QualityScore> {
    // Detect language from file extension
    let extension = file_path.split('.').last().unwrap_or("");
    let language = match SupportedLanguage::from_extension(extension) {
        Some(lang) => lang,
        None => {
            eprintln!("AST Analysis: Unsupported file type: {}", extension);
            return None;
        }
    };
    
    // Special handling for Rust - use syn crate
    if language == SupportedLanguage::Rust {
        // For now, skip Rust AST analysis (requires syn integration)
        eprintln!("AST Analysis: Rust analysis via syn not yet integrated");
        return None;
    }
    
    // Create AST quality scorer
    let scorer = AstQualityScorer::new();
    
    // Perform analysis
    match scorer.analyze(content, language) {
        Ok(score) => {
            // Return score for JSON output (don't log to stderr)
            Some(score)
        }
        Err(e) => {
            eprintln!("AST Analysis Error: {}", e);
            None
        }
    }
}

/// Format AST analysis results for AI context (without scores to avoid duplication)
fn format_ast_results(score: &QualityScore) -> String {
    let mut result = String::with_capacity(2000);
    
    // Only pass concrete issues to AI, not scores (to avoid duplication)
    if !score.concrete_issues.is_empty() {
        result.push_str("\n\nAST DETECTED ISSUES (Automated):\n");
        
        // Group by severity
        let mut critical_issues = Vec::new();
        let mut major_issues = Vec::new();
        let mut minor_issues = Vec::new();
        
        for issue in &score.concrete_issues {
            match issue.severity {
                IssueSeverity::Critical => critical_issues.push(issue),
                IssueSeverity::Major => major_issues.push(issue),
                IssueSeverity::Minor => minor_issues.push(issue),
            }
        }
        
        if !critical_issues.is_empty() {
            result.push_str("\n🔴 CRITICAL (P1 - Fix immediately):\n");
            for issue in critical_issues {
                result.push_str(&format!(
                    "  Line {}: {} [{}] (-{} points)\n",
                    issue.line, issue.message, issue.rule_id, issue.points_deducted
                ));
            }
        }
        
        if !major_issues.is_empty() {
            result.push_str("\n🟡 MAJOR (P2 - Fix soon):\n");
            for issue in major_issues {
                result.push_str(&format!(
                    "  Line {}: {} [{}] (-{} points)\n",
                    issue.line, issue.message, issue.rule_id, issue.points_deducted
                ));
            }
        }
        
        if !minor_issues.is_empty() {
            result.push_str("\n🟢 MINOR (P3 - Nice to fix):\n");
            for issue in minor_issues {
                result.push_str(&format!(
                    "  Line {}: {} [{}] (-{} points)\n",
                    issue.line, issue.message, issue.rule_id, issue.points_deducted
                ));
            }
        }
    } else {
        // No issues detected - return empty string to not add unnecessary context
        return String::new();
    }
    
    result.push_str("\nNote: Use AST issues as baseline. Add context-aware insights.\n");
    
    result
}

/// Main function for the PostToolUse hook
#[tokio::main]
async fn main() -> Result<()> {
    // Read hook input from stdin
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read stdin")?;

    // Parse the input
    let hook_input: HookInput =
        serde_json::from_str(&buffer).context("Failed to parse input JSON")?;
    
    // DEBUG: Write the exact hook input to a file for inspection
    if let Ok(mut debug_file) = tokio::fs::File::create("hook-input-debug.json").await {
        use tokio::io::AsyncWriteExt;
        let _ = debug_file.write_all(buffer.as_bytes()).await;
        eprintln!("DEBUG: Full hook input written to hook-input-debug.json");
    }

    // Only analyze Write, Edit, and MultiEdit operations
    if !matches!(
        hook_input.tool_name.as_str(),
        "Write" | "Edit" | "MultiEdit"
    ) {
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

    // Extract content based on tool type
    let content = match hook_input.tool_name.as_str() {
        "Write" => hook_input
            .tool_input
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Edit" => hook_input
            .tool_input
            .get("new_string")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "MultiEdit" => {
            // For MultiEdit, aggregate all new_strings
            hook_input
                .tool_input
                .get("edits")
                .and_then(|v| v.as_array())
                .map(|edits| {
                    edits
                        .iter()
                        .filter_map(|edit| edit.get("new_string").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default()
        }
        _ => String::new(),
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
    eprintln!("DEBUG: Starting AST analysis for file: {}", file_path);
    let ast_analysis = perform_ast_analysis(&content, file_path).await;
    eprintln!("DEBUG: AST analysis result: {:?}", ast_analysis.is_some());

    // Load configuration from environment
    let config = Config::from_env().context("Failed to load configuration")?;

    // Load the analysis prompt
    let prompt = load_prompt_file("post_edit_validation.txt")
        .await
        .context("Failed to load prompt")?;

    // Get project structure with caching and metrics
    let cache_path = PathBuf::from(".claude_project_cache.json");
    let project_context = match scan_project_with_cache(".", Some(&cache_path), None) {
        Ok((structure, metrics, incremental)) => {
            // Use compressed format for large projects
            let use_compression = structure.files.len() > 100;
            let mut formatted = format_project_structure_for_ai_with_metrics(
                &structure,
                Some(&metrics),
                use_compression,
            );

            // Add incremental update info if available
            if let Some(inc) = incremental {
                formatted.push_str(&format!("\n{}", inc));
            }

            // Log metrics to stderr for debugging
            eprintln!(
                "Project metrics: {} LOC, {} files, complexity: {:.1}",
                metrics.total_lines_of_code,
                structure.files.len(),
                metrics.project_complexity_score
            );

            Some(formatted)
        }
        Err(e) => {
            eprintln!("Failed to scan project structure: {}", e);
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
                    eprintln!("Found {} duplicate/conflict groups", duplicates.len());
                    Some(detector.format_report(&duplicates))
                } else {
                    None
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to scan for duplicates: {}", e);
                None
            }
        }
    };

    // Analyze project dependencies for AI context
    let dependencies_context = match analyze_project_dependencies(&std::path::Path::new(".")).await {
        Ok(deps) => {
            eprintln!("Dependencies analysis: {} total dependencies", deps.total_count);
            if deps.outdated_count > 0 {
                eprintln!("Found {} potentially outdated dependencies", deps.outdated_count);
            }
            Some(deps.format_for_ai())
        }
        Err(e) => {
            eprintln!("Warning: Failed to analyze dependencies: {}", e);
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
            eprintln!("Warning: Failed to generate diff context: {}", e);
            String::new()
        }
    };

    // Read conversation transcript for context (10 messages, max 2000 chars)
    let transcript_context = if let Some(transcript_path) = &hook_input.transcript_path {
        match read_transcript_summary(transcript_path, 10, 2000).await {
            Ok(summary) => {
                eprintln!("DEBUG: Transcript summary read successfully:");
                // Safe UTF-8 truncation using standard library
                let truncated: String = summary.chars().take(500).collect();
                eprintln!("DEBUG: First ~500 chars: {}", truncated);
                Some(summary)
            }
            Err(e) => {
                eprintln!("Warning: Failed to read transcript: {}", e);
                None
            }
        }
    } else {
        eprintln!("DEBUG: No transcript_path provided");
        None
    };

    // Include AST analysis results in context if available
    let ast_context = ast_analysis.as_ref().map(format_ast_results);
    
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
    if let Ok(mut debug_file) = tokio::fs::File::create("post-context.txt").await {
        use tokio::io::AsyncWriteExt;
        let _ = debug_file.write_all(formatted_prompt.as_bytes()).await;
        let _ = debug_file.write_all(b"\n\n=== END OF PROMPT ===\n").await;
        eprintln!("DEBUG: Full prompt written to post-context.txt");
    }

    // Create AI client and perform analysis
    let client = UniversalAIClient::new(config.clone()).context("Failed to create AI client")?;

    // Analyze code using the configured provider - returns raw response
    match client
        .analyze_code_posttool(&content, &formatted_prompt)
        .await
    {
        Ok(ai_response) => {
            // Combine AST results with AI response for full visibility
            let mut final_response = String::new();
            
            // Add AST results first if available
            if let Some(ast_score) = &ast_analysis {
                final_response.push_str(&format!("<Deterministic Score: {}/1000>\n", ast_score.total_score));
                
                if !ast_score.concrete_issues.is_empty() {
                    final_response.push_str("Concrete Issues Found:\n");
                    for issue in &ast_score.concrete_issues {
                        final_response.push_str(&format!(
                            "• Line {}: {} (-{} pts)\n",
                            issue.line, issue.message, issue.points_deducted
                        ));
                    }
                } else {
                    final_response.push_str("No concrete issues found by AST analysis.\n");
                }
                final_response.push_str("\n");
            }
            
            // Add AI response
            final_response.push_str(&ai_response);
            
            let output = PostToolUseOutput {
                hook_specific_output: PostToolUseHookOutput {
                    hook_event_name: "PostToolUse".to_string(),
                    additional_context: final_response,
                },
            };

            println!(
                "{}",
                serde_json::to_string(&output).context("Failed to serialize output")?
            );
        }
        Err(e) => {
            // Log error but don't block the operation
            eprintln!("PostToolUse analysis error: {}", e);

            // Pass through with minimal feedback
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
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_transcript_summary_with_user_messages() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = fs::File::create(&transcript_path).unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"Help me write a function"}}"#
        )
        .unwrap();
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
        // Messages should appear in order
        if let (Some(first_pos), Some(second_pos), Some(recent_pos)) = (
            summary.find("First message"),
            summary.find("Second message"),
            summary.find("Most recent message"),
        ) {
            assert!(first_pos < second_pos);
            assert!(second_pos < recent_pos);
        } else {
            panic!("Expected messages not found in summary");
        }
    }

    #[tokio::test]
    async fn test_validate_transcript_path() {
        assert!(!validate_transcript_path("../../etc/passwd"));
        assert!(!validate_transcript_path("~/secrets"));
        assert!(!validate_transcript_path("\\\\server\\share"));
        assert!(!validate_transcript_path("file%20with%20encoding"));
        assert!(!validate_transcript_path("file\0with\0nulls"));
        assert!(validate_transcript_path("/valid/path/transcript.jsonl"));
        assert!(validate_transcript_path(
            "C:/Users/1/Documents/transcript.jsonl"
        ));
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
        let message_count =
            summary.matches("[user]:").count() + summary.matches("[ai-assistant]:").count();
        assert_eq!(message_count, 5);
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
        let message_count = summary.matches("[user]:").count();
        assert!(message_count < 10);
        assert!(message_count > 0); // Should have at least one message
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
        writeln!(file, "").unwrap(); // Empty line
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
        writeln!(
            file,
            r#"{{"role":"user","content":"Emoji 🎉🚀💻 message"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"Cyrillic текст сообщение"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"CJK 中文 日本語 한국어"}}"#
        )
        .unwrap();
        writeln!(
            file,
            "{{\"role\":\"user\",\"content\":\"Zero-width \u{200B}\u{200C}\u{200D} chars\"}}"
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"RTL text مرحبا עברית"}}"#
        )
        .unwrap();
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

        // Should contain recent messages, not old ones from beginning
        assert!(summary.contains("Recent message"));
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
        file.write_all(
            b"{\"role\":\"user\",\"content\":\"Here is JSON: {\\\"key\\\": \\\"value\\\"}\"}\n",
        )
        .unwrap();
        // Message with escaped characters
        writeln!(
            file,
            r#"{{"role":"user","content":"Path: C:\\Users\\Test\\file.txt"}}"#
        )
        .unwrap();
        // Message with newlines
        writeln!(
            file,
            r#"{{"role":"user","content":"Line 1\nLine 2\nLine 3"}}"#
        )
        .unwrap();
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
        assert_eq!(result, "𝄞𝄞...");
    }
}
