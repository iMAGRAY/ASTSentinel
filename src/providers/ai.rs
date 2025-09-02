/// Multi-provider AI client implementation
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use serde_json;
use std::time::Duration;

use crate::{Config, SecurityValidation, GrokCodeAnalysis};

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
        write!(f, "{}", name)
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
            _ => Err(anyhow::anyhow!("Invalid provider: {}. Supported: openai, anthropic, google, xai", s)),
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
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;
            
        Ok(Self { client, config })
    }
    
    /// Validate security using the configured pretool provider
    pub async fn validate_security_pretool(
        &self,
        code: &str,
        prompt: &str,
    ) -> Result<SecurityValidation> {
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
    pub async fn analyze_code_posttool(
        &self,
        code: &str,
        prompt: &str,
    ) -> Result<GrokCodeAnalysis> {
        match self.config.posttool_provider {
            AIProvider::OpenAI => {
                // Check if it's GPT-5 (uses different API)
                if self.config.posttool_model.starts_with("gpt-5") {
                    self.analyze_with_gpt5(code, prompt).await
                } else {
                    self.analyze_with_openai(code, prompt).await
                }
            }
            AIProvider::Anthropic => self.analyze_with_anthropic(code, prompt).await,
            AIProvider::Google => self.analyze_with_google(code, prompt).await,
            AIProvider::XAI => self.analyze_with_xai(code, prompt).await,
        }
    }
    
    /// GPT-5 specific implementation (uses Responses API)
    async fn validate_with_gpt5(&self, code: &str, prompt: &str) -> Result<SecurityValidation> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::OpenAI);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::OpenAI);
        
        // GPT-5 uses the new Responses API format
        let request_body = serde_json::json!({
            "model": self.config.pretool_model,
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
                            "text": format!("Analyze this code for security risks:\n\n{}", code)
                        }
                    ]
                }
            ],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "SecurityValidation",
                    "schema": self.get_security_validation_schema()
                }
            },
            "max_output_tokens": 1024,
            "reasoning": {
                "effort": "medium"
            },
            "store": false
        });
        
        let response = self.client
            .post(format!("{}/responses", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
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
        
        let gpt5_response: Gpt5Response = response.json().await
            .context("Failed to parse GPT-5 response")?;
            
        let validation: SecurityValidation = serde_json::from_str(&gpt5_response.output_text)
            .context("Failed to parse security validation from GPT-5")?;
            
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
                    "content": format!("Analyze this code for security risks:\n\n{}", code)
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
        
        let response = self.client
            .post(format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
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
                    "content": format!("{}\n\nAnalyze this code for security risks:\n\n{}", prompt, code)
                }
            ],
            "max_tokens": 1024,
            "temperature": self.config.temperature,
            "system": prompt
        });
        
        let response = self.client
            .post(format!("{}/v1/messages", base_url))
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
                            "text": format!("{}\n\nAnalyze this code for security risks:\n\n{}", prompt, code)
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
        let response = self.client
            .post(format!("{}/models/{}:generateContent?key={}", base_url, model_name, api_key))
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
                    "content": format!("Analyze this code for security risks:\n\n{}", code)
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
        
        let response = self.client
            .post(format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to xAI")?;
            
        self.parse_openai_response(response).await  // xAI uses OpenAI-compatible format
    }
    
    /// Get the JSON schema for security validation
    fn get_security_validation_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["decision", "reason", "risk_level"],
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
                    "type": "array",
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
    
    /// Get the JSON schema for code analysis
    fn get_code_analysis_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["summary", "overall_quality", "issues", "suggestions"],
            "additionalProperties": false,
            "properties": {
                "summary": {
                    "type": "string",
                    "maxLength": 500
                },
                "overall_quality": {
                    "type": "string",
                    "enum": ["excellent", "good", "needs_improvement", "poor"]
                },
                "issues": {
                    "type": "array",
                    "maxItems": 20,
                    "items": {
                        "type": "object",
                        "required": ["severity", "category", "message"],
                        "properties": {
                            "severity": {
                                "type": "string",
                                "enum": ["info", "minor", "major", "critical", "blocker"]
                            },
                            "category": {
                                "type": "string",
                                "enum": ["intent", "correctness", "security", "robustness", "maintainability", "performance", "tests", "lint"]
                            },
                            "message": {
                                "type": "string",
                                "maxLength": 300
                            },
                            "line": {"type": "integer"},
                            "impact": {"type": "integer", "minimum": 1, "maximum": 3},
                            "fix_cost": {"type": "integer", "minimum": 1, "maximum": 3},
                            "confidence": {"type": "number", "minimum": 0.5, "maximum": 1.0},
                            "fix_suggestion": {"type": "string", "maxLength": 200}
                        }
                    }
                },
                "suggestions": {
                    "type": "array",
                    "maxItems": 10,
                    "items": {
                        "type": "object",
                        "required": ["category", "description", "priority"],
                        "properties": {
                            "category": {"type": "string"},
                            "description": {"type": "string", "maxLength": 300},
                            "priority": {
                                "type": "string",
                                "enum": ["high", "medium", "low"]
                            },
                            "priority_score": {"type": "number", "minimum": 0, "maximum": 100},
                            "code_example": {"type": "string"}
                        }
                    }
                },
                "metrics": {
                    "type": "object",
                    "properties": {
                        "complexity": {
                            "type": "string",
                            "enum": ["low", "medium", "high"]
                        },
                        "readability": {
                            "type": "string",
                            "enum": ["excellent", "good", "fair", "poor"]
                        },
                        "test_coverage": {
                            "type": "string",
                            "enum": ["none", "partial", "good", "excellent"]
                        }
                    }
                }
            }
        })
    }
    
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
        
        let api_response: OpenAIResponse = response.json().await
            .context("Failed to parse API response")?;
            
        let content = api_response.choices.first()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?
            .message.content.clone();
            
        let validation: SecurityValidation = serde_json::from_str(&content)
            .context("Failed to parse security validation")?;
            
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
        
        let api_response: AnthropicResponse = response.json().await
            .context("Failed to parse Anthropic response")?;
            
        let text = api_response.content.first()
            .ok_or_else(|| anyhow::anyhow!("No content in Anthropic response"))?
            .text.clone();
            
        let validation: SecurityValidation = serde_json::from_str(&text)
            .context("Failed to parse security validation from Anthropic")?;
            
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
        
        let api_response: GoogleResponse = response.json().await
            .context("Failed to parse Google response")?;
            
        let text = api_response.candidates.first()
            .and_then(|c| c.content.parts.first())
            .ok_or_else(|| anyhow::anyhow!("No content in Google response"))?
            .text.clone();
            
        let validation: SecurityValidation = serde_json::from_str(&text)
            .context("Failed to parse security validation from Google")?;
            
        Ok(validation)
    }
    
    // Code analysis methods for PostToolUse hook
    
    /// Analyze code with GPT-5 (uses Responses API)
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
                            "text": format!("Analyze this code and provide detailed review:\n\n{}", code)
                        }
                    ]
                }
            ],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "GrokCodeAnalysis",
                    "schema": self.get_code_analysis_schema()
                }
            },
            "max_output_tokens": 2048,
            "reasoning": {
                "effort": "high"
            },
            "store": false
        });
        
        let response = self.client
            .post(format!("{}/responses", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
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
        
        let gpt5_response: Gpt5Response = response.json().await
            .context("Failed to parse GPT-5 response")?;
            
        let analysis: GrokCodeAnalysis = serde_json::from_str(&gpt5_response.output_text)
            .context("Failed to parse code analysis from GPT-5")?;
            
        Ok(analysis)
    }
    
    /// Analyze code with standard OpenAI models
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
                    "content": format!("Analyze this code and provide detailed review:\n\n{}", code)
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
        
        let response = self.client
            .post(format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;
            
        self.parse_openai_analysis_response(response).await
    }
    
    /// Analyze code with Anthropic Claude
    async fn analyze_with_anthropic(&self, code: &str, prompt: &str) -> Result<GrokCodeAnalysis> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::Anthropic);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::Anthropic);
        
        let request_body = serde_json::json!({
            "model": self.config.posttool_model,
            "messages": [
                {
                    "role": "user",
                    "content": format!("{}\\n\\nAnalyze this code and provide detailed review:\\n\\n{}", prompt, code)
                }
            ],
            "max_tokens": 2048,
            "temperature": self.config.temperature,
            "system": prompt
        });
        
        let response = self.client
            .post(format!("{}/v1/messages", base_url))
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
    async fn analyze_with_google(&self, code: &str, prompt: &str) -> Result<GrokCodeAnalysis> {
        let api_key = self.config.get_api_key_for_provider(&AIProvider::Google);
        let base_url = self.config.get_base_url_for_provider(&AIProvider::Google);
        
        let request_body = serde_json::json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": format!("{}\\n\\nAnalyze this code and provide detailed review:\\n\\n{}", prompt, code)
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
        let response = self.client
            .post(format!("{}/models/{}:generateContent?key={}", base_url, model_name, api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Google")?;
            
        self.parse_google_analysis_response(response).await
    }
    
    /// Analyze code with xAI Grok
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
                    "content": format!("Analyze this code and provide detailed review:\\n\\n{}", code)
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
        
        let response = self.client
            .post(format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to xAI")?;
            
        self.parse_openai_analysis_response(response).await  // xAI uses OpenAI-compatible format
    }
    
    // Helper methods to parse analysis responses
    
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
        
        let api_response: OpenAIResponse = response.json().await
            .context("Failed to parse API response")?;
            
        let content = api_response.choices.first()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?
            .message.content.clone();
            
        let analysis: GrokCodeAnalysis = serde_json::from_str(&content)
            .context("Failed to parse code analysis")?;
            
        Ok(analysis)
    }
    
    async fn parse_anthropic_analysis_response(&self, response: reqwest::Response) -> Result<GrokCodeAnalysis> {
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
        
        let api_response: AnthropicResponse = response.json().await
            .context("Failed to parse Anthropic response")?;
            
        let text = api_response.content.first()
            .ok_or_else(|| anyhow::anyhow!("No content in Anthropic response"))?
            .text.clone();
            
        let analysis: GrokCodeAnalysis = serde_json::from_str(&text)
            .context("Failed to parse code analysis from Anthropic")?;
            
        Ok(analysis)
    }
    
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
        
        let api_response: GoogleResponse = response.json().await
            .context("Failed to parse Google response")?;
            
        let text = api_response.candidates.first()
            .and_then(|c| c.content.parts.first())
            .ok_or_else(|| anyhow::anyhow!("No content in Google response"))?
            .text.clone();
            
        let analysis: GrokCodeAnalysis = serde_json::from_str(&text)
            .context("Failed to parse code analysis from Google")?;
            
        Ok(analysis)
    }
}