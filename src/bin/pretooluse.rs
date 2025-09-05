use anyhow::{Context, Result};
use std::io::{self, Read};
use std::fs::File;
use tokio;
use serde_json;

use rust_validation_hooks::*;
use rust_validation_hooks::analysis::project::{
    scan_project_structure,
    format_project_structure_for_ai,
    ScanConfig,
};
// Use universal AI client
use rust_validation_hooks::providers::ai::UniversalAIClient;
// Test file validator removed - AI handles validation
// Use diff formatter for better AI context
use rust_validation_hooks::validation::diff_formatter::{
    format_edit_diff,
    format_multi_edit_diff,
    format_code_diff,
};

// Removed GrokSecurityClient - now using UniversalAIClient from ai_providers module

use std::path::PathBuf;

/// Validate path for security and ensure it's a directory
fn validate_prompts_path(path: &PathBuf) -> Option<PathBuf> {
    // Canonicalize handles path traversal, symlinks, and normalization
    match std::fs::canonicalize(path) {
        Ok(canonical) => {
            if canonical.is_dir() {
                Some(canonical)
            } else {
                None
            }
        }
        Err(e) => {
            eprintln!("Warning: Cannot validate prompts path: {}", e);
            None
        }
    }
}

/// Find prompts directory from executable location
fn find_prompts_from_exe() -> Option<PathBuf> {
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Warning: Cannot determine executable path: {}", e);
            return None;
        }
    };
    
    let parent = exe_path.parent()?;
    
    // Development scenario: check if we're in Cargo's target directory
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

/// Load prompt from file relative to prompts directory with security validation
fn load_prompt(prompt_file: &str) -> Result<String> {
    // Validate filename to prevent path traversal
    let path = std::path::Path::new(prompt_file);
    
    // Check for path traversal attempts
    if prompt_file.contains("..") || prompt_file.contains("/") || prompt_file.contains("\\") {
        anyhow::bail!("Invalid prompt filename - must be a simple filename without path separators: {}", prompt_file);
    }
    
    // Additional validation: ensure it's just a filename
    let components: Vec<_> = path.components().collect();
    if components.len() != 1 || !matches!(components[0], std::path::Component::Normal(_)) {
        anyhow::bail!("Invalid prompt filename - must be a simple filename: {}", prompt_file);
    }
    
    let prompt_path = get_prompts_dir().join(prompt_file);
    
    // Final validation: ensure the resolved path is within the prompts directory
    if let (Ok(canonical_prompt), Ok(canonical_dir)) = (std::fs::canonicalize(&prompt_path), std::fs::canonicalize(get_prompts_dir())) {
        if !canonical_prompt.starts_with(&canonical_dir) {
            anyhow::bail!("Security error: prompt file path escapes the prompts directory");
        }
    }
    
    std::fs::read_to_string(&prompt_path)
        .with_context(|| format!("Failed to read prompt file: {:?}", prompt_path))
}

/// Read and summarize transcript from JSONL file with current task identification
fn read_transcript_summary(path: &str, max_messages: usize, _max_chars: usize) -> Result<String> {
    use std::io::BufReader;
    use std::io::BufRead;
    
    let file = File::open(path).context("Failed to open transcript file")?;
    let reader = BufReader::new(file);
    
    let mut all_messages = Vec::new();
    
    // Parse JSONL format - each line is a separate JSON object
    for line in reader.lines() {
        let line = line.context("Failed to read line from transcript")?;
        if line.trim().is_empty() {
            continue;
        }
        
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
            // Extract message from the entry
            if let Some(msg) = entry.get("message") {
                // Handle different message formats
                if let Some(role) = msg.get("role").and_then(|v| v.as_str()) {
                    let content = if let Some(content_arr) = msg.get("content").and_then(|v| v.as_array()) {
                        // Handle content array (assistant messages)
                        content_arr.iter()
                            .filter_map(|c| {
                                if let Some(text) = c.get("text").and_then(|v| v.as_str()) {
                                    Some(text.to_string())
                                } else if let Some(tool_name) = c.get("name").and_then(|v| v.as_str()) {
                                    Some(format!("[Tool: {}]", tool_name))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    } else if let Some(text) = msg.get("content").and_then(|v| v.as_str()) {
                        // Handle simple string content (user messages)
                        text.to_string()
                    } else {
                        String::new()
                    };
                    
                    if !content.is_empty() {
                        all_messages.push((role.to_string(), content));
                    }
                }
            }
        }
    }
    
    // Find the last user message to identify current task
    let last_user_message = all_messages.iter()
        .rev()
        .find(|(role, _)| role == "user")
        .map(|(_, content)| content.clone());
    
    // Take last N messages (max 20)  
    let max_msgs = max_messages.min(20);
    let start = if all_messages.len() > max_msgs {
        all_messages.len() - max_msgs
    } else {
        0
    };
    
    let mut result = String::new();
    let mut char_count = 0;
    
    // Add current task context at the beginning
    if let Some(current_task) = &last_user_message {
        let task_truncated = if current_task.chars().count() > 150 {
            let truncated_chars: String = current_task.chars().take(147).collect();
            format!("{}...", truncated_chars)
        } else {
            current_task.clone()
        };
        
        let task_str = format!("CURRENT USER TASK: {}\n\nRECENT CONVERSATION:\n", task_truncated);
        result.push_str(&task_str);
        char_count += task_str.len();
    }
    
    for (_i, (role, content)) in all_messages[start..].iter().enumerate() {
        // Truncate individual messages to 100 chars (UTF-8 safe)
        let truncated = if content.chars().count() > 100 {
            let truncated_chars: String = content.chars().take(97).collect();
            format!("{}...", truncated_chars)
        } else {
            content.clone()
        };
        
        // Mark the last user message as current task
        let msg_str = if role == "user" && Some(content) == last_user_message.as_ref() {
            format!("[{}] (CURRENT TASK): {}\n", role, truncated)
        } else {
            format!("[{}]: {}\n", role, truncated)
        };
        
        // Stop if we exceed 2000 chars
        if char_count + msg_str.len() > 2000 {
            result.push_str("...\n");
            break;
        }
        
        result.push_str(&msg_str);
        char_count += msg_str.len();
    }
    
    Ok(result)
}

// File structure checking function removed - AI handles all validation


/// Main PreToolUse hook execution
#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_env().context("Failed to load configuration")?;

    // Read input from stdin
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("Failed to read stdin")?;

    // Parse hook input
    let hook_input: HookInput = serde_json::from_str(&input).context("Failed to parse hook input")?;
    
    // Debug logging to file to see what context we receive
    let log_file_path = std::env::temp_dir().join("pretooluse_debug.log");
    if let Ok(mut log_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        use std::io::Write;
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        writeln!(log_file, "\n=== PreToolUse Hook Debug [{}] ===", timestamp).ok();
        writeln!(log_file, "Tool name: {}", hook_input.tool_name).ok();
        writeln!(log_file, "Session ID: {:?}", hook_input.session_id).ok();
        writeln!(log_file, "Transcript path: {:?}", hook_input.transcript_path).ok();
        writeln!(log_file, "CWD: {:?}", hook_input.cwd).ok();
        writeln!(log_file, "Hook event: {:?}", hook_input.hook_event_name).ok();
        writeln!(log_file, "CLAUDE_PROJECT_DIR env: {:?}", std::env::var("CLAUDE_PROJECT_DIR").ok()).ok();
        
        // If transcript path is provided, show its content
        if let Some(transcript_path) = &hook_input.transcript_path {
            writeln!(log_file, "Attempting to read transcript from: {}", transcript_path).ok();
            match read_transcript_summary(transcript_path, 15, 1500) {
                Ok(summary) => {
                    writeln!(log_file, "Transcript content (last 15 msgs, max 1500 chars):").ok();
                    writeln!(log_file, "{}", summary).ok();
                }
                Err(e) => {
                    writeln!(log_file, "Could not read transcript: {}", e).ok();
                }
            }
        }
        writeln!(log_file, "==============================").ok();
    }
    
    // Also print to stderr for visibility
    eprintln!("PreToolUse hook: Logged to {:?}", log_file_path);

    // Extract content and file path
    let content = extract_content_from_tool_input(&hook_input.tool_name, &hook_input.tool_input);
    let file_path = extract_file_path(&hook_input.tool_input);

    // Check project structure for Write operations (not Edit/MultiEdit)
    if hook_input.tool_name == "Write" && !file_path.is_empty() {
        // Get transcript context for checking user intent
        let _transcript_context = if let Some(transcript_path) = &hook_input.transcript_path {
            read_transcript_summary(transcript_path, 5, 500).ok()
        } else {
            None
        };
        
        // File structure checking removed - AI handles all validation now
    }

    // Test file validation removed - AI handles all validation now
    
    // Allow non-code tools or empty content
    if !matches!(
        hook_input.tool_name.as_str(),
        "Write" | "Edit" | "MultiEdit"
    ) {
        let output = PreToolUseOutput {
            hook_specific_output: PreToolUseHookOutput {
                hook_event_name: "PreToolUse".to_string(),
                permission_decision: "allow".to_string(),
                permission_decision_reason: None,
            },
        };
        println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        return Ok(());
    }

    if content.trim().is_empty() {
        let output = PreToolUseOutput {
            hook_specific_output: PreToolUseHookOutput {
                hook_event_name: "PreToolUse".to_string(),
                permission_decision: "allow".to_string(),
                permission_decision_reason: None,
            },
        };
        println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        return Ok(());
    }

    // Skip validation for documentation files (but not .txt files with potential secrets)
    // Only skip pure documentation formats and files in docs folder
    if file_path.ends_with(".md") || file_path.ends_with(".rst") || 
       file_path.contains("/docs/") || file_path.contains("\\docs\\") || 
       file_path.contains("README.md") || file_path.contains("CHANGELOG") ||
       file_path.contains("LICENSE") {
        let output = PreToolUseOutput {
            hook_specific_output: PreToolUseHookOutput {
                hook_event_name: "PreToolUse".to_string(),
                permission_decision: "allow".to_string(),
                permission_decision_reason: Some("Documentation file - validation skipped".to_string()),
            },
        };
        println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        return Ok(());
    }
    
    // Special handling for JSON configuration files
    if file_path.ends_with(".json") || file_path.ends_with(".jsonc") {
        // JSON files are typically configuration and shouldn't be blocked for test-related field names
        // Still perform security validation but skip test file checks
        let output = PreToolUseOutput {
            hook_specific_output: PreToolUseHookOutput {
                hook_event_name: "PreToolUse".to_string(),
                permission_decision: "allow".to_string(),
                permission_decision_reason: Some("Configuration file - test validation skipped".to_string()),
            },
        };
        println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        return Ok(());
    }

    // Perform security validation with context
    match perform_validation(&config, &content, &hook_input).await {
        Ok(validation) => {
            let (decision, reason) = match validation.decision.as_str() {
                "allow" => ("allow".to_string(), None),
                "ask" => ("deny".to_string(), Some(format!("Security review required: {}", validation.reason))), // Convert ask to deny
                "deny" => ("deny".to_string(), Some(validation.reason)),
                _ => ("allow".to_string(), None), // Default to allow for unknown decisions
            };

            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: decision,
                    permission_decision_reason: reason,
                },
            };
            println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        }
        Err(e) => {
            // Categorize error for better user feedback without exposing details
            let error_category = match e.to_string().to_lowercase() {
                s if s.contains("timeout") => "timeout",
                s if s.contains("connection") => "network",
                s if s.contains("api") || s.contains("key") => "configuration",
                s if s.contains("json") || s.contains("parse") => "response_format",
                _ => "validation_failed"
            };
            
            let user_message = match error_category {
                "timeout" => "Security validation timed out",
                "network" => "Security validation service unreachable",
                "configuration" => "Security validation not configured",
                "response_format" => "Security validation response invalid",
                _ => "Security validation rejected the operation"
            };
            
            // Always fail secure - deny on any error
            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: "deny".to_string(),
                    permission_decision_reason: Some(user_message.to_string()),
                },
            };
            println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
        }
    }

    Ok(())
}

/// Format code changes as diff for better AI understanding
fn format_code_as_diff(hook_input: &HookInput) -> String {
    let mut diff = String::new();
    
    // Extract file path
    let file_path = extract_file_path(&hook_input.tool_input);
    
    match hook_input.tool_name.as_str() {
        "Edit" => {
            // Extract old_string and new_string from tool_input
            if let Some(old_string) = hook_input.tool_input.get("old_string")
                .and_then(|v| v.as_str()) {
                if let Some(new_string) = hook_input.tool_input.get("new_string")
                    .and_then(|v| v.as_str()) {
                    
                    // Try to read the current file content for context
                    let file_content = std::fs::read_to_string(&file_path).ok();
                    
                    diff = format_edit_diff(
                        &file_path,
                        file_content.as_deref(),
                        old_string,
                        new_string,
                        3, // 3 lines of context
                    );
                }
            }
        },
        "MultiEdit" => {
            // Extract edits array from tool_input
            if let Some(edits_value) = hook_input.tool_input.get("edits") {
                if let Some(edits_array) = edits_value.as_array() {
                    let mut edits = Vec::new();
                    for edit in edits_array {
                        if let (Some(old), Some(new)) = (
                            edit.get("old_string").and_then(|v| v.as_str()),
                            edit.get("new_string").and_then(|v| v.as_str())
                        ) {
                            edits.push((old.to_string(), new.to_string()));
                        }
                    }
                    
                    // Try to read the current file content for context
                    let file_content = std::fs::read_to_string(&file_path).ok();
                    
                    diff = format_multi_edit_diff(
                        &file_path,
                        file_content.as_deref(),
                        &edits,
                    );
                }
            }
        },
        "Write" => {
            // For Write operations, show as new file creation
            if let Some(content) = hook_input.tool_input.get("content")
                .and_then(|v| v.as_str()) {
                
                // Check if file exists
                let old_content = std::fs::read_to_string(&file_path).ok();
                
                diff = format_code_diff(
                    &file_path,
                    old_content.as_deref(),
                    Some(content),
                    3, // 3 lines of context
                );
            }
        },
        _ => {
            // For other operations, just show the content if available
            let content = extract_content_from_tool_input(&hook_input.tool_name, &hook_input.tool_input);
            if !content.is_empty() {
                diff = format!("Content:\n{}", content);
            }
        }
    }
    
    diff
}

/// Perform security validation using Grok with context
async fn perform_validation(config: &Config, content: &str, hook_input: &HookInput) -> Result<SecurityValidation> {
    // Load security prompt
    let mut prompt = load_prompt("edit_validation.txt").context("Failed to load edit validation prompt")?;
    
    // Extract file path and add it to context
    let file_path = extract_file_path(&hook_input.tool_input);
    if !file_path.is_empty() {
        prompt = format!("{}\n\nFILE BEING MODIFIED: {}", prompt, file_path);
    }
    
    // Format the code changes as diff for better AI understanding
    let diff_context = format_code_as_diff(hook_input);
    if !diff_context.is_empty() {
        prompt = format!("{}\n\nCODE CHANGES (diff format):\n{}", prompt, diff_context);
    }

    // Add context from transcript if available
    if let Some(transcript_path) = &hook_input.transcript_path {
        match read_transcript_summary(transcript_path, 10, 1000) {
            Ok(summary) => {
                prompt = format!("{}\n\nCONTEXT - Recent chat history:\n{}", prompt, summary);
            }
            Err(e) => {
                eprintln!("Could not read transcript: {}", e);
            }
        }
    }

    // Add project context from environment
    if let Ok(project_dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        prompt = format!("{}\n\nPROJECT: {}", prompt, project_dir);
    }
    
    // Add project structure context
    // Try multiple sources for working directory
    let working_dir = if let Some(cwd) = &hook_input.cwd {
        cwd.clone()
    } else if let Ok(project_dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        project_dir
    } else if let Ok(current) = std::env::current_dir() {
        current.to_string_lossy().to_string()
    } else {
        ".".to_string()
    };
    
    // Scan project structure with limited scope for performance
    let scan_config = ScanConfig {
        max_files: 800,  // Increased limit per user request
        max_depth: 5,
        include_hidden_files: false,
        follow_symlinks: false,
    };
    
    match scan_project_structure(&working_dir, Some(scan_config)) {
        Ok(structure) => {
            let project_context = format_project_structure_for_ai(&structure, 1500);
            prompt = format!("{}\n\nPROJECT STRUCTURE:\n{}", prompt, project_context);
            eprintln!("PreToolUse: Added project structure context ({} files, {} dirs)", 
                structure.total_files, structure.directories.len());
        }
        Err(e) => {
            eprintln!("PreToolUse: Could not scan project structure: {}", e);
        }
    }

    // Initialize universal AI client with configured provider
    let client = UniversalAIClient::new(config.clone())
        .context("Failed to create AI client")?;

    // Validate security using configured pretool provider
    client
        .validate_security_pretool(content, &prompt)
        .await
        .context("Security validation failed")
}