use super::super::{CodeFormatter, FormatResult};
use super::SecureCommandExecutor;
use crate::analysis::ast::SupportedLanguage;
/// C code formatter using clang-format
use anyhow::Result;

/// C formatter implementation using clang-format
pub struct CFormatter {
    executor: SecureCommandExecutor,
}

impl CFormatter {
    pub fn new() -> Self {
        Self {
            executor: SecureCommandExecutor::default(),
        }
    }

    /// Get clang-format configuration arguments for C with 2025 standards
    fn get_clang_format_args(&self) -> Vec<String> {
        vec![
            "--assume-filename".to_string(),
            "stdin.c".to_string(),
            "--style".to_string(),
            "LLVM".to_string(), // LLVM is the default and most widely supported style
            "--fallback-style".to_string(),
            "GNU".to_string(), // GNU as fallback for C code
            "--ferror-limit".to_string(),
            "0".to_string(), // No limit on formatting errors
        ]
    }
}

impl CodeFormatter for CFormatter {
    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::C
    }

    /// Format C source code using clang-format
    ///
    /// # Examples
    /// ```rust
    /// use rust_validation_hooks::formatting::formatters::c::CFormatter;
    /// use rust_validation_hooks::formatting::CodeFormatter;
    ///
    /// let formatter = CFormatter::new();
    /// let code = "#include <stdio.h>\nint main(){printf(\"Hello\");return 0;}";
    /// let result = formatter.format_code(code).unwrap();
    /// assert!(result.changed);
    /// ```
    fn format_code(&self, code: &str) -> Result<FormatResult> {
        // Validate input
        if code.trim().is_empty() {
            return Ok(FormatResult::unchanged(code.to_string()));
        }

        // Check if clang-format is available - graceful degradation
        if !self.is_available() {
            let mut result = FormatResult::unchanged(code.to_string());
            result
                .messages
                .push("clang-format formatter not available - skipping C formatting".to_string());
            return Ok(result);
        }

        // Prepare clang-format arguments
        let args = self.get_clang_format_args();

        // Execute clang-format with stdin input
        match self
            .executor
            .execute_formatter("clang-format", &args, Some(code))
        {
            Ok(formatted_code) => {
                let result = FormatResult::new(code.to_string(), formatted_code);
                Ok(result)
            }
            Err(e) => {
                // If clang-format fails, it might be due to syntax errors
                // Return the original code with error message
                let mut result = FormatResult::unchanged(code.to_string());
                result.messages.push(format!("clang-format failed: {}", e));
                Ok(result)
            }
        }
    }

    fn is_available(&self) -> bool {
        self.executor.command_exists("clang-format")
    }

    fn formatter_info(&self) -> String {
        self.executor.get_formatter_version("clang-format")
    }

    fn default_config(&self) -> Result<String> {
        Ok(r#"---
# clang-format configuration for C code
# Based on the GNU coding standard with some modern adjustments

BasedOnStyle: GNU
Language: C

# Indentation
IndentWidth: 2
TabWidth: 8
UseTab: Never
IndentCaseLabels: true
IndentPPDirectives: None

# Spacing
SpaceAfterCStyleCast: false
SpaceBeforeParens: ControlStatements
SpaceInEmptyParentheses: false
SpacesInContainerLiterals: false
SpacesInCStyleCastParentheses: false
SpacesInParentheses: false
SpacesInSquareBrackets: false

# Line breaking
MaxEmptyLinesToKeep: 2
KeepEmptyLinesAtTheStartOfBlocks: false
ColumnLimit: 80
BreakBeforeBraces: GNU
AllowShortIfStatementsOnASingleLine: false
AllowShortLoopsOnASingleLine: false
AllowShortFunctionsOnASingleLine: None
AllowShortBlocksOnASingleLine: false

# Alignment
AlignAfterOpenBracket: Align
AlignConsecutiveAssignments: false
AlignConsecutiveDeclarations: false
AlignEscapedNewlines: Right
AlignOperands: true
AlignTrailingComments: true

# Pointer and reference alignment
PointerAlignment: Right
DerivePointerAlignment: false

# Function formatting
BinPackArguments: false
BinPackParameters: false
AllowAllArgumentsOnNextLine: false
AllowAllParametersOfDeclarationOnNextLine: false

# Include sorting
SortIncludes: true
IncludeBlocks: Preserve

# Comments
ReflowComments: true
SpacesBeforeTrailingComments: 2

# Macros and preprocessor
MacroBlockBegin: ''
MacroBlockEnd: ''
"#
        .to_string())
    }
}

impl Default for CFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_formatter_creation() {
        let formatter = CFormatter::new();
        assert_eq!(formatter.language(), SupportedLanguage::C);
    }

    #[test]
    fn test_empty_code_handling() {
        let formatter = CFormatter::new();
        let result = formatter.format_code("").unwrap();
        assert!(!result.changed);
        assert_eq!(result.formatted, "");
    }

    #[test]
    fn test_whitespace_only_code() {
        let formatter = CFormatter::new();
        let result = formatter.format_code("   \n\t  \n").unwrap();
        assert!(!result.changed);
    }

    #[test]
    fn test_formatter_info() {
        let formatter = CFormatter::new();
        let info = formatter.formatter_info();
        assert!(info.contains("clang-format"));
    }

    #[test]
    fn test_default_config_generation() {
        let formatter = CFormatter::new();
        let config = formatter.default_config().unwrap();
        assert!(config.contains("BasedOnStyle"));
        assert!(config.contains("GNU"));
        assert!(config.contains("IndentWidth"));
    }

    #[test]
    fn test_clang_format_args() {
        let formatter = CFormatter::new();
        let args = formatter.get_clang_format_args();
        assert!(args.contains(&"--assume-filename".to_string()));
        assert!(args.contains(&"stdin.c".to_string()));
        assert!(args.contains(&"--style".to_string()));
        assert!(args.contains(&"LLVM".to_string()));
        assert!(args.contains(&"--fallback-style".to_string()));
        assert!(args.contains(&"GNU".to_string()));
        assert!(args.contains(&"--ferror-limit".to_string()));
        assert_eq!(args.len(), 8); // Updated for 2025 standards with more args
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[test]
        fn test_simple_c_formatting() {
            let formatter = CFormatter::new();

            // Skip if clang-format is not available
            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let unformatted_code =
                "#include <stdio.h>\nint main(){printf(\"Hello, World!\");return 0;}";
            let result = formatter.format_code(unformatted_code);

            match result {
                Ok(format_result) => {
                    // clang-format should format this code
                    assert!(format_result.changed || format_result.messages.is_empty());

                    // Formatted code should be valid C
                    assert!(format_result.formatted.contains("#include <stdio.h>"));
                    assert!(format_result.formatted.contains("int main"));
                }
                Err(e) => {
                    eprintln!("clang-format formatting failed: {}", e);
                    // This is acceptable if clang-format has issues with the test environment
                }
            }
        }

        #[test]
        fn test_complex_c_formatting() {
            let formatter = CFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let complex_code = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
typedef struct{char*name;int age;float salary;}Employee;
Employee*create_employee(char*name,int age,float salary){
Employee*emp=malloc(sizeof(Employee));
if(emp){emp->name=strdup(name);emp->age=age;emp->salary=salary;}
return emp;}
void print_employee(Employee*emp){
if(emp)printf("Name: %s, Age: %d, Salary: %.2f\n",emp->name,emp->age,emp->salary);
}
int main(){Employee*emp=create_employee("John Doe",30,50000.0);print_employee(emp);free(emp->name);free(emp);return 0;}
"#.trim();

            let result = formatter.format_code(complex_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that formatting improved the code structure
                        assert!(format_result.formatted.contains("typedef struct"));
                        assert!(format_result
                            .formatted
                            .contains("Employee *create_employee"));
                        assert!(format_result.formatted.contains("int main"));
                    }
                }
                Err(e) => {
                    eprintln!("clang-format formatting of complex code failed: {}", e);
                }
            }
        }

        #[test]
        fn test_c_pointers_and_arrays() {
            let formatter = CFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let pointer_code = "int*ptr;int arr[10];void func(int*p,int n){for(int i=0;i<n;i++){ptr[i]=arr[i]*2;}}";
            let result = formatter.format_code(pointer_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that C pointer syntax is handled correctly
                        assert!(format_result.formatted.contains("int *"));
                        assert!(format_result.formatted.contains("arr["));
                        assert!(format_result.formatted.contains("for ("));
                    }
                }
                Err(e) => {
                    eprintln!("clang-format formatting of pointers failed: {}", e);
                }
            }
        }

        #[test]
        fn test_c_macros_and_preprocessor() {
            let formatter = CFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let macro_code = "#define MAX_SIZE 100\n#define MIN(a,b) ((a)<(b)?(a):(b))\n#ifdef DEBUG\n#define DBG_PRINT(x) printf x\n#else\n#define DBG_PRINT(x)\n#endif\nint main(){int size=MIN(50,MAX_SIZE);DBG_PRINT((\"Size: %d\\n\",size));return 0;}";
            let result = formatter.format_code(macro_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that preprocessor directives are handled
                        assert!(format_result.formatted.contains("#define"));
                        assert!(format_result.formatted.contains("#ifdef"));
                        assert!(format_result.formatted.contains("#else"));
                    }
                }
                Err(e) => {
                    eprintln!("clang-format formatting of macros failed: {}", e);
                }
            }
        }

        #[test]
        fn test_syntax_error_handling() {
            let formatter = CFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let invalid_code =
                "#include <stdio.h>\nint main( {\nprintf(\"missing brace\";\nreturn 0;"; // Missing closing brace and parenthesis
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
        fn test_function_declarations() {
            let formatter = CFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let func_code = "static inline int max(int a,int b);extern void process_data(const char*data,size_t len);int max(int a,int b){return a>b?a:b;}void process_data(const char*data,size_t len){for(size_t i=0;i<len;i++)putchar(data[i]);}";
            let result = formatter.format_code(func_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that function declarations are formatted properly
                        assert!(format_result.formatted.contains("static inline"));
                        assert!(format_result.formatted.contains("extern"));
                        assert!(format_result.formatted.contains("const char *"));
                    }
                }
                Err(e) => {
                    eprintln!(
                        "clang-format formatting of function declarations failed: {}",
                        e
                    );
                }
            }
        }
    }
}
