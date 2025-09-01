use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use std::io::{self, Read};
use std::fs::File;
use std::path::Path;
use tokio;
use serde_json;

use rust_validation_hooks::*;

/// UTF-8 safe string truncation with ellipsis (counts Unicode chars, not bytes)
fn truncate_utf8_safe(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}


/// Enhanced xAI Grok client with structured JSON schema
struct GrokAnalysisClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl GrokAnalysisClient {
    fn new(api_key: String, base_url: String, model: String, config: &Config) -> Result<Self> {
        // Create client with configurable timeout settings - proper error handling
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .user_agent("claude-code-rust-hooks/1.0")
            .tcp_keepalive(Duration::from_secs(30))
            .tcp_nodelay(true)
            .pool_max_idle_per_host(0)  // Disable connection pooling
            .build()
            .context("Failed to create HTTP client with configured timeouts")?;
            
        Ok(Self {
            client,
            api_key,
            base_url,
            model,
        })
    }

    async fn analyze_code(&self, code: &str, prompt: &str, config: &Config) -> Result<GrokCodeAnalysis> {
        // Log API call details for debugging
        if let Ok(mut log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("posttooluse_debug.log"))
        {
            use std::io::Write;
            writeln!(log_file, "\n=== GROK API CALL ===").ok();
            writeln!(log_file, "Model: {}", self.model).ok();
            writeln!(log_file, "Base URL: {}", self.base_url).ok();
            writeln!(log_file, "API Key present: {}", !self.api_key.is_empty()).ok();
            writeln!(log_file, "API Key length: {}", self.api_key.len()).ok();
            writeln!(log_file, "API Key prefix: {}", &self.api_key.chars().take(8).collect::<String>()).ok();
            writeln!(log_file, "Code length: {}", code.len()).ok();
            writeln!(log_file, "Prompt length: {}", prompt.len()).ok();
            writeln!(log_file, "Max tokens: {}", config.max_tokens).ok();
            writeln!(log_file, "Temperature: {}", config.temperature).ok();
        }

        // Create request without strict JSON schema
        let request = GrokRequest {
            model: self.model.clone(),
            messages: vec![
                GrokMessage {
                    role: "system".to_string(),
                    content: format!("{}

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
      \"description\": \"Improvement\",
      \"priority\": \"high|medium|low\",
      \"priority_score\": 0-100
    }}
  ],
  \"metrics\": {{
    \"complexity\": \"low|medium|high\",
    \"readability\": \"excellent|good|fair|poor\",
    \"test_coverage\": \"none|partial|good|excellent\"
  }}
}}", prompt),
                },
                GrokMessage {
                    role: "user".to_string(),
                    content: format!("Analyze this code and output JSON only:\n\n```\n{}\n```", code),
                },
            ],
            max_tokens: config.max_tokens,  // Configurable per environment
            temperature: config.temperature,  // Configurable per environment
            stream: false,
            response_format: Some(GrokResponseFormat {
                format_type: "json_schema".to_string(),
                json_schema: GrokJsonSchema {
                    name: "GrokCodeAnalysis".to_string(),
                    schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "summary": {"type": "string"},
                            "overall_quality": {"type": "string", "enum": ["excellent", "good", "needs_improvement", "poor"]},
                            "issues": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "severity": {"type": "string", "enum": ["info", "minor", "major", "critical", "blocker"]},
                                        "category": {"type": "string", "enum": ["intent", "correctness", "security", "robustness", "maintainability", "performance", "tests", "lint"]},
                                        "message": {"type": "string"},
                                        "impact": {"type": "integer", "minimum": 1, "maximum": 3},
                                        "fix_cost": {"type": "integer", "minimum": 1, "maximum": 3},
                                        "confidence": {"type": "number", "minimum": 0.5, "maximum": 1.0},
                                        "fix_suggestion": {"type": "string"}
                                    },
                                    "required": ["severity", "category", "message"]
                                }
                            },
                            "suggestions": {
                                "type": "array", 
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "category": {"type": "string"},
                                        "description": {"type": "string"},
                                        "priority": {"type": "string", "enum": ["high", "medium", "low"]},
                                        "priority_score": {"type": "number", "minimum": 0, "maximum": 100}
                                    },
                                    "required": ["category", "description", "priority"]
                                }
                            },
                            "metrics": {
                                "type": "object",
                                "properties": {
                                    "complexity": {"type": "string", "enum": ["low", "medium", "high"]},
                                    "readability": {"type": "string", "enum": ["excellent", "good", "fair", "poor"]},
                                    "test_coverage": {"type": "string", "enum": ["none", "partial", "good", "excellent"]}
                                }
                            }
                        },
                        "required": ["summary", "overall_quality", "issues", "suggestions"]
                    })
                }
            }),
        };

        // Log request payload (without sensitive data)
        if let Ok(mut log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("posttooluse_debug.log"))
        {
            use std::io::Write;
            writeln!(log_file, "Request model: {}", request.model).ok();
            writeln!(log_file, "Request messages count: {}", request.messages.len()).ok();
            writeln!(log_file, "System prompt length: {}", request.messages[0].content.len()).ok();
            writeln!(log_file, "User message length: {}", request.messages[1].content.len()).ok();
            writeln!(log_file, "Making HTTP request...").ok();
        }

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Grok Code Fast 1")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            
            // Detailed error logging
            if let Ok(mut log_file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(std::env::temp_dir().join("posttooluse_debug.log"))
            {
                use std::io::Write;
                writeln!(log_file, "\n=== GROK API ERROR ===").ok();
                writeln!(log_file, "HTTP Status: {}", status).ok();
                writeln!(log_file, "Status Code: {}", status.as_u16()).ok();
                writeln!(log_file, "Error Response Length: {}", error_text.len()).ok();
                writeln!(log_file, "Error Response: {}", error_text).ok();
                writeln!(log_file, "Request URL: {}/chat/completions", self.base_url).ok();
                writeln!(log_file, "========================").ok();
            }
            
            eprintln!("Grok API error - Status: {}, Response: {}", status, error_text);
            return Err(anyhow::anyhow!(
                "xAI Grok API error: {} {}",
                status,
                error_text
            ));
        }

        let grok_response: GrokResponse = response
            .json()
            .await
            .context("Failed to parse Grok response")?;

        // Log successful response details
        if let Ok(mut log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("posttooluse_debug.log"))
        {
            use std::io::Write;
            writeln!(log_file, "\n=== GROK API SUCCESS ===").ok();
            writeln!(log_file, "Response choices count: {}", grok_response.choices.len()).ok();
            if !grok_response.choices.is_empty() {
                let content = &grok_response.choices[0].message.content;
                writeln!(log_file, "Response content length: {}", content.len()).ok();
                writeln!(log_file, "Response content preview (first 300 chars): {}", &content.chars().take(300).collect::<String>()).ok();
                writeln!(log_file, "Response content ends with: {}", &content.chars().rev().take(100).collect::<String>()).ok();
            }
            writeln!(log_file, "About to parse response...").ok();
        }

        // Try to parse the response, with auto-fix for common issues
        let content = &grok_response.choices[0].message.content;
        let analysis_result = parse_grok_response(content)?;

        Ok(analysis_result)
    }
}

/// Load prompt from file relative to executable location (ASYNC)
async fn load_prompt(prompt_file: &str) -> Result<String> {
    // Validate prompt filename to prevent path traversal
    let path = Path::new(prompt_file);
    let components: Vec<_> = path.components().collect();
    
    // Ensure it's a simple filename (single Normal component)
    if components.len() != 1 || !matches!(components[0], std::path::Component::Normal(_)) {
        anyhow::bail!("Invalid prompt filename, must be simple filename: {}", prompt_file);
    }
    
    // Always use path relative to the executable location for universal deployment
    let exe_path = std::env::current_exe().context("Failed to get executable path")?;
    let exe_dir = exe_path.parent().context("Failed to get executable directory")?;
    let prompt_path = exe_dir.join("prompts").join(prompt_file);
    
    tokio::fs::read_to_string(&prompt_path)
        .await
        .with_context(|| format!("Failed to read prompt file: {:?}", prompt_path))
}

/// Validate transcript path for security
/// Parse Grok response with robust JSON extraction and detailed logging
fn parse_grok_response(content: &str) -> Result<GrokCodeAnalysis> {
    // Detailed logging of raw response for debugging
    if let Ok(mut log_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::temp_dir().join("posttooluse_debug.log"))
    {
        use std::io::Write;
        writeln!(log_file, "\n=== GROK RESPONSE PARSING ===").ok();
        writeln!(log_file, "Raw response length: {} chars", content.len()).ok();
        writeln!(log_file, "Raw response preview (first 500 chars): {}", &content.chars().take(500).collect::<String>()).ok();
        writeln!(log_file, "Contains markdown: {}", content.contains("```")).ok();
        writeln!(log_file, "Contains json marker: {}", content.contains("```json")).ok();
    }
    
    // Strategy 1: Direct JSON parsing (most common case)
    let trimmed = content.trim();
    if let Ok(result) = serde_json::from_str::<GrokCodeAnalysis>(trimmed) {
        log_parsing_success("Direct parsing");
        return Ok(result);
    }
    
    // Strategy 2: Extract from JSON code blocks
    let extracted_json = extract_json_from_content(content);
    if !extracted_json.is_empty() && extracted_json != content {
        if let Ok(result) = serde_json::from_str::<GrokCodeAnalysis>(&extracted_json) {
            log_parsing_success("JSON block extraction");
            return Ok(result);
        }
    }
    
    // Strategy 3: Find JSON object boundaries
    if let Some(json_content) = find_json_object(content) {
        if let Ok(result) = serde_json::from_str::<GrokCodeAnalysis>(&json_content) {
            log_parsing_success("JSON boundary detection");
            return Ok(result);
        }
    }
    
    // Strategy 4: Manual JSON repair (last resort)
    if let Some(repaired) = attempt_json_repair(content) {
        if let Ok(result) = serde_json::from_str::<GrokCodeAnalysis>(&repaired) {
            log_parsing_success("JSON repair");
            return Ok(result);
        }
    }
    
    // Log complete parsing failure with full details
    log_parsing_failure(content, &extracted_json);
    
    // Return fallback analysis
    Ok(create_fallback_analysis("JSON parsing failed - check debug logs"))
}

/// Extract JSON from various content formats
fn extract_json_from_content(content: &str) -> String {
    // Try JSON code blocks first
    if let Some(start) = content.find("```json") {
        let after_marker = &content[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim().to_string();
        }
    }
    
    // Try generic code blocks
    if let Some(start) = content.find("```") {
        let after_marker = &content[start + 3..];
        // Skip language identifier line if present
        let content_start = if let Some(newline) = after_marker.find('\n') {
            &after_marker[newline + 1..]
        } else {
            after_marker
        };
        
        if let Some(end) = content_start.find("```") {
            let candidate = content_start[..end].trim();
            // Only return if it looks like JSON
            if candidate.starts_with('{') && candidate.ends_with('}') {
                return candidate.to_string();
            }
        }
    }
    
    String::new()
}

/// Find JSON object within text by looking for braces
fn find_json_object(content: &str) -> Option<String> {
    let mut brace_count = 0;
    let mut start_pos = None;
    
    for (i, ch) in content.char_indices() {
        match ch {
            '{' => {
                if brace_count == 0 {
                    start_pos = Some(i);
                }
                brace_count += 1;
            }
            '}' => {
                brace_count -= 1;
                if brace_count == 0 && start_pos.is_some() {
                    let start = start_pos.unwrap();
                    let candidate = content[start..=i].trim();
                    // Basic validation that this looks like JSON
                    if candidate.len() > 10 && candidate.contains('"') {
                        return Some(candidate.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    
    None
}

/// Attempt to repair malformed JSON (conservative approach)
fn attempt_json_repair(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    
    // Only try to repair if we have a clear truncation scenario
    if trimmed.ends_with(',') || trimmed.ends_with(':') {
        // Find the last complete field
        let mut fixed = String::new();
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        
        for ch in trimmed.chars() {
            if escape_next {
                escape_next = false;
                fixed.push(ch);
                continue;
            }
            
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => brace_count -= 1,
                _ => {}
            }
            
            fixed.push(ch);
        }
        
        // Close unclosed braces
        while brace_count > 0 {
            fixed.push('}');
            brace_count -= 1;
        }
        
        return Some(fixed);
    }
    
    None
}

/// Create fallback analysis when parsing fails
fn create_fallback_analysis(error_msg: &str) -> GrokCodeAnalysis {
    GrokCodeAnalysis {
        summary: format!("[ ATTEMPT 1 | Rewards +0 | Penalty +0 | Code Quality: 50/100 ] {}", error_msg),
        overall_quality: "needs_improvement".to_string(),
        issues: vec![GrokCodeIssue {
            severity: "major".to_string(),
            category: "lint".to_string(),
            message: "Analysis service temporarily unavailable".to_string(),
            line: None,
            impact: Some(1),
            fix_cost: Some(1),
            confidence: Some(1.0),
            fix_suggestion: Some("Check service logs for details".to_string()),
        }],
        suggestions: vec![],
        metrics: Some(GrokCodeMetrics {
            complexity: Some("unknown".to_string()),
            readability: Some("unknown".to_string()),
            test_coverage: Some("none".to_string()),
        }),
    }
}

/// Log parsing success with method used
fn log_parsing_success(method: &str) {
    if let Ok(mut log_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::temp_dir().join("posttooluse_debug.log"))
    {
        use std::io::Write;
        writeln!(log_file, "PARSING SUCCESS: {} method worked", method).ok();
    }
}

/// Log detailed parsing failure information
fn log_parsing_failure(original: &str, extracted: &str) {
    if let Ok(mut log_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::temp_dir().join("posttooluse_debug.log"))
    {
        use std::io::Write;
        writeln!(log_file, "PARSING FAILED - All strategies exhausted").ok();
        writeln!(log_file, "Original length: {}, Extracted length: {}", original.len(), extracted.len()).ok();
        writeln!(log_file, "Original starts with: {}", &original.chars().take(50).collect::<String>()).ok();
        writeln!(log_file, "Original ends with: {}", &original.chars().rev().take(50).collect::<String>()).ok();
        if !extracted.is_empty() {
            writeln!(log_file, "Extracted JSON attempt: {}", &extracted.chars().take(200).collect::<String>()).ok();
        }
        writeln!(log_file, "==============================").ok();
    }
}

fn validate_transcript_path(path: &str) -> bool {
    // Check for path traversal attempts
    if path.contains("..") || path.contains("~") || path.contains("\\\\") {
        return false;
    }
    
    // Check for suspicious patterns
    if path.contains("%") || path.contains('\0') {
        return false;
    }
    
    // Allow only paths within temp directories or project directories
    // Claude Code typically uses temp directories for transcripts
    // Additional check: ensure path exists and is a file
    match std::fs::metadata(path) {
        Ok(metadata) => metadata.is_file(),
        Err(_) => false
    }
}

/// Read and summarize transcript from JSONL file with current task identification
fn read_transcript_summary(path: &str, max_messages: usize, _max_chars: usize) -> Result<String> {
    use std::io::{BufReader, BufRead};
    
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
        
        // Stop if we exceed 1000 chars  
        if char_count + msg_str.len() > 1000 {
            result.push_str("...\n");
            break;
        }
        
        result.push_str(&msg_str);
        char_count += msg_str.len();
    }
    
    Ok(result)
}

/// Format analysis into clean, structured output for terminal display
fn format_structured_analysis(analysis: &GrokCodeAnalysis) -> String {
    let mut output = String::new();
    
    // Header with summary 
    output.push_str(&analysis.summary);
    
    // Add line separator
    output.push_str("\n");
    output.push_str("=".repeat(60).as_str());
    output.push_str("\n");
    
    // Issues section
    if !analysis.issues.is_empty() {
        output.push_str("ОБНАРУЖЕННЫЕ ПРОБЛЕМЫ:\n");
        for (i, issue) in analysis.issues.iter().enumerate() {
            let severity_prefix = match issue.severity.as_str() {
                "critical" | "blocker" => "[КРИТИЧНО]",
                "major" => "[ВАЖНО]",
                "minor" => "[ВНИМАНИЕ]", 
                "info" => "[ИНФО]",
                _ => "[?]"
            };
            
            let category_ru = match issue.category.as_str() {
                "intent" => "Логика",
                "correctness" => "Корректность", 
                "security" => "Безопасность",
                "robustness" => "Надежность",
                "maintainability" => "Сопровождаемость",
                "performance" => "Производительность", 
                "tests" => "Тесты",
                "lint" => "Стиль кода",
                _ => &issue.category
            };
            
            output.push_str(&format!("{}. {} [{}] {}\n", 
                i + 1, severity_prefix, category_ru, issue.message));
                
            if let Some(fix) = &issue.fix_suggestion {
                output.push_str(&format!("   -> Решение: {}\n", fix));
            }
        }
        output.push_str("\n");
    }
    
    // Suggestions section
    if !analysis.suggestions.is_empty() {
        output.push_str("РЕКОМЕНДАЦИИ:\n");
        for (i, suggestion) in analysis.suggestions.iter().enumerate() {
            let priority_prefix = match suggestion.priority.as_str() {
                "high" => "[ВЫСОКИЙ]",
                "medium" => "[СРЕДНИЙ]",
                "low" => "[НИЗКИЙ]", 
                _ => "[?]"
            };
            
            let category_ru = match suggestion.category.as_str() {
                "performance" => "Производительность",
                "security" => "Безопасность",
                "maintainability" => "Сопровождаемость",
                "tests" => "Тесты", 
                "correctness" => "Корректность",
                _ => &suggestion.category
            };
            
            output.push_str(&format!("{}. {} [{}] {}\n", 
                i + 1, priority_prefix, category_ru, suggestion.description));
        }
        output.push_str("\n");
    }
    
    // Metrics section
    if let Some(metrics) = &analysis.metrics {
        output.push_str("МЕТРИКИ КОДА:\n");
        
        if let Some(complexity) = &metrics.complexity {
            let complexity_text = match complexity.as_str() {
                "low" => "Простая",
                "medium" => "Средняя",
                "high" => "Высокая", 
                _ => complexity
            };
            output.push_str(&format!("- Сложность: {}\n", complexity_text));
        }
        
        if let Some(readability) = &metrics.readability {
            let readability_text = match readability.as_str() {
                "excellent" => "Отличная",
                "good" => "Хорошая", 
                "fair" => "Приемлемая",
                "poor" => "Плохая",
                _ => readability
            };
            output.push_str(&format!("- Читаемость: {}\n", readability_text));
        }
        
        if let Some(test_coverage) = &metrics.test_coverage {
            let coverage_text = match test_coverage.as_str() {
                "excellent" => "Отличное",
                "good" => "Хорошее",
                "partial" => "Частичное",
                "none" => "Отсутствует",
                _ => test_coverage
            };
            output.push_str(&format!("- Покрытие тестами: {}\n", coverage_text));
        }
    }
    
    output
}

/// Safe output helper - use reason field for proper newline display
fn output_simple_text_response(message: &str) {
    // Use the same approach as main analysis - output with reason field
    // This ensures newlines are preserved for all outputs
    // Duplicate message in additionalContext to ensure it's always visible
    let output = serde_json::json!({
        "reason": message,  // Newlines preserved here
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": message  // Duplicate message instead of "OK"
        }
    });
    println!("{}", output);
}

/// Main PostToolUse hook execution
#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_env().context("Failed to load configuration")?;

    // Read input from stdin
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("Failed to read stdin")?;

    // Handle empty input
    if input.trim().is_empty() {
        // Log the skip reason
        if let Ok(mut log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("posttooluse_debug.log"))
        {
            use std::io::Write;
            writeln!(log_file, "SKIPPED: Empty input received").ok();
        }
        output_simple_text_response("Пропущено: пустой вход");
        return Ok(());
    }

    // Parse hook input
    let hook_input: HookInput = serde_json::from_str(&input).context("Failed to parse hook input")?;
    
    // Debug logging to file to see what context we receive
    let log_file_path = std::env::temp_dir().join("posttooluse_debug.log");
    if let Ok(mut log_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        use std::io::Write;
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        writeln!(log_file, "\n=== PostToolUse Hook Debug [{}] ===", timestamp).ok();
        writeln!(log_file, "Tool name: {}", hook_input.tool_name).ok();
        writeln!(log_file, "Session ID: {:?}", hook_input.session_id).ok();
        writeln!(log_file, "Transcript path: {:?}", hook_input.transcript_path).ok();
        writeln!(log_file, "CWD: {:?}", hook_input.cwd).ok();
        writeln!(log_file, "Hook event: {:?}", hook_input.hook_event_name).ok();
        writeln!(log_file, "CLAUDE_PROJECT_DIR env: {:?}", std::env::var("CLAUDE_PROJECT_DIR").ok()).ok();
        
        // Log if transcript was used
        if let Some(transcript_path) = &hook_input.transcript_path {
            writeln!(log_file, "Transcript path provided: {}", transcript_path).ok();
            if validate_transcript_path(transcript_path) {
                writeln!(log_file, "Transcript path validated successfully").ok();
            } else {
                writeln!(log_file, "Transcript path validation failed").ok();
            }
        } else {
            writeln!(log_file, "No transcript path provided").ok();
        }
        writeln!(log_file, "==============================").ok();
    }
    
    // Also print to stderr for visibility
    eprintln!("PostToolUse hook: Logged to {:?}", log_file_path);

    // Extract content and file path
    let content = extract_content_from_tool_input(&hook_input.tool_name, &hook_input.tool_input);
    let file_path = extract_file_path(&hook_input.tool_input);

    // Only analyze specific tool types
    if !matches!(
        hook_input.tool_name.as_str(),
        "Write" | "Edit" | "MultiEdit"
    ) {
        // Log the skip reason
        if let Ok(mut log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("posttooluse_debug.log"))
        {
            use std::io::Write;
            writeln!(log_file, "SKIPPED: Tool not monitored: {}", hook_input.tool_name).ok();
        }
        output_simple_text_response(&format!("Пропущено: {} не требует анализа", hook_input.tool_name));
        return Ok(());
    }

    // Skip if not a code file or empty content
    if !should_validate_file(&file_path) || content.trim().is_empty() {
        // Log the skip reason
        if let Ok(mut log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("posttooluse_debug.log"))
        {
            use std::io::Write;
            let reason = if content.trim().is_empty() {
                format!("SKIPPED: Empty content for file: {}", file_path)
            } else {
                format!("SKIPPED: Not a code file: {}", file_path)
            };
            writeln!(log_file, "{}", reason).ok();
        }
        let message = if content.trim().is_empty() {
            format!("Пропущено: пустое содержимое для {}", file_path)
        } else {
            format!("Пропущено: {} не является кодовым файлом", file_path)
        };
        output_simple_text_response(&message);
        return Ok(());
    }

    // Log before analysis to track progress  
    if let Ok(mut log_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::temp_dir().join("posttooluse_debug.log"))
    {
        use std::io::Write;
        writeln!(log_file, "Starting analysis - Content length: {}, File: {}", content.len(), file_path).ok();
        writeln!(log_file, "Config API key present: {}", !config.xai_api_key.is_empty()).ok();
    }

    // Execute enhanced Grok analysis with context
    match perform_analysis(&config, &content, &file_path, &hook_input).await {
        Ok(analysis) => {
            // Create compact, readable summary without complex formatting
            let mut summary_parts = vec![];
            
            // Add the ATTEMPT line with a newline after it
            if !analysis.summary.is_empty() {
                summary_parts.push(analysis.summary.clone());
                summary_parts.push(String::new()); // Empty line after ATTEMPT
            }
            
            // Add issues if any
            if !analysis.issues.is_empty() {
                // Create indices for sorting without cloning
                let mut issue_indices: Vec<usize> = (0..analysis.issues.len()).collect();
                
                // Sort indices by issue priority, including impact and confidence
                issue_indices.sort_by(|&a, &b| {
                    let issue_a = &analysis.issues[a];
                    let issue_b = &analysis.issues[b];
                    
                    // Calculate priority score: severity_weight * impact * confidence
                    let calc_priority = |issue: &GrokCodeIssue| -> i32 {
                        let severity_weight = match issue.severity.as_str() {
                            "blocker" => 1000,
                            "critical" => 100,
                            "major" => 10,
                            "minor" => 1,
                            "info" => 0,
                            _ => 0
                        };
                        
                        let impact = issue.impact.unwrap_or(5) as i32;
                        let confidence = (issue.confidence.unwrap_or(0.5) * 10.0) as i32;
                        
                        severity_weight * impact * confidence
                    };
                    
                    let priority_a = calc_priority(issue_a);
                    let priority_b = calc_priority(issue_b);
                    
                    // Higher priority first
                    priority_b.cmp(&priority_a)
                });
                
                // Count issues by severity
                let critical_count = analysis.issues.iter()
                    .filter(|i| matches!(i.severity.as_str(), "critical" | "blocker"))
                    .count();
                let major_count = analysis.issues.iter()
                    .filter(|i| i.severity == "major")
                    .count();
                let minor_count = analysis.issues.iter()
                    .filter(|i| matches!(i.severity.as_str(), "minor" | "info"))
                    .count();
                
                // Build issue count summary
                let mut issue_summary = Vec::new();
                if critical_count > 0 {
                    issue_summary.push(format!("{} критических", critical_count));
                }
                if major_count > 0 {
                    issue_summary.push(format!("{} важных", major_count));
                }
                if minor_count > 0 {
                    issue_summary.push(format!("{} минорных", minor_count));
                }
                
                summary_parts.push(format!("Найдено {} проблем: {}", 
                    analysis.issues.len(),
                    issue_summary.join(", ")
                ));
                summary_parts.push(String::new()); // Empty line after problem count
                
                // Add detailed priority recommendation based on the first issue
                if !issue_indices.is_empty() {
                    let first_issue = &analysis.issues[issue_indices[0]];
                    let priority_text = match first_issue.severity.as_str() {
                        "blocker" => format!("🔴 БЛОКЕР! Начните с: {}", first_issue.message),
                        "critical" => format!("⚠ КРИТИЧНО! Начните с: {}", first_issue.message),
                        "major" => format!("→ Начните с важной проблемы: {}", first_issue.message),
                        "minor" => "→ Все проблемы минорные, начните с первой по списку".to_string(),
                        _ => "→ Начните с первой проблемы по списку".to_string()
                    };
                    summary_parts.push(priority_text);
                }
                
                // Show ALL issues sorted by priority
                for (i, &idx) in issue_indices.iter().enumerate() {
                    let issue = &analysis.issues[idx];
                    let category_ru = match issue.category.as_str() {
                        "intent" => "Логика",
                        "correctness" => "Корректность",
                        "security" => "Безопасность",
                        "robustness" => "Надежность",
                        "maintainability" => "Сопровождаемость",
                        "performance" => "Производительность",
                        "tests" => "Тесты",
                        "lint" => "Стиль кода",
                        _ => &issue.category
                    };
                    
                    // Include severity/priority
                    let priority = match issue.severity.as_str() {
                        "blocker" => " [БЛОКЕР]",
                        "critical" => " [КРИТИЧНО]",
                        "major" => " [ВАЖНО]",
                        "minor" => " [МИНОР]",
                        _ => ""
                    };
                    
                    summary_parts.push(format!("{}. {}{} - {}", 
                        i + 1, 
                        category_ru,
                        priority,
                        issue.message
                    ));
                    
                    // Add fix suggestion if available
                    if let Some(fix) = &issue.fix_suggestion {
                        summary_parts.push(format!("   → Рекомендация: {}", fix));
                    }
                }
            }
            
            let compact_summary = summary_parts.join("\n");
            
            // Create actual issues from Grok analysis
            let mut converted_issues = Vec::new();
            for issue in &analysis.issues {
                let issue_sev = match issue.severity.as_str() {
                    "critical" | "blocker" => "error",
                    "major" => "warn", 
                    _ => "info"
                };
                
                let category_ru = match issue.category.as_str() {
                    "intent" => "Логика",
                    "correctness" => "Корректность",
                    "security" => "Безопасность", 
                    "robustness" => "Надежность",
                    "maintainability" => "Сопровождаемость",
                    "performance" => "Производительность",
                    "tests" => "Тесты",
                    "lint" => "Стиль кода",
                    _ => &issue.category
                };
                
                let msg = if let Some(fix) = &issue.fix_suggestion {
                    format!("[{}] {} -> {}", category_ru, issue.message, fix)
                } else {
                    format!("[{}] {}", category_ru, issue.message)
                };
                
                converted_issues.push(SoftFeedbackIssue {
                    sev: issue_sev.to_string(),
                    msg,
                    loc: SoftFeedbackLocation { line: issue.line.map(|l| l as i32) },
                });
            }
            
            // Check if we should block based on critical issues
            let critical_count = analysis.issues.iter()
                .filter(|i| matches!(i.severity.as_str(), "critical" | "blocker"))
                .count();
            
            // Block if quality is poor or critical issues found
            let should_block = matches!(analysis.overall_quality.as_str(), "poor" | "needs_improvement") 
                || critical_count > 0;
            
            // ALWAYS use structure with "reason" field - it's the ONLY field that preserves newlines
            // The trick: for non-blocking, just output reason WITHOUT decision field
            if should_block {
                // For blocking, include both decision and reason
                let output = serde_json::json!({
                    "decision": "block",
                    "reason": compact_summary,
                    "hookSpecificOutput": {
                        "hookEventName": "PostToolUse"
                    }
                });
                println!("{}", output);
            } else {
                // For non-blocking, output detailed analysis in additionalContext
                let context_message = if analysis.issues.is_empty() {
                    format!("{}\n\n✅ Отличная работа! Серьезных проблем не обнаружено.", compact_summary)
                } else {
                    format!("{}\n\nАнализ завершен: найденные проблемы отображены выше", compact_summary)
                };
                
                let output = serde_json::json!({
                    "reason": compact_summary,  // Newlines should be preserved here!
                    "hookSpecificOutput": {
                        "hookEventName": "PostToolUse",
                        "additionalContext": context_message
                    }
                });
                println!("{}", output);
            }
        }
        Err(e) => {
            // Fail open on errors - provide informative warning
            eprintln!("PostToolUse hook error: {}", e);
            
            // Log detailed error to debug file if possible
            let log_file_path = std::env::temp_dir().join("posttooluse_debug.log");
            if let Ok(mut log_file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_path)
            {
                use std::io::Write;
                let error_details = format!("Analysis error: {:?}\nCaused by: {:?}", e, e.source());
                writeln!(log_file, "{}", error_details).ok();
            }
            
            // Create fallback SoftFeedback without emoji (spec compliant)
            // Sanitize error message to prevent injection attacks
            let sanitized_error = e.to_string().chars().take(100).collect::<String>();
            let feedback_spec = SoftFeedbackSpec {
                summary: truncate_utf8_safe(&format!("Analysis unavailable: {}", sanitized_error), 280),
                files: None,
            };
            
            // Use standard PostToolUse schema (as expected by Claude Code)
            let additional_context = match serde_json::to_string(&feedback_spec) {
                Ok(json) => json,
                Err(_) => format!(r#"{{\"summary\":\"Analysis unavailable\",\"files\":null}}"#),
            };
            
            let output = PostToolUseOutput {
                hook_specific_output: PostToolUseHookOutput {
                    hook_event_name: "PostToolUse".to_string(),
                    additional_context,
                },
            };
            
            // Safe JSON serialization with error handling
            match serde_json::to_string(&output) {
                Ok(json) => println!("{}", json),
                Err(serialize_err) => {
                    eprintln!("Critical: Failed to serialize fallback response: {}", serialize_err);
                    output_simple_text_response("Critical serialization error");
                }
            }
        }
    }

    Ok(())
}

/// Perform code quality analysis using Grok-code-fast-1 with context
async fn perform_analysis(config: &Config, content: &str, file_path: &str, hook_input: &HookInput) -> Result<GrokCodeAnalysis> {
    // Load analysis prompt
    let mut prompt = load_prompt("post_edit_validation.txt").await.context("Failed to load post-edit validation prompt")?;
    
    // Log prompt preview for debugging
    if let Ok(mut log_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::temp_dir().join("posttooluse_debug.log"))
    {
        use std::io::Write;
        let prompt_preview = prompt.chars().take(100).collect::<String>();
        writeln!(log_file, "Loaded prompt preview: {}...", prompt_preview).ok();
    }

    // Add context from transcript if available
    if let Some(transcript_path) = &hook_input.transcript_path {
        // Validate transcript path for security
        if !validate_transcript_path(transcript_path) {
            eprintln!("Warning: Invalid transcript path, skipping context: {}", transcript_path);
        } else {
            match read_transcript_summary(transcript_path, 10, 1000) {
                Ok(summary) => {
                    prompt = format!("{}\n\nCONTEXT - Recent chat history:\n{}", prompt, summary);
                }
                Err(e) => {
                    eprintln!("Warning: Could not read transcript: {}", e);
                }
            }
        }
    }

    // Add project context from environment
    if let Ok(project_dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        prompt = format!("{}\n\nPROJECT: {}", prompt, project_dir);
    }

    // Initialize Grok client
    let client = GrokAnalysisClient::new(
        config.xai_api_key.clone(),
        config.xai_base_url.clone(),
        config.posttool_model.clone(),
        config,
    )?;

    // Analyze with Grok-code-fast-1
    client
        .analyze_code(content, &prompt, config)
        .await
        .context("Grok code analysis failed")
}