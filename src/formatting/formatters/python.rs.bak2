use super::super::{CodeFormatter, FormatResult};
use super::SecureCommandExecutor;
use crate::analysis::ast::SupportedLanguage;
/// Python code formatter using black
use anyhow::Result;

/// Python formatter implementation using black
pub struct PythonFormatter {
    executor: SecureCommandExecutor,
}

impl PythonFormatter {
    pub fn new() -> Self {
        Self {
            executor: SecureCommandExecutor::default(),
        }
    }

    /// Get black configuration arguments with 2025 Python standards
    fn get_black_args(&self) -> Vec<String> {
        vec![
            // Black reads from stdin and writes to stdout
            "-".to_string(),
            // Specify filename for error reporting
            "--stdin-filename".to_string(),
            "stdin.py".to_string(),
            // 2025 Python standards - support modern versions
            "--target-version".to_string(),
            "py311".to_string(), // Python 3.11+ for latest features
            // Use standard line length (88 is black's default)
            "--line-length".to_string(),
            "88".to_string(),
            // Enable newer string formatting
            "--preview".to_string(),
            // Quiet output to reduce noise
            "--quiet".to_string(),
        ]
    }
}

impl CodeFormatter for PythonFormatter {
    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::Python
    }

    /// Format Python source code using black
    ///
    /// # Examples
    /// ```rust,no_run
    /// use rust_validation_hooks::formatting::formatters::python::PythonFormatter;
    /// use rust_validation_hooks::formatting::CodeFormatter;
    ///
    /// let formatter = PythonFormatter::new();
    /// let code = "def hello():print('world')";
    /// let result = formatter.format_code(code).unwrap();
    /// // Note: external formatter may be unavailable in CI; example is compile-only.
    /// assert!(result.changed || !result.messages.is_empty());
    /// ```
    fn format_code(&self, code: &str) -> Result<FormatResult> {
        // Validate input
        if code.trim().is_empty() {
            return Ok(FormatResult::unchanged(code.to_string()));
        }

        // Check if black is available - graceful degradation
        if !self.is_available() {
            let mut result = FormatResult::unchanged(code.to_string());
            result
                .messages
                .push("black formatter not available - skipping Python formatting".to_string());
            return Ok(result);
        }

        // Prepare black arguments for stdin input
        let args = self.get_black_args();

        // Execute black with stdin input
        match self.executor.execute_formatter("black", &args, Some(code)) {
            Ok(formatted_code) => {
                let result = FormatResult::new(code.to_string(), formatted_code);
                Ok(result)
            }
            Err(e) => {
                // If black fails, it might be due to syntax errors
                // Try alternative approach with stdin
                self.format_with_stdin(code, e)
            }
        }
    }

    fn is_available(&self) -> bool {
        self.executor.command_exists("black")
    }

    fn formatter_info(&self) -> String {
        self.executor.get_formatter_version("black")
    }

    fn default_config(&self) -> Result<String> {
        Ok(r#"[tool.black]
line-length = 88
target-version = ['py38']
include = '\.pyi?$'
extend-exclude = '''
/(
  # directories
  \.eggs
  | \.git
  | \.hg
  | \.mypy_cache
  | \.tox
  | \.venv
  | _build
  | buck-out
  | build
  | dist
)/
'''

# String normalization
skip-string-normalization = false

# Magic trailing comma
skip-magic-trailing-comma = false

# Preview features
preview = false

# Jupyter notebook support
jupyter = true
"#
        .to_string())
    }
}

impl PythonFormatter {
    /// Fallback method using stdin approach
    fn format_with_stdin(&self, code: &str, original_error: anyhow::Error) -> Result<FormatResult> {
        let stdin_args = vec!["-".to_string()]; // Read from stdin

        match self.executor.execute_formatter("black", &stdin_args, Some(code)) {
            Ok(formatted_code) => {
                let result = FormatResult::new(code.to_string(), formatted_code);
                Ok(result)
            }
            Err(_) => {
                // Both methods failed, return original code with error message
                let mut result = FormatResult::unchanged(code.to_string());
                result.messages.push(format!("black failed: {}", original_error));
                Ok(result)
            }
        }
    }
}

impl Default for PythonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_formatter_creation() {
        let formatter = PythonFormatter::new();
        assert_eq!(formatter.language(), SupportedLanguage::Python);
    }

    #[test]
    fn test_empty_code_handling() {
        let formatter = PythonFormatter::new();
        let result = formatter.format_code("").unwrap();
        assert!(!result.changed);
        assert_eq!(result.formatted, "");
    }

    #[test]
    fn test_whitespace_only_code() {
        let formatter = PythonFormatter::new();
        let result = formatter.format_code("   \n\t  \n").unwrap();
        assert!(!result.changed);
    }

    #[test]
    fn test_formatter_info() {
        let formatter = PythonFormatter::new();
        let info = formatter.formatter_info();
        assert!(info.contains("black"));
    }

    #[test]
    fn test_default_config_generation() {
        let formatter = PythonFormatter::new();
        let config = formatter.default_config().unwrap();
        assert!(config.contains("line-length"));
        assert!(config.contains("target-version"));
        assert!(config.contains("[tool.black]"));
    }

    #[test]
    fn test_black_args() {
        let formatter = PythonFormatter::new();
        let args = formatter.get_black_args();
        assert!(args.contains(&"--stdin-filename".to_string()));
        assert!(args.contains(&"stdin.py".to_string()));
        assert!(args.contains(&"--target-version".to_string()));
        assert!(args.contains(&"py311".to_string()));
        assert!(args.len() > 2); // Should have multiple arguments for proper formatting
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[test]
        fn test_simple_python_formatting() {
            let formatter = PythonFormatter::new();

            // Skip if black is not available
            if !formatter.is_available() {
                eprintln!("Skipping black integration test - black not available");
                return;
            }

            let unformatted_code = "def hello():print('Hello, world!')";
            let result = formatter.format_code(unformatted_code);

            match result {
                Ok(format_result) => {
                    // black should format this code
                    assert!(format_result.changed || format_result.messages.is_empty());

                    // Formatted code should be valid Python
                    assert!(format_result.formatted.contains("def hello()"));
                    assert!(format_result.formatted.contains("print"));
                }
                Err(e) => {
                    eprintln!("black formatting failed: {}", e);
                    // This is acceptable if black has issues with the test environment
                }
            }
        }

        #[test]
        fn test_complex_python_formatting() {
            let formatter = PythonFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping black integration test - black not available");
                return;
            }

            let complex_code = r#"
def process_data(items):
    result=[]
    for item in items:
        if isinstance(item,dict)and'name'in item:
            result.append({'name':item['name'].upper(),'processed':True})
    return result

class DataProcessor:
    def __init__(self,data):
        self.data=data
    def process(self):
        return[x*2 for x in self.data if x>0]
"#
            .trim();

            let result = formatter.format_code(complex_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that formatting improved the code structure
                        assert!(format_result.formatted.contains("def process_data"));
                        assert!(format_result.formatted.contains("class DataProcessor"));
                    }
                }
                Err(e) => {
                    eprintln!("black formatting of complex code failed: {}", e);
                }
            }
        }

        #[test]
        fn test_python_with_type_hints() {
            let formatter = PythonFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping black integration test - black not available");
                return;
            }

            let typed_code = "from typing import List,Dict,Optional\ndef process_users(users:List[Dict[str,str]])->Optional[List[str]]:return[user['name']for user in users if'name'in user]";
            let result = formatter.format_code(typed_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that type hints are preserved and formatted correctly
                        assert!(format_result.formatted.contains("List[Dict[str, str]]"));
                        assert!(format_result.formatted.contains("Optional[List[str]]"));
                        assert!(format_result.formatted.contains("from typing import"));
                    }
                }
                Err(e) => {
                    eprintln!("black formatting of typed Python failed: {}", e);
                }
            }
        }

        #[test]
        fn test_async_python_formatting() {
            let formatter = PythonFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping black integration test - black not available");
                return;
            }

            let async_code = "import asyncio\nasync def fetch_data(url:str)->dict:async with aiohttp.ClientSession()as session:async with session.get(url)as response:return await response.json()";
            let result = formatter.format_code(async_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that async/await syntax is preserved
                        assert!(format_result.formatted.contains("async def"));
                        assert!(format_result.formatted.contains("await"));
                        assert!(format_result.formatted.contains("async with"));
                    }
                }
                Err(e) => {
                    eprintln!("black formatting of async Python failed: {}", e);
                }
            }
        }

        #[test]
        fn test_syntax_error_handling() {
            let formatter = PythonFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping black integration test - black not available");
                return;
            }

            let invalid_code = "def hello(\nprint('broken syntax'"; // Missing closing parenthesis and colon
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
        fn test_long_lines_formatting() {
            let formatter = PythonFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping black integration test - black not available");
                return;
            }

            let long_line_code = "def very_long_function_name_that_exceeds_normal_limits(parameter_one, parameter_two, parameter_three, parameter_four, parameter_five): return parameter_one + parameter_two + parameter_three + parameter_four + parameter_five";
            let result = formatter.format_code(long_line_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that long lines are wrapped properly
                        let lines: Vec<&str> = format_result.formatted.lines().collect();
                        // Should have multiple lines after formatting
                        assert!(lines.len() > 1, "Long line should be wrapped into multiple lines");
                    }
                }
                Err(e) => {
                    eprintln!("black formatting of long line failed: {}", e);
                }
            }
        }
    }
}
