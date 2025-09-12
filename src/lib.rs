#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::dbg_macro
    )
)]

// Allow inline format args (use `{var}`) lint to be fixed incrementally across codebase.
#![allow(clippy::uninlined_format_args)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Common utilities for Claude Code hooks
/// Safely truncate a UTF-8 string to a maximum number of characters
/// Handles zero-width characters properly for accurate length calculation
pub fn truncate_utf8_safe(s: &str, max_chars: usize) -> String {
    // Count visible characters, excluding zero-width characters
    let visible_chars: Vec<char> = s.chars().filter(|&c| !is_zero_width_char(c)).collect();
    let visible_count = visible_chars.len();

    if visible_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = visible_chars.iter().take(max_chars.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

/// Sanitize input string by removing potentially malicious zero-width characters
/// Used to prevent obfuscation attacks in user input
pub fn sanitize_zero_width_chars(input: &str) -> String {
    input.chars().filter(|&c| !is_zero_width_char(c)).collect()
}

/// Check if character is a zero-width character that could be used for obfuscation
fn is_zero_width_char(c: char) -> bool {
    matches!(
        c,
        '\u{200B}' |  // Zero Width Space
        '\u{200C}' |  // Zero Width Non-Joiner  
        '\u{200D}' |  // Zero Width Joiner
        '\u{2060}' |  // Word Joiner
        '\u{FEFF}' // Zero Width No-Break Space
    )
}

/// Code analysis modules for project inspection, AST parsing, and metrics
pub mod analysis;

/// Validation modules for code and file checks
pub mod validation;

/// External service providers for AI and other integrations
pub mod providers;

/// Caching modules for performance optimization
pub mod cache;

/// Code formatting modules for multi-language formatting
pub mod formatting;

/// Centralized validation constants for memory optimization
pub mod validation_constants;

/// Runtime configuration (sensitivity, ignore globs, environment)
pub mod config;

/// Short, action-oriented messages glossary
pub mod messages;

/// Telemetry: structured logging initialization (tracing)
pub mod telemetry;

// Re-export commonly used types for convenience
pub use analysis::ast::{ComplexityVisitor, MultiLanguageAnalyzer, SupportedLanguage};
pub use analysis::{
    format_project_structure_for_ai, scan_project_structure, ComplexityMetrics, ProjectStructure, ScanConfig,
};
pub use cache::ProjectCache;
pub use formatting::{CodeFormatter, FormatResult, FormattingService};
pub use providers::{AIProvider, UniversalAIClient};

/// Claude Code Hook input data structure - actual fields from Claude Code
#[derive(Debug, Deserialize)]
pub struct HookInput {
    pub tool_name: String,
    pub tool_input: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub tool_response: Option<serde_json::Value>, // Tool response data (for post-tool hooks)
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub transcript_path: Option<String>, // Path to conversation JSON file
    #[serde(default)]
    pub cwd: Option<String>, // Current working directory
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

/// UserPromptSubmit hook output
#[derive(Debug, Serialize)]
pub struct UserPromptSubmitOutput {
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: UserPromptSubmitHookOutput,
}

#[derive(Debug, Serialize)]
pub struct UserPromptSubmitHookOutput {
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
    pub complexity: Option<String>,    // "low", "medium", "high"
    pub readability: Option<String>,   // "excellent", "good", "fair", "poor"
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

    // Provider-specific output token limits (based on documentation)
    pub gpt5_max_output_tokens: u32,   // GPT-5: 128K output tokens
    pub claude_max_output_tokens: u32, // Claude: 4K typical, 8K max
    pub gemini_max_output_tokens: u32, // Gemini: Variable, 32K max
    pub grok_max_output_tokens: u32,   // Grok: 8K typical
}

impl Config {
    /// Attempt to load runtime configuration from a config file (JSON / YAML / TOML).
    /// If a file is not present or fails to parse, returns None and callers may fallback to env.
    fn try_from_config_file_internal() -> Option<Self> {
        use std::fs;
        use std::path::PathBuf;
        use regex::Regex;

        #[derive(Debug, Default, Deserialize)]
        struct FileCfg {
            // API keys
            openai_api_key: Option<String>,
            anthropic_api_key: Option<String>,
            google_api_key: Option<String>,
            xai_api_key: Option<String>,
            // Base URLs
            openai_base_url: Option<String>,
            anthropic_base_url: Option<String>,
            google_base_url: Option<String>,
            xai_base_url: Option<String>,
            // Providers / models
            pretool_provider: Option<String>,
            posttool_provider: Option<String>,
            pretool_model: Option<String>,
            posttool_model: Option<String>,
            // Limits / timeouts
            max_tokens: Option<u32>,
            temperature: Option<f32>,
            max_issues: Option<usize>,
            request_timeout_secs: Option<u64>,
            connect_timeout_secs: Option<u64>,
            // Provider-specific output token caps
            gpt5_max_output_tokens: Option<u32>,
            claude_max_output_tokens: Option<u32>,
            gemini_max_output_tokens: Option<u32>,
            grok_max_output_tokens: Option<u32>,
        }

        fn parse_any(path: &PathBuf) -> Option<FileCfg> {
            let text = fs::read_to_string(path).ok()?;
            match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
                "json" => serde_json::from_str::<FileCfg>(&text).ok(),
                "yml" | "yaml" => serde_yaml::from_str::<FileCfg>(&text).ok(),
                "toml" => toml::from_str::<FileCfg>(&text).ok(),
                _ => None,
            }
        }

        // Resolution order: explicit env var HOOKS_CONFIG_FILE, then defaults near CWD/exe
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(p) = std::env::var("HOOKS_CONFIG_FILE") { candidates.push(PathBuf::from(p)); }
        // Common default names
        for name in [
            ".hooks-config.json",
            ".hooks-config.yaml",
            ".hooks-config.yml",
            ".hooks-config.toml",
            "hooks.config.json",
            "hooks.config.yaml",
            "hooks.config.yml",
            "hooks.config.toml",
        ] { candidates.push(PathBuf::from(name)); }

        // Next to executable as well
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                for name in [
                    ".hooks-config.json",
                    ".hooks-config.yaml",
                    ".hooks-config.yml",
                    ".hooks-config.toml",
                ] {
                    candidates.push(dir.join(name));
                }
            }
        }

        let fc = candidates
            .into_iter()
            .find(|p| p.exists())
            .and_then(|p| parse_any(&p))?;

        // Compose final Config with sensible defaults if not provided
        // Environment variable expansion like ${VAR}
        fn expand_env(s: String) -> String {
            let re = match Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}") {
                Ok(r) => r,
                Err(_) => return s, // defensive: if regex fails, return original string
            };
            let result = re.replace_all(&s, |caps: &regex::Captures| {
                let key = &caps[1];
                std::env::var(key)
                    .or_else(|_| std::env::var(key.to_ascii_uppercase()))
                    .unwrap_or_else(|_| "".to_string())
            });
            result.into_owned()
        }
        fn map_opt(v: Option<String>) -> Option<String> {
            v.map(expand_env)
        }

        let pretool_provider_str = map_opt(fc.pretool_provider)
            .unwrap_or_else(|| "xai".to_string());
        let posttool_provider_str = map_opt(fc.posttool_provider)
            .unwrap_or_else(|| "xai".to_string());

        let pretool_provider = pretool_provider_str.parse::<providers::AIProvider>().ok()?;
        let posttool_provider = posttool_provider_str.parse::<providers::AIProvider>().ok()?;

        let cfg = Config {
            openai_api_key: map_opt(fc.openai_api_key).unwrap_or_default(),
            anthropic_api_key: map_opt(fc.anthropic_api_key).unwrap_or_default(),
            google_api_key: map_opt(fc.google_api_key).unwrap_or_default(),
            xai_api_key: map_opt(fc.xai_api_key).unwrap_or_default(),
            openai_base_url: map_opt(fc
                .openai_base_url
                ).unwrap_or_else(|| providers::AIProvider::OpenAI.default_base_url().to_string()),
            anthropic_base_url: map_opt(fc
                .anthropic_base_url
                ).unwrap_or_else(|| providers::AIProvider::Anthropic.default_base_url().to_string()),
            google_base_url: map_opt(fc
                .google_base_url
                ).unwrap_or_else(|| providers::AIProvider::Google.default_base_url().to_string()),
            xai_base_url: map_opt(fc
                .xai_base_url
                ).unwrap_or_else(|| providers::AIProvider::XAI.default_base_url().to_string()),
            pretool_provider,
            posttool_provider,
            pretool_model: map_opt(fc.pretool_model).unwrap_or_else(|| "grok-code-fast-1".to_string()),
            posttool_model: map_opt(fc.posttool_model).unwrap_or_else(|| "grok-code-fast-1".to_string()),
            max_tokens: fc.max_tokens.unwrap_or(4000).clamp(100, 100_000),
            temperature: fc.temperature.unwrap_or(0.1).clamp(0.0, 2.0),
            max_issues: fc.max_issues.unwrap_or(10).clamp(1, 50),
            request_timeout_secs: fc.request_timeout_secs.unwrap_or(60).clamp(10, 600),
            connect_timeout_secs: fc.connect_timeout_secs.unwrap_or(30).clamp(5, 120),
            gpt5_max_output_tokens: fc
                .gpt5_max_output_tokens
                .unwrap_or(12000)
                .clamp(1000, 128_000),
            claude_max_output_tokens: fc
                .claude_max_output_tokens
                .unwrap_or(4000)
                .clamp(1000, 8000),
            gemini_max_output_tokens: fc
                .gemini_max_output_tokens
                .unwrap_or(8000)
                .clamp(1000, 32_000),
            grok_max_output_tokens: fc
                .grok_max_output_tokens
                .unwrap_or(8000)
                .clamp(1000, 8000),
        };

        // Validate basics (do not require keys here; PreToolUse may require them separately)
        if let Err(e) = cfg.validate() {
            tracing::warn!(error=%e, "Runtime file config validation failed");
        }
        Some(cfg)
    }

    /// Preferred loader: try configuration file (JSON/YAML/TOML). If not present, fall back to env/.env.
    pub fn from_file_or_env() -> Result<Self> {
        if let Some(cfg) = Self::try_from_config_file_internal() { return Ok(cfg); }
        Self::from_env()
    }

    /// Preferred loader (graceful): try config file, else graceful env loader.
    pub fn from_file_or_env_graceful() -> Result<Self> {
        if let Some(cfg) = Self::try_from_config_file_internal() { return Ok(cfg); }
        Self::from_env_graceful()
    }
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

            // Provider-specific token limits (based on LLM API documentation)
            gpt5_max_output_tokens: 12000,  // Conservative limit for stability
            claude_max_output_tokens: 4096, // Claude's typical max_tokens
            gemini_max_output_tokens: 8192, // Gemini reasonable limit
            grok_max_output_tokens: 8192,   // Grok conservative limit
        }
    }

    /// Validate configuration and return errors if invalid
    pub fn validate(&self) -> Result<()> {
        // Validate API keys for required providers
        if self.get_api_key_for_provider(&self.pretool_provider).is_empty() {
            return Err(anyhow::anyhow!(
                "API key missing for pretool provider: {}",
                self.pretool_provider
            ));
        }

        if self.get_api_key_for_provider(&self.posttool_provider).is_empty() {
            return Err(anyhow::anyhow!(
                "API key missing for posttool provider: {}",
                self.posttool_provider
            ));
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

    /// Get the appropriate max output tokens for a given provider
    /// This helps solve the token limit inconsistency problem
    pub fn get_max_output_tokens_for_provider(&self, provider: &providers::AIProvider) -> u32 {
        match provider {
            providers::AIProvider::OpenAI => self.gpt5_max_output_tokens,
            providers::AIProvider::Anthropic => self.claude_max_output_tokens,
            providers::AIProvider::Google => self.gemini_max_output_tokens,
            providers::AIProvider::XAI => self.grok_max_output_tokens,
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
                tracing::debug!(path=?env_file, "Looking for .env");

                if env_file.exists() {
                    tracing::debug!(".env file found, loading");
                    // Clear existing API keys to ensure .env takes priority
                    std::env::remove_var("OPENAI_API_KEY");
                    std::env::remove_var("ANTHROPIC_API_KEY");
                    std::env::remove_var("GOOGLE_API_KEY");
                    std::env::remove_var("XAI_API_KEY");

                    if let Err(e) = dotenvy::from_path(&env_file) {
                        tracing::warn!(error=%e, "Failed to load .env");
                    } else {
                        tracing::debug!(".env loaded successfully (cleared system variables first)");
                    }
                } else {
                    tracing::debug!(".env file not found");
                }

                // Additionally check for .env.local to override settings only in debug/test builds
                #[cfg(any(debug_assertions, test))]
                {
                    let env_local = exe_dir.join(".env.local");
                    if env_local.exists() {
                        tracing::debug!("Found .env.local, loading overrides");
                        if let Err(e) = dotenvy::from_path(&env_local) {
                            tracing::warn!(error=%e, "Failed to load .env.local");
                        } else {
                            tracing::debug!(".env.local loaded successfully");
                        }
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
            .with_context(|| format!("Invalid PRETOOL_PROVIDER: {pretool_provider_str}. Supported: openai, anthropic, google, xai"))?;
        let posttool_provider = posttool_provider_str.parse::<providers::AIProvider>()
            .with_context(|| format!("Invalid POSTTOOL_PROVIDER: {posttool_provider_str}. Supported: openai, anthropic, google, xai"))?;

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
            pretool_model: std::env::var("PRETOOL_MODEL").unwrap_or_else(|_| "grok-code-fast-1".to_string()),
            posttool_model: std::env::var("POSTTOOL_MODEL")
                .unwrap_or_else(|_| "grok-code-fast-1".to_string()),
            max_tokens,
            temperature,
            max_issues,
            request_timeout_secs,
            connect_timeout_secs,

            // Provider-specific output token limits (based on documentation)
            gpt5_max_output_tokens: std::env::var("GPT5_MAX_OUTPUT_TOKENS")
                .unwrap_or_else(|_| "12000".to_string())
                .parse::<u32>()
                .unwrap_or(12000)
                .clamp(1000, 128000), // GPT-5: 128K output tokens max
            claude_max_output_tokens: std::env::var("CLAUDE_MAX_OUTPUT_TOKENS")
                .unwrap_or_else(|_| "4000".to_string())
                .parse::<u32>()
                .unwrap_or(4000)
                .clamp(1000, 8000), // Claude: 4K typical, 8K max
            gemini_max_output_tokens: std::env::var("GEMINI_MAX_OUTPUT_TOKENS")
                .unwrap_or_else(|_| "8000".to_string())
                .parse::<u32>()
                .unwrap_or(8000)
                .clamp(1000, 32000), // Gemini: Variable, 32K max
            grok_max_output_tokens: std::env::var("GROK_MAX_OUTPUT_TOKENS")
                .unwrap_or_else(|_| "8000".to_string())
                .parse::<u32>()
                .unwrap_or(8000)
                .clamp(1000, 8000), // Grok: 8K typical
        };

        // Validate configuration before returning
        config
            .validate()
            .with_context(|| "Configuration validation failed")?;

        Ok(config)
    }

    /// Load configuration from environment with graceful degradation
    /// Returns config even if API keys are missing, allowing AST-only mode
    pub fn from_env_graceful() -> Result<Self> {
        // Try to load .env from the same directory as the executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Load .env (production) and optionally override with .env.local (development)
                let env_file = exe_dir.join(".env");
                if env_file.exists() {
                    // Clear existing API keys to ensure .env takes priority
                    std::env::remove_var("OPENAI_API_KEY");
                    std::env::remove_var("ANTHROPIC_API_KEY");
                    std::env::remove_var("GOOGLE_API_KEY");
                    std::env::remove_var("XAI_API_KEY");

                    if let Err(e) = dotenvy::from_path(&env_file) {
                        tracing::warn!(error=%e, "Failed to load .env");
                    }
                }

                // .env.local overrides only in debug/test builds
                #[cfg(any(debug_assertions, test))]
                {
                    let env_local = exe_dir.join(".env.local");
                    if env_local.exists() {
                        if let Err(e) = dotenvy::from_path(&env_local) {
                            tracing::warn!(error=%e, "Failed to load .env.local");
                        }
                    }
                }
            }
        }

        // Load all API keys from environment (graceful - empty if not found)
        let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
        let google_api_key = std::env::var("GOOGLE_API_KEY").unwrap_or_default();
        let xai_api_key = std::env::var("XAI_API_KEY").unwrap_or_default();

        // Parse provider selections with validation (but don't fail on missing keys)
        let pretool_provider_str = std::env::var("PRETOOL_PROVIDER").unwrap_or_else(|_| "xai".to_string());
        let posttool_provider_str = std::env::var("POSTTOOL_PROVIDER").unwrap_or_else(|_| "xai".to_string());

        let pretool_provider = pretool_provider_str.parse::<providers::AIProvider>()
            .with_context(|| format!("Invalid PRETOOL_PROVIDER: {pretool_provider_str}. Supported: openai, anthropic, google, xai"))?;
        let posttool_provider = posttool_provider_str.parse::<providers::AIProvider>()
            .with_context(|| format!("Invalid POSTTOOL_PROVIDER: {posttool_provider_str}. Supported: openai, anthropic, google, xai"))?;

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
            pretool_model: std::env::var("PRETOOL_MODEL").unwrap_or_else(|_| "grok-code-fast-1".to_string()),
            posttool_model: std::env::var("POSTTOOL_MODEL")
                .unwrap_or_else(|_| "grok-code-fast-1".to_string()),
            max_tokens,
            temperature,
            max_issues,
            request_timeout_secs,
            connect_timeout_secs,

            // Provider-specific output token limits (based on documentation)
            gpt5_max_output_tokens: std::env::var("GPT5_MAX_OUTPUT_TOKENS")
                .unwrap_or_else(|_| "12000".to_string())
                .parse::<u32>()
                .unwrap_or(12000)
                .clamp(1000, 128000), // GPT-5: 128K output tokens max
            claude_max_output_tokens: std::env::var("CLAUDE_MAX_OUTPUT_TOKENS")
                .unwrap_or_else(|_| "4000".to_string())
                .parse::<u32>()
                .unwrap_or(4000)
                .clamp(1000, 8000), // Claude: 4K typical, 8K max
            gemini_max_output_tokens: std::env::var("GEMINI_MAX_OUTPUT_TOKENS")
                .unwrap_or_else(|_| "8000".to_string())
                .parse::<u32>()
                .unwrap_or(8000)
                .clamp(1000, 32000), // Gemini: Variable, 32K max
            grok_max_output_tokens: std::env::var("GROK_MAX_OUTPUT_TOKENS")
                .unwrap_or_else(|_| "8000".to_string())
                .parse::<u32>()
                .unwrap_or(8000)
                .clamp(1000, 8000), // Grok: 8K typical
        };

        // Skip strict validation - allow missing API keys for graceful degradation
        // Only validate basic configuration parameters
        if config.max_tokens == 0 || config.max_tokens > 100_000 {
            return Err(anyhow::anyhow!("max_tokens must be between 1 and 100,000"));
        }

        if config.temperature < 0.0 || config.temperature > 2.0 {
            return Err(anyhow::anyhow!("temperature must be between 0.0 and 2.0"));
        }

        if config.request_timeout_secs == 0 || config.request_timeout_secs > 600 {
            return Err(anyhow::anyhow!("request_timeout_secs must be between 1 and 600"));
        }

        Ok(config)
    }
}

/// File extension validation
pub fn should_validate_file(file_path: &str) -> bool {
    let code_extensions = [
        ".js", ".ts", ".jsx", ".tsx", ".mjs", ".cjs", ".py", ".pyw", ".pyc", ".pyo", ".java", ".class",
        ".jar", ".cpp", ".c", ".cc", ".cxx", ".h", ".hpp", ".cs", ".vb", ".php", ".php3", ".php4", ".php5",
        ".phtml", ".rb", ".rbw", ".go", ".rs", ".kt", ".kts", ".swift", ".sql", ".sh", ".bash", ".zsh",
        ".fish", ".ps1", ".psm1", ".html", ".htm", ".xhtml",
    ];

    code_extensions
        .iter()
        .any(|ext| file_path.to_lowercase().ends_with(ext))
}

/// Extract content from tool input based on tool type
pub fn extract_content_from_tool_input(
    tool_name: &str,
    tool_input: &HashMap<String, serde_json::Value>,
) -> String {
    match tool_name {
        "Write" => tool_input
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Edit" => tool_input
            .get("new_string")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "MultiEdit" => tool_input
            .get("edits")
            .and_then(|v| v.as_array())
            .map(|edits| {
                edits
                    .iter()
                    .filter_map(|edit| edit.get("new_string")?.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Get file path from tool input
pub fn extract_file_path(tool_input: &HashMap<String, serde_json::Value>) -> String {
    tool_input
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}
