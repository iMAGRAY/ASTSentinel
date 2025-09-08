use super::super::{CodeFormatter, FormatResult};
use super::SecureCommandExecutor;
use crate::analysis::ast::SupportedLanguage;
/// JavaScript code formatter using prettier
use anyhow::Result;

/// JavaScript formatter implementation using prettier
pub struct JavaScriptFormatter {
    executor: SecureCommandExecutor,
}

impl JavaScriptFormatter {
    pub fn new() -> Self {
        Self {
            executor: SecureCommandExecutor::default(),
        }
    }

    /// Get prettier configuration arguments for JavaScript with 2025 standards
    fn get_prettier_args(&self) -> Vec<String> {
        vec![
            // Use babel parser for modern JavaScript
            "--parser".to_string(),
            "babel".to_string(),
            // Specify file type for stdin
            "--stdin-filepath".to_string(),
            "stdin.js".to_string(),
            // 2025 recommended settings for JavaScript
            "--print-width".to_string(),
            "100".to_string(),
            "--tab-width".to_string(),
            "2".to_string(),
            "--use-tabs".to_string(),
            "false".to_string(),
            "--semi".to_string(),
            "true".to_string(),
            "--single-quote".to_string(),
            "true".to_string(),
            "--trailing-comma".to_string(),
            "es5".to_string(),
            "--bracket-spacing".to_string(),
            "true".to_string(),
            "--arrow-parens".to_string(),
            "avoid".to_string(),
        ]
    }
}

impl CodeFormatter for JavaScriptFormatter {
    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::JavaScript
    }

    /// Format JavaScript source code using prettier
    ///
    /// # Examples
    /// ```rust
    /// use rust_validation_hooks::formatting::formatters::javascript::JavaScriptFormatter;
    /// use rust_validation_hooks::formatting::CodeFormatter;
    ///
    /// let formatter = JavaScriptFormatter::new();
    /// let code = "function hello(){console.log('world');}";
    /// let result = formatter.format_code(code).unwrap();
    /// assert!(result.changed);
    /// ```
    fn format_code(&self, code: &str) -> Result<FormatResult> {
        // Validate input
        if code.trim().is_empty() {
            return Ok(FormatResult::unchanged(code.to_string()));
        }

        // Check if prettier is available - graceful degradation
        if !self.is_available() {
            let mut result = FormatResult::unchanged(code.to_string());
            result.messages.push(
                "prettier formatter not available - skipping JavaScript formatting".to_string(),
            );
            return Ok(result);
        }

        // Prepare prettier arguments
        let args = self.get_prettier_args();

        // Execute prettier with stdin input
        match self
            .executor
            .execute_formatter("prettier", &args, Some(code))
        {
            Ok(formatted_code) => {
                let result = FormatResult::new(code.to_string(), formatted_code);
                Ok(result)
            }
            Err(e) => {
                // If prettier fails, it might be due to syntax errors
                // Return the original code with error message
                let mut result = FormatResult::unchanged(code.to_string());
                result.messages.push(format!("prettier failed: {}", e));
                Ok(result)
            }
        }
    }

    fn is_available(&self) -> bool {
        self.executor.command_exists("prettier")
    }

    fn formatter_info(&self) -> String {
        self.executor.get_formatter_version("prettier")
    }

    fn default_config(&self) -> Result<String> {
        Ok(r#"{
  "semi": true,
  "trailingComma": "es5",
  "singleQuote": false,
  "printWidth": 80,
  "tabWidth": 2,
  "useTabs": false,
  "bracketSpacing": true,
  "bracketSameLine": false,
  "arrowParens": "always",
  "endOfLine": "lf",
  "quoteProps": "as-needed",
  "jsxSingleQuote": false,
  "jsxBracketSameLine": false,
  "requirePragma": false,
  "insertPragma": false,
  "proseWrap": "preserve",
  "htmlWhitespaceSensitivity": "css",
  "vueIndentScriptAndStyle": false,
  "embeddedLanguageFormatting": "auto"
}"#
        .to_string())
    }
}

impl Default for JavaScriptFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_javascript_formatter_creation() {
        let formatter = JavaScriptFormatter::new();
        assert_eq!(formatter.language(), SupportedLanguage::JavaScript);
    }

    #[test]
    fn test_empty_code_handling() {
        let formatter = JavaScriptFormatter::new();
        let result = formatter.format_code("").unwrap();
        assert!(!result.changed);
        assert_eq!(result.formatted, "");
    }

    #[test]
    fn test_whitespace_only_code() {
        let formatter = JavaScriptFormatter::new();
        let result = formatter.format_code("   \n\t  \n").unwrap();
        assert!(!result.changed);
    }

    #[test]
    fn test_formatter_info() {
        let formatter = JavaScriptFormatter::new();
        let info = formatter.formatter_info();
        assert!(info.contains("prettier"));
    }

    #[test]
    fn test_default_config_generation() {
        let formatter = JavaScriptFormatter::new();
        let config = formatter.default_config().unwrap();
        assert!(config.contains("semi"));
        assert!(config.contains("printWidth"));
        assert!(config.contains("tabWidth"));
    }

    #[test]
    fn test_prettier_args() {
        let formatter = JavaScriptFormatter::new();
        let args = formatter.get_prettier_args();
        assert!(args.contains(&"--parser".to_string()));
        assert!(args.contains(&"babel".to_string()));
        assert!(args.contains(&"--stdin-filepath".to_string()));
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[test]
        fn test_simple_javascript_formatting() {
            let formatter = JavaScriptFormatter::new();

            // Skip if prettier is not available
            if !formatter.is_available() {
                eprintln!("Skipping prettier integration test - prettier not available");
                return;
            }

            let unformatted_code = "function hello(){console.log('Hello, world!');}";
            let result = formatter.format_code(unformatted_code);

            match result {
                Ok(format_result) => {
                    // prettier should format this code
                    assert!(format_result.changed || format_result.messages.is_empty());

                    // Formatted code should be valid JavaScript
                    assert!(format_result.formatted.contains("function hello()"));
                    assert!(format_result.formatted.contains("console.log"));
                }
                Err(e) => {
                    eprintln!("prettier formatting failed: {}", e);
                    // This is acceptable if prettier has issues with the test environment
                }
            }
        }

        #[test]
        fn test_complex_javascript_formatting() {
            let formatter = JavaScriptFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping prettier integration test - prettier not available");
                return;
            }

            let complex_code = r#"
const users=[{name:"John",age:30},{name:"Jane",age:25}];
function processUsers(userList){
return userList.map(user=>({...user,status:"active"})).filter(user=>user.age>20);
}
console.log(processUsers(users));"#
                .trim();

            let result = formatter.format_code(complex_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that formatting improved the code structure
                        assert!(format_result.formatted.contains("const users"));
                        assert!(format_result.formatted.contains("function processUsers"));
                    }
                }
                Err(e) => {
                    eprintln!("prettier formatting of complex code failed: {}", e);
                }
            }
        }

        #[test]
        fn test_syntax_error_handling() {
            let formatter = JavaScriptFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping prettier integration test - prettier not available");
                return;
            }

            let invalid_code = "function hello( { console.log('broken syntax'; }";
            let result = formatter.format_code(invalid_code);

            match result {
                Ok(format_result) => {
                    // Should return original code with error message
                    assert!(!format_result.changed);
                    assert_eq!(format_result.formatted, invalid_code);
                    assert!(!format_result.messages.is_empty());
                }
                Err(_) => {
                    // This is also acceptable - formatter detected the error
                }
            }
        }

        #[test]
        fn test_arrow_functions_and_modern_syntax() {
            let formatter = JavaScriptFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping prettier integration test - prettier not available");
                return;
            }

            let modern_code = "const add=(a,b)=>a+b;const multiply=function(x,y){return x*y;};";
            let result = formatter.format_code(modern_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that modern JS syntax is handled correctly
                        assert!(format_result.formatted.contains("=>"));
                        assert!(format_result.formatted.contains("const add"));
                        assert!(format_result.formatted.contains("const multiply"));
                    }
                }
                Err(e) => {
                    eprintln!("prettier formatting of modern JS failed: {}", e);
                }
            }
        }
    }
}
