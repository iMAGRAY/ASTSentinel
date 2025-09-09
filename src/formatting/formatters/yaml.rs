use super::super::{CodeFormatter, FormatResult};
use crate::analysis::ast::SupportedLanguage;
use anyhow::Result;
use regex::Regex;

/// YAML formatter implementation using custom logic
pub struct YamlFormatter;

impl YamlFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Format YAML content with consistent indentation and structure
    fn format_yaml_content(&self, content: &str) -> Result<String> {
        // Validate basic YAML structure first
        self.validate_yaml_syntax(content)?;

        let mut formatted_lines = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let indent_size = 2; // Standard YAML indent

        for line in lines {
            let trimmed = line.trim();

            // Skip empty lines (preserve them as-is)
            if trimmed.is_empty() {
                formatted_lines.push(String::new());
                continue;
            }

            if trimmed.starts_with('#') {
                // Preserve comments but ensure single space after #
                let comment_text = trimmed.trim_start_matches('#').trim_start();
                formatted_lines.push(format!("# {}", comment_text));
                continue;
            }

            // Calculate proper indentation based on original line structure
            let original_spaces = line.len() - line.trim_start().len();

            // Normalize indentation to proper multiples of indent_size
            let proper_indent = if original_spaces == 0 {
                0 // Top-level items
            } else {
                // Round to nearest indent_size multiple
                let calculated_level = original_spaces.div_ceil(indent_size);
                calculated_level * indent_size
            };

            // Format the line with proper spacing and normalized indentation
            let formatted_line = self.format_yaml_line(trimmed, proper_indent)?;
            formatted_lines.push(formatted_line);
        }

        // Join lines and ensure file ends with single newline
        let result = formatted_lines.join("\n");
        Ok(format!("{}\n", result.trim_end()))
    }

    /// Format a single YAML line with proper spacing and structure
    fn format_yaml_line(&self, line: &str, indent: usize) -> Result<String> {
        let prefix = " ".repeat(indent);

        // Handle list items (both "- item" and "-item")
        if line.starts_with("- ") {
            let content = line.trim_start_matches("- ").trim();
            return Ok(format!("{}- {}", prefix, content));
        } else if line.starts_with("-") && !line.starts_with("--") {
            let content = line.trim_start_matches('-').trim();
            return Ok(format!("{}- {}", prefix, content));
        }

        // Handle key-value pairs
        if line.contains(':') {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                let key = parts[0].trim();
                let value = parts[1].trim();

                if value.is_empty() {
                    // Key without value (parent key)
                    return Ok(format!("{}{}:", prefix, key));
                } else {
                    // Key with value - normalize spacing
                    return Ok(format!("{}{}: {}", prefix, key, value));
                }
            }
        }

        // Default: just add indentation
        Ok(format!("{}{}", prefix, line))
    }

    /// Basic YAML syntax validation
    fn validate_yaml_syntax(&self, content: &str) -> Result<()> {
        let lines: Vec<&str> = content.lines().collect();
        let mut brace_count = 0;
        let mut bracket_count = 0;

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Basic bracket/brace matching for inline YAML
            for ch in trimmed.chars() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    '[' => bracket_count += 1,
                    ']' => bracket_count -= 1,
                    _ => {}
                }
            }

            // Check for some common YAML syntax errors
            if trimmed.contains(":\t") {
                return Err(anyhow::anyhow!(
                    "YAML syntax error at line {}: Use spaces instead of tabs after colon",
                    line_num + 1
                ));
            }

            // Note: We don't validate list syntax here since the formatter can fix it
            // The formatter will convert "-item" to "- item" automatically
        }

        if brace_count != 0 {
            return Err(anyhow::anyhow!("YAML syntax error: Unmatched braces {{}}"));
        }

        if bracket_count != 0 {
            return Err(anyhow::anyhow!("YAML syntax error: Unmatched brackets []"));
        }

        Ok(())
    }

    /// Check for common YAML security issues
    fn check_yaml_security(&self, content: &str) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check for potentially dangerous YAML constructs
        let dangerous_patterns = [
            (
                r"!!python/",
                "Potentially dangerous Python object deserialization",
            ),
            (
                r"!!java/",
                "Potentially dangerous Java object deserialization",
            ),
            (
                r"&\w+",
                "YAML anchors detected - verify they're used safely",
            ),
            (
                r"\*\w+",
                "YAML aliases detected - verify they're used safely",
            ),
        ];

        for (pattern, warning) in dangerous_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(content) {
                    warnings.push(warning.to_string());
                }
            }
        }

        warnings
    }
}

impl CodeFormatter for YamlFormatter {
    fn format_code(&self, content: &str) -> Result<FormatResult> {
        let mut messages = Vec::new();

        // Check for security issues first
        let security_warnings = self.check_yaml_security(content);
        messages.extend(
            security_warnings
                .into_iter()
                .map(|w| format!("YAML security warning: {}", w)),
        );

        // Validate syntax
        if let Err(e) = self.validate_yaml_syntax(content) {
            messages.push(format!("YAML syntax error: {}", e));
            return Ok(FormatResult {
                original: content.to_string(),
                formatted: content.to_string(),
                changed: false,
                messages,
            });
        }

        // Format the YAML
        match self.format_yaml_content(content) {
            Ok(formatted) => {
                let changed = formatted.trim() != content.trim();

                Ok(FormatResult {
                    original: content.to_string(),
                    formatted,
                    changed,
                    messages,
                })
            }
            Err(e) => {
                messages.push(format!("YAML formatting error: {}", e));
                Ok(FormatResult {
                    original: content.to_string(),
                    formatted: content.to_string(),
                    changed: false,
                    messages,
                })
            }
        }
    }

    fn is_available(&self) -> bool {
        // YAML formatter is always available since it uses built-in regex
        true
    }

    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::Yaml
    }

    fn formatter_info(&self) -> String {
        "Built-in YAML formatter with security checking".to_string()
    }
}

impl Default for YamlFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_basic_yaml() {
        let formatter = YamlFormatter::new();
        let input = "name:   test\nage:30\nitems:\n  - a\n  -b\n  - c";

        let result = formatter.format_code(input).unwrap();
        assert!(result.changed);
        assert!(result.messages.is_empty());

        let formatted = result.formatted;
        assert!(formatted.contains("name: test"));
        assert!(formatted.contains("age: 30"));
        assert!(formatted.contains("  - a"));
        assert!(formatted.contains("  - b"));
    }

    #[test]
    fn test_format_with_comments() {
        let formatter = YamlFormatter::new();

        // Test with unformatted input that needs formatting
        let unformatted_input = "#comment\nname:test\nage:30";
        let result = formatter.format_code(unformatted_input).unwrap();

        assert!(result.changed);
        assert!(result.messages.is_empty());

        let formatted = result.formatted;
        assert!(formatted.contains("# comment"));
        assert!(formatted.contains("name: test"));
        assert!(formatted.contains("age: 30"));
    }

    #[test]
    fn test_validate_yaml_syntax_errors() {
        let formatter = YamlFormatter::new();

        // Test tab after colon
        let input_with_tabs = "name:\ttest";
        let result = formatter.format_code(input_with_tabs).unwrap();
        assert!(!result.changed);
        assert!(!result.messages.is_empty());
        assert!(result.messages[0].contains("tabs after colon"));

        // Test improper list syntax that formatter can fix
        let input_bad_list = "-item without space";
        let result = formatter.format_code(input_bad_list).unwrap();
        assert!(result.changed); // Formatter should fix this
        assert!(result.messages.is_empty()); // No errors, just fixed
        assert!(result.formatted.contains("- item without space")); // Should be fixed
    }

    #[test]
    fn test_security_warnings() {
        let formatter = YamlFormatter::new();
        let input = "data: !!python/object:some.class {}";

        let result = formatter.format_code(input).unwrap();
        assert!(!result.messages.is_empty());
        assert!(result
            .messages
            .iter()
            .any(|msg| msg.contains("dangerous Python object")));
    }

    #[test]
    fn test_is_available() {
        let formatter = YamlFormatter::new();
        assert!(formatter.is_available());
    }

    #[test]
    fn test_supports_language() {
        let formatter = YamlFormatter::new();
        assert_eq!(formatter.language(), SupportedLanguage::Yaml);
    }

    #[test]
    fn test_empty_yaml() {
        let formatter = YamlFormatter::new();
        let input = "";

        let result = formatter.format_code(input).unwrap();
        assert!(result.messages.is_empty());
        assert_eq!(result.formatted.trim(), "");
    }

    #[test]
    fn test_yaml_with_nested_structure() {
        let formatter = YamlFormatter::new();
        let input = "parent:\n  child1: value1\n  child2:\n    grandchild: value2";

        let result = formatter.format_code(input).unwrap();
        assert!(result.messages.is_empty());

        let formatted = result.formatted;
        assert!(formatted.contains("parent:"));
        assert!(formatted.contains("  child1: value1"));
        assert!(formatted.contains("  child2:"));
        assert!(formatted.contains("    grandchild: value2"));
    }
}
