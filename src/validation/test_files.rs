// Module for validating test file placement
use std::path::Path;
use std::collections::HashSet;
use std::env;

/// Test file validation configuration
#[derive(Debug, Clone)]
pub struct TestFileConfig {
    /// Allowed directories for test files (stored as HashSet for O(1) lookup)
    pub test_directories: HashSet<String>,
    /// Test file patterns to check (stored as HashSet for O(1) contains check)
    pub test_patterns: HashSet<String>,
    /// File extensions that commonly indicate test files
    pub test_extensions: HashSet<String>,
    /// Whether to block test files outside test directories
    pub strict_mode: bool,
}

impl TestFileConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        // Check if test file validation is enabled
        let enabled = env::var("ENABLE_TEST_FILE_VALIDATION")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);
        
        if !enabled {
            // Return config with strict_mode=false to disable blocking
            return Self {
                test_directories: HashSet::new(),
                test_patterns: HashSet::new(),
                test_extensions: HashSet::new(),
                strict_mode: false,
            };
        }
        
        // Read strict mode setting
        let strict_mode = env::var("TEST_FILE_STRICT_MODE")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);
        
        // Read test directories from env or use defaults
        let test_directories = env::var("TEST_DIRECTORIES")
            .map(|dirs| {
                dirs.split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect::<HashSet<String>>()
            })
            .unwrap_or_else(|_| Self::default_test_directories());
        
        // Read test patterns from env or use defaults
        let test_patterns = env::var("TEST_PATTERNS")
            .map(|patterns| {
                patterns.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<HashSet<String>>()
            })
            .unwrap_or_else(|_| Self::default_test_patterns());
        
        // Test extensions are not configurable via env for now
        let test_extensions = Self::default_test_extensions();
        
        Self {
            test_directories,
            test_patterns,
            test_extensions,
            strict_mode,
        }
    }
    
    fn default_test_directories() -> HashSet<String> {
        let mut dirs = HashSet::new();
        dirs.insert("test".to_string());
        dirs.insert("tests".to_string());
        dirs.insert("__tests__".to_string());
        dirs.insert("spec".to_string());
        dirs.insert("specs".to_string());
        dirs.insert("test_fixtures".to_string());
        dirs.insert("fixtures".to_string());
        dirs.insert(".test".to_string());
        dirs.insert("testing".to_string());
        dirs
    }
    
    fn default_test_patterns() -> HashSet<String> {
        let mut patterns = HashSet::new();
        // Word boundary patterns - will be checked with word boundaries
        patterns.insert("mock".to_string());
        patterns.insert("stub".to_string());
        patterns.insert("fake".to_string());
        patterns.insert("dummy".to_string());
        patterns.insert("fixture".to_string());
        patterns.insert("example".to_string());
        patterns.insert("sample".to_string());
        patterns.insert("demo".to_string());
        patterns
    }
    
    fn default_test_extensions() -> HashSet<String> {
        let mut extensions = HashSet::new();
        extensions.insert("_test.js".to_string());
        extensions.insert("_test.ts".to_string());
        extensions.insert("_test.py".to_string());
        extensions.insert("_test.rs".to_string());
        extensions.insert("_test.go".to_string());
        extensions.insert(".test.js".to_string());
        extensions.insert(".test.ts".to_string());
        extensions.insert(".test.jsx".to_string());
        extensions.insert(".test.tsx".to_string());
        extensions.insert(".spec.js".to_string());
        extensions.insert(".spec.ts".to_string());
        extensions.insert("_spec.rb".to_string());
        extensions
    }
}

impl Default for TestFileConfig {
    fn default() -> Self {
        Self {
            test_directories: Self::default_test_directories(),
            test_patterns: Self::default_test_patterns(),
            test_extensions: Self::default_test_extensions(),
            strict_mode: true,
        }
    }
}

/// Result of test file validation
#[derive(Debug)]
pub struct TestFileValidation {
    pub is_test_file: bool,
    pub is_in_test_directory: bool,
    pub should_block: bool,
    pub reason: String,
    pub matched_pattern: Option<String>,
    pub matched_directory: Option<String>,
}

/// Validates if a file is a test file and whether it should be blocked
pub fn validate_test_file(file_path: &str, config: &TestFileConfig) -> TestFileValidation {
    let path = Path::new(file_path);
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // Check if file matches test patterns or extensions (optimized checks)
    let (pattern_match, matched_pattern) = check_test_patterns(&file_name, &config.test_patterns);
    let extension_match = check_test_extension(&file_name, &config.test_extensions);
    let is_test_file = pattern_match || extension_match;
    let matched_pattern = if pattern_match {
        matched_pattern
    } else if extension_match {
        Some("test extension".to_string())
    } else {
        None
    };
    
    // Check if file is in a test directory
    let (is_in_test_directory, matched_directory) = check_test_directory(file_path, &config.test_directories);
    
    // Determine if should block
    let should_block = if !is_test_file {
        // Not a test file, allow
        false
    } else if is_in_test_directory {
        // Test file in correct location, allow
        false
    } else if config.strict_mode {
        // Test file outside test directory in strict mode, block
        true
    } else {
        // Test file outside test directory but not strict mode, allow
        false
    };
    
    // Generate reason
    let reason = if should_block {
        let dirs: Vec<&String> = config.test_directories.iter().collect();
        format!(
            "Test file '{}' detected outside of designated test directories. Test files should be placed in: {}",
            file_name,
            dirs.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        )
    } else if is_test_file && is_in_test_directory {
        format!("Test file correctly placed in test directory")
    } else if is_test_file {
        format!("Test file detected but strict mode is disabled")
    } else {
        format!("Not a test file")
    };
    
    TestFileValidation {
        is_test_file,
        is_in_test_directory,
        should_block,
        reason,
        matched_pattern,
        matched_directory,
    }
}

/// Check if filename matches test patterns with word boundary checking
fn check_test_patterns(file_name: &str, patterns: &HashSet<String>) -> (bool, Option<String>) {
    // Check prefix patterns first (most specific)
    if file_name.starts_with("test_") || file_name.starts_with("spec_") {
        return (true, Some("prefix pattern".to_string()));
    }
    
    // Check infix patterns with underscore or dot boundaries
    if file_name.contains("_test.") || file_name.contains(".test.") ||
       file_name.contains("_spec.") || file_name.contains(".spec.") {
        return (true, Some("infix test pattern".to_string()));
    }
    
    // For word patterns, check with word boundaries to avoid false positives
    // Split filename into parts by common delimiters
    let parts: Vec<&str> = file_name.split(&['.', '_', '-'][..]).collect();
    
    for pattern in patterns {
        // Check if any part exactly matches the pattern (word boundary check)
        for part in &parts {
            if part.eq_ignore_ascii_case(pattern) {
                return (true, Some(format!("word pattern: {}", pattern)));
            }
        }
    }
    
    (false, None)
}

/// Check if file extension indicates a test file (O(1) lookup)
fn check_test_extension(file_name: &str, extensions: &HashSet<String>) -> bool {
    for ext in extensions {
        if file_name.ends_with(ext) {
            return true;
        }
    }
    false
}

/// Check if file is in a test directory using path components (optimized with HashSet)
fn check_test_directory(file_path: &str, test_dirs: &HashSet<String>) -> (bool, Option<String>) {
    // Security validation: reject dangerous path patterns
    if file_path.is_empty() || file_path.contains('\0') {
        return (false, None);
    }
    
    // Check for path traversal attempts
    if file_path.contains("..") {
        return (false, None);
    }
    
    let path = Path::new(file_path);
    
    // Iterate through path components to find test directories
    for component in path.components() {
        if let std::path::Component::Normal(dir_name) = component {
            if let Some(dir_str) = dir_name.to_str() {
                // Check against configured test directories (O(1) lookup)
                let dir_lower = dir_str.to_ascii_lowercase();
                if test_dirs.contains(&dir_lower) {
                    return (true, Some(dir_lower));
                }
                // Also check exact match for case-sensitive systems
                if test_dirs.contains(dir_str) {
                    return (true, Some(dir_str.to_string()));
                }
            }
        }
    }
    
    (false, None)
}

/// Advanced test file detection using content analysis (excludes comments)
pub fn detect_test_content(content: &str) -> bool {
    // First, remove comments to avoid false positives
    let code_without_comments = strip_comments(content);
    let content_lower = code_without_comments.to_lowercase();
    
    // Strong indicators - test frameworks
    let test_frameworks = [
        // JavaScript/TypeScript
        "describe(",
        "it(",
        "test(",
        "expect(",
        "jest.",
        "mocha",
        "chai",
        "vitest",
        "@testing-library",
        "beforeeach(",
        "aftereach(",
        "beforeall(",
        "afterall(",
        
        // Python
        "import unittest",
        "import pytest",
        "from unittest",
        "def test_",
        "class test",
        "@pytest.",
        
        // Rust
        "#[test]",
        "#[cfg(test)]",
        "mod tests {",
        
        // Go
        "func test",
        "testing.t",
        
        // Ruby
        "rspec",
        "describe '",
        "context '",
    ];
    
    // Check for test frameworks (strong signal)
    for framework in &test_frameworks {
        if content_lower.contains(framework) {
            return true;
        }
    }
    
    // Check for mock/stub/fake IMPLEMENTATIONS (not just mentions)
    // Must have both the word AND implementation patterns
    let has_mock_words = check_for_mock_words(&content_lower);
    let has_mock_implementation = check_for_mock_implementation(&content_lower);
    
    if has_mock_words && has_mock_implementation {
        return true;
    }
    
    // Check for high concentration of test-related words (excluding common false positives)
    let test_word_count = count_test_words(&content_lower);
    let line_count = content.lines().count().max(1);
    
    // Higher threshold to reduce false positives
    if test_word_count > 0 && (test_word_count * 100 / line_count) > 10 {
        return true;
    }
    
    false
}

/// Strip comments from code to avoid false positives
fn strip_comments(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut in_string = false;
    let mut string_char = ' ';
    
    while let Some(ch) = chars.next() {
        // Handle string literals
        if !in_line_comment && !in_block_comment {
            if ch == '"' || ch == '\'' || ch == '`' {
                if !in_string {
                    in_string = true;
                    string_char = ch;
                } else if ch == string_char {
                    in_string = false;
                }
                result.push(ch);
                continue;
            }
        }
        
        if in_string {
            result.push(ch);
            if ch == '\\' {
                // Skip escaped character
                if let Some(next_ch) = chars.next() {
                    result.push(next_ch);
                }
            }
            continue;
        }
        
        // Check for comment starts
        if !in_line_comment && !in_block_comment {
            if ch == '/' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '/' {
                        in_line_comment = true;
                        chars.next(); // consume second /
                        continue;
                    } else if next_ch == '*' {
                        in_block_comment = true;
                        chars.next(); // consume *
                        continue;
                    }
                }
                result.push(ch);
            } else {
                result.push(ch);
            }
        } else if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                result.push(ch); // Keep newline
            }
        } else if in_block_comment {
            if ch == '*' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '/' {
                        in_block_comment = false;
                        chars.next(); // consume /
                    }
                }
            }
        }
    }
    
    result
}

/// Check for mock/stub/fake words (but not in common false positive contexts)
fn check_for_mock_words(content: &str) -> bool {
    // Words that indicate mocking/stubbing
    let mock_words = ["mock", "stub", "fake", "dummy"];
    
    for word in &mock_words {
        if content.contains(word) {
            return true;
        }
    }
    
    false
}

/// Check for actual mock implementation patterns
fn check_for_mock_implementation(content: &str) -> bool {
    // Patterns that indicate actual mock implementation
    let impl_patterns = [
        "return true;",  // Common stub pattern
        "return false;",
        "return {};",
        "return [];",
        "{ success: true",
        "throw new error('not implemented')",
        "notimplemented",
        "pass", // Python stub
    ];
    
    for pattern in &impl_patterns {
        if content.contains(pattern) {
            return true;
        }
    }
    
    false
}

/// Count test-related words, excluding false positives
fn count_test_words(content: &str) -> usize {
    let mut count = 0;
    
    // Create a whitelist of words that contain "test" but are not test-related
    let whitelist = ["contest", "contests", "contested", "latest", "fastest", "attestation", 
                      "greatest", "protest", "intestate", "detest", "manifest"];
    
    // Split content into proper words (alphanumeric sequences)
    let mut current_word = String::new();
    let mut words = Vec::new();
    
    for ch in content.chars() {
        if ch.is_alphanumeric() {
            current_word.push(ch);
        } else {
            if !current_word.is_empty() {
                words.push(current_word.clone());
                current_word.clear();
            }
        }
    }
    if !current_word.is_empty() {
        words.push(current_word);
    }
    
    for word in words {
        let word_lower = word.to_lowercase();
        
        // Skip whitelisted words
        if whitelist.iter().any(|&w| word_lower == w) {
            continue;
        }
        
        // Check for exact word matches (not substrings)
        match word_lower.as_str() {
            "test" | "tests" | "testing" | "tested" | "tester" => count += 1,
            "expect" | "expects" | "expected" | "expecting" | "expectation" => count += 1,
            "assert" | "asserts" | "assertion" | "assertions" | "asserting" => count += 1,
            _ => {}
        }
    }
    
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_test_file_in_correct_location() {
        let config = TestFileConfig::default();
        
        let result = validate_test_file("tests/test_example.js", &config);
        assert!(result.is_test_file);
        assert!(result.is_in_test_directory);
        assert!(!result.should_block);
    }
    
    #[test]
    fn test_validate_test_file_outside_test_dir() {
        let config = TestFileConfig::default();
        
        let result = validate_test_file("src/test_example.js", &config);
        assert!(result.is_test_file);
        assert!(!result.is_in_test_directory);
        assert!(result.should_block);
    }
    
    #[test]
    fn test_validate_non_test_file() {
        let config = TestFileConfig::default();
        
        let result = validate_test_file("src/main.rs", &config);
        assert!(!result.is_test_file);
        assert!(!result.should_block);
    }
    
    #[test]
    fn test_detect_test_content() {
        let test_code = r#"
            describe('Calculator', () => {
                it('should add numbers', () => {
                    expect(add(2, 3)).toBe(5);
                });
            });
        "#;
        
        assert!(detect_test_content(test_code));
        
        let normal_code = r#"
            function calculateSum(a, b) {
                return a + b;
            }
        "#;
        
        assert!(!detect_test_content(normal_code));
    }
}