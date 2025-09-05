use anyhow::{Context, Result};
use std::io::{self, Read};
use tokio;
use serde_json;

use rust_validation_hooks::*;
use rust_validation_hooks::truncate_utf8_safe;
// Use universal AI client for multi-provider support
use rust_validation_hooks::providers::ai::UniversalAIClient;
// Use project context for better AI understanding  
use rust_validation_hooks::analysis::project::{scan_project_with_cache, format_project_structure_for_ai_with_metrics};
use std::path::PathBuf;
// Use diff formatter for better AI context - full file context for complete visibility
use rust_validation_hooks::validation::diff_formatter::{
    format_full_file_with_changes,
    format_edit_full_context,
    format_multi_edit_full_context,
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

/// Find prompts directory from executable location
/// 
/// This function assumes the following directory structures:
/// - Development: executable in target/debug/ or target/release/, prompts in project root
/// - Production: executable and prompts directory in the same parent directory
fn find_prompts_from_exe() -> Option<PathBuf> {
    // Get current executable path, may fail if exe was moved/deleted
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Warning: Cannot determine executable path: {}", e);
            return None;
        }
    };
    
    let parent = exe_path.parent()?;
    
    // Development scenario: check if we're in Cargo's target directory
    // Structure: project_root/target/{debug|release}/binary
    let parent_name = parent.file_name()?.to_str()?;
    if parent_name == "debug" || parent_name == "release" {
        // Navigate up: binary -> debug/release -> target -> project_root
        let project_root = parent.parent()?.parent()?;
        let prompts_path = project_root.join("prompts");
        if let Some(validated) = validate_prompts_path(&prompts_path) {
            return Some(validated);
        }
    }
    
    // Production scenario: prompts directory next to executable
    let prompts_path = parent.join("prompts");
    validate_prompts_path(&prompts_path)
}

/// Get the prompts directory path from environment or use default
fn get_prompts_dir() -> PathBuf {
    // Priority 1: Environment variable (with validation via canonicalize)
    if let Ok(prompts_dir) = std::env::var("PROMPTS_DIR") {
        let path = PathBuf::from(prompts_dir);
        
        // canonicalize already prevents path traversal, no need for string checks
        if let Some(validated) = validate_prompts_path(&path) {
            return validated;
        }
        eprintln!("Warning: PROMPTS_DIR path validation failed, trying fallbacks");
    }
    
    // Priority 2: Find from executable location
    if let Some(path) = find_prompts_from_exe() {
        return path;
    }
    
    // Priority 3: Fallback to current working directory
    eprintln!("Warning: Using fallback prompts directory in current working directory");
    PathBuf::from("prompts")
}

/// Load prompt content from file in prompts directory
async fn load_prompt_file(filename: &str) -> Result<String> {
    let prompt_path = get_prompts_dir().join(filename);
    tokio::fs::read_to_string(prompt_path)
        .await
        .with_context(|| format!("Failed to load prompt file: {}", filename))
}

/// Format the analysis prompt with instructions, project context and conversation
async fn format_analysis_prompt(
    prompt: &str, 
    project_context: Option<&str>, 
    diff_context: Option<&str>,
    transcript_context: Option<&str>
) -> Result<String> {
    
    // Load language instruction from file
    let language_instruction = load_prompt_file("language_instruction.txt").await?;
    
    // Load JSON template from file
    let json_template = load_prompt_file("json_template.txt").await?;
    
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
    
    // Build prompt with pre-allocated capacity for better performance
    const TOKEN_LIMIT_WARNING: &str = "\n\nCRITICAL TOKEN LIMIT: Your response must NOT exceed 4500 tokens. \
         Keep analysis concise but thorough.\n\n";
    const FINAL_INSTRUCTIONS: &str = "\n\nNEVER include text outside JSON. Output ONLY the JSON object.\n\
         TOKEN LIMIT: Keep response under 4500 tokens.";
    const SEPARATOR: &str = "\n\n";
    
    let estimated_capacity = prompt.len() 
        + SEPARATOR.len()
        + language_instruction.len() 
        + transcript_section.len() 
        + context_section.len() 
        + diff_section.len() 
        + TOKEN_LIMIT_WARNING.len()
        + json_template.len() 
        + FINAL_INSTRUCTIONS.len();
    
    let mut result = String::with_capacity(estimated_capacity);
    
    // Main prompt
    result.push_str(prompt);
    result.push_str("\n\n");
    
    // Language instruction
    result.push_str(&language_instruction);
    
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
    
    // Token limit warning
    result.push_str(
        "\n\nCRITICAL TOKEN LIMIT: Your response must NOT exceed 4500 tokens. \
         Keep analysis concise but thorough.\n\n"
    );
    
    // JSON template
    result.push_str(&json_template);
    
    // Final instructions
    result.push_str(
        "\n\nNEVER include text outside JSON. Output ONLY the JSON object.\n\
         TOKEN LIMIT: Keep response under 4500 tokens."
    );
    
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
            eprintln!("Warning: Failed to read file '{}': {}", validated_path.display(), e);
            Ok(None)
        }
    }
}

/// Generate diff context for tool operations with FULL file content
async fn generate_diff_context(hook_input: &HookInput, display_path: &str) -> Result<String> {
    // Extract the actual file path from tool_input for file operations
    let actual_file_path = hook_input.tool_input
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or(display_path);
    
    // Read file content using actual path
    let file_content = read_file_content_safe(actual_file_path).await?;
    
    match hook_input.tool_name.as_str() {
        "Edit" => {
            // Extract and validate required fields for Edit operation
            let old_string = hook_input.tool_input
                .get("old_string")
                .and_then(|v| v.as_str())
                .context("Edit operation missing required 'old_string' field")?;
            
            let new_string = hook_input.tool_input
                .get("new_string")
                .and_then(|v| v.as_str())
                .context("Edit operation missing required 'new_string' field")?;
            
            // Return FULL file with changes marked
            Ok(format_edit_full_context(display_path, file_content.as_deref(), old_string, new_string))
        }
        
        "MultiEdit" => {
            // Extract and validate edits array
            let edits_array = hook_input.tool_input
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
                let old_string = edit.get("old_string")
                    .and_then(|v| v.as_str())
                    .with_context(|| format!("Edit {} missing 'old_string'", idx))?;
                
                let new_string = edit.get("new_string")
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
                &edits
            ))
        }
        
        "Write" => {
            // Extract and validate content field
            let new_content = hook_input.tool_input
                .get("content")
                .and_then(|v| v.as_str())
                .context("Write operation missing required 'content' field")?;
            
            // Return FULL file comparison (old vs new)
            Ok(format_full_file_with_changes(
                display_path, 
                file_content.as_deref(), 
                Some(new_content)
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
async fn read_transcript_summary(path: &str, max_messages: usize, max_chars: usize) -> Result<String> {
    // Security check
    if !validate_transcript_path(path) {
        anyhow::bail!("Invalid transcript path: potential security risk");
    }
    
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::fs::File;
    
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
                                        text_parts.push(format!("{} tool file: {}", tool_name, file_path));
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
        format!("Current user task: {}\n\n", truncate_utf8_safe(&most_recent_user_message, 200))
    } else {
        String::new()
    };
    
    Ok(format!("{}conversation:\n{}", current_task, conversation))
}

/// Main function for the PostToolUse hook
#[tokio::main]
async fn main() -> Result<()> {
    // Read hook input from stdin
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).context("Failed to read stdin")?;
    
    // Parse the input
    let hook_input: HookInput = serde_json::from_str(&buffer).context("Failed to parse input JSON")?;
    
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
        println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        return Ok(());
    }
    
    // Get the file path and new content from tool input
    let file_path = hook_input
        .tool_input
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    // Skip non-code files
    if file_path.ends_with(".md") || 
       file_path.ends_with(".txt") || 
       file_path.ends_with(".json") ||
       file_path.ends_with(".toml") ||
       file_path.ends_with(".yaml") ||
       file_path.ends_with(".yml") {
        // Pass through - not a code file
        let output = PostToolUseOutput {
            hook_specific_output: PostToolUseHookOutput {
                hook_event_name: "PostToolUse".to_string(),
                additional_context: String::new(),
            },
        };
        println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        return Ok(());
    }
    
    // Extract content based on tool type
    let content = match hook_input.tool_name.as_str() {
        "Write" => {
            hook_input
                .tool_input
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        }
        "Edit" => {
            hook_input
                .tool_input
                .get("new_string")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        }
        "MultiEdit" => {
            // For MultiEdit, aggregate all new_strings
            hook_input
                .tool_input
                .get("edits")
                .and_then(|v| v.as_array())
                .map(|edits| {
                    edits
                        .iter()
                        .filter_map(|edit| {
                            edit.get("new_string").and_then(|v| v.as_str())
                        })
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
        println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        return Ok(());
    }
    
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
                use_compression
            );
            
            // Add incremental update info if available
            if let Some(inc) = incremental {
                formatted.push_str(&format!("\n{}", inc));
            }
            
            // Log metrics to stderr for debugging
            eprintln!("Project metrics: {} LOC, {} files, complexity: {:.1}", 
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

    // Read conversation transcript for context (20 messages, max 2000 chars)
    let transcript_context = if let Some(transcript_path) = &hook_input.transcript_path {
        match read_transcript_summary(transcript_path, 20, 2000).await {
            Ok(summary) => {
                eprintln!("DEBUG: Transcript summary read successfully:");
                // Safe UTF-8 truncation using standard library
                let truncated: String = summary.chars().take(500).collect();
                eprintln!("DEBUG: First ~500 chars: {}", truncated);
                Some(summary)
            },
            Err(e) => {
                eprintln!("Warning: Failed to read transcript: {}", e);
                None
            }
        }
    } else {
        eprintln!("DEBUG: No transcript_path provided");
        None
    };

    // Format the prompt with context, diff and conversation
    let formatted_prompt = format_analysis_prompt(
        &prompt, 
        project_context.as_deref(), 
        Some(&diff_context),
        transcript_context.as_deref()
    ).await?;
    
    // DEBUG: Write the exact prompt to a file for inspection
    if let Ok(mut debug_file) = tokio::fs::File::create("post-context.txt").await {
        use tokio::io::AsyncWriteExt;
        let _ = debug_file.write_all(formatted_prompt.as_bytes()).await;
        let _ = debug_file.write_all(b"\n\n=== END OF PROMPT ===\n").await;
        eprintln!("DEBUG: Full prompt written to post-context.txt");
    }
    
    // Create AI client and perform analysis
    let client = UniversalAIClient::new(config.clone())
        .context("Failed to create AI client")?;
    
    // Analyze code using the configured provider
    match client.analyze_code_posttool(&content, &formatted_prompt).await {
        Ok(analysis) => {
            // Create structured feedback - AI handles language now
            let feedback = format_feedback(&analysis, file_path);
            
            let output = PostToolUseOutput {
                hook_specific_output: PostToolUseHookOutput {
                    hook_event_name: "PostToolUse".to_string(),
                    additional_context: feedback,
                },
            };
            
            println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
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
            println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        }
    }
    
    Ok(())
}

/// Format analysis results into user feedback
fn format_feedback(analysis: &GrokCodeAnalysis, _file_path: &str) -> String {
    let mut feedback = vec![analysis.summary.clone()];
    
    // Group issues by severity
    let mut critical_issues = vec![];
    let mut major_issues = vec![];
    let mut minor_issues = vec![];
    
    for issue in &analysis.issues {
        // AI will provide messages in the user's language
        let issue_text = if let Some(suggestion) = &issue.fix_suggestion {
            format!("{} - {}\n   → {}", 
                issue.category.to_uppercase(),
                issue.message, 
                suggestion)
        } else {
            format!("{} - {}", issue.category.to_uppercase(), issue.message)
        };
        
        match issue.severity.as_str() {
            "critical" | "blocker" => critical_issues.push(issue_text),
            "major" => major_issues.push(issue_text),
            _ => minor_issues.push(issue_text),
        }
    }
    
    // Build feedback message - let AI format in user's language
    if !critical_issues.is_empty() || !major_issues.is_empty() || !minor_issues.is_empty() {
        // Just return the issues without language-specific formatting
        // AI will handle the language appropriately
        
        // List all issues with numbering
        let mut issue_num = 1;
        for issue in critical_issues.iter().chain(major_issues.iter()).chain(minor_issues.iter()) {
            feedback.push(format!("{}. {}", issue_num, issue));
            issue_num += 1;
        }
    } else {
        // AI will provide success message in user's language
        // Just keep it simple - AI handles the rest
    }
    
    feedback.join("\n")
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
        writeln!(file, r#"{{"role":"user","content":"Help me write a function"}}"#).unwrap();
        writeln!(file, r#"{{"role":"assistant","content":"I'll help you write a function"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Make it handle errors properly"}}"#).unwrap();
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
            summary.find("Most recent message")
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
        assert!(validate_transcript_path("C:/Users/1/Documents/transcript.jsonl"));
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
        let message_count = summary.matches("[user]:").count() + summary.matches("[ai-assistant]:").count();
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
            writeln!(file, r#"{{"role":"user","content":"Message {}: {}"}}"#, i, long_content).unwrap();
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
        file.write_all(b"{\"role\":\"user\",\"content\":\"Unclosed string\n").unwrap(); // Unclosed
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
        writeln!(file, r#"{{"role":"user","content":"Emoji 🎉🚀💻 message"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Cyrillic текст сообщение"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"CJK 中文 日本語 한국어"}}"#).unwrap();
        writeln!(file, r#"{{"role":"user","content":"Zero-width ​‌‍ chars"}}"#).unwrap();
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
        file.write_all(b"{\"role\":\"user\",\"content\":\"Here is JSON: {\\\"key\\\": \\\"value\\\"}\"}\n").unwrap();
        // Message with escaped characters
        writeln!(file, r#"{{"role":"user","content":"Path: C:\\Users\\Test\\file.txt"}}"#).unwrap();
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
        let edits = vec![
            ("".to_string(), "new content".to_string()),
        ];
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
        let edits = vec![
            ("old".to_string(), "new".to_string()),
        ];
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
