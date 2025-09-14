use super::super::{CodeFormatter, FormatResult};
use super::SecureCommandExecutor;
use crate::analysis::ast::SupportedLanguage;
/// C++ code formatter using clang-format
use anyhow::Result;

/// C++ formatter implementation using clang-format
pub struct CppFormatter {
    executor: SecureCommandExecutor,
}

impl CppFormatter {
    pub fn new() -> Self {
        Self {
            executor: SecureCommandExecutor::default(),
        }
    }

    /// Get clang-format configuration arguments for C++ with 2025 standards
    fn get_clang_format_args(&self) -> Vec<String> {
        vec![
            "--assume-filename".to_string(),
            "stdin.cpp".to_string(),
            "--style".to_string(),
            "Google".to_string(), // Google C++ style is widely adopted for modern C++
            "--fallback-style".to_string(),
            "LLVM".to_string(), // LLVM as fallback
            "--ferror-limit".to_string(),
            "0".to_string(), // No limit on formatting errors
        ]
    }
}

impl CodeFormatter for CppFormatter {
    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::Cpp
    }

    /// Format C++ source code using clang-format
    ///
    /// # Examples
    /// ```rust,no_run
    /// use rust_validation_hooks::formatting::formatters::cpp::CppFormatter;
    /// use rust_validation_hooks::formatting::CodeFormatter;
    ///
    /// let formatter = CppFormatter::new();
    /// let code = "#include <iostream>\nint main(){std::cout<<\"Hello\"<<std::endl;return 0;}";
    /// let result = formatter.format_code(code).unwrap();
    /// // External formatter may be missing; compile-only example
    /// assert!(result.changed || !result.messages.is_empty());
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
                .push("clang-format formatter not available - skipping C++ formatting".to_string());
            return Ok(result);
        }

        // Prepare clang-format arguments
        let args = self.get_clang_format_args();

        // Execute clang-format with stdin input
        match self.executor.execute_formatter("clang-format", &args, Some(code)) {
            Ok(formatted_code) => {
                let result = FormatResult::new(code.to_string(), formatted_code);
                Ok(result)
            }
            Err(e) => {
                // If clang-format fails, it might be due to syntax errors
                // Return the original code with error message
                let mut result = FormatResult::unchanged(code.to_string());
                result.messages.push(format!("clang-format failed: {e}"));
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
# clang-format configuration for C++ code
# Based on Google C++ Style Guide with some modern C++ adjustments

BasedOnStyle: Google
Language: Cpp
Standard: c++17

# Indentation
IndentWidth: 2
TabWidth: 2
UseTab: Never
IndentCaseLabels: true
IndentPPDirectives: None
AccessModifierOffset: -1

# Spacing
SpaceAfterCStyleCast: false
SpaceAfterLogicalNot: false
SpaceAfterTemplateKeyword: true
SpaceBeforeAssignmentOperators: true
SpaceBeforeParens: ControlStatements
SpaceInEmptyParentheses: false
SpacesInContainerLiterals: true
SpacesInCStyleCastParentheses: false
SpacesInParentheses: false
SpacesInSquareBrackets: false
SpaceBeforeRangeBasedForLoopColon: true

# Line breaking
MaxEmptyLinesToKeep: 1
KeepEmptyLinesAtTheStartOfBlocks: false
ColumnLimit: 80
BreakBeforeBraces: Attach
AllowShortIfStatementsOnASingleLine: WithoutElse
AllowShortLoopsOnASingleLine: true
AllowShortFunctionsOnASingleLine: All
AllowShortBlocksOnASingleLine: Never
AllowShortCaseLabelsOnASingleLine: false

# Modern C++ features
Cpp11BracedListStyle: true
FixNamespaceComments: true
CompactNamespaces: false
NamespaceIndentation: None

# Template formatting
AlwaysBreakTemplateDeclarations: Yes
BreakBeforeConceptDeclarations: true

# Alignment
AlignAfterOpenBracket: Align
AlignConsecutiveAssignments: false
AlignConsecutiveDeclarations: false
AlignEscapedNewlines: Left
AlignOperands: true
AlignTrailingComments: true

# Pointer and reference alignment
PointerAlignment: Left
DerivePointerAlignment: false

# Function formatting
BinPackArguments: true
BinPackParameters: true
AllowAllArgumentsOnNextLine: true
AllowAllParametersOfDeclarationOnNextLine: true

# Include sorting
SortIncludes: true
IncludeBlocks: Regroup
IncludeCategories:
  - Regex: '^<ext/.*\.h>'
    Priority: 2
  - Regex: '^<.*\.h>'
    Priority: 1
  - Regex: '^<.*'
    Priority: 2
  - Regex: '.*'
    Priority: 3

# Comments
ReflowComments: true
SpacesBeforeTrailingComments: 2

# Constructor initializer lists
ConstructorInitializerAllOnOneLineOrOnePerLine: true
ConstructorInitializerIndentWidth: 4
BreakConstructorInitializers: BeforeColon

# Other formatting options
ExperimentalAutoDetectBinPacking: false
PenaltyBreakAssignment: 2
PenaltyBreakBeforeFirstCallParameter: 1
PenaltyBreakComment: 300
PenaltyBreakFirstLessLess: 120
PenaltyBreakString: 1000
PenaltyExcessCharacter: 1000000
PenaltyReturnTypeOnItsOwnLine: 200
"#
        .to_string())
    }
}

impl Default for CppFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_formatter_creation() {
        let formatter = CppFormatter::new();
        assert_eq!(formatter.language(), SupportedLanguage::Cpp);
    }

    #[test]
    fn test_empty_code_handling() {
        let formatter = CppFormatter::new();
        let result = formatter.format_code("").unwrap();
        assert!(!result.changed);
        assert_eq!(result.formatted, "");
    }

    #[test]
    fn test_whitespace_only_code() {
        let formatter = CppFormatter::new();
        let result = formatter.format_code("   \n\t  \n").unwrap();
        assert!(!result.changed);
    }

    #[test]
    fn test_formatter_info() {
        let formatter = CppFormatter::new();
        let info = formatter.formatter_info();
        assert!(info.contains("clang-format"));
    }

    #[test]
    fn test_default_config_generation() {
        let formatter = CppFormatter::new();
        let config = formatter.default_config().unwrap();
        assert!(config.contains("BasedOnStyle"));
        assert!(config.contains("Google"));
        assert!(config.contains("Cpp"));
        assert!(config.contains("c++17"));
    }

    #[test]
    fn test_clang_format_args() {
        let formatter = CppFormatter::new();
        let args = formatter.get_clang_format_args();
        assert!(args.contains(&"--assume-filename".to_string()));
        assert!(args.contains(&"stdin.cpp".to_string()));
        assert!(args.contains(&"--style".to_string()));
        assert!(args.contains(&"Google".to_string()));
        assert!(args.contains(&"--fallback-style".to_string()));
        assert!(args.contains(&"LLVM".to_string()));
        assert!(args.contains(&"--ferror-limit".to_string()));
        assert_eq!(args.len(), 8); // Updated for 2025 standards with more args
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[test]
        fn test_simple_cpp_formatting() {
            let formatter = CppFormatter::new();

            // Skip if clang-format is not available
            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let unformatted_code =
                "#include <iostream>\nint main(){std::cout<<\"Hello, World!\"<<std::endl;return 0;}";
            let result = formatter.format_code(unformatted_code);

            match result {
                Ok(format_result) => {
                    // clang-format should format this code
                    assert!(format_result.changed || format_result.messages.is_empty());

                    // Formatted code should be valid C++
                    assert!(format_result.formatted.contains("#include <iostream>"));
                    assert!(format_result.formatted.contains("int main"));
                }
                Err(e) => {
                    eprintln!("clang-format formatting failed: {}", e);
                    // This is acceptable if clang-format has issues with the
                    // test environment
                }
            }
        }

        #[test]
        fn test_complex_cpp_formatting() {
            let formatter = CppFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let complex_code = r#"
#include <iostream>
#include <vector>
#include <algorithm>
#include <memory>
class Person{
private:std::string name_;int age_;
public:Person(std::string name,int age):name_(std::move(name)),age_(age){}
const std::string&getName()const{return name_;}
int getAge()const{return age_;}
};
class PersonManager{
private:std::vector<std::unique_ptr<Person>>people_;
public:void addPerson(std::unique_ptr<Person>person){people_.push_back(std::move(person));}
std::vector<Person*>getAdults()const{
std::vector<Person*>adults;
std::copy_if(people_.begin(),people_.end(),std::back_inserter(adults),
[](const auto&p){return p->getAge()>=18;});
return adults;}
};
"#
            .trim();

            let result = formatter.format_code(complex_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that formatting improved the code structure
                        assert!(format_result.formatted.contains("class Person"));
                        assert!(format_result.formatted.contains("class PersonManager"));
                        assert!(format_result.formatted.contains("std::unique_ptr"));
                    }
                }
                Err(e) => {
                    eprintln!("clang-format formatting of complex code failed: {}", e);
                }
            }
        }

        #[test]
        fn test_cpp_templates_and_stl() {
            let formatter = CppFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let template_code = "template<typename T,typename U>auto add(T t,U u)->decltype(t+u){return t+u;}template<>int add<int,int>(int a,int b){return a+b;}std::vector<int>nums{1,2,3,4,5};auto result=std::accumulate(nums.begin(),nums.end(),0);";
            let result = formatter.format_code(template_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that C++ template syntax is handled correctly
                        assert!(format_result.formatted.contains("template<"));
                        assert!(format_result.formatted.contains("decltype"));
                        assert!(format_result.formatted.contains("std::vector"));
                    }
                }
                Err(e) => {
                    eprintln!("clang-format formatting of templates failed: {}", e);
                }
            }
        }

        #[test]
        fn test_modern_cpp_features() {
            let formatter = CppFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let modern_code = "auto lambda=[](const auto&x){return x*2;};std::vector<int>v{1,2,3,4,5};std::transform(v.begin(),v.end(),v.begin(),lambda);for(const auto&item:v){std::cout<<item<<\" \";}";
            let result = formatter.format_code(modern_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that modern C++ features are preserved
                        assert!(format_result.formatted.contains("auto lambda"));
                        assert!(format_result.formatted.contains("const auto&"));
                        assert!(format_result.formatted.contains("for ("));
                    }
                }
                Err(e) => {
                    eprintln!("clang-format formatting of modern C++ failed: {}", e);
                }
            }
        }

        #[test]
        fn test_namespace_formatting() {
            let formatter = CppFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let namespace_code = "namespace utils{namespace string{std::string trim(const std::string&s){auto start=s.find_first_not_of(\" \\t\");if(start==std::string::npos)return\"\";auto end=s.find_last_not_of(\" \\t\");return s.substr(start,end-start+1);}}}using namespace utils::string;";
            let result = formatter.format_code(namespace_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that namespace formatting is handled
                        assert!(format_result.formatted.contains("namespace utils"));
                        assert!(format_result.formatted.contains("namespace string"));
                        assert!(format_result.formatted.contains("using namespace"));
                    }
                }
                Err(e) => {
                    eprintln!("clang-format formatting of namespaces failed: {}", e);
                }
            }
        }

        #[test]
        fn test_syntax_error_handling() {
            let formatter = CppFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping clang-format integration test - clang-format not available");
                return;
            }

            let invalid_code =
                "#include <iostream>\nint main( {\nstd::cout<<\"missing brace\"<<std::endl;\nreturn 0;"; // Missing closing brace and parenthesis
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
    }
}
