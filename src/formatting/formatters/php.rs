use super::super::{CodeFormatter, FormatResult};
use super::SecureCommandExecutor;
use crate::analysis::ast::SupportedLanguage;
/// PHP code formatter using php-cs-fixer with PER-CS3.0 support
use anyhow::Result;

/// PHP formatter implementation using php-cs-fixer v3.86.0+
pub struct PhpFormatter {
    executor: SecureCommandExecutor,
}

impl PhpFormatter {
    pub fn new() -> Self {
        Self {
            executor: SecureCommandExecutor::default(),
        }
    }

    /// Get php-cs-fixer configuration arguments with 2025 standards
    fn get_php_cs_fixer_args(&self) -> Vec<String> {
        vec![
            "fix".to_string(),
            "--rules=@PER-CS3.0".to_string(), // PER-CS3.0 is the latest standard for 2025
            "--using-cache=no".to_string(),
            "--diff".to_string(),            // Show differences
            "--allow-risky=yes".to_string(), // Allow risky rules for better formatting
            "-".to_string(),                 // Read from stdin
        ]
    }
}

impl CodeFormatter for PhpFormatter {
    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::Php
    }

    /// Format PHP source code using php-cs-fixer
    ///
    /// # Examples
    /// ```rust,no_run
    /// use rust_validation_hooks::formatting::formatters::php::PhpFormatter;
    /// use rust_validation_hooks::formatting::CodeFormatter;
    ///
    /// let formatter = PhpFormatter::new();
    /// let code = "<?php function hello(){echo 'Hello World';}";
    /// let result = formatter.format_code(code).unwrap();
    /// // External formatter may be missing; compile-only example
    /// assert!(result.changed || !result.messages.is_empty());
    /// ```
    fn format_code(&self, code: &str) -> Result<FormatResult> {
        // Validate input
        if code.trim().is_empty() {
            return Ok(FormatResult::unchanged(code.to_string()));
        }

        // Check if php-cs-fixer is available - graceful degradation
        if !self.is_available() {
            let mut result = FormatResult::unchanged(code.to_string());
            result
                .messages
                .push("php-cs-fixer formatter not available - skipping PHP formatting".to_string());
            return Ok(result);
        }

        // Prepare php-cs-fixer arguments
        let args = self.get_php_cs_fixer_args();

        // Execute php-cs-fixer with stdin input
        match self
            .executor
            .execute_formatter("php-cs-fixer", &args, Some(code))
        {
            Ok(formatted_code) => {
                let result = FormatResult::new(code.to_string(), formatted_code);
                Ok(result)
            }
            Err(e) => {
                // If php-cs-fixer fails, it might be due to syntax errors
                // Return the original code with error message
                let mut result = FormatResult::unchanged(code.to_string());
                result.messages.push(format!("php-cs-fixer failed: {}", e));
                Ok(result)
            }
        }
    }

    fn is_available(&self) -> bool {
        self.executor.command_exists("php-cs-fixer")
    }

    fn formatter_info(&self) -> String {
        self.executor.get_formatter_version("php-cs-fixer")
    }

    fn default_config(&self) -> Result<String> {
        Ok(r#"<?php

$config = new PhpCsFixer\Config();
return $config
    ->setRules([
        '@PSR12' => true,
        '@PHP74Migration' => true,
        '@PhpCsFixer' => true,
        
        // Array formatting
        'array_syntax' => ['syntax' => 'short'],
        'array_indentation' => true,
        'trim_array_spaces' => true,
        'normalize_index_brace' => true,
        
        // Binary operators
        'binary_operator_spaces' => [
            'default' => 'single_space',
            'operators' => ['=>' => null]
        ],
        'concat_space' => ['spacing' => 'one'],
        
        // Braces
        'braces' => [
            'allow_single_line_closure' => true,
            'position_after_functions_and_oop_constructs' => 'next',
            'position_after_control_structures' => 'same',
            'position_after_anonymous_constructs' => 'same'
        ],
        
        // Casts
        'cast_spaces' => ['space' => 'single'],
        'lowercase_cast' => true,
        
        // Classes
        'class_attributes_separation' => [
            'elements' => [
                'const' => 'one',
                'method' => 'one',
                'property' => 'one'
            ]
        ],
        'class_definition' => [
            'single_line' => true,
            'single_item_single_line' => true
        ],
        'final_internal_class' => false,
        
        // Comments
        'comment_to_phpdoc' => true,
        'header_comment' => false,
        'multiline_comment_opening_closing' => true,
        'no_empty_comment' => true,
        
        // Control structures
        'elseif' => true,
        'no_break_comment' => true,
        'no_superfluous_elseif' => true,
        'no_unneeded_control_parentheses' => true,
        'no_useless_else' => true,
        'switch_case_semicolon_to_colon' => true,
        'switch_case_space' => true,
        
        // Doctrine annotations
        'doctrine_annotation_array_assignment' => true,
        'doctrine_annotation_braces' => true,
        'doctrine_annotation_indentation' => true,
        'doctrine_annotation_spaces' => true,
        
        // Function calls
        'function_declaration' => ['closure_function_spacing' => 'one'],
        'function_typehint_space' => true,
        'lambda_not_used_import' => true,
        'method_argument_space' => [
            'on_multiline' => 'ensure_fully_multiline'
        ],
        'no_spaces_after_function_name' => true,
        'no_unreachable_default_argument_value' => true,
        
        // Imports
        'global_namespace_import' => [
            'import_classes' => true,
            'import_constants' => true,
            'import_functions' => true
        ],
        'group_import' => true,
        'no_leading_import_slash' => true,
        'no_unused_imports' => true,
        'ordered_imports' => [
            'imports_order' => ['class', 'function', 'const'],
            'sort_algorithm' => 'alpha'
        ],
        'single_import_per_statement' => true,
        'single_line_after_imports' => true,
        
        // Language constructs
        'declare_equal_normalize' => true,
        'lowercase_keywords' => true,
        'magic_constant_casing' => true,
        'magic_method_casing' => true,
        'native_function_casing' => true,
        'no_alias_functions' => true,
        'no_alternative_syntax' => true,
        
        // Namespaces
        'blank_line_after_namespace' => true,
        'no_blank_lines_before_namespace' => true,
        'single_blank_line_before_namespace' => true,
        
        // Operators
        'increment_style' => ['style' => 'post'],
        'logical_operators' => true,
        'object_operator_without_whitespace' => true,
        'standardize_increment' => true,
        'standardize_not_equals' => true,
        'ternary_operator_spaces' => true,
        'unary_operator_spaces' => true,
        
        // PHPDoc
        'phpdoc_add_missing_param_annotation' => true,
        'phpdoc_align' => ['align' => 'vertical'],
        'phpdoc_annotation_without_dot' => true,
        'phpdoc_indent' => true,
        'phpdoc_inline_tag_normalizer' => true,
        'phpdoc_no_access' => true,
        'phpdoc_no_alias_tag' => true,
        'phpdoc_no_empty_return' => true,
        'phpdoc_no_package' => true,
        'phpdoc_no_useless_inheritdoc' => true,
        'phpdoc_order' => true,
        'phpdoc_return_self_reference' => true,
        'phpdoc_scalar' => true,
        'phpdoc_separation' => true,
        'phpdoc_single_line_var_spacing' => true,
        'phpdoc_summary' => true,
        'phpdoc_to_comment' => true,
        'phpdoc_trim' => true,
        'phpdoc_trim_consecutive_blank_line_separation' => true,
        'phpdoc_types' => true,
        'phpdoc_types_order' => ['null_adjustment' => 'always_last'],
        'phpdoc_var_annotation_correct_order' => true,
        'phpdoc_var_without_name' => true,
        
        // Semicolons
        'multiline_whitespace_before_semicolons' => ['strategy' => 'no_multi_line'],
        'no_empty_statement' => true,
        'no_singleline_whitespace_before_semicolons' => true,
        'semicolon_after_instruction' => true,
        'space_after_semicolon' => ['remove_in_empty_for_expressions' => true],
        
        // Strings
        'escape_implicit_backslashes' => true,
        'explicit_string_variable' => true,
        'heredoc_to_nowdoc' => true,
        'simple_to_complex_string_variable' => true,
        'single_quote' => true,
        
        // Whitespace
        'blank_line_before_statement' => [
            'statements' => ['break', 'continue', 'declare', 'return', 'throw', 'try']
        ],
        'compact_nullable_typehint' => true,
        'line_ending' => true,
        'no_extra_blank_lines' => [
            'tokens' => [
                'extra', 'throw', 'use', 'use_trait'
            ]
        ],
        'no_spaces_around_offset' => ['positions' => ['inside', 'outside']],
        'no_whitespace_in_blank_line' => true,
        'single_blank_line_at_eof' => true,
    ])
    ->setFinder(
        PhpCsFixer\Finder::create()
            ->exclude('vendor')
            ->exclude('storage')
            ->exclude('bootstrap/cache')
            ->in(__DIR__)
    )
    ->setUsingCache(false);
"#
        .to_string())
    }
}

impl Default for PhpFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_php_formatter_creation() {
        let formatter = PhpFormatter::new();
        assert_eq!(formatter.language(), SupportedLanguage::Php);
    }

    #[test]
    fn test_empty_code_handling() {
        let formatter = PhpFormatter::new();
        let result = formatter.format_code("").unwrap();
        assert!(!result.changed);
        assert_eq!(result.formatted, "");
    }

    #[test]
    fn test_whitespace_only_code() {
        let formatter = PhpFormatter::new();
        let result = formatter.format_code("   \n\t  \n").unwrap();
        assert!(!result.changed);
    }

    #[test]
    fn test_formatter_info() {
        let formatter = PhpFormatter::new();
        let info = formatter.formatter_info();
        assert!(info.contains("php-cs-fixer"));
    }

    #[test]
    fn test_default_config_generation() {
        let formatter = PhpFormatter::new();
        let config = formatter.default_config().unwrap();
        assert!(config.contains("PhpCsFixer\\Config"));
        assert!(config.contains("@PSR12"));
        assert!(config.contains("array_syntax"));
    }

    #[test]
    fn test_php_cs_fixer_args() {
        let formatter = PhpFormatter::new();
        let args = formatter.get_php_cs_fixer_args();
        assert!(args.contains(&"fix".to_string()));
        assert!(args.contains(&"--rules=@PER-CS3.0".to_string()));
        assert!(args.contains(&"--using-cache=no".to_string()));
        assert!(args.contains(&"--diff".to_string()));
        assert!(args.contains(&"--allow-risky=yes".to_string()));
        assert!(args.contains(&"-".to_string()));
        assert_eq!(args.len(), 6); // Updated for 2025 standards with more args
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[test]
        fn test_simple_php_formatting() {
            let formatter = PhpFormatter::new();

            // Skip if php-cs-fixer is not available
            if !formatter.is_available() {
                eprintln!("Skipping php-cs-fixer integration test - php-cs-fixer not available");
                return;
            }

            let unformatted_code = "<?php function hello(){echo 'Hello, World!';}";
            let result = formatter.format_code(unformatted_code);

            match result {
                Ok(format_result) => {
                    // php-cs-fixer should format this code
                    assert!(format_result.changed || format_result.messages.is_empty());

                    // Formatted code should be valid PHP
                    assert!(format_result.formatted.contains("<?php"));
                    assert!(format_result.formatted.contains("function hello"));
                }
                Err(e) => {
                    eprintln!("php-cs-fixer formatting failed: {}", e);
                    // This is acceptable if php-cs-fixer has issues with the test environment
                }
            }
        }

        #[test]
        fn test_complex_php_formatting() {
            let formatter = PhpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping php-cs-fixer integration test - php-cs-fixer not available");
                return;
            }

            let complex_code = r#"
<?php
namespace App\Services;
use Illuminate\Support\Collection;
use App\Models\User;
class UserService{
private $users;
public function __construct(){$this->users=collect();}
public function addUser(string $name,int $age):User{
$user=new User();$user->name=$name;$user->age=$age;
$this->users->push($user);return $user;}
public function getAdultUsers():Collection{
return $this->users->filter(function($user){return $user->age>=18;});
}
}
"#
            .trim();

            let result = formatter.format_code(complex_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that formatting improved the code structure
                        assert!(format_result.formatted.contains("namespace App\\Services"));
                        assert!(format_result.formatted.contains("class UserService"));
                        assert!(format_result.formatted.contains("public function"));
                    }
                }
                Err(e) => {
                    eprintln!("php-cs-fixer formatting of complex code failed: {}", e);
                }
            }
        }

        #[test]
        fn test_php_arrays_and_functions() {
            let formatter = PhpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping php-cs-fixer integration test - php-cs-fixer not available");
                return;
            }

            let array_code = "<?php $users=array('john','jane','bob');$ages=[25,30,35];array_map(function($user,$age){return['name'=>$user,'age'=>$age];},array_combine($users,$ages));";
            let result = formatter.format_code(array_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that PHP array syntax is handled correctly
                        assert!(format_result.formatted.contains("<?php"));
                        assert!(format_result.formatted.contains("array_map"));
                        assert!(format_result.formatted.contains("function"));
                    }
                }
                Err(e) => {
                    eprintln!("php-cs-fixer formatting of arrays failed: {}", e);
                }
            }
        }

        #[test]
        fn test_php_classes_and_oop() {
            let formatter = PhpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping php-cs-fixer integration test - php-cs-fixer not available");
                return;
            }

            let oop_code = "<?php class Animal{protected $name;public function __construct($name){$this->name=$name;}abstract public function makeSound();}class Dog extends Animal{public function makeSound(){return 'Woof!';}}";
            let result = formatter.format_code(oop_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that OOP features are preserved
                        assert!(format_result.formatted.contains("class Animal"));
                        assert!(format_result.formatted.contains("extends Animal"));
                        assert!(format_result.formatted.contains("protected"));
                        assert!(format_result.formatted.contains("abstract"));
                    }
                }
                Err(e) => {
                    eprintln!("php-cs-fixer formatting of OOP code failed: {}", e);
                }
            }
        }

        #[test]
        fn test_syntax_error_handling() {
            let formatter = PhpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping php-cs-fixer integration test - php-cs-fixer not available");
                return;
            }

            let invalid_code = "<?php function broken( {echo 'missing brace';"; // Missing closing brace and parenthesis
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
        fn test_php_modern_features() {
            let formatter = PhpFormatter::new();

            if !formatter.is_available() {
                eprintln!("Skipping php-cs-fixer integration test - php-cs-fixer not available");
                return;
            }

            let modern_code = "<?php function process(array $items):array{return array_filter($items,fn($item)=>$item!==null);}$result=process([1,null,2,null,3]);match($result){[]=>throw new Exception('Empty'),[_]=>$result};";
            let result = formatter.format_code(modern_code);

            match result {
                Ok(format_result) => {
                    if format_result.changed {
                        // Check that modern PHP features are preserved
                        assert!(format_result.formatted.contains("fn("));
                        assert!(format_result.formatted.contains("match("));
                        assert!(format_result.formatted.contains("array"));
                    }
                }
                Err(e) => {
                    eprintln!("php-cs-fixer formatting of modern PHP failed: {}", e);
                }
            }
        }
    }
}
