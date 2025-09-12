use super::super::{CodeFormatter, FormatResult};
use crate::analysis::ast::SupportedLanguage;
use anyhow::Result;
use regex::Regex;

/// TOML formatter implementation using custom parsing and formatting logic
pub struct TomlFormatter;

impl TomlFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Format TOML content with consistent structure and spacing
    fn format_toml_content(&self, content: &str) -> Result<String> {
        // Validate basic TOML structure first
        self.validate_toml_syntax(content)?;

        let mut sections = self.parse_toml_sections(content)?;
        let formatted = self.format_toml_sections(&mut sections)?;

        Ok(formatted)
    }

    /// Parse TOML content into logical sections for formatting
    fn parse_toml_sections(&self, content: &str) -> Result<Vec<TomlSection>> {
        let mut sections = Vec::new();
        let mut current_section = TomlSection::new("".to_string());

        let lines: Vec<&str> = content.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            // Empty lines
            if trimmed.is_empty() {
                current_section.add_line(TomlLine::Empty);
                continue;
            }

            // Comments
            if trimmed.starts_with('#') {
                let comment = self.clean_comment(trimmed);
                current_section.add_line(TomlLine::Comment(comment));
                continue;
            }

            // Section headers [section] or [[array_section]]
            if trimmed.starts_with('[') {
                // Save current section if it has content
                if !current_section.is_empty() {
                    sections.push(current_section);
                }

                let section_name = self.extract_section_name(trimmed)?;
                current_section = TomlSection::new(section_name);
                current_section.add_line(TomlLine::SectionHeader(trimmed.to_string()));
                continue;
            }

            // Key-value pairs
            if trimmed.contains('=') {
                let kv = self.parse_key_value(trimmed)?;
                current_section.add_line(TomlLine::KeyValue(kv));
                continue;
            }

            // Other content (should be rare in valid TOML)
            current_section.add_line(TomlLine::Other(trimmed.to_string()));
        }

        // Add final section
        if !current_section.is_empty() {
            sections.push(current_section);
        }

        Ok(sections)
    }

    /// Format parsed TOML sections into a well-formatted string
    fn format_toml_sections(&self, sections: &mut [TomlSection]) -> Result<String> {
        let mut formatted_parts = Vec::new();

        // Sort sections: root section first, then alphabetically by name
        sections.sort_by(|a, b| {
            if a.name.is_empty() && !b.name.is_empty() {
                std::cmp::Ordering::Less
            } else if !a.name.is_empty() && b.name.is_empty() {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        for (i, section) in sections.iter().enumerate() {
            let formatted_section = section.format()?;

            // Add separator between sections (but not before first)
            if i > 0 && !formatted_section.trim().is_empty() {
                formatted_parts.push("\n".to_string());
            }

            formatted_parts.push(formatted_section);
        }

        let result = formatted_parts.join("");
        Ok(format!("{}\n", result.trim_end()))
    }

    /// Extract section name from section header
    fn extract_section_name(&self, header: &str) -> Result<String> {
        let inner = header.trim_start_matches('[').trim_end_matches(']');

        if inner.is_empty() {
            return Err(anyhow::anyhow!("Empty section name in header: {}", header));
        }

        // Handle array tables [[name]]
        let section_name = if header.starts_with("[[") && header.ends_with("]]") {
            inner.trim_start_matches('[').trim_end_matches(']')
        } else {
            inner
        };

        Ok(section_name.to_string())
    }

    /// Parse a key-value line
    fn parse_key_value(&self, line: &str) -> Result<TomlKeyValue> {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid key-value format: {}", line));
        }

        let key = parts[0].trim().to_string();
        let value = parts[1].trim().to_string();

        if key.is_empty() {
            return Err(anyhow::anyhow!("Empty key in line: {}", line));
        }

        Ok(TomlKeyValue { key, value })
    }

    /// Clean up comment formatting
    fn clean_comment(&self, comment: &str) -> String {
        let text = comment.trim_start_matches('#').trim_start();
        if text.is_empty() {
            "#".to_string()
        } else {
            format!("# {}", text)
        }
    }

    /// Basic TOML syntax validation
    fn validate_toml_syntax(&self, content: &str) -> Result<()> {
        let lines: Vec<&str> = content.lines().collect();
        let _bracket_count = 0;
        let _in_multiline_string = false;
        let _quote_char = '\0';

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check for section headers
            if trimmed.starts_with('[') {
                if !trimmed.ends_with(']') {
                    return Err(anyhow::anyhow!(
                        "TOML syntax error at line {}: Unclosed section header",
                        line_num + 1
                    ));
                }

                // Validate section name
                let inner = trimmed.trim_start_matches('[').trim_end_matches(']');
                if inner.trim_start_matches('[').trim_end_matches(']').is_empty() {
                    return Err(anyhow::anyhow!(
                        "TOML syntax error at line {}: Empty section name",
                        line_num + 1
                    ));
                }
                continue;
            }

            // Check key-value pairs
            if trimmed.contains('=') {
                let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
                if parts.len() != 2 {
                    return Err(anyhow::anyhow!(
                        "TOML syntax error at line {}: Invalid key-value format",
                        line_num + 1
                    ));
                }

                let key = parts[0].trim();
                if key.is_empty() {
                    return Err(anyhow::anyhow!(
                        "TOML syntax error at line {}: Empty key",
                        line_num + 1
                    ));
                }

                // Basic value validation
                let value = parts[1].trim();
                if value.is_empty() {
                    return Err(anyhow::anyhow!(
                        "TOML syntax error at line {}: Empty value for key '{}'",
                        line_num + 1,
                        key
                    ));
                }
                continue;
            }

            // If we reach here, it's likely invalid TOML
            return Err(anyhow::anyhow!(
                "TOML syntax error at line {}: Unexpected content '{}'",
                line_num + 1,
                trimmed
            ));
        }

        Ok(())
    }

    /// Check for common TOML security and quality issues
    fn check_toml_quality(&self, content: &str) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check for potential security issues
        let security_patterns = [
            (
                r#"(?i)password\s*=\s*["'][^"']+["']"#,
                "Hardcoded password detected",
            ),
            (
                r#"(?i)api[_-]?key\s*=\s*["'][^"']+["']"#,
                "Hardcoded API key detected",
            ),
            (r#"(?i)secret\s*=\s*["'][^"']+["']"#, "Hardcoded secret detected"),
            (r#"(?i)token\s*=\s*["'][^"']+["']"#, "Hardcoded token detected"),
        ];

        for (pattern, warning) in security_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(content) {
                    warnings.push(warning.to_string());
                }
            }
        }

        // Check for common quality issues
        if content.lines().any(|line| line.contains('\t')) {
            warnings.push("Consider using spaces instead of tabs for consistency".to_string());
        }

        // Check for very long lines
        for (line_num, line) in content.lines().enumerate() {
            if line.len() > 120 {
                warnings.push(format!(
                    "Line {} is very long ({} chars) - consider breaking it up",
                    line_num + 1,
                    line.len()
                ));
            }
        }

        warnings
    }
}

impl CodeFormatter for TomlFormatter {
    fn format_code(&self, content: &str) -> Result<FormatResult> {
        let mut messages = Vec::new();

        // Check for quality issues first
        messages.extend(
            self.check_toml_quality(content)
                .into_iter()
                .map(|w| format!("TOML quality warning: {}", w)),
        );

        // Validate syntax
        if let Err(e) = self.validate_toml_syntax(content) {
            messages.push(format!("TOML syntax error: {}", e));
            return Ok(FormatResult {
                original: content.to_string(),
                formatted: content.to_string(),
                changed: false,
                messages,
            });
        }

        // Format the TOML
        match self.format_toml_content(content) {
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
                messages.push(format!("TOML formatting error: {}", e));
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
        // TOML formatter is always available since it uses built-in regex
        true
    }

    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::Toml
    }

    fn formatter_info(&self) -> String {
        "Built-in TOML formatter with security checking".to_string()
    }
}

impl Default for TomlFormatter {
    fn default() -> Self {
        Self::new()
    }
}

// Helper structs for TOML parsing

#[derive(Debug, Clone)]
struct TomlSection {
    name: String,
    lines: Vec<TomlLine>,
}

#[derive(Debug, Clone)]
enum TomlLine {
    Empty,
    Comment(String),
    SectionHeader(String),
    KeyValue(TomlKeyValue),
    Other(String),
}

#[derive(Debug, Clone)]
struct TomlKeyValue {
    key: String,
    value: String,
}

impl TomlSection {
    fn new(name: String) -> Self {
        Self {
            name,
            lines: Vec::new(),
        }
    }

    fn add_line(&mut self, line: TomlLine) {
        self.lines.push(line);
    }

    fn is_empty(&self) -> bool {
        self.lines.is_empty() || self.lines.iter().all(|line| matches!(line, TomlLine::Empty))
    }

    fn format(&self) -> Result<String> {
        let mut result = Vec::new();
        let mut key_values = Vec::new();

        // First pass: collect key-value pairs for sorting
        for line in &self.lines {
            match line {
                TomlLine::KeyValue(kv) => {
                    key_values.push(kv.clone());
                }
                TomlLine::Empty => result.push("".to_string()),
                TomlLine::Comment(comment) => result.push(comment.clone()),
                TomlLine::SectionHeader(header) => result.push(header.clone()),
                TomlLine::Other(other) => result.push(other.clone()),
            }
        }

        // Sort key-value pairs alphabetically by key
        key_values.sort_by(|a, b| a.key.cmp(&b.key));

        // Add sorted key-value pairs
        for kv in key_values {
            result.push(format!("{} = {}", kv.key, kv.value));
        }

        Ok(result.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_basic_toml() {
        let formatter = TomlFormatter::new();
        let input = "name=\"test\"\nage=30\nactive=true";

        let result = formatter.format_code(input).unwrap();
        assert!(result.changed);
        assert!(result.messages.is_empty());

        let formatted = result.formatted;
        assert!(formatted.contains("active = true"));
        assert!(formatted.contains("age = 30"));
        assert!(formatted.contains("name = \"test\""));
    }

    #[test]
    fn test_format_with_sections() {
        let formatter = TomlFormatter::new();
        let input = "[section1]\nkey1=\"value1\"\n[section2]\nkey2=\"value2\"";

        let result = formatter.format_code(input).unwrap();
        assert!(result.messages.is_empty());

        let formatted = result.formatted;
        assert!(formatted.contains("[section1]"));
        assert!(formatted.contains("[section2]"));
        assert!(formatted.contains("key1 = \"value1\""));
        assert!(formatted.contains("key2 = \"value2\""));
    }

    #[test]
    fn test_format_with_comments() {
        let formatter = TomlFormatter::new();
        let input = "# This is a comment\nname=\"test\"\n#Another comment\nage=30";

        let result = formatter.format_code(input).unwrap();
        assert!(result.messages.is_empty());

        let formatted = result.formatted;
        assert!(formatted.contains("# This is a comment"));
        assert!(formatted.contains("# Another comment"));
    }

    #[test]
    fn test_validate_toml_syntax_errors() {
        let formatter = TomlFormatter::new();

        // Test unclosed section
        let input_bad_section = "[section";
        let result = formatter.format_code(input_bad_section).unwrap();
        assert!(!result.changed);
        assert!(!result.messages.is_empty());
        assert!(result.messages[0].contains("Unclosed section"));

        // Test empty key
        let input_empty_key = "=value";
        let result = formatter.format_code(input_empty_key).unwrap();
        assert!(!result.changed);
        assert!(!result.messages.is_empty());
        assert!(result.messages[0].contains("Empty key"));
    }

    #[test]
    fn test_security_warnings() {
        let formatter = TomlFormatter::new();
        let input = "api_key=\"secret123\"";

        let result = formatter.format_code(input).unwrap();
        assert!(!result.messages.is_empty());
        assert!(result.messages.iter().any(|msg| msg.contains("API key")));
    }

    #[test]
    fn test_is_available() {
        let formatter = TomlFormatter::new();
        assert!(formatter.is_available());
    }

    #[test]
    fn test_supports_language() {
        let formatter = TomlFormatter::new();
        assert_eq!(formatter.language(), SupportedLanguage::Toml);
    }

    #[test]
    fn test_empty_toml() {
        let formatter = TomlFormatter::new();
        let input = "";

        let result = formatter.format_code(input).unwrap();
        assert!(result.messages.is_empty());
        assert_eq!(result.formatted.trim(), "");
    }

    #[test]
    fn test_array_section() {
        let formatter = TomlFormatter::new();
        let input = "[[products]]\nname=\"Hammer\"\n[[products]]\nname=\"Nail\"";

        let result = formatter.format_code(input).unwrap();
        assert!(result.messages.is_empty());

        let formatted = result.formatted;
        assert!(formatted.contains("[[products]]"));
        assert!(formatted.contains("name = \"Hammer\""));
        assert!(formatted.contains("name = \"Nail\""));
    }

    #[test]
    fn test_key_sorting() {
        let formatter = TomlFormatter::new();
        let input = "zebra=\"z\"\nalpha=\"a\"\nbeta=\"b\"";

        let result = formatter.format_code(input).unwrap();
        let formatted = result.formatted;

        // Keys should be sorted alphabetically
        let alpha_pos = formatted.find("alpha").unwrap();
        let beta_pos = formatted.find("beta").unwrap();
        let zebra_pos = formatted.find("zebra").unwrap();

        assert!(alpha_pos < beta_pos);
        assert!(beta_pos < zebra_pos);
    }
}
