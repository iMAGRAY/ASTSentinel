use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Common utilities for Claude Code hooks

/// Safely truncate a UTF-8 string to a maximum number of characters
pub fn truncate_utf8_safe(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

/// Code analysis modules for project inspection, AST parsing, and metrics
pub mod analysis;

/// Validation modules for code and file checks
pub mod validation;

/// External service providers for AI and other integrations
pub mod providers;

/// Caching modules for performance optimization
pub mod cache;

// Re-export commonly used types for convenience
pub use analysis::{ComplexityMetrics, ProjectStructure, scan_project_structure, format_project_structure_for_ai, ScanConfig};
pub use analysis::ast::{SupportedLanguage, MultiLanguageAnalyzer, ComplexityVisitor};
// Test file validation removed - AI handles all validation now
pub use providers::{UniversalAIClient, AIProvider};
pub use cache::ProjectCache;

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


/// Environment configuration with validation and type safety
#[derive(Clone)]
pub struct Config {
    // Multi-provider API configurations
    pub openai_api_key: String,
    pub anthropic_api_key: String,
    pub google_api_key: String,
    pub xai_api_key: String,
    
    // Base URLs for different providers (can be overridden)
    pub openai_base_url: String,
    pub anthropic_base_url: String,
    pub google_base_url: String,
    pub xai_base_url: String,
    
    // Provider selection for each hook (type-safe)
    pub pretool_provider: providers::AIProvider,
    pub posttool_provider: providers::AIProvider,
    
    // Model specifications
    pub pretool_model: String,
    pub posttool_model: String,
    
    // Common settings with defaults
    pub max_tokens: u32,
    pub temperature: f32,
    pub max_issues: usize,
    pub request_timeout_secs: u64,
    pub connect_timeout_secs: u64,
}

impl Config {
    /// Create a new validated Config instance
    pub fn new(
        pretool_provider: providers::AIProvider,
        posttool_provider: providers::AIProvider,
        pretool_model: String,
        posttool_model: String,
    ) -> Self {
        Self {
            openai_api_key: String::new(),
            anthropic_api_key: String::new(),
            google_api_key: String::new(),
            xai_api_key: String::new(),
            
            openai_base_url: providers::AIProvider::OpenAI.default_base_url().to_string(),
            anthropic_base_url: providers::AIProvider::Anthropic.default_base_url().to_string(),
            google_base_url: providers::AIProvider::Google.default_base_url().to_string(),
            xai_base_url: providers::AIProvider::XAI.default_base_url().to_string(),
            
            pretool_provider,
            posttool_provider,
            pretool_model,
            posttool_model,
            
            // Sensible defaults
            max_tokens: 4000,
            temperature: 0.1,
            max_issues: 10,
            request_timeout_secs: 60,
            connect_timeout_secs: 30,
        }
    }
    
    /// Validate configuration and return errors if invalid
    pub fn validate(&self) -> Result<()> {
        // Validate API keys for required providers
        if self.get_api_key_for_provider(&self.pretool_provider).is_empty() {
            return Err(anyhow::anyhow!("API key missing for pretool provider: {}", self.pretool_provider));
        }
        
        if self.get_api_key_for_provider(&self.posttool_provider).is_empty() {
            return Err(anyhow::anyhow!("API key missing for posttool provider: {}", self.posttool_provider));
        }
        
        // Validate models are not empty
        if self.pretool_model.trim().is_empty() {
            return Err(anyhow::anyhow!("Pretool model cannot be empty"));
        }
        
        if self.posttool_model.trim().is_empty() {
            return Err(anyhow::anyhow!("Posttool model cannot be empty"));
        }
        
        // Validate reasonable ranges
        if self.max_tokens == 0 || self.max_tokens > 100_000 {
            return Err(anyhow::anyhow!("max_tokens must be between 1 and 100,000"));
        }
        
        if self.temperature < 0.0 || self.temperature > 2.0 {
            return Err(anyhow::anyhow!("temperature must be between 0.0 and 2.0"));
        }
        
        if self.request_timeout_secs == 0 || self.request_timeout_secs > 600 {
            return Err(anyhow::anyhow!("request_timeout_secs must be between 1 and 600"));
        }
        
        Ok(())
    }
    
    /// Get the appropriate API key for a given provider
    pub fn get_api_key_for_provider(&self, provider: &providers::AIProvider) -> &str {
        match provider {
            providers::AIProvider::OpenAI => &self.openai_api_key,
            providers::AIProvider::Anthropic => &self.anthropic_api_key,
            providers::AIProvider::Google => &self.google_api_key,
            providers::AIProvider::XAI => &self.xai_api_key,
        }
    }
    
    /// Get the appropriate base URL for a given provider
    pub fn get_base_url_for_provider(&self, provider: &providers::AIProvider) -> &str {
        match provider {
            providers::AIProvider::OpenAI => &self.openai_base_url,
            providers::AIProvider::Anthropic => &self.anthropic_base_url,
            providers::AIProvider::Google => &self.google_base_url,
            providers::AIProvider::XAI => &self.xai_base_url,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Try to load .env from the same directory as the executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Load .env (production) and optionally override with .env.local (development)
                let env_file = exe_dir.join(".env");
                eprintln!("Looking for .env at: {:?}", env_file);
                
                if env_file.exists() {
                    eprintln!(".env file found, loading...");
                    // Clear existing API keys to ensure .env takes priority
                    std::env::remove_var("OPENAI_API_KEY");
                    std::env::remove_var("ANTHROPIC_API_KEY");
                    std::env::remove_var("GOOGLE_API_KEY");
                    std::env::remove_var("XAI_API_KEY");
                    
                    if let Err(e) = dotenv::from_path(&env_file) {
                        eprintln!("Failed to load .env: {}", e);
                    } else {
                        eprintln!(".env loaded successfully (cleared system variables first)");
                    }
                } else {
                    eprintln!(".env file not found");
                }
                
                // Additionally check for .env.local to override settings in development
                let env_local = exe_dir.join(".env.local");
                if env_local.exists() {
                    eprintln!("Found .env.local, loading overrides...");
                    if let Err(e) = dotenv::from_path(&env_local) {
                        eprintln!("Failed to load .env.local: {}", e);
                    } else {
                        eprintln!(".env.local loaded successfully");
                    }
                }
            }
        }
        
        // Load all API keys from environment
        let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
        let google_api_key = std::env::var("GOOGLE_API_KEY").unwrap_or_default();
        let xai_api_key = std::env::var("XAI_API_KEY").unwrap_or_default();
        
        // Parse provider selections with validation
        let pretool_provider_str = std::env::var("PRETOOL_PROVIDER").unwrap_or_else(|_| "xai".to_string());
        let posttool_provider_str = std::env::var("POSTTOOL_PROVIDER").unwrap_or_else(|_| "xai".to_string());
        
        let pretool_provider = pretool_provider_str.parse::<providers::AIProvider>()
            .with_context(|| format!("Invalid PRETOOL_PROVIDER: {}. Supported: openai, anthropic, google, xai", pretool_provider_str))?;
        let posttool_provider = posttool_provider_str.parse::<providers::AIProvider>()
            .with_context(|| format!("Invalid POSTTOOL_PROVIDER: {}. Supported: openai, anthropic, google, xai", posttool_provider_str))?;
        
        // Parse and validate configurable values with proper bounds
        let max_tokens = std::env::var("MAX_TOKENS")
            .unwrap_or_else(|_| "4000".to_string())
            .parse::<u32>()
            .unwrap_or(4000)
            .clamp(100, 100_000);
        
        let temperature = std::env::var("TEMPERATURE")
            .unwrap_or_else(|_| "0.1".to_string())
            .parse::<f32>()
            .unwrap_or(0.1)
            .clamp(0.0, 2.0);
        
        let max_issues = std::env::var("MAX_ISSUES")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<usize>()
            .unwrap_or(10)
            .clamp(1, 50);
        
        let request_timeout_secs = std::env::var("REQUEST_TIMEOUT_SECS")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .unwrap_or(60)
            .clamp(10, 600);
        
        let connect_timeout_secs = std::env::var("CONNECT_TIMEOUT_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .unwrap_or(30)
            .clamp(5, 120);
        
        // Create configuration with all providers supported
        let config = Config {
            openai_api_key,
            anthropic_api_key,
            google_api_key,
            xai_api_key,
            
            // Load custom base URLs or use defaults
            openai_base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| providers::AIProvider::OpenAI.default_base_url().to_string()),
            anthropic_base_url: std::env::var("ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| providers::AIProvider::Anthropic.default_base_url().to_string()),
            google_base_url: std::env::var("GOOGLE_BASE_URL")
                .unwrap_or_else(|_| providers::AIProvider::Google.default_base_url().to_string()),
            xai_base_url: std::env::var("XAI_BASE_URL")
                .unwrap_or_else(|_| providers::AIProvider::XAI.default_base_url().to_string()),
            
            pretool_provider,
            posttool_provider,
            pretool_model: std::env::var("PRETOOL_MODEL")
                .unwrap_or_else(|_| "grok-code-fast-1".to_string()),
            posttool_model: std::env::var("POSTTOOL_MODEL")
                .unwrap_or_else(|_| "grok-code-fast-1".to_string()),
            max_tokens,
            temperature,
            max_issues,
            request_timeout_secs,
            connect_timeout_secs,
        };
        
        // Validate configuration before returning
        config.validate().with_context(|| "Configuration validation failed")?;
        
        Ok(config)
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