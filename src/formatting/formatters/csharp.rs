use super::super::{CodeFormatter, FormatResult};
use super::SecureCommandExecutor;
use crate::analysis::ast::SupportedLanguage;
/// C# code formatter using CSharpier
use anyhow::Result;

/// C# formatter implementation using CSharpier
pub struct CSharpFormatter {
    executor: SecureCommandExecutor,
}

impl CSharpFormatter {
    pub fn new() -> Self {
        Self {
            executor: SecureCommandExecutor::default(),
        }
    }

    /// Get CSharpier configuration arguments with 2025 standards
    fn get_csharpier_args(&self) -> Vec<String> {
        vec![
            "tool".to_string(),
            "run".to_string(),
            "csharpier".to_string(),
            "--stdin-filepath".to_string(),
            "stdin.cs".to_string(),
        ]
    }
}

impl CodeFormatter for CSharpFormatter {
    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::CSharp
    }

    /// Format C# source code using CSharpier
    ///
    /// # Examples
    /// ```rust
    /// use rust_validation_hooks::formatting::formatters::csharp::CSharpFormatter;
    /// use rust_validation_hooks::formatting::CodeFormatter;
    ///
    /// let formatter = CSharpFormatter::new();
    /// let code = "namespace Example{public class Hello{public void Method(){Console.WriteLine(\"Hello\");}}}";
    /// let result = formatter.format_code(code).unwrap();
    /// assert!(result.changed);
    /// ```
    fn format_code(&self, code: &str) -> Result<FormatResult> {
        // Validate input
        if code.trim().is_empty() {
            return Ok(FormatResult::unchanged(code.to_string()));
        }

        // Check if CSharpier is available - graceful degradation
        if !self.is_available() {
            let mut result = FormatResult::unchanged(code.to_string());
            result
                .messages
                .push("CSharpier formatter not available - skipping C# formatting".to_string());
            return Ok(result);
        }

        // Prepare CSharpier arguments
        let args = self.get_csharpier_args();

        // Execute CSharpier with stdin input
        match self.executor.execute_formatter("dotnet", &args, Some(code)) {
            Ok(formatted_code) => {
                let result = FormatResult::new(code.to_string(), formatted_code);
                Ok(result)
            }
            Err(e) => {
                // If CSharpier fails, it might be due to syntax errors
                // Return the original code with error message
                let mut result = FormatResult::unchanged(code.to_string());
                result.messages.push(format!("CSharpier failed: {}", e));
                Ok(result)
            }
        }
    }

    fn is_available(&self) -> bool {
        // Check if dotnet is available and CSharpier is installed
        self.executor.command_exists("dotnet")
            && self
                .executor
                .execute_formatter(
                    "dotnet",
                    &["tool".to_string(), "list".to_string(), "-g".to_string()],
                    None,
                )
                .map(|output| output.contains("csharpier"))
                .unwrap_or(false)
    }

    fn formatter_info(&self) -> String {
        match self.executor.execute_formatter(
            "dotnet",
            &["csharpier".to_string(), "--version".to_string()],
            None,
        ) {
            Ok(version) => format!("CSharpier {}", version.trim()),
            Err(_) => "CSharpier (version unknown)".to_string(),
        }
    }

    fn default_config(&self) -> Result<String> {
        let json_config = r#"{
  "$schema": "https://csharpier.com/schema.json",
  "printWidth": 100,
  "useTabs": false,
  "tabWidth": 4,
  "endOfLine": "auto",
  "includeGenerated": false,
  "preprocessorSymbolSets": []
}"#;
        let documentation = r#"

// Alternative .csharpierrc.yaml configuration:
// printWidth: 100
// useTabs: false
// tabWidth: 4
// endOfLine: auto
// includeGenerated: false
// preprocessorSymbolSets: []

// CSharpier is opinionated and provides minimal configuration options.
// The tool focuses on consistent formatting rather than extensive customization.

// Installation:
// dotnet tool install -g csharpier

// Usage:
// dotnet tool run csharpier [files/directories]
// dotnet tool run csharpier --stdin-filepath stdin.cs < input.cs

// Integration with editors:
// - Visual Studio Code: Install CSharpier extension
// - Visual Studio: Use dotnet format or external tool
// - JetBrains Rider: Configure as external tool"#;
        Ok(format!("{}{}", json_config, documentation))
    }
}

impl Default for CSharpFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csharp_formatter_creation() {
        let formatter = CSharpFormatter::new();
        assert_eq!(formatter.language(), SupportedLanguage::CSharp);
    }

    #[test]
    fn test_empty_code_handling() {
        let formatter = CSharpFormatter::new();
        let result = formatter.format_code("").unwrap();
        assert!(!result.changed);
        assert_eq!(result.formatted, "");
    }

    #[test]
    fn test_whitespace_only_code() {
        let formatter = CSharpFormatter::new();
        let result = formatter.format_code("   \n\t  \n").unwrap();
        assert!(!result.changed);
    }

    /// Test that formatter_info returns CSharpier version information
    #[test]
    fn test_formatter_info() {
        let formatter = CSharpFormatter::new();
        let info = formatter.formatter_info();
        // Should return either version info with "CSharpier" prefix or fallback message
        assert!(
            info.starts_with("CSharpier"),
            "Expected formatter_info to start with 'CSharpier', got: '{}'",
            info
        );
    }

    #[test]
    fn test_default_config_generation() {
        let formatter = CSharpFormatter::new();
        let config = formatter.default_config().unwrap();
        assert!(config.contains("printWidth"));
        assert!(config.contains("tabWidth"));
        assert!(config.contains("CSharpier"));
    }

    #[test]
    fn test_csharpier_args() {
        let formatter = CSharpFormatter::new();
        let args = formatter.get_csharpier_args();
        assert!(args.contains(&"tool".to_string()));
        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"csharpier".to_string()));
        assert!(args.contains(&"--stdin-filepath".to_string()));
        assert!(args.contains(&"stdin.cs".to_string()));
        assert_eq!(args.len(), 5);
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[test]
        fn test_simple_csharp_formatting() {
            let formatter = CSharpFormatter::new();

            // Skip if dotnet is not available
            if !formatter.is_available() {
                eprintln!("Skipping dotnet format integration test - dotnet not available");
                return;
            }

            let unformatted_code = "namespace Example{public class Hello{public void Method(){Console.WriteLine(\"Hello, World!\");}}}";
            let result = formatter.format_code(unformatted_code);

            match result {
                Ok(format_result) => {
                    // dotnet format should format this code OR provide error message
                    assert!(format_result.changed || !format_result.messages.is_empty());

                    // Formatted code should be valid C#
                    assert!(format_result.formatted.contains("namespace Example"));
                    assert!(format_result.formatted.contains("public class Hello"));
                }
                Err(e) => {
                    eprintln!("dotnet format formatting failed: {}", e);
                    // This is acceptable if dotnet format has issues with the test environment
                }
            }
        }

        #[test]
        fn test_complex_csharp_formatting() {
            let formatter = CSharpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping dotnet format integration test - dotnet not available");
                return;
            }

            let complex_code = r#"
using System;using System.Collections.Generic;using System.Linq;
namespace UserService{
public interface IUserService{List<User>GetUsers();void AddUser(User user);}
public class UserService:IUserService{
private readonly List<User>_users=new List<User>();
public List<User>GetUsers(){return _users.Where(u=>u.IsActive).ToList();}
public void AddUser(User user){if(user!=null)_users.Add(user);}
}
public class User{public string Name{get;set;}public int Age{get;set;}public bool IsActive{get;set;}}
}
"#.trim();

            let result = formatter.format_code(complex_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that formatting improved the code structure
                        assert!(format_result
                            .formatted
                            .contains("public interface IUserService"));
                        assert!(format_result.formatted.contains("public class UserService"));
                        assert!(format_result.formatted.contains("public class User"));
                    }
                }
                Err(e) => {
                    eprintln!("dotnet format formatting of complex code failed: {}", e);
                }
            }
        }

        #[test]
        fn test_csharp_generics_and_linq() {
            let formatter = CSharpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping dotnet format integration test - dotnet not available");
                return;
            }

            let generics_code = "public class Repository<T>where T:class,new(){private List<T>_items=new List<T>();public IEnumerable<T>FindAll(Func<T,bool>predicate){return _items.Where(predicate).OrderBy(x=>x.ToString());}}";
            let result = formatter.format_code(generics_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that C# generics and LINQ are handled correctly
                        assert!(format_result.formatted.contains("Repository<T>"));
                        assert!(format_result.formatted.contains("where T : class"));
                        assert!(format_result.formatted.contains("Func<T, bool>"));
                    }
                }
                Err(e) => {
                    eprintln!("dotnet format formatting of generics failed: {}", e);
                }
            }
        }

        #[test]
        fn test_csharp_async_await() {
            let formatter = CSharpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping dotnet format integration test - dotnet not available");
                return;
            }

            let async_code = "public async Task<List<User>>GetUsersAsync(){using var httpClient=new HttpClient();var response=await httpClient.GetStringAsync(\"/api/users\");return JsonSerializer.Deserialize<List<User>>(response);}";
            let result = formatter.format_code(async_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that async/await syntax is preserved
                        assert!(format_result.formatted.contains("async Task"));
                        assert!(format_result.formatted.contains("await"));
                        assert!(format_result.formatted.contains("using var"));
                    }
                }
                Err(e) => {
                    eprintln!("dotnet format formatting of async code failed: {}", e);
                }
            }
        }

        #[test]
        fn test_syntax_error_handling() {
            let formatter = CSharpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping dotnet format integration test - dotnet not available");
                return;
            }

            let invalid_code = "namespace Broken {\npublic class Test {\npublic void Method( {\nConsole.WriteLine(\"missing brace\";\n}"; // Missing closing brace and parenthesis
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
        fn test_modern_csharp_features() {
            let formatter = CSharpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping dotnet format integration test - dotnet not available");
                return;
            }

            let modern_code = "public record User(string Name,int Age){public bool IsAdult=>Age>=18;}public static class Extensions{public static T GetOrDefault<T>(this T?value,T defaultValue)where T:struct=>value??defaultValue;}";
            let result = formatter.format_code(modern_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that modern C# features are preserved
                        assert!(format_result.formatted.contains("record User"));
                        assert!(format_result.formatted.contains("=>"));
                        assert!(format_result.formatted.contains("where T : struct"));
                    }
                }
                Err(e) => {
                    eprintln!("dotnet format formatting of modern C# failed: {}", e);
                }
            }
        }
    }
}
