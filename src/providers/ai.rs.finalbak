/// Multi-provider AI client implementation
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::Duration;
use url::Url;

use crate::{Config, SecurityValidation};
#[cfg(test)]
use crate::{GrokCodeAnalysis, GrokCodeIssue, GrokCodeMetrics, GrokCodeSuggestion};
use std::collections::HashMap;

// AI-assisted code review structures for enhanced analysis with security constraints
const MAX_REVIEW_TEXT_LENGTH: usize = 2000;
const MAX_SUGGESTIONS_PER_REVIEW: usize = 20;
const MAX_ISSUES_PER_REVIEW: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCodeReview {
    pub overall_score: f32, // 0.0 to 10.0
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<AiReviewIssue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<AiCodeSuggestion>,
    pub complexity_assessment: AiComplexityAssessment,
    pub security_analysis: AiSecurityAnalysis,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub performance_notes: Vec<String>,
    pub maintainability_score: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub test_coverage_recommendations: Vec<String>,
    pub ai_confidence: f32, // 0.0 to 1.0
}

impl AiCodeReview {
    pub fn validate_and_sanitize(&mut self) -> Result<(), String> {
        // Clamp scores to valid ranges
        self.overall_score = self.overall_score.clamp(0.0, 10.0);
        self.maintainability_score = self.maintainability_score.clamp(0.0, 10.0);
        self.ai_confidence = self.ai_confidence.clamp(0.0, 1.0);

        // Limit collection sizes to prevent memory exhaustion
        if self.issues.len() > MAX_ISSUES_PER_REVIEW {
            self.issues.truncate(MAX_ISSUES_PER_REVIEW);
        }

        if self.suggestions.len() > MAX_SUGGESTIONS_PER_REVIEW {
            self.suggestions.truncate(MAX_SUGGESTIONS_PER_REVIEW);
        }

        // Sanitize text fields
        for issue in &mut self.issues {
            issue.sanitize_text_fields()?;
        }

        for suggestion in &mut self.suggestions {
            suggestion.sanitize_text_fields()?;
        }

        // Truncate performance notes if too long
        self.performance_notes.truncate(10);
        for note in &mut self.performance_notes {
            if note.len() > MAX_REVIEW_TEXT_LENGTH {
                note.truncate(MAX_REVIEW_TEXT_LENGTH);
                note.push_str("... [truncated]");
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiReviewIssue {
    pub severity: AiIssueSeverity,
    pub category: AiIssueCategory,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<String>,
    pub confidence: f32, // 0.0 to 1.0
    pub impact_assessment: AiImpactAssessment,
}

impl AiReviewIssue {
    fn sanitize_text_fields(&mut self) -> Result<(), String> {
        // Truncate and sanitize text fields
        if self.title.len() > MAX_REVIEW_TEXT_LENGTH {
            self.title.truncate(MAX_REVIEW_TEXT_LENGTH);
            self.title.push_str("...");
        }

        if self.description.len() > MAX_REVIEW_TEXT_LENGTH {
            self.description.truncate(MAX_REVIEW_TEXT_LENGTH);
            self.description.push_str("...");
        }

        if let Some(ref mut fix) = self.suggested_fix {
            if fix.len() > MAX_REVIEW_TEXT_LENGTH {
                fix.truncate(MAX_REVIEW_TEXT_LENGTH);
                fix.push_str("...");
            }
        }

        // Clamp confidence
        self.confidence = self.confidence.clamp(0.0, 1.0);

        // Validate line/column numbers
        if let Some(line) = self.line_number {
            if line == 0 || line > 100_000 {
                return Err("Invalid line number".to_string());
            }
        }

        if let Some(col) = self.column_number {
            if col > 1000 {
                return Err("Invalid column number".to_string());
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiIssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
    Blocker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiIssueCategory {
    Syntax,
    Logic,
    Performance,
    Security,
    Maintainability,
    Style,
    Testing,
    Documentation,
    Architecture,
    Dependencies,
    Concurrency,
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCodeSuggestion {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    pub impact: AiImpactLevel,
    pub effort: AiEffortLevel,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub category: String,
    pub priority_score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation_notes: Option<String>,
}

impl AiCodeSuggestion {
    fn sanitize_text_fields(&mut self) -> Result<(), String> {
        // Truncate text fields
        if self.title.len() > MAX_REVIEW_TEXT_LENGTH {
            self.title.truncate(MAX_REVIEW_TEXT_LENGTH);
        }

        if self.description.len() > MAX_REVIEW_TEXT_LENGTH {
            self.description.truncate(MAX_REVIEW_TEXT_LENGTH);
        }

        // Sanitize code examples
        if let Some(ref mut before) = self.before {
            if before.len() > MAX_REVIEW_TEXT_LENGTH * 2 {
                before.truncate(MAX_REVIEW_TEXT_LENGTH * 2);
            }
        }

        if let Some(ref mut after) = self.after {
            if after.len() > MAX_REVIEW_TEXT_LENGTH * 2 {
                after.truncate(MAX_REVIEW_TEXT_LENGTH * 2);
            }
        }

        // Clamp priority score
        self.priority_score = self.priority_score.clamp(0.0, 100.0);

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiImpactLevel {
    Negligible,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiEffortLevel {
    Trivial,   // < 1 hour
    Low,       // 1-4 hours
    Medium,    // 4-16 hours
    High,      // 16-40 hours
    Extensive, // > 40 hours
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImpactAssessment {
    pub technical_debt: f32,         // 0.0 to 10.0
    pub maintainability_impact: f32, // -5.0 to 5.0
    pub performance_impact: f32,     // -5.0 to 5.0
    pub security_impact: f32,        // 0.0 to 10.0
}

impl AiImpactAssessment {
    pub fn validate(&mut self) {
        self.technical_debt = self.technical_debt.clamp(0.0, 10.0);
        self.maintainability_impact = self.maintainability_impact.clamp(-5.0, 5.0);
        self.performance_impact = self.performance_impact.clamp(-5.0, 5.0);
        self.security_impact = self.security_impact.clamp(0.0, 10.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiComplexityAssessment {
    pub cognitive_complexity: u32,
    pub cyclomatic_complexity: u32,
    pub maintainability_index: f32,
    pub technical_debt_hours: f32,
    pub refactoring_priority: AiRefactoringPriority,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub complexity_hotspots: Vec<AiComplexityHotspot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiRefactoringPriority {
    None,
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiComplexityHotspot {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub function_name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub complexity_score: f32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub recommended_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSecurityAnalysis {
    pub risk_level: AiSecurityRiskLevel,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vulnerabilities: Vec<AiSecurityIssue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub best_practices_violations: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recommendations: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owasp_categories: Vec<String>,
    pub security_score: f32, // 0.0 to 10.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiSecurityRiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSecurityIssue {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub vulnerability_type: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwe_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvss_score: Option<f32>,
    pub severity: AiSecurityRiskLevel,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub remediation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBatchReviewResult {
    pub total_files: usize,
    pub successful_reviews: usize,
    pub failed_reviews: usize,
    pub reviews: HashMap<String, AiCodeReview>,
    pub errors: HashMap<String, String>,
    pub summary_statistics: AiReviewSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiReviewSummary {
    pub average_overall_score: f32,
    pub total_issues_found: usize,
    pub issues_by_severity: HashMap<String, usize>,
    pub most_common_categories: Vec<String>,
    pub estimated_total_fix_hours: f32,
    pub security_risk_distribution: HashMap<String, usize>,
}

/// Supported AI providers for validation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AIProvider {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "google")]
    Google,
    #[serde(rename = "xai")]
    XAI,
}

impl std::fmt::Display for AIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            AIProvider::OpenAI => "openai",
            AIProvider::Anthropic => "anthropic",
            AIProvider::Google => "google",
            AIProvider::XAI => "xai",
        };
        write!(f, "{name}")
    }
}

impl std::str::FromStr for AIProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(AIProvider::OpenAI),
            "anthropic" => Ok(AIProvider::Anthropic),
            "google" => Ok(AIProvider::Google),
            "xai" => Ok(AIProvider::XAI),
            _ => Err(anyhow::anyhow!(
                "Invalid provider: {}. Supported: openai, anthropic, google, xai",
                s
            )),
        }
    }
}

impl AIProvider {
    /// Get the default base URL for this provider
    pub fn default_base_url(&self) -> &'static str {
        match self {
            AIProvider::OpenAI => "https://api.openai.com/v1",
            AIProvider::Anthropic => "https://api.anthropic.com",
            AIProvider::Google => "https://generativelanguage.googleapis.com/v1",
            AIProvider::XAI => "https://api.x.ai/v1",
        }
    }
}

/// Universal AI client that supports multiple providers
pub struct UniversalAIClient {
    client: Client,
    config: Config,
}

impl UniversalAIClient {
    #[cfg(test)]
    #[allow(dead_code)]
    fn get_code_analysis_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["summary", "overall_quality", "issues", "suggestions"],
            "additionalProperties": false,
            "properties": {
                "summary": {"type": "string"},
                "overall_quality": {"type": "string"},
                "issues": {"type": "array", "items": {"type": "object"}},
                "suggestions": {"type": "array", "items": {"type": "object"}}
            }
        })
    }
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    /// Validate security using the configured pretool provider
    pub async fn validate_security_pretool(&self, code: &str, prompt: &str) -> Result<SecurityValidation> {
        match self.config.pretool_provider {
            AIProvider::OpenAI => {
                // Check if it's GPT-5 (uses different API)
                if self.config.pretool_model.starts_with("gpt-5") {
                    self.validate_with_gpt5(code, prompt).await
                } else {
                    self.validate_with_openai(code, prompt).await
                }
            }
            AIProvider::Anthropic => self.validate_with_anthropic(code, prompt).await,
            AIProvider::Google => self.validate_with_google(code, prompt).await,
            AIProvider::XAI => self.validate_with_xai(code, prompt).await,
        }
    }

    /// Analyze code using the configured posttool provider
    pub async fn analyze_code_posttool(&self, code: &str, prompt: &str) -> Result<String> {
        // Return raw response from AI instead of parsing into structures
        match self.config.posttool_provider {
            AIProvider::OpenAI => {
                // For GPT-5 family use Responses API with strict JSON schema; otherwise use Chat Completions
                if self.config.posttool_model.starts_with("gpt-5") {
                    self.analyze_with_gpt5_raw(code, prompt).await
                } else {
                    self.analyze_with_openai_raw(code, prompt).await
                }
            }
            AIProvider::Anthropic => self.analyze_with_anthropic_raw(code, prompt).await,
            AIProvider::Google => self.analyze_with_google_raw(code, prompt).await,
            AIProvider::XAI => self.analyze_with_xai_raw(code, prompt).await,
        }
    }

    /// GPT-5 specific implementation (uses Responses API)
    async fn validate_with_gpt5(&self, code: &str, prompt: &str) -> Result<SecurityValidation> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::OpenAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::OpenAI);

        // GPT-5-nano uses Chat Completions API with special parameters
        // No temperature, top_p, or stop allowed for reasoning models
        let request_body = serde_json::json!({
            "model": self.config.pretool_model,
            "messages": [
                {
                    "role": "system",
                    "content": prompt
                },
                {
                    "role": "user",
                    "content": format!("Analyze this code for security risks:\n\n{code}")
                }
            ],
            "max_completion_tokens": 10000,  // Increased to 10k tokens for comprehensive validation
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "SecurityValidation",
                    "schema": self.get_security_validation_schema(),
                    "strict": true
                }
            }
        });

        let response = self
            .client
            .post(format!("{base_url}/chat/completions"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            // Don't add OpenAI-Project header for gpt-5-nano
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to GPT-5")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("GPT-5 API error: {}", error_text));
        }

        // GPT-5 uses standard Chat Completions response format
        #[derive(Deserialize)]
        struct ChatCompletionResponse {
            choices: Vec<ChatChoice>,
        }

        #[derive(Deserialize)]
        struct ChatChoice {
            message: ChatMessage,
        }

        #[derive(Deserialize)]
        struct ChatMessage {
            content: String,
        }

        // Parse response body
        let response_text = response
            .text()
            .await
            .context("Failed to read GPT-5 response body")?;

        // Check for empty response
        if response_text.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty response from GPT-5"));
        }

        // Log only in debug mode
        if debug_hooks_enabled() {
            tracing::debug!(len = response_text.len(), "GPT-5 response length");
            if response_text.len() < 500 {
                tracing::debug!(response=%response_text, "GPT-5 response");
            }
        }

        // Parse as ChatCompletionResponse
        let gpt5_response: ChatCompletionResponse =
            serde_json::from_str(&response_text).with_context(|| {
                format!(
                    "Failed to parse GPT-5 response. First 200 chars: {}",
                    &response_text.chars().take(200).collect::<String>()
                )
            })?;

        // Extract content from response
        let json_text = gpt5_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No valid response in GPT-5 choices"))?;

        // Check for empty content
        if json_text.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty content from GPT-5"));
        }

        // Log only in debug mode
        if debug_hooks_enabled() {
            tracing::debug!(snippet=%json_text.chars().take(200).collect::<String>(), "GPT-5 content to parse");
        }

        // Try to parse as SecurityValidation with better error handling
        let validation: SecurityValidation = serde_json::from_str(&json_text)
            .or_else(|e| {
                // If JSON parsing fails, try to create a validation from plain text
                if json_text.to_lowercase().contains("deny")
                    || json_text.to_lowercase().contains("block")
                    || json_text.to_lowercase().contains("dangerous")
                {
                    Ok(SecurityValidation {
                        decision: "deny".to_string(),
                        reason: json_text.clone(),
                        security_concerns: Some(vec![json_text.clone()]),
                        risk_level: "high".to_string(),
                    })
                } else if json_text.to_lowercase().contains("allow")
                    || json_text.to_lowercase().contains("safe")
                {
                    Ok(SecurityValidation {
                        decision: "allow".to_string(),
                        reason: "Code appears safe".to_string(),
                        security_concerns: None,
                        risk_level: "low".to_string(),
                    })
                } else {
                    Err(e)
                }
            })
            .with_context(|| {
                format!(
                    "Failed to parse security validation. Content: {}",
                    &json_text.chars().take(200).collect::<String>()
                )
            })?;

        Ok(validation)
    }

    /// Standard OpenAI implementation (GPT-4, GPT-3.5, etc.)
    async fn validate_with_openai(&self, code: &str, prompt: &str) -> Result<SecurityValidation> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::OpenAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::OpenAI);

        let request_body = serde_json::json!({
            "model": self.config.pretool_model,
            "messages": [
                {
                    "role": "system",
                    "content": prompt
                },
                {
                    "role": "user",
                    "content": format!("Analyze this code for security risks:\n\n{code}")
                }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "SecurityValidation",
                    "schema": self.get_security_validation_schema(),
                    "strict": true
                }
            },
            "max_tokens": 1024,
            "temperature": self.config.temperature
        });

        let response = self
            .client
            .post(format!("{base_url}/chat/completions"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        self.parse_openai_response(response).await
    }

    /// Anthropic (Claude) implementation
    async fn validate_with_anthropic(&self, code: &str, prompt: &str) -> Result<SecurityValidation> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::Anthropic);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::Anthropic);

        let request_body = serde_json::json!({
            "model": self.config.pretool_model,
            "messages": [
                {
                    "role": "user",
                    "content": format!("{prompt}\n\nAnalyze this code for security risks:\n\n{code}")
                }
            ],
            "max_tokens": 1024,
            "temperature": self.config.temperature,
            "system": prompt
        });

        let response = self
            .client
            .post(format!("{base_url}/v1/messages"))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        self.parse_anthropic_response(response).await
    }

    /// Google (Gemini) implementation
    async fn validate_with_google(&self, code: &str, prompt: &str) -> Result<SecurityValidation> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::Google);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::Google);

        let request_body = serde_json::json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": format!("{prompt}\n\nAnalyze this code for security risks:\n\n{code}")
                        }
                    ]
                }
            ],
            "generationConfig": {
                "temperature": self.config.temperature,
                "maxOutputTokens": 1024,
                "responseMimeType": "application/json",
                "responseSchema": self.get_security_validation_schema()
            }
        });

        let model_name = &self.config.pretool_model;
        let response = self
            .client
            .post(format!("{base_url}/models/{model_name}:generateContent?key={api_key}"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Google")?;

        self.parse_google_response(response).await
    }

    /// xAI (Grok) implementation
    async fn validate_with_xai(&self, code: &str, prompt: &str) -> Result<SecurityValidation> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::XAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::XAI);

        let request_body = serde_json::json!({
            "model": self.config.pretool_model,
            "messages": [
                {
                    "role": "system",
                    "content": prompt
                },
                {
                    "role": "user",
                    "content": format!("Analyze this code for security risks:\n\n{code}")
                }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "SecurityValidation",
                    "schema": self.get_security_validation_schema()
                }
            },
            "max_tokens": 1024,
            "temperature": self.config.temperature,
            "stream": false
        });

        let response = self
            .client
            .post(format!("{base_url}/chat/completions"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to xAI")?;

        self.parse_openai_response(response).await // xAI uses OpenAI-compatible format
    }

    /// Get the JSON schema for security validation
    fn get_security_validation_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["decision", "reason", "risk_level", "security_concerns"],
            "additionalProperties": false,
            "properties": {
                "decision": {
                    "type": "string",
                    "enum": ["allow", "ask", "deny"]
                },
                "reason": {
                    "type": "string",
                    "maxLength": 500
                },
                "security_concerns": {
                    "type": ["array", "null"],
                    "maxItems": 5,
                    "items": {
                        "type": "string",
                        "maxLength": 200
                    }
                },
                "risk_level": {
                    "type": "string",
                    "enum": ["low", "medium", "high", "critical"]
                }
            }
        })
    }

    // (removed unused structured code analysis schema)

    /// Parse OpenAI-compatible response
    async fn parse_openai_response(&self, response: reqwest::Response) -> Result<SecurityValidation> {
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<Choice>,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: Message,
        }

        #[derive(Deserialize)]
        struct Message {
            content: String,
        }

        let api_response: OpenAIResponse = response.json().await.context("Failed to parse API response")?;

        let content = api_response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?
            .message
            .content
            .clone();

        let validation: SecurityValidation =
            serde_json::from_str(&content).context("Failed to parse security validation")?;

        Ok(validation)
    }

    /// Parse Anthropic response
    async fn parse_anthropic_response(&self, response: reqwest::Response) -> Result<SecurityValidation> {
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<Content>,
        }

        #[derive(Deserialize)]
        struct Content {
            text: String,
        }

        let api_response: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        let text = api_response
            .content
            .first()
            .ok_or_else(|| anyhow::anyhow!("No content in Anthropic response"))?
            .text
            .clone();

        let validation: SecurityValidation =
            serde_json::from_str(&text).context("Failed to parse security validation from Anthropic")?;

        Ok(validation)
    }

    /// Parse Google response
    async fn parse_google_response(&self, response: reqwest::Response) -> Result<SecurityValidation> {
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Google API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct GoogleResponse {
            candidates: Vec<Candidate>,
        }

        #[derive(Deserialize)]
        struct Candidate {
            content: ContentPart,
        }

        #[derive(Deserialize)]
        struct ContentPart {
            parts: Vec<Part>,
        }

        #[derive(Deserialize)]
        struct Part {
            text: String,
        }

        let api_response: GoogleResponse =
            response.json().await.context("Failed to parse Google response")?;

        let text = api_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .ok_or_else(|| anyhow::anyhow!("No content in Google response"))?
            .text
            .clone();

        let validation: SecurityValidation =
            serde_json::from_str(&text).context("Failed to parse security validation from Google")?;

        Ok(validation)
    }

    // Code analysis methods for PostToolUse hook

    /// Analyze code with GPT-5 (uses Responses API)
    #[cfg(test)]
    #[allow(dead_code)]
    async fn analyze_with_gpt5(&self, code: &str, prompt: &str) -> Result<GrokCodeAnalysis> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::OpenAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::OpenAI);

        let request_body = serde_json::json!({
            "model": self.config.posttool_model,
            "input": [
                {
                    "role": "system",
                    "content": [
                        {
                            "type": "input_text",
                            "text": prompt
                        }
                    ]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": format!("Analyze this code and provide detailed review:\n\n{code}")
                        }
                    ]
                }
            ],
            "text": {
                "format": {
                    "type": "json"
                }
            },
            "max_output_tokens": 2048,
            "reasoning": {
                "effort": "high"
            },
            "store": false
        });

        let response = self
            .client
            .post(format!("{base_url}/responses"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to GPT-5")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("GPT-5 API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct Gpt5Response {
            output_text: String,
        }

        let gpt5_response: Gpt5Response = response.json().await.context("Failed to parse GPT-5 response")?;

        // Try to parse new format first, fallback to old format
        if let Ok(new_format) = serde_json::from_str::<serde_json::Value>(&gpt5_response.output_text) {
            if new_format.get("validation_result").is_some() {
                // Convert new format to old GrokCodeAnalysis format
                return self.convert_new_format_to_grok_analysis(new_format);
            }
        }

        // Fallback to old format parsing
        let analysis: GrokCodeAnalysis = serde_json::from_str(&gpt5_response.output_text)
            .context("Failed to parse code analysis from GPT-5")?;

        Ok(analysis)
    }

    /// Analyze with GPT-5 (Responses API) and return raw JSON string (strict JSON via json_schema)
    async fn analyze_with_gpt5_raw(&self, code: &str, prompt: &str) -> Result<String> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::OpenAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::OpenAI);

        // Use proper GPT-5 Responses API format
        let combined_input = format!("{prompt}\n\nCode to analyze:\n{code}");

        let request_body = serde_json::json!({
            "model": self.config.posttool_model,
            "input": combined_input,
            "max_output_tokens": self.config.get_max_output_tokens_for_provider(&AIProvider::OpenAI),
            "reasoning": { "effort": "medium" },
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "AgentActionPlan",
                    "schema": self.get_agent_action_schema(),
                    "strict": true
                }
            },
            "store": false
        });

        let response = self
            .client
            .post(format!("{base_url}/responses"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to GPT-5")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("GPT-5 API error: {}", error_text));
        }

        let v: serde_json::Value = response.json().await.context("Failed to parse GPT-5 response")?;
        // Extract concatenated text from output.message.content[].text
        let mut collected = String::new();
        if let Some(arr) = v.get("output").and_then(|x| x.as_array()) {
            for item in arr {
                if item.get("type").and_then(|t| t.as_str()) == Some("message") {
                    if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                        for c in content {
                            if let Some(t) = c.get("text").and_then(|t| t.as_str()) {
                                if !collected.is_empty() { collected.push('\n'); }
                                collected.push_str(t);
                            }
                        }
                    }
                }
            }
        }
        if collected.trim().is_empty() {
            // Fallback: try a top-level message.content[0].text shape if present
            if let Some(text) = v.get("output_text").and_then(|t| t.as_str()) {
                collected = text.to_string();
            }
        }
        Ok(collected)
    }

    /// Analyze with OpenAI and return raw response
    async fn analyze_with_openai_raw(&self, code: &str, prompt: &str) -> Result<String> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::OpenAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::OpenAI);

        // Enforce strict JSON using Chat Completions + json_schema
        let system = format!("{}\n\nIMPORTANT: Respond ONLY a single JSON object that conforms to the provided schema. No code fences. No extra text.", prompt);
        let user = format!("Code to analyze:\n{code}\n\nReturn ONLY JSON.");

        let request_body = serde_json::json!({
            "model": self.config.posttool_model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user",   "content": user }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "AgentActionPlan",
                    "schema": self.get_agent_action_schema(),
                    "strict": true
                }
            },
            "max_tokens": 2000,
            "reasoning_effort": "medium"
        });

        let response = self
            .client
            .post(format!("{base_url}/chat/completions"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct ChatResponse {
            choices: Vec<ChatChoice>,
        }

        #[derive(Deserialize)]
        struct ChatChoice {
            message: ChatMessage,
        }

        #[derive(Deserialize)]
        struct ChatMessage {
            content: String,
        }

        let chat_response: ChatResponse = response.json().await.context("Failed to parse OpenAI response")?;

        chat_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))
    }

    fn get_agent_action_schema(&self) -> serde_json::Value {
        serde_json::json!({
          "type": "object",
          "additionalProperties": false,
          "required": ["schema_version", "quality", "risk_summary", "actions"],
          "properties": {
            "schema_version": {"type":"string"},
            "quality": {"type":"object", "additionalProperties": false, "required":["overall","confidence"],
              "properties": {
                "overall": {"type":"integer","minimum":0,"maximum":1000},
                "confidence": {"type":"number","minimum":0.0,"maximum":1.0}
              }
            },
            "risk_summary": {"type":"array", "maxItems": 10, "items": {"type":"object", "additionalProperties": false,
              "required":["severity","msg"],
              "properties": {
                "severity": {"type":"string","enum":["critical","major","minor"]},
                "line": {"type":["integer","null"]},
                "rule": {"type":["string","null"]},
                "msg": {"type":"string","maxLength": 200}
              }
            }},
            "api_contract": {"type":"object", "additionalProperties": false, "properties": {
              "removed_functions": {"type":"array","maxItems":20,"items": {"type":"object", "additionalProperties": false,
                "required":["name","file"], "properties": {"name":{"type":"string"}, "file":{"type":"string"}}
              }},
              "param_changes": {"type":"array","maxItems":20,"items": {"type":"object", "additionalProperties": false,
                "required":["name","file","removed"], "properties": {"name":{"type":"string"}, "file":{"type":"string"}, "removed": {"type":"array","items":{"type":"string"}, "maxItems": 20}}
              }}
            }},
            "actions": {"type":"object", "additionalProperties": false, "required":["edits"], "properties": {
              "edits": {"type":"array","maxItems": 10, "items": {"type":"object", "additionalProperties": false,
                "required":["file","op","anchor","after"],
                "properties": {
                  "file": {"type":"string"},
                  "op": {"type":"string","enum":["replace","insert","delete"]},
                  "anchor": {"type":"object", "additionalProperties": false, "required":["type","value"], "properties": {
                    "type": {"type":"string","enum":["line","regex","symbol"]},
                    "value": {"type":"string"}
                  }},
                  "before": {"type":["string","null"]},
                  "after": {"type":"string"}
                }
              }},
              "tests": {"type":"array","maxItems": 5, "items": {"type":"object", "additionalProperties": false,
                "required":["file","framework","name","content"],
                "properties": {"file":{"type":"string"},"framework":{"type":"string"},"name":{"type":"string"},"content":{"type":"string"}}
              }},
              "refactors": {"type":"array","maxItems":5, "items": {"type":"object", "additionalProperties": false,
                "required":["file","entity","goal","steps"],
                "properties": {"file":{"type":"string"}, "entity":{"type":"string"}, "goal":{"type":"string"}, "steps":{"type":"array","maxItems":6, "items":{"type":"string"}}}
              }}
            }},
            "followup_tools": {"type":"array","maxItems":5, "items": {"type":"object", "additionalProperties": false,
              "required":["tool","file"], "properties": {"tool": {"type":"string","enum":["Write","Edit","MultiEdit"]}, "file": {"type":"string"}, "note": {"type":"string"}}}},
            "notes": {"type":"array","maxItems":8, "items": {"type":"string"}}
          }
        })
    }

    /// Analyze with Anthropic and return raw response
    async fn analyze_with_anthropic_raw(&self, code: &str, prompt: &str) -> Result<String> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::Anthropic);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::Anthropic);

        let request_body = serde_json::json!({
            "model": self.config.posttool_model,
            "messages": [
                {
                    "role": "user",
                    "content": format!("Analyze this code and provide detailed review:\n\n{code}")
                }
            ],
            "max_tokens": self.config.get_max_output_tokens_for_provider(&AIProvider::Anthropic),
            "temperature": self.config.temperature,
            "system": prompt
        });

        let response = self
            .client
            .post(format!("{base_url}/v1/messages"))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<ContentBlock>,
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: String,
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        anthropic_response
            .content
            .first()
            .map(|block| block.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from Anthropic"))
    }

    /// Analyze with Google and return raw response
    async fn analyze_with_google_raw(&self, code: &str, prompt: &str) -> Result<String> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::Google);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::Google);

        let request_body = serde_json::json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": format!("{prompt}\n\nAnalyze this code and provide detailed review:\n\n{code}")
                        }
                    ]
                }
            ],
            "generationConfig": {
                "temperature": self.config.temperature,
                "maxOutputTokens": self.config.get_max_output_tokens_for_provider(&AIProvider::Google),
            }
        });

        let response = self
            .client
            .post(format!("{base_url}?key={api_key}"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Google")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Google API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct GoogleResponse {
            candidates: Vec<Candidate>,
        }

        #[derive(Deserialize)]
        struct Candidate {
            content: Content,
        }

        #[derive(Deserialize)]
        struct Content {
            parts: Vec<Part>,
        }

        #[derive(Deserialize)]
        struct Part {
            text: String,
        }

        let google_response: GoogleResponse =
            response.json().await.context("Failed to parse Google response")?;

        google_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from Google"))
    }

    /// Analyze with xAI and return raw response
    async fn analyze_with_xai_raw(&self, code: &str, prompt: &str) -> Result<String> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::XAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::XAI);

        // Helper: OpenAI-compatible route
        #[allow(clippy::too_many_arguments)]
        async fn xai_chat(
            client: &reqwest::Client,
            base_url: &str,
            api_key: &str,
            model: &str,
            system: &str,
            user: &str,
            max_tokens: u32,
            temperature: f32,
        ) -> anyhow::Result<String> {
            let body = serde_json::json!({
                "model": model,
                "messages": [
                    {"role":"system","content": system},
                    {"role":"user","content": user}
                ],
                "max_tokens": max_tokens,
                "temperature": temperature
            });
            let resp = client
                .post(format!("{base_url}/chat/completions"))
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .context("Failed to send request to xAI (chat/completions)")?;
            if !resp.status().is_success() {
                let err = resp.text().await.unwrap_or_default();
                anyhow::bail!("xAI API error: {}", err);
            }
            #[derive(Deserialize)]
            struct XaiResponse { choices: Vec<XaiChoice> }
            #[derive(Deserialize)]
            struct XaiChoice { message: XaiMessage }
            #[derive(Deserialize)]
            struct XaiMessage { content: String }
            let xai: XaiResponse = resp.json().await.context("Failed to parse xAI response (chat)")?;
            let content = xai
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .ok_or_else(|| anyhow::anyhow!("No response from xAI (chat)"))?;
            Ok(content)
        }

        // Helper: Anthropic-compatible route (messages). Non-streaming.
        async fn xai_messages(
            client: &reqwest::Client,
            base_url: &str,
            api_key: &str,
            model: &str,
            user: &str,
        ) -> anyhow::Result<String> {
            let body = serde_json::json!({
                "model": model,
                "messages": [ {"role":"user","content": user} ],
                "stream": false
            });
            let resp = client
                .post(format!("{base_url}/messages"))
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .context("Failed to send request to xAI (messages)")?;
            if !resp.status().is_success() {
                let err = resp.text().await.unwrap_or_default();
                anyhow::bail!("xAI API error (messages): {}", err);
            }
            // Try two common shapes: {content:[{text:"..."}]} or {choices:[{message:{content:"..."}}]}
            let text = resp.text().await.unwrap_or_default();
            #[derive(Deserialize)]
            struct ContentText { text: String }
            #[derive(Deserialize)]
            struct MsgContent { content: Vec<ContentText> }
            #[derive(Deserialize)]
            struct RespA { content: Vec<ContentText> }
            if let Ok(a) = serde_json::from_str::<RespA>(&text) {
                if let Some(first) = a.content.first() { return Ok(first.text.clone()); }
            }
            #[derive(Deserialize)] struct Choice { message: MsgContent }
            #[derive(Deserialize)] struct RespB { choices: Vec<Choice> }
            if let Ok(b) = serde_json::from_str::<RespB>(&text) {
                if let Some(first) = b.choices.first().and_then(|c| c.message.content.first()) {
                    return Ok(first.text.clone());
                }
            }
            // Fallback to raw text
            Ok(text)
        }

        let system = prompt;
        let user = format!("Analyze this code and provide detailed review:\n\n{code}");
        let max = self.config.get_max_output_tokens_for_provider(&AIProvider::XAI);
        let temp = self.config.temperature;

        match xai_chat(&self.client, base_url, api_key, &self.config.posttool_model, system, &user, max, temp).await {
            Ok(s) => Ok(s),
            Err(e1) => {
                // Fallback to messages endpoint
                tracing::warn!(error=%e1, "xAI chat/completions failed; trying /v1/messages");
                xai_messages(&self.client, base_url, api_key, &self.config.posttool_model, &user).await
            }
        }
    }

    /// Convert new validation format to GrokCodeAnalysis format
    #[cfg(test)]
    #[allow(dead_code)]
    fn convert_new_format_to_grok_analysis(&self, new_format: serde_json::Value) -> Result<GrokCodeAnalysis> {
        let validation_result = new_format
            .get("validation_result")
            .ok_or_else(|| anyhow::anyhow!("Missing validation_result in response"))?;

        // Extract overall assessment
        let overall_assessment = validation_result
            .get("overall_assessment")
            .ok_or_else(|| anyhow::anyhow!("Missing overall_assessment"))?;

        let quality_score = overall_assessment
            .get("quality_score")
            .and_then(|v| v.as_u64())
            .unwrap_or(500);

        let status = overall_assessment
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("Неизвестно");

        // Extract executive summary
        let summary = validation_result
            .get("executive_summary")
            .and_then(|v| v.as_str())
            .unwrap_or(status)
            .to_string();

        // Determine overall quality based on score
        let overall_quality = match quality_score {
            850..=1000 => "excellent",
            700..=849 => "good",
            500..=699 => "needs_improvement",
            _ => "poor",
        }
        .to_string();

        // Convert critical issues and improvements to GrokCodeIssue format
        let mut issues = Vec::new();

        // Add critical issues
        if let Some(critical_issues) = validation_result
            .get("critical_issues")
            .and_then(|v| v.as_array())
        {
            for issue in critical_issues {
                if let Some(issue_obj) = issue.as_object() {
                    issues.push(GrokCodeIssue {
                        severity: "critical".to_string(),
                        category: issue_obj
                            .get("category")
                            .and_then(|v| v.as_str())
                            .unwrap_or("correctness")
                            .to_lowercase(),
                        message: issue_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Критическая проблема")
                            .to_string(),
                        line: issue_obj
                            .get("line_reference")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        impact: Some(3),
                        fix_cost: Some(2),
                        confidence: Some(0.95),
                        fix_suggestion: issue_obj
                            .get("solution")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
        }

        // Add improvement opportunities as issues
        if let Some(improvements) = validation_result
            .get("improvement_opportunities")
            .and_then(|v| v.as_array())
        {
            for improvement in improvements {
                if let Some(imp_obj) = improvement.as_object() {
                    let priority = imp_obj.get("priority").and_then(|v| v.as_str()).unwrap_or("P2");

                    let severity = match priority {
                        "P0" | "P1" => "major",
                        "P2" => "minor",
                        _ => "info",
                    };

                    issues.push(GrokCodeIssue {
                        severity: severity.to_string(),
                        category: imp_obj
                            .get("category")
                            .and_then(|v| v.as_str())
                            .unwrap_or("maintainability")
                            .to_lowercase(),
                        message: imp_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Возможность улучшения")
                            .to_string(),
                        line: imp_obj
                            .get("line_reference")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        impact: Some(match priority {
                            "P0" | "P1" => 2,
                            _ => 1,
                        }),
                        fix_cost: Some(2),
                        confidence: Some(0.8),
                        fix_suggestion: imp_obj
                            .get("solution")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
        }

        // Convert positive aspects to suggestions
        let mut suggestions = Vec::new();
        if let Some(positive_aspects) = validation_result
            .get("positive_aspects")
            .and_then(|v| v.as_array())
        {
            for (i, aspect) in positive_aspects.iter().enumerate() {
                if let Some(text) = aspect.as_str() {
                    if i < 3 {
                        // Limit to first 3 positive aspects
                        suggestions.push(GrokCodeSuggestion {
                            category: "strengths".to_string(),
                            description: text.to_string(),
                            priority: "low".to_string(),
                            priority_score: Some(10.0),
                            code_example: None,
                        });
                    }
                }
            }
        }

        // Add metrics if available
        let metrics = overall_assessment.get("score_breakdown").map(|breakdown| {
            // Determine complexity based on overall score
            let complexity = match quality_score {
                800..=1000 => "low",
                600..=799 => "medium",
                _ => "high",
            }
            .to_string();

            GrokCodeMetrics {
                complexity: Some(complexity),
                readability: breakdown
                    .get("maintainability")
                    .and_then(|m| m.get("score"))
                    .and_then(|s| s.as_u64())
                    .map(|score| {
                        match score {
                            170..=200 => "excellent",
                            140..=169 => "good",
                            100..=139 => "fair",
                            _ => "poor",
                        }
                        .to_string()
                    }),
                test_coverage: None, // Not available in new format
            }
        });

        Ok(GrokCodeAnalysis {
            summary,
            overall_quality,
            issues,
            suggestions,
            metrics,
        })
    }

    /// Analyze code with standard OpenAI models
    #[cfg(test)]
    #[allow(dead_code)]
    async fn analyze_with_openai(&self, code: &str, prompt: &str) -> Result<GrokCodeAnalysis> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::OpenAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::OpenAI);

        let request_body = serde_json::json!({
            "model": self.config.posttool_model,
            "messages": [
                {
                    "role": "system",
                    "content": prompt
                },
                {
                    "role": "user",
                    "content": format!("Analyze this code and provide detailed review:\n\n{code}")
                }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "GrokCodeAnalysis",
                    "schema": self.get_code_analysis_schema(),
                    "strict": true
                }
            },
            "max_tokens": 2048,
            "temperature": self.config.temperature
        });

        let response = self
            .client
            .post(format!("{base_url}/chat/completions"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        self.parse_openai_analysis_response(response).await
    }

    /// Analyze code with Anthropic Claude
    #[cfg(test)]
    #[allow(dead_code)]
    async fn analyze_with_anthropic(&self, code: &str, prompt: &str) -> Result<GrokCodeAnalysis> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::Anthropic);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::Anthropic);

        let request_body = serde_json::json!({
            "model": self.config.posttool_model,
            "messages": [
                {
                    "role": "user",
                    "content": format!("{prompt}\\n\\nAnalyze this code and provide detailed review:\\n\\n{code}")
                }
            ],
            "max_tokens": 2048,
            "temperature": self.config.temperature,
            "system": prompt
        });

        let response = self
            .client
            .post(format!("{base_url}/v1/messages"))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        self.parse_anthropic_analysis_response(response).await
    }

    /// Analyze code with Google Gemini
    #[cfg(test)]
    #[allow(dead_code)]
    async fn analyze_with_google(&self, code: &str, prompt: &str) -> Result<GrokCodeAnalysis> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::Google);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::Google);

        let request_body = serde_json::json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": format!("{prompt}\\n\\nAnalyze this code and provide detailed review:\\n\\n{code}")
                        }
                    ]
                }
            ],
            "generationConfig": {
                "temperature": self.config.temperature,
                "maxOutputTokens": 2048,
                "responseMimeType": "application/json",
                "responseSchema": self.get_code_analysis_schema()
            }
        });

        let model_name = &self.config.posttool_model;
        let response = self
            .client
            .post(format!(
                "{}/models/{}:generateContent?key={}",
                base_url, model_name, api_key
            ))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Google")?;

        self.parse_google_analysis_response(response).await
    }

    /// Analyze code with xAI Grok
    #[cfg(test)]
    #[allow(dead_code)]
    async fn analyze_with_xai(&self, code: &str, prompt: &str) -> Result<GrokCodeAnalysis> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::XAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::XAI);

        let request_body = serde_json::json!({
            "model": self.config.posttool_model,
            "messages": [
                {
                    "role": "system",
                    "content": prompt
                },
                {
                    "role": "user",
                    "content": format!("Analyze this code and provide detailed review:\\n\\n{code}")
                }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "GrokCodeAnalysis",
                    "schema": self.get_code_analysis_schema()
                }
            },
            "max_tokens": 2048,
            "temperature": self.config.temperature,
            "stream": false
        });

        let response = self
            .client
            .post(format!("{base_url}/chat/completions"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to xAI")?;

        self.parse_openai_analysis_response(response).await // xAI uses OpenAI-compatible format
    }

    // Helper methods to parse analysis responses

    #[cfg(test)]
    #[allow(dead_code)]
    async fn parse_openai_analysis_response(&self, response: reqwest::Response) -> Result<GrokCodeAnalysis> {
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<Choice>,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: Message,
        }

        #[derive(Deserialize)]
        struct Message {
            content: String,
        }

        let api_response: OpenAIResponse = response.json().await.context("Failed to parse API response")?;

        let content = api_response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?
            .message
            .content
            .clone();

        let analysis: GrokCodeAnalysis =
            serde_json::from_str(&content).context("Failed to parse code analysis")?;

        Ok(analysis)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    async fn parse_anthropic_analysis_response(
        &self,
        response: reqwest::Response,
    ) -> Result<GrokCodeAnalysis> {
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<Content>,
        }

        #[derive(Deserialize)]
        struct Content {
            text: String,
        }

        let api_response: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        let text = api_response
            .content
            .first()
            .ok_or_else(|| anyhow::anyhow!("No content in Anthropic response"))?
            .text
            .clone();

        let analysis: GrokCodeAnalysis =
            serde_json::from_str(&text).context("Failed to parse code analysis from Anthropic")?;

        Ok(analysis)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    async fn parse_google_analysis_response(&self, response: reqwest::Response) -> Result<GrokCodeAnalysis> {
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Google API error: {}", error_text));
        }

        #[derive(Deserialize)]
        struct GoogleResponse {
            candidates: Vec<Candidate>,
        }

        #[derive(Deserialize)]
        struct Candidate {
            content: ContentPart,
        }

        #[derive(Deserialize)]
        struct ContentPart {
            parts: Vec<Part>,
        }

        #[derive(Deserialize)]
        struct Part {
            text: String,
        }

        let api_response: GoogleResponse =
            response.json().await.context("Failed to parse Google response")?;

        let text = api_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .ok_or_else(|| anyhow::anyhow!("No content in Google response"))?
            .text
            .clone();

        let analysis: GrokCodeAnalysis =
            serde_json::from_str(&text).context("Failed to parse code analysis from Google")?;

        Ok(analysis)
    }

    /// Extract text content from GPT-5 output array
    /// Handles mixed JSON responses where schema and data are returned together
    fn extract_text_from_output_array(output: &[serde_json::Value]) -> String {
        for output_item in output.iter() {
            // Check status first - skip incomplete reasoning entries but accept incomplete content
            if let Some(item_type) = output_item.get("type").and_then(|v| v.as_str()) {
                if item_type == "reasoning" {
                    // Skip reasoning entries as they don't contain actual content
                    continue;
                }
            }

            // Check if this is an incomplete message - we'll accept incomplete content for now
            if let Some(status) = output_item.get("status").and_then(|v| v.as_str()) {
                if status == "incomplete" {
                    tracing::warn!("GPT-5 returned incomplete response, extracting partial content");
                }
            }

            // Handle GPT-5 message structure: output_item.content[].text
            if let Some(content_array) = output_item.get("content").and_then(|v| v.as_array()) {
                for content_entry in content_array {
                    if let Some(text_content) = content_entry.get("text").and_then(|v| v.as_str()) {
                        let trimmed_text = text_content.trim();
                        if !trimmed_text.is_empty() {
                            eprintln!("Extracted {} characters from GPT-5 content", trimmed_text.len());

                            // Handle mixed JSON response (schema + data)
                            // GPT-5 sometimes returns: {schema},\n{data}
                            // We need to extract only the data part
                            if trimmed_text.contains("},\n{") || trimmed_text.contains("},\\n{") {
                                eprintln!("Detected mixed JSON response, extracting data section");

                                // Try to find and parse individual JSON objects
                                let mut last_valid_json = String::new();
                                let mut brace_count = 0;
                                let mut current_json = String::new();
                                let mut in_string = false;
                                let mut escape_next = false;

                                for ch in trimmed_text.chars() {
                                    if escape_next {
                                        current_json.push(ch);
                                        escape_next = false;
                                        continue;
                                    }

                                    if ch == '\\' && in_string {
                                        escape_next = true;
                                        current_json.push(ch);
                                        continue;
                                    }

                                    if ch == '"' && !escape_next {
                                        in_string = !in_string;
                                    }

                                    if !in_string {
                                        if ch == '{' {
                                            if brace_count == 0 && !current_json.is_empty() {
                                                // Try to parse completed JSON
                                                if let Ok(parsed) =
                                                    serde_json::from_str::<serde_json::Value>(&current_json)
                                                {
                                                    // Check if this is data (has active_context) not schema
                                                    if parsed.get("active_context").is_some()
                                                        || parsed.get("technical_state").is_some()
                                                        || parsed.get("key_insights").is_some()
                                                    {
                                                        last_valid_json = current_json.clone();
                                                        tracing::debug!("Found valid data JSON object");
                                                    }
                                                }
                                                current_json.clear();
                                            }
                                            brace_count += 1;
                                        } else if ch == '}' {
                                            brace_count -= 1;
                                        }
                                    }

                                    current_json.push(ch);

                                    if brace_count == 0 && current_json.trim().ends_with('}') {
                                        // Try to parse completed JSON
                                        if let Ok(parsed) =
                                            serde_json::from_str::<serde_json::Value>(&current_json)
                                        {
                                            // Check if this is data (has active_context) not schema
                                            if parsed.get("active_context").is_some()
                                                || parsed.get("technical_state").is_some()
                                                || parsed.get("key_insights").is_some()
                                            {
                                                last_valid_json = current_json.clone();
                                                tracing::debug!("Found valid data JSON object at end");
                                            }
                                        }
                                    }
                                }

                                if !last_valid_json.is_empty() {
                                    tracing::debug!(chars=%last_valid_json.len(), "Returning extracted data JSON");
                                    return last_valid_json;
                                }
                            }

                            // If no mixed JSON detected or extraction failed, return as-is
                            return trimmed_text.to_string();
                        }
                    }
                }
            } else if let Some(direct_content) = output_item.get("content").and_then(|v| v.as_str()) {
                // Fallback for direct string content
                let trimmed_content = direct_content.trim();
                if !trimmed_content.is_empty() {
                    tracing::debug!(chars=%trimmed_content.len(), "Extracted GPT-5 direct content chars");
                    return trimmed_content.to_string();
                }
            }
        }
        String::new() // Return empty string if no content found
    }

    /// Optimize memory using GPT-5 with Responses API
    pub async fn optimize_memory_gpt5(
        &self,
        context: &str,
        prompt: &str,
        model: &str,
    ) -> Result<serde_json::Value> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::OpenAI);
        if api_key.is_empty() {
            return Err(anyhow::anyhow!("OpenAI API key not configured"));
        }

        // Validate API key format for OpenAI (should start with 'sk-')
        if !api_key.starts_with("sk-") {
            return Err(anyhow::anyhow!(
                "Invalid OpenAI API key format. Key should start with 'sk-'"
            ));
        }

        let base_url = self.config.get_base_url_for_provider(&AIProvider::OpenAI);

        // GPT-5 Responses API - request JSON format in prompt
        let json_schema_str = serde_json::to_string_pretty(&self.get_memory_optimization_schema())?;
        let request_body = serde_json::json!({
            "model": model,
            "input": format!("{}\n\n{}\n\nIMPORTANT: Return ONLY the data object that matches this schema. Do NOT include the schema itself in your response. Return a single JSON object starting with {{ and ending with }}.\n\nRequired schema for reference:\n{}",
                prompt, context, json_schema_str),
            "max_output_tokens": 12000,
            "reasoning": {
                "effort": "medium"
            }
        });

        // Debug logging for GPT-5 troubleshooting (only in debug mode)
        if debug_hooks_enabled() {
            tracing::debug!(model=%model, "GPT-5 Debug: model");
        }

        // Implement retry logic for transient failures
        let mut retries = 3;
        let mut last_error = None;

        while retries > 0 {
            // Construct proper URL - build directly since base_url already includes /v1
            let responses_url_string = format!("{}/responses", base_url.trim_end_matches('/'));
            let responses_url = Url::parse(&responses_url_string)
                .with_context(|| format!("Failed to parse responses URL: {responses_url_string}"))?;

            tracing::debug!(url=%responses_url, "GPT-5 Debug: requesting URL");

            let response = self
                .client
                .post(responses_url)
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .header("User-Agent", "rust-validation-hooks/0.1.0")
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        #[derive(Deserialize)]
                        struct Gpt5Response {
                            output_text: Option<String>,
                            output: Option<Vec<serde_json::Value>>,
                        }

                        match resp.json::<Gpt5Response>().await {
                            Ok(gpt5_response) => {
                                tracing::debug!("GPT-5 response received, extracting content");

                                // Try output_text first, then extract from output array
                                let text_content = if let Some(output_text) = gpt5_response.output_text {
                                    tracing::debug!("Using output_text field");
                                    output_text
                                } else if let Some(output) = gpt5_response.output {
                                    tracing::debug!(items=%output.len(), "Extracting from output array");
                                    Self::extract_text_from_output_array(&output)
                                } else {
                                    return Err(anyhow::anyhow!("No output content found in response"));
                                };

                                // Check if content is empty
                                if text_content.trim().is_empty() {
                                    last_error = Some(anyhow::anyhow!("Response content is empty"));
                                    tracing::warn!("GPT-5 response content is empty");
                                } else {
                                    tracing::debug!(len=%text_content.len(), "Parsing content");
                                    // GPT-5 with JSON Schema should return valid JSON
                                    match serde_json::from_str::<serde_json::Value>(&text_content) {
                                        Ok(optimization) => {
                                            tracing::debug!("GPT-5 returned valid JSON structure");
                                            return Ok(optimization);
                                        }
                                        Err(e) => {
                                            tracing::warn!(error=%e, "Failed to parse JSON response from GPT-5");
                                            tracing::debug!(content=%text_content, "Response content");
                                            last_error =
                                                Some(anyhow::anyhow!("GPT-5 JSON parsing failed: {}", e));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                last_error = Some(anyhow::anyhow!("Failed to parse response: {}", e));
                            }
                        }
                    } else {
                        let status = resp.status();
                        // Try to get error details for debugging
                        let error_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unable to read error".to_string());
                        // Log first 500 chars of error for debugging
                        let error_preview: String = error_text.chars().take(500).collect();
                        eprintln!(
                            "GPT-5 API error (attempt {}): Status {} - {}",
                            4 - retries,
                            status,
                            error_preview
                        );
                        last_error = Some(anyhow::anyhow!(
                            "API request failed with status {}. {}",
                            status,
                            if error_preview.contains("context_length_exceeded") {
                                "Context too long"
                            } else if error_preview.contains("invalid_schema") {
                                "Invalid JSON schema"
                            } else {
                                "Memory not optimized"
                            }
                        ));
                    }
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!("Network error: {}", e));
                    tracing::warn!(attempt=%(4 - retries), error=%e, "Network error");
                }
            }

            retries -= 1;
            if retries > 0 {
                // Wait before retrying (exponential backoff with jitter)
                let base_delay_secs = 2_u64.pow(3 - retries as u32);
                let jitter = rand::random::<f64>() * 0.5 + 0.75; // 0.75 to 1.25 multiplier
                let delay_ms = (base_delay_secs as f64 * 1000.0 * jitter) as u64;
                let delay_with_jitter = delay_ms.clamp(100, 300_000); // Min 100ms, Max 5 minutes
                tokio::time::sleep(Duration::from_secs_f64(delay_with_jitter as f64 / 1000.0)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to call GPT-5 after 3 attempts")))
    }

    // Removed deprecated text heuristics (detect_code_content/detect_error_content)

    /// Get the JSON schema for memory optimization
    fn get_memory_optimization_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["active_context", "technical_state", "user_patterns", "key_insights", "solutions_archive", "total_tokens", "reduction_ratio"],
            "additionalProperties": false,
            "properties": {
                "active_context": {
                    "type": "object",
                    "required": ["current_task", "last_action", "next_steps", "status"],
                    "properties": {
                        "current_task": {
                            "type": "string",
                            "description": "What the user is working on right now"
                        },
                        "working_files": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Files currently being modified"
                        },
                        "last_action": {
                            "type": "string",
                            "description": "Last thing that was done"
                        },
                        "next_steps": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "What needs to happen next"
                        },
                        "status": {
                            "type": "string",
                            "enum": ["in_progress", "blocked", "ready_for_testing", "completed"],
                            "description": "Current status of work"
                        }
                    }
                },
                "technical_state": {
                    "type": "array",
                    "description": "Specific technical details that must be preserved",
                    "items": {
                        "type": "object",
                        "required": ["type", "content"],
                        "properties": {
                            "type": {
                                "type": "string",
                                "enum": ["file_path", "function_name", "command", "error", "config", "code_snippet"]
                            },
                            "content": {"type": "string"},
                            "location": {"type": "string"},
                            "status": {"type": "string", "enum": ["working", "broken", "needs_testing"]}
                        }
                    }
                },
                "user_patterns": {
                    "type": "object",
                    "description": "How this user prefers to work",
                    "properties": {
                        "detail_level": {
                            "type": "string",
                            "enum": ["brief", "moderate", "detailed"],
                            "description": "Preferred level of explanation detail"
                        },
                        "common_requests": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Types of tasks user frequently requests"
                        },
                        "problem_solving_style": {
                            "type": "string",
                            "description": "How user likes problems approached"
                        }
                    }
                },
                "key_insights": {
                    "type": "array",
                    "description": "Critical discoveries and learnings from the conversation",
                    "items": {
                        "type": "string",
                        "description": "A specific insight or discovery made during the conversation"
                    }
                },
                "solutions_archive": {
                    "type": "array",
                    "description": "Completed solutions with full context",
                    "items": {
                        "type": "object",
                        "required": ["problem", "solution", "verification"],
                        "properties": {
                            "problem": {"type": "string"},
                            "solution": {"type": "string"},
                            "files_changed": {
                                "type": "array",
                                "items": {"type": "string"}
                            },
                            "verification": {"type": "string"},
                            "why_this_approach": {"type": "string"}
                        }
                    }
                },
                "ai_error_patterns": {
                    "type": "array",
                    "description": "AI error patterns for learning and improvement",
                    "items": {
                        "type": "object",
                        "required": ["error_type", "pattern", "frequency", "guidance"],
                        "properties": {
                            "error_type": {"type": "string"},
                            "pattern": {"type": "string"},
                            "frequency": {"type": "integer", "minimum": 1},
                            "last_seen": {"type": "string"},
                            "context": {"type": "string"},
                            "guidance": {"type": "string"}
                        }
                    }
                },
                "learning_insights": {
                    "type": "array",
                    "description": "Learning insights for better AI guidance",
                    "items": {
                        "type": "object",
                        "required": ["category", "insight", "confidence", "source", "timestamp"],
                        "properties": {
                            "category": {"type": "string"},
                            "insight": {"type": "string"},
                            "confidence": {"type": "number", "minimum": 0.0, "maximum": 1.0},
                            "source": {"type": "string"},
                            "timestamp": {"type": "string"}
                        }
                    }
                },
                "documentation_refs": {
                    "type": "array",
                    "description": "Relevant documentation references",
                    "items": {
                        "type": "object",
                        "required": ["file_path", "section", "summary", "relevance"],
                        "properties": {
                            "file_path": {"type": "string"},
                            "section": {"type": "string"},
                            "summary": {"type": "string"},
                            "relevance": {"type": "number", "minimum": 0.0, "maximum": 1.0}
                        }
                    }
                },
                "total_tokens": {
                    "type": "integer",
                    "minimum": 0
                },
                "reduction_ratio": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0
                }
            }
        })
    }
}
#[inline]
fn debug_hooks_enabled() -> bool {
    #[cfg(debug_assertions)]
    {
        std::env::var("DEBUG_HOOKS").unwrap_or_default() == "true"
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}





