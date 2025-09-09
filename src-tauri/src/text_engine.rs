// Text processing engine for Tauri application
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TextAnalysis {
    pub word_count: usize,
    pub char_count: usize,
    pub line_count: usize,
    pub language_detected: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextProcessor {
    pub filters: Vec<String>,
    pub options: HashMap<String, String>,
}

impl TextProcessor {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            options: HashMap::new(),
        }
    }

    pub fn analyze_text(&self, content: &str) -> TextAnalysis {
        let word_count = content.split_whitespace().count();
        let char_count = content.chars().count();
        let line_count = content.lines().count();
        let language_detected = self.detect_language(content);

        TextAnalysis {
            word_count,
            char_count,
            line_count,
            language_detected,
        }
    }

    fn detect_language(&self, content: &str) -> String {
        // Simple language detection based on common patterns
        if content.contains("fn ") && content.contains("->") {
            "rust".to_string()
        } else if content.contains("function") || content.contains("const ") {
            "javascript".to_string()
        } else if content.contains("def ") || content.contains("import ") {
            "python".to_string()
        } else {
            "text".to_string()
        }
    }

    pub fn process_text(&self, content: &str) -> Result<String, String> {
        if content.trim().is_empty() {
            return Err("Empty content provided".to_string());
        }

        let mut processed = content.to_string();
        
        // Apply filters
        for filter in &self.filters {
            match filter.as_str() {
                "trim" => processed = processed.trim().to_string(),
                "lowercase" => processed = processed.to_lowercase(),
                "uppercase" => processed = processed.to_uppercase(),
                _ => continue,
            }
        }

        Ok(processed)
    }
}

impl Default for TextProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_analysis() {
        let processor = TextProcessor::new();
        let analysis = processor.analyze_text("Hello world\nThis is a test");
        
        assert_eq!(analysis.word_count, 6);
        assert_eq!(analysis.line_count, 2);
        assert_eq!(analysis.language_detected, "text");
    }

    #[test]
    fn test_rust_detection() {
        let processor = TextProcessor::new();
        let rust_code = "fn main() -> i32 { 42 }";
        let analysis = processor.analyze_text(rust_code);
        
        assert_eq!(analysis.language_detected, "rust");
    }

    #[test]
    fn test_text_processing() {
        let mut processor = TextProcessor::new();
        processor.filters.push("trim".to_string());
        processor.filters.push("lowercase".to_string());
        
        let result = processor.process_text("  HELLO WORLD  ");
        assert_eq!(result.unwrap(), "hello world");
    }
}