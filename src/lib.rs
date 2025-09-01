use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Common utilities for Claude Code hooks
pub mod project_context;

/// Claude Code Hook input data structure - actual fields from Claude Code
#[derive(Debug, Deserialize)]
pub struct HookInput {
    pub tool_name: String,
    pub tool_input: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub transcript_path: Option<String>,  // Path to conversation JSON file
    #[serde(default)]
    pub cwd: Option<String>,  // Current working directory
    #[serde(default)]
    pub hook_event_name: Option<String>,
}

/// PreToolUse hook output
#[derive(Debug, Serialize)]
pub struct PreToolUseOutput {
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: PreToolUseHookOutput,
}

#[derive(Debug, Serialize)]
pub struct PreToolUseHookOutput {
    #[serde(rename = "hookEventName")]
    pub hook_event_name: String,
    #[serde(rename = "permissionDecision")]
    pub permission_decision: String,
    #[serde(rename = "permissionDecisionReason", skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,
}

/// PostToolUse hook output
#[derive(Debug, Serialize)]
pub struct PostToolUseOutput {
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: PostToolUseHookOutput,
}

#[derive(Debug, Serialize)]
pub struct PostToolUseHookOutput {
    #[serde(rename = "hookEventName")]
    pub hook_event_name: String,
    #[serde(rename = "additionalContext")]
    pub additional_context: String,
}

/// SoftFeedback structure according to Claude Code Hooks specification
#[derive(Debug, Serialize)]
pub struct SoftFeedbackSpec {
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<SoftFeedbackFile>>,
}

#[derive(Debug, Serialize)]
pub struct SoftFeedbackFile {
    pub path: String,
    pub issues: Vec<SoftFeedbackIssue>,
}

#[derive(Debug, Serialize)]
pub struct SoftFeedbackIssue {
    pub sev: String, // "info", "warn", "error"
    pub msg: String,
    pub loc: SoftFeedbackLocation,
}

#[derive(Debug, Serialize)]
pub struct SoftFeedbackLocation {
    pub line: Option<i32>,
}

/// GPT-5 Responses API structure
#[derive(Debug, Serialize)]
pub struct Gpt5RequestInput {
    pub role: String,
    pub content: Vec<Gpt5Content>,
}

#[derive(Debug, Serialize)]
pub struct Gpt5Content {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Gpt5Request {
    pub model: String,
    pub input: Vec<Gpt5RequestInput>,
    pub text: Gpt5TextFormat,
    pub max_output_tokens: u32,
    pub store: bool,
    pub reasoning: Gpt5Reasoning,
}

#[derive(Debug, Serialize)]
pub struct Gpt5TextFormat {
    pub format: Gpt5JsonSchema,
}

#[derive(Debug, Serialize)]
pub struct Gpt5JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub name: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct Gpt5Reasoning {
    pub effort: String,
}

#[derive(Debug, Deserialize)]
pub struct Gpt5Response {
    pub output_text: String,
}

/// Security validation result from Grok
#[derive(Debug, Deserialize)]
pub struct SecurityValidation {
    pub decision: String, // "allow", "ask", "deny"
    pub reason: String,
    pub security_concerns: Option<Vec<String>>,
    pub risk_level: String, // "low", "medium", "high", "critical"
}

/// Grok API structures
#[derive(Debug, Serialize, Deserialize)]
pub struct GrokMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct GrokRequest {
    pub model: String,
    pub messages: Vec<GrokMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<GrokResponseFormat>,
}

#[derive(Debug, Serialize)]
pub struct GrokResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    pub json_schema: GrokJsonSchema,
}

#[derive(Debug, Serialize)]
pub struct GrokJsonSchema {
    pub name: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct GrokResponse {
    pub choices: Vec<GrokChoice>,
}

#[derive(Debug, Deserialize)]
pub struct GrokChoice {
    pub message: GrokMessage,
}

/// Enhanced Grok Code Analysis with structured schema
#[derive(Debug, Serialize, Deserialize)]
pub struct GrokCodeAnalysis {
    pub summary: String,
    pub overall_quality: String, // "excellent", "good", "needs_improvement", "poor"
    pub issues: Vec<GrokCodeIssue>,
    pub suggestions: Vec<GrokCodeSuggestion>,
    pub metrics: Option<GrokCodeMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrokCodeIssue {
    pub severity: String, // "info", "minor", "major", "critical", "blocker"
    pub category: String, // "intent", "correctness", "security", "robustness", "maintainability", "performance", "tests", "lint"
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<u8>, // 1-3: 1=local, 2=module, 3=systemwide
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_cost: Option<u8>, // 1-3: 1=quick, 2=moderate, 3=refactor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>, // 0.5-1.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_suggestion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrokCodeSuggestion {
    pub category: String,
    pub description: String,
    pub priority: String, // "high", "medium", "low"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_score: Option<f32>, // 0-100
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_example: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrokCodeMetrics {
    pub complexity: Option<String>, // "low", "medium", "high"
    pub readability: Option<String>, // "excellent", "good", "fair", "poor"
    pub test_coverage: Option<String>, // "none", "partial", "good", "excellent"
}

/// Environment configuration
pub struct Config {
    pub openai_api_key: String,
    pub xai_api_key: String,
    pub xai_base_url: String,
    pub pretool_model: String,
    pub posttool_model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub max_issues: usize,
    pub request_timeout_secs: u64,
    pub connect_timeout_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Try to load .env from the same directory as the executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let env_file = exe_dir.join(".env");
                eprintln!("Looking for .env at: {:?}", env_file);
                if env_file.exists() {
                    eprintln!(".env file found, loading...");
                    if let Err(e) = dotenv::from_path(&env_file) {
                        eprintln!("Failed to load .env: {}", e);
                    } else {
                        eprintln!(".env loaded successfully");
                    }
                } else {
                    eprintln!(".env file not found");
                }
            }
        }
        
        // Fallback to system environment variables
        let xai_key = std::env::var("XAI_API_KEY").unwrap_or_default();
        let key_display = if xai_key.is_empty() { 
            "NOT SET".to_string()
        } else { 
            format!("{}...", &xai_key.chars().take(10).collect::<String>())
        };
        eprintln!("XAI_API_KEY loaded: {}", key_display);
        
        // Parse and validate configurable values
        let max_tokens = std::env::var("POSTTOOL_MAX_TOKENS")
            .unwrap_or_else(|_| "5000".to_string())
            .parse::<u32>()
            .unwrap_or(5000)
            .min(8192); // Cap at reasonable limit per Grok docs
        
        let temperature = std::env::var("POSTTOOL_TEMPERATURE")
            .unwrap_or_else(|_| "0.2".to_string())
            .parse::<f32>()
            .unwrap_or(0.2)
            .max(2.0)
            .min(0.0); // Valid range per API docs
        
        let max_issues = std::env::var("POSTTOOL_MAX_ISSUES")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<usize>()
            .unwrap_or(3)
            .min(1)
            .max(3); // Claude Code Hooks spec limit
        
        // Parse and validate timeout settings
        let request_timeout_secs = std::env::var("REQUEST_TIMEOUT_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .unwrap_or(30);
        
        let connect_timeout_secs = std::env::var("CONNECT_TIMEOUT_SECS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u64>()
            .unwrap_or(10);
        
        Ok(Config {
            openai_api_key: std::env::var("OPENAI_API_KEY")
                .unwrap_or_default(),
            xai_api_key: xai_key,
            xai_base_url: std::env::var("XAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.x.ai/v1".to_string()),
            pretool_model: std::env::var("PRETOOL_MODEL")
                .unwrap_or_else(|_| "gpt-5".to_string()),
            posttool_model: std::env::var("POSTTOOL_MODEL")
                .unwrap_or_else(|_| "grok-code-fast-1".to_string()),
            max_tokens,
            temperature,
            max_issues,
            request_timeout_secs,
            connect_timeout_secs,
        })
    }
}

/// File extension validation
pub fn should_validate_file(file_path: &str) -> bool {
    let code_extensions = [
        ".js", ".ts", ".jsx", ".tsx", ".mjs", ".cjs",
        ".py", ".pyw", ".pyc", ".pyo",
        ".java", ".class", ".jar",
        ".cpp", ".c", ".cc", ".cxx", ".h", ".hpp",
        ".cs", ".vb",
        ".php", ".php3", ".php4", ".php5", ".phtml",
        ".rb", ".rbw",
        ".go",
        ".rs",
        ".kt", ".kts",
        ".swift",
        ".sql",
        ".sh", ".bash", ".zsh", ".fish",
        ".ps1", ".psm1",
        ".html", ".htm", ".xhtml"
    ];
    
    code_extensions.iter().any(|ext| file_path.to_lowercase().ends_with(ext))
}

/// Extract content from tool input based on tool type
pub fn extract_content_from_tool_input(tool_name: &str, tool_input: &HashMap<String, serde_json::Value>) -> String {
    match tool_name {
        "Write" => {
            tool_input.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        },
        "Edit" => {
            tool_input.get("new_string")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        },
        "MultiEdit" => {
            tool_input.get("edits")
                .and_then(|v| v.as_array())
                .map(|edits| {
                    edits.iter()
                        .filter_map(|edit| edit.get("new_string")?.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default()
        },
        _ => String::new(),
    }
}

/// Get file path from tool input
pub fn extract_file_path(tool_input: &HashMap<String, serde_json::Value>) -> String {
    tool_input.get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}