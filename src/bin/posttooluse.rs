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

// Removed GrokAnalysisClient - now using UniversalAIClient from ai_providers module

/// Format the analysis prompt with instructions and project context
fn format_analysis_prompt(prompt: &str, project_context: Option<&str>) -> String {
    let context_section = if let Some(context) = project_context {
        format!("\n\nPROJECT CONTEXT:\n{}\n", context)
    } else {
        String::new()
    };
    
    format!("{}{}

CRITICAL TOKEN LIMIT: Your response must NOT exceed 4500 tokens. Keep analysis concise but thorough.

IMPORTANT: Output ONLY valid JSON object with these fields:
{{
  \"summary\": \"[ ATTEMPT N | Rewards +X | Penalty +Y | Code Quality: Z/100 ] Brief assessment\",
  \"overall_quality\": \"excellent|good|needs_improvement|poor\",
  \"issues\": [
    {{
      \"severity\": \"info|minor|major|critical|blocker\",
      \"category\": \"intent|correctness|security|robustness|maintainability|performance|tests|lint\",
      \"message\": \"Issue description\",
      \"impact\": 1-3,
      \"fix_cost\": 1-3,
      \"confidence\": 0.5-1.0,
      \"fix_suggestion\": \"How to fix\"
    }}
  ],
  \"suggestions\": [
    {{
      \"category\": \"performance|security|maintainability|tests\",
      \"description\": \"Improvement description\",
      \"priority\": \"high|medium|low\"
    }}
  ],
  \"metrics\": {{
    \"complexity\": \"low|medium|high|extreme\",
    \"readability\": \"excellent|good|fair|poor\",
    \"test_coverage\": \"none|partial|good|excellent\"
  }}
}}

NEVER include text outside JSON. Output ONLY the JSON object.
TOKEN LIMIT: Keep response under 4500 tokens.", prompt, context_section)
}

// Use analysis structures from lib.rs

/// Load prompt from file relative to executable location
async fn load_prompt(prompt_file: &str) -> Result<String> {
    // Use Path for robust handling
    let exe_path = std::env::current_exe().context("Failed to get executable path")?;
    let exe_dir = exe_path.parent().context("Executable has no parent directory")?;
    let prompt_path = exe_dir.join("prompts").join(prompt_file);
    
    tokio::fs::read_to_string(&prompt_path)
        .await
        .with_context(|| format!("Failed to read prompt file: {:?}", prompt_path))
}

#[allow(dead_code)]
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

#[allow(dead_code)]
/// Read and summarize transcript from JSONL file
async fn read_transcript_summary(path: &str, max_messages: usize, max_chars: usize) -> Result<String> {
    // Security check
    if !validate_transcript_path(path) {
        anyhow::bail!("Invalid transcript path: potential security risk");
    }
    
    let contents = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read transcript file")?;
    
    let mut messages = Vec::new();
    let mut total_chars = 0;
    
    // Parse JSONL format - each line is a separate JSON object
    for line in contents.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
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
                        let msg_summary = format!("{}: {}", role, truncate_utf8_safe(&content, 200));
                        total_chars += msg_summary.len();
                        
                        messages.push(msg_summary);
                        
                        if messages.len() >= max_messages || total_chars >= max_chars {
                            break;
                        }
                    }
                }
            }
        }
    }
    
    // Reverse to get chronological order
    messages.reverse();
    
    Ok(messages.join("\n"))
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
    let prompt = load_prompt("post_edit_validation.txt")
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

    // Format the prompt with context
    let formatted_prompt = format_analysis_prompt(&prompt, project_context.as_deref());
    
    // Create AI client and perform analysis
    let client = UniversalAIClient::new(config.clone())
        .context("Failed to create AI client")?;
    
    // Analyze code using the configured provider
    match client.analyze_code_posttool(&content, &formatted_prompt).await {
        Ok(analysis) => {
            // Create structured feedback
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
        let issue_text = if let Some(suggestion) = &issue.fix_suggestion {
            format!("{} - {}\n   → Рекомендация: {}", 
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
    
    // Build feedback message
    if !critical_issues.is_empty() || !major_issues.is_empty() || !minor_issues.is_empty() {
        let total = critical_issues.len() + major_issues.len() + minor_issues.len();
        let important = critical_issues.len() + major_issues.len();
        
        feedback.push(format!("\nНайдено {} проблем: {} важных, {} минорных", 
            total, important, minor_issues.len()));
        
        // Add guidance on what to fix first
        if !critical_issues.is_empty() {
            feedback.push(format!("\n→ Начните с критической проблемы: {}", 
                critical_issues[0].split(" - ").nth(1).unwrap_or("").split('\n').next().unwrap_or("")));
        } else if !major_issues.is_empty() {
            feedback.push(format!("\n→ Начните с важной проблемы: {}", 
                major_issues[0].split(" - ").nth(1).unwrap_or("").split('\n').next().unwrap_or("")));
        } else if !minor_issues.is_empty() {
            feedback.push("→ Все проблемы минорные, начните с первой по списку".to_string());
        }
        
        // List all issues with numbering
        let mut issue_num = 1;
        for issue in critical_issues.iter().chain(major_issues.iter()).chain(minor_issues.iter()) {
            let severity_tag = if critical_issues.contains(issue) {
                "[КРИТИЧНО]"
            } else if major_issues.contains(issue) {
                "[ВАЖНО]"
            } else {
                "[МИНОР]"
            };
            
            feedback.push(format!("{}. {} {}", issue_num, issue, severity_tag));
            issue_num += 1;
        }
    } else {
        feedback.push("\n✅ Отличная работа! Серьезных проблем не обнаружено.".to_string());
    }
    
    feedback.join("\n")
}

