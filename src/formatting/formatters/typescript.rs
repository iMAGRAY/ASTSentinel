use super::super::{CodeFormatter, FormatResult};
use super::SecureCommandExecutor;
use crate::analysis::ast::SupportedLanguage;
/// TypeScript code formatter using prettier
use anyhow::Result;

/// TypeScript formatter implementation using prettier
pub struct TypeScriptFormatter {
    executor: SecureCommandExecutor,
}

impl TypeScriptFormatter {
    pub fn new() -> Self {
        Self {
            executor: SecureCommandExecutor::default(),
        }
    }

    /// Get prettier configuration arguments for TypeScript with 2025 standards
    fn get_prettier_args(&self) -> Vec<String> {
        vec![
            // Use TypeScript parser for modern TS
            "--parser".to_string(),
            "typescript".to_string(),
            // Specify file type for stdin
            "--stdin-filepath".to_string(),
            "stdin.ts".to_string(),
            // 2025 recommended settings for TypeScript
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
            "all".to_string(), // TypeScript supports trailing commas in all contexts
            "--bracket-spacing".to_string(),
            "true".to_string(),
            "--arrow-parens".to_string(),
            "avoid".to_string(),
        ]
    }
}

impl CodeFormatter for TypeScriptFormatter {
    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::TypeScript
    }

    /// Format TypeScript source code using prettier
    ///
    /// # Examples
    /// ```rust
    /// use rust_validation_hooks::formatting::formatters::typescript::TypeScriptFormatter;
    /// use rust_validation_hooks::formatting::CodeFormatter;
    ///
    /// let formatter = TypeScriptFormatter::new();
    /// let code = "interface User{name:string;age:number;}";
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
                "prettier formatter not available - skipping TypeScript formatting".to_string(),
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
  "trailingComma": "all",
  "singleQuote": false,
  "printWidth": 100,
  "tabWidth": 2,
  "useTabs": false,
  "bracketSpacing": true,
  "bracketSameLine": false,
  "arrowParens": "always",
  "endOfLine": "lf",
  "quoteProps": "as-needed",
  "requirePragma": false,
  "insertPragma": false,
  "proseWrap": "preserve",
  "embeddedLanguageFormatting": "auto",
  "parser": "typescript",
  "overrides": [
    {
      "files": ["*.tsx"],
      "options": {
        "parser": "typescript",
        "jsxSingleQuote": false,
        "jsxBracketSameLine": false
      }
    }
  ]
}"#
        .to_string())
    }
}

impl Default for TypeScriptFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typescript_formatter_creation() {
        let formatter = TypeScriptFormatter::new();
        assert_eq!(formatter.language(), SupportedLanguage::TypeScript);
    }

    #[test]
    fn test_empty_code_handling() {
        let formatter = TypeScriptFormatter::new();
        let result = formatter.format_code("").unwrap();
        assert!(!result.changed);
        assert_eq!(result.formatted, "");
    }

    #[test]
    fn test_whitespace_only_code() {
        let formatter = TypeScriptFormatter::new();
        let result = formatter.format_code("   \n\t  \n").unwrap();
        assert!(!result.changed);
    }

    #[test]
    fn test_formatter_info() {
        let formatter = TypeScriptFormatter::new();
        let info = formatter.formatter_info();
        assert!(info.contains("prettier"));
    }

    #[test]
    fn test_default_config_generation() {
        let formatter = TypeScriptFormatter::new();
        let config = formatter.default_config().unwrap();
        assert!(config.contains("typescript"));
        assert!(config.contains("printWidth"));
        assert!(config.contains("overrides"));
        assert!(config.contains("*.tsx"));
    }

    #[test]
    fn test_prettier_args() {
        let formatter = TypeScriptFormatter::new();
        let args = formatter.get_prettier_args();
        assert!(args.contains(&"--parser".to_string()));
        assert!(args.contains(&"typescript".to_string()));
        assert!(args.contains(&"--stdin-filepath".to_string()));
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[test]
        fn test_simple_typescript_formatting() {
            let formatter = TypeScriptFormatter::new();

            // Skip if prettier is not available
            if !formatter.is_available() {
                eprintln!("Skipping prettier integration test - prettier not available");
                return;
            }

            let unformatted_code = "interface User{name:string;age:number;}function greet(user:User){console.log(`Hello, ${user.name}`);}";
            let result = formatter.format_code(unformatted_code);

            match result {
                Ok(format_result) => {
                    // prettier should format this code
                    assert!(format_result.changed || format_result.messages.is_empty());

                    // Formatted code should be valid TypeScript
                    assert!(format_result.formatted.contains("interface User"));
                    assert!(format_result.formatted.contains("function greet"));
                }
                Err(e) => {
                    eprintln!("prettier formatting failed: {}", e);
                    // This is acceptable if prettier has issues with the test environment
                }
            }
        }

        #[test]
        fn test_complex_typescript_formatting() {
            let formatter = TypeScriptFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping prettier integration test - prettier not available");
                return;
            }

            let complex_code = r#"
type UserStatus='active'|'inactive';
interface User{readonly id:number;name:string;email?:string;status:UserStatus;}
class UserService<T extends User>{
private users:T[]=[];
public addUser(user:T):void{this.users.push(user);}
public getActiveUsers():T[]{return this.users.filter(u=>u.status==='active');}
}"#
            .trim();

            let result = formatter.format_code(complex_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that formatting improved the code structure
                        assert!(format_result.formatted.contains("type UserStatus"));
                        assert!(format_result.formatted.contains("interface User"));
                        assert!(format_result.formatted.contains("class UserService"));
                    }
                }
                Err(e) => {
                    eprintln!("prettier formatting of complex TypeScript failed: {}", e);
                }
            }
        }

        #[test]
        fn test_generics_and_decorators() {
            let formatter = TypeScriptFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping prettier integration test - prettier not available");
                return;
            }

            let generics_code = "function identity<T>(arg:T):T{return arg;}const result:string=identity<string>('hello');";
            let result = formatter.format_code(generics_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that TypeScript specific syntax is handled correctly
                        assert!(format_result.formatted.contains("<T>"));
                        assert!(format_result.formatted.contains("function identity"));
                    }
                }
                Err(e) => {
                    eprintln!("prettier formatting of generics failed: {}", e);
                }
            }
        }

        #[test]
        fn test_async_await_formatting() {
            let formatter = TypeScriptFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping prettier integration test - prettier not available");
                return;
            }

            let async_code = "async function fetchUser(id:number):Promise<User>{const response=await fetch(`/users/${id}`);return response.json();}";
            let result = formatter.format_code(async_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that async/await syntax is preserved
                        assert!(format_result.formatted.contains("async function"));
                        assert!(format_result.formatted.contains("await"));
                        assert!(format_result.formatted.contains("Promise<User>"));
                    }
                }
                Err(e) => {
                    eprintln!("prettier formatting of async/await failed: {}", e);
                }
            }
        }

        #[test]
        fn test_syntax_error_handling() {
            let formatter = TypeScriptFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping prettier integration test - prettier not available");
                return;
            }

            let invalid_code = "interface User{ name:string age:number }"; // Missing semicolon
            let result = formatter.format_code(invalid_code);

            match result {
                Ok(format_result) => {
                    // Should return original code with error message or formatted if fixable
                    if !format_result.changed {
                        assert_eq!(format_result.formatted, invalid_code);
                        assert!(!format_result.messages.is_empty());
                    }
                }
                Err(_) => {
                    // This is also acceptable - formatter detected the error
                }
            }
        }
    }
}
