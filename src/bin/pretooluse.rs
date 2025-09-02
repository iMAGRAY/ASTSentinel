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
// Use test file validator
use rust_validation_hooks::validation::test_files::{
    validate_test_file,
    detect_test_content,
    TestFileConfig,
};
// Use diff formatter for better AI context
use rust_validation_hooks::validation::diff_formatter::{
    format_edit_diff,
    format_multi_edit_diff,
    format_code_diff,
};

// Removed GrokSecurityClient - now using UniversalAIClient from ai_providers module

/// Load prompt from file relative to executable location  
fn load_prompt(prompt_file: &str) -> Result<String> {
    // Robust path validation using Path components
    let path = std::path::Path::new(prompt_file);
    let components: Vec<_> = path.components().collect();
    
    // Ensure it's a simple filename (single Normal component)
    if components.len() != 1 || !matches!(components[0], std::path::Component::Normal(_)) {
        anyhow::bail!("Invalid prompt filename, must be simple filename: {}", prompt_file);
    }
    
    // Get executable directory with explicit handling
    let exe_path = std::env::current_exe().context("Failed to get executable path")?;
    let exe_dir = exe_path.parent().context("Executable has no parent directory")?;
    let prompt_path = exe_dir.join("prompts").join(prompt_file);
    
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

/// Simple check for obvious file placement issues
fn check_file_structure(file_path: &str, transcript_context: Option<&str>) -> Option<String> {
    // Normalize path separators
    let path = file_path.replace('\\', "/").to_lowercase();
    
    // Check if user explicitly requested this location in current task
    if let Some(context) = transcript_context {
        let context_lower = context.to_lowercase();
        // Look for explicit location requests
        if context_lower.contains("in root") || 
           context_lower.contains("in the root") ||
           context_lower.contains("at root") ||
           context_lower.contains("create it wherever") ||
           context_lower.contains("put it anywhere") {
            return None; // User explicitly requested, allow
        }
    }
    
    // Extract filename
    let filename = path.split('/').last().unwrap_or(&path);
    
    // ONLY CHECK: Test files outside test directories
    if filename.contains("test.") || filename.contains("_test.") || 
       filename.contains(".spec.") || filename.contains("_spec.") {
        // Check if NOT in test-related directories
        // Path can start with test/tests or contain /test anywhere
        let in_test_dir = path.starts_with("test") || path.contains("/test") || 
                          path.starts_with("spec") || path.contains("/spec") ||
                          path.contains("__test");
        
        if !in_test_dir {
            return Some("DENY: Test files must be placed in test/tests/spec directories".to_string());
        }
    }
    
    None // Let AI handle everything else
}


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
        let transcript_context = if let Some(transcript_path) = &hook_input.transcript_path {
            read_transcript_summary(transcript_path, 5, 500).ok()
        } else {
            None
        };
        
        // Check if file placement violates project structure
        if let Some(structure_issue) = check_file_structure(&file_path, transcript_context.as_deref()) {
            // Check if it's a DENY case (starts with "DENY:")
            let (decision, reason) = if structure_issue.starts_with("DENY:") {
                ("deny".to_string(), structure_issue.replace("DENY: ", ""))
            } else {
                ("deny".to_string(), format!("Project structure violation: {}. File placement violates project conventions.", structure_issue))
            };
            
            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: decision,
                    permission_decision_reason: Some(reason),
                },
            };
            println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
            return Ok(());
        }
    }

    // Check for test files in write operations
    if matches!(hook_input.tool_name.as_str(), "Write" | "Edit" | "MultiEdit") && !content.is_empty() {
        // Check for test files outside test directories
        // Skip validation for source files in src/ directory - they're not tests
        let should_check_test_files = !file_path.contains("/src/") && !file_path.contains("\\src\\");
        
        if should_check_test_files {
            let test_config = TestFileConfig::from_env();
            let test_validation = validate_test_file(&file_path, &test_config);
            
            // Only check content for JavaScript/TypeScript files, not Rust source files
            let is_js_ts = file_path.ends_with(".js") || file_path.ends_with(".ts") || 
                          file_path.ends_with(".jsx") || file_path.ends_with(".tsx");
            
            let is_test_by_content = if !test_validation.is_test_file && !content.is_empty() && is_js_ts {
                detect_test_content(&content)
            } else {
                false
            };
            
            // Block test files outside test directories
            if test_validation.should_block || (is_test_by_content && !test_validation.is_in_test_directory) {
            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: "deny".to_string(),
                    permission_decision_reason: Some(format!(
                        "TEST FILE BLOCKED: {}. Place test files in designated directories: tests/, test/, __tests__, spec/, or fixtures/",
                        if test_validation.should_block { 
                            test_validation.reason 
                        } else {
                            format!("Test content detected in '{}' outside test directories", file_path)
                        }
                    )),
                },
            };
            println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
            eprintln!("⚠️ BLOCKED TEST FILE: {} - should be in test directory", file_path);
            return Ok(());
            }
        }
    }
    
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

    // Skip validation for documentation files
    if file_path.ends_with(".md") || file_path.ends_with(".rst") || 
       file_path.ends_with(".txt") || file_path.contains("/docs/") ||
       file_path.contains("\\docs\\") || file_path.contains("README") {
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
                        3, // 3 lines of context
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