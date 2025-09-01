use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use std::io::{self, Read};
use std::fs::File;
use tokio;
use serde_json;

use rust_validation_hooks::*;
use rust_validation_hooks::project_context::{
    scan_project_structure,
    format_project_structure_for_ai,
    ScanConfig,
};

/// xAI Grok client for security validation
struct GrokSecurityClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl GrokSecurityClient {
    fn new(api_key: String, base_url: String, model: String) -> Self {
        // Create client with optimized timeout settings
        let client = Client::builder()
            .timeout(Duration::from_secs(10))  // 10 second timeout for faster failures
            .connect_timeout(Duration::from_secs(5))  // 5 second connection timeout
            .build()
            .unwrap_or_else(|_| Client::new());
            
        Self {
            client,
            api_key,
            base_url,
            model,
        }
    }

    async fn validate_security(&self, code: &str, prompt: &str) -> Result<SecurityValidation> {
        // Create strict JSON schema for security validation
        let json_schema = serde_json::json!({
            "type": "object",
            "required": ["decision", "reason", "risk_level"],
            "additionalProperties": false,
            "properties": {
                "decision": {
                    "type": "string",
                    "enum": ["allow", "ask", "deny"],
                    "description": "Permission decision"
                },
                "reason": {
                    "type": "string",
                    "maxLength": 200,
                    "description": "Brief explanation for the decision"
                },
                "security_concerns": {
                    "type": "array",
                    "maxItems": 3,
                    "items": {
                        "type": "string",
                        "maxLength": 100
                    },
                    "description": "List of security concerns found"
                },
                "risk_level": {
                    "type": "string",
                    "enum": ["low", "medium", "high", "critical"],
                    "description": "Overall risk assessment"
                }
            }
        });

        let request_body = serde_json::json!({
            "messages": [
                {
                    "role": "system",
                    "content": prompt
                },
                {
                    "role": "user",
                    "content": format!("Analyze this code for security risks before execution:\n\n{}", code)
                }
            ],
            "model": self.model,
            "max_tokens": 1024,
            "temperature": 0.1,
            "stream": false,
            "response_format": {
                "type": "json_schema", 
                "json_schema": {
                    "name": "SecurityValidation",
                    "description": "Security validation decision",
                    "schema": json_schema,
                    "strict": true
                }
            }
        });

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send Grok security validation request")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Grok API error: {}", error_text);
        }

        let grok_response: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Grok response")?;

        // Safe response parsing with proper error handling
        let choices = grok_response["choices"].as_array()
            .context("Grok response missing 'choices' array")?;
        
        if choices.is_empty() {
            anyhow::bail!("Grok response contains empty 'choices' array");
        }
        
        let message = choices[0]["message"].as_object()
            .context("Grok response missing 'message' object")?;
            
        let content = message["content"].as_str()
            .context("Grok response missing 'content' field")?;

        let security_result: SecurityValidation = serde_json::from_str(content)
            .context("Failed to parse security validation result")?;

        Ok(security_result)
    }
}

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

    // Perform security validation with context
    match perform_validation(&config, &content, &hook_input).await {
        Ok(validation) => {
            let (decision, reason) = match validation.decision.as_str() {
                "allow" => ("allow".to_string(), None),
                "ask" => ("ask".to_string(), Some(validation.reason)),
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
            // Fail open on errors - allow without reason (per Claude Code Hooks spec)
            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: "allow".to_string(),
                    permission_decision_reason: None,
                },
            };
            println!("{}", serde_json::to_string(&output).context("Failed to serialize output")?);
            eprintln!("PreToolUse hook error: ⚠️ Security validation unavailable: {} - Operation allowed but not validated", e);
        }
    }

    Ok(())
}

/// Perform security validation using Grok with context
async fn perform_validation(config: &Config, content: &str, hook_input: &HookInput) -> Result<SecurityValidation> {
    // Load security prompt
    let mut prompt = load_prompt("edit_validation.txt").context("Failed to load edit validation prompt")?;

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

    // Initialize Grok client
    let client = GrokSecurityClient::new(
        config.xai_api_key.clone(), 
        config.xai_base_url.clone(),
        config.pretool_model.clone()
    );

    // Validate with Grok
    client
        .validate_security(content, &prompt)
        .await
        .context("Grok security validation failed")
}