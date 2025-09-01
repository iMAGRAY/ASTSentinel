/// Advanced AI Deception Detector with context-aware analysis
/// Zero false positives through intelligent pattern matching
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use lazy_static::lazy_static;

/// Patterns that commonly appear in deceptive code but may be legitimate in strings
/// These are pre-lowercased for efficient comparison
static SUSPICIOUS_STRING_PATTERNS: &[&str] = &[
    "todo", "fixme", "implement", "mock", "fake", "stub", "dummy", 
    "placeholder", "coming soon", "work in progress", "not implemented"
];


/// Report containing analysis results from deception detection
#[derive(Debug, Clone)]
pub struct DeceptionReport {
    pub is_deceptive: bool,
    pub severity: DeceptionSeverity,
    pub violations: Vec<Violation>,
    pub confidence: f32,
    pub recommendation: String,
}

/// Individual violation found during deception analysis
#[derive(Debug, Clone)]
pub struct Violation {
    pub category: ViolationCategory,
    pub message: String,
    pub line_number: Option<usize>,
    pub confidence: f32,
    pub context: String,
}

/// Categories of deceptive patterns that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ViolationCategory {
    IncompleteTodo,
    FakeImplementation,
    MockCode,
    ErrorHiding,
    HardcodedLogic,
    SimulatedWork,
    DeceptiveComment,
}

/// Severity levels for detected deceptions
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DeceptionSeverity {
    None,
    Low,      
    Medium,   
    High,     
    Critical,
}

/// Context analyzer for understanding code semantics
/// Provides detailed analysis of code structure and characteristics
struct CodeContext {
    /// Whether the code contains actual business logic (conditionals, loops, etc.)
    has_real_logic: bool,
    /// Whether proper error handling is present
    has_error_handling: bool,
    /// Number of functions/methods defined in the file
    function_count: usize,
    /// Whether this is a test file based on path/name
    is_test_file: bool,
    /// Whether this is a configuration file
    is_config_file: bool,
    /// Whether this is a mock/stub file (allowed in tests)
    is_mock_file: bool,
    /// Whether this is a type definition file (d.ts, interfaces)
    is_type_definition: bool,
    /// Whether this is a development/staging file
    is_dev_file: bool,
    /// Programming language detected
    language: Language,
}

/// Supported programming languages for context-aware detection
#[derive(Debug, Clone, PartialEq)]
enum Language {
    JavaScript,
    TypeScript,
    Python,
    Rust,
    Go,
    Java,
    CSharp,
    Unknown,
}

impl CodeContext {
    fn analyze(content: &str, file_path: &str) -> Self {
        // Detect language
        let language = detect_language(file_path, content);
        
        // Analyze file type
        let path_lower = file_path.to_lowercase();
        let is_test_file = path_lower.contains("/test") || 
                          path_lower.contains("test.") || 
                          path_lower.contains(".spec.") ||
                          path_lower.contains("_test.") ||
                          path_lower.contains(".test.");
                          
        let is_config_file = path_lower.ends_with(".config.js") ||
                            path_lower.ends_with(".config.ts") ||
                            path_lower.ends_with("config.json") ||
                            path_lower.ends_with(".env") ||
                            path_lower.ends_with("settings.py");
                            
        let is_mock_file = is_test_file && (
            path_lower.contains("mock") ||
            path_lower.contains("__mocks__") ||
            path_lower.contains("fixture")
        );
        
        // Detect if this is a dev/staging file
        let is_dev_file = path_lower.contains(".dev.") ||
                         path_lower.contains(".development.") ||
                         path_lower.contains(".staging.") ||
                         path_lower.contains("/dev/") ||
                         path_lower.contains("/staging/") ||
                         path_lower.contains("seed") ||
                         path_lower.contains("fixture") ||
                         path_lower.contains("example");
        
        // Count real logic patterns using precompiled regexes
        lazy_static! {
            static ref LOGIC_PATTERNS: Vec<Regex> = vec![
                Regex::new(r"\b(if|else if|else)\b").expect("Valid regex"),
                Regex::new(r"\b(for|while|do)\b").expect("Valid regex"),
                Regex::new(r"\b(switch|case|match)\b").expect("Valid regex"),
                Regex::new(r"\b(map|filter|reduce|forEach)\b").expect("Valid regex"),
                Regex::new(r"[+\-*/]").expect("Valid regex"),
                Regex::new(r"[<>]=?").expect("Valid regex"),
                Regex::new(r"&&|\|\|").expect("Valid regex"),
                Regex::new(r"\?.+:").expect("Valid regex"),
            ];
        }
        
        let mut logic_score = 0;
        for regex in LOGIC_PATTERNS.iter() {
            logic_score += regex.find_iter(content).count();
        }
        
        let has_real_logic = logic_score > 3;
        
        // Check error handling
        let error_patterns = match language {
            Language::JavaScript | Language::TypeScript => {
                vec![
                    r#"try\s*\{[^}]+\}\s*catch\s*\([^)]+\)\s*\{[^}]+\}"#,
                    r#"\.catch\s*\([^)]+\)"#,
                    r#"throw\s+new\s+Error"#,
                    r#"Promise\.reject"#,
                ]
            },
            Language::Python => {
                vec![
                    r#"try:\s*\n.+except\s+\w+"#,
                    r#"raise\s+\w+"#,
                    r#"with\s+\w+.*:"#,
                ]
            },
            Language::Rust => {
                vec![
                    r#"Result<"#,
                    r#"Option<"#,
                    r#"\.unwrap_or"#,
                    r#"\?;"#,
                ]
            },
            _ => vec![],
        };
        
        let mut has_error_handling = false;
        for pattern in &error_patterns {
            if Regex::new(pattern).unwrap().is_match(content) {
                has_error_handling = true;
                break;
            }
        }
        
        
        // Count functions
        let function_patterns = match language {
            Language::JavaScript | Language::TypeScript => {
                vec![r#"function\s+\w+"#, r#"const\s+\w+\s*=\s*(?:async\s*)?\("#, r#"=>\s*\{"#]
            },
            Language::Python => {
                vec![r#"def\s+\w+"#, r#"class\s+\w+"#]
            },
            Language::Rust => {
                vec![r#"fn\s+\w+"#, r#"impl\s+"#]
            },
            _ => vec![],
        };
        
        let mut function_count = 0;
        for pattern in &function_patterns {
            function_count += Regex::new(pattern).unwrap().find_iter(content).count();
        }
        
        
        let is_type_definition = path_lower.ends_with(".d.ts") ||
                                (language == Language::TypeScript && 
                                 (content.contains("interface ") || 
                                  content.contains("type ") ||
                                  content.contains("enum "))) ||
                                (language == Language::Python && 
                                 content.contains("TypedDict") ||
                                 content.contains("@dataclass"));
        
        CodeContext {
            has_real_logic,
            has_error_handling,
            function_count,
            is_test_file,
            is_config_file,
            is_mock_file,
            is_type_definition,
            is_dev_file,
            language,
        }
    }
}

/// Check if suspicious code appears only in test contexts (strings, assertions)
fn has_code_only_in_test_contexts(content: &str) -> bool {
    // This function checks if ALL suspicious code is inside string literals or test assertions
    // If ANY suspicious code is found in actual executable code, return false
    
    let lines: Vec<&str> = content.lines().collect();
    let mut in_template_literal = false;
    
    for (_i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Track multi-line template literals
        if line.contains('`') {
            let backtick_count = line.matches('`').count();
            if backtick_count % 2 == 1 {
                in_template_literal = !in_template_literal;
            }
        }
        
        // Skip if we're inside a template literal
        if in_template_literal {
            continue;
        }
        
        // Skip pure comment lines
        if trimmed.starts_with("//") || trimmed.starts_with("#") || 
           trimmed.starts_with("*") || trimmed.starts_with("/*") {
            continue;
        }
        
        // Skip lines that are defining test code examples in strings
        if line.contains("const code = ") || line.contains("const example = ") ||
           line.contains("let code = ") || line.contains("let example = ") ||
           line.contains("var code = ") || line.contains("var example = ") {
            continue;
        }
        
        // Skip lines inside test assertions
        if line.contains("expect(") || line.contains("assert") || 
           line.contains("should(") || line.contains(".toBe(") ||
           line.contains("test(") || line.contains("it(") ||
           line.contains("describe(") {
            continue;
        }
        
        // Now check for dangerous patterns in actual code
        let line_lower = line.to_lowercase();
        
        // These patterns in executable code are always suspicious
        if (line_lower.contains("not") && line_lower.contains("implemented")) ||
           (line.contains("throw") && line.contains("Error") && 
            (line_lower.contains("not") || line_lower.contains("implement"))) ||
           line_lower.contains("coming soon") ||
           line_lower.contains("placeholder") ||
           line_lower.contains("todo") || line_lower.contains("fixme") {
            // Found suspicious pattern in actual code
            return false;
        }
    }
    
    // All suspicious patterns were in safe contexts
    true
}

/// Check if this is a test file containing code examples (not production code)
fn is_test_file_with_examples(file_path: &str, content: &str) -> bool {
    let path_lower = file_path.to_lowercase();
    
    // Check if it's a test file
    if !path_lower.contains("test") && !path_lower.contains("spec") && 
       !path_lower.ends_with("_test.rs") && !path_lower.ends_with("_test.js") &&
       !path_lower.ends_with(".test.js") && !path_lower.ends_with(".spec.js") {
        return false;
    }
    
    // Check for test framework patterns (these files contain example code)
    let test_patterns = [
        r#"#\[test\]"#,           // Rust tests
        r#"#\[cfg\(test\)\]"#,    // Rust test modules
        r#"describe\(["']"#,      // JavaScript test suites
        r#"it\(["']"#,            // JavaScript test cases
        r#"test\(["']"#,          // Jest/other test frameworks
        r#"@Test"#,               // Java tests
        r#"def test_"#,           // Python tests
        r#"class Test"#,          // Test classes
    ];
    
    for pattern in &test_patterns {
        if content.contains(pattern) {
            return true;
        }
    }
    
    // Check for code inside string literals (common in tests)
    if content.contains(r#"r#""#) || // Rust raw strings
       content.contains("```") ||     // Markdown code blocks
       content.contains("const code = `") || // Template literals with code
       content.contains("const example = `") || // Example code in tests
       content.matches(r#"["']"#).count() > 20 { // Many quoted strings
        return true;
    }
    
    false
}

/// Enhanced documentation file detection - covers all directories and formats
fn is_documentation_file(file_path: &str) -> bool {
    let path_lower = file_path.to_lowercase();
    
    // File extension based detection
    if path_lower.ends_with(".md") || path_lower.ends_with(".rst") || 
       path_lower.ends_with(".txt") || path_lower.ends_with(".adoc") ||
       path_lower.ends_with(".wiki") || path_lower.ends_with(".org") {
        return true;
    }
    
    // Directory-based detection (cross-platform)
    if path_lower.contains("/docs/") || path_lower.contains("\\docs\\") ||
       path_lower.contains("/doc/") || path_lower.contains("\\doc\\") ||
       path_lower.contains("/documentation/") || path_lower.contains("\\documentation\\") ||
       path_lower.contains("/man/") || path_lower.contains("\\man\\") ||
       path_lower.contains("/.github/") || path_lower.contains("\\.github\\") {
        return true;
    }
    
    // Common documentation filenames (anywhere in path)
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
        
    if filename.contains("readme") || filename.contains("changelog") ||
       filename.contains("license") || filename.contains("contributing") ||
       filename.contains("authors") || filename.contains("credits") ||
       filename.contains("history") || filename.contains("news") ||
       filename.contains("install") || filename.contains("usage") ||
       filename.contains("tutorial") || filename.contains("guide") ||
       filename == "todo" || filename == "fixme" {
        return true;
    }
    
    false
}

fn detect_language(file_path: &str, content: &str) -> Language {
    let ext = file_path.split('.').last().unwrap_or("").to_lowercase();
    
    match ext.as_str() {
        "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
        "ts" | "tsx" => Language::TypeScript,
        "py" | "pyw" => Language::Python,
        "rs" => Language::Rust,
        "go" => Language::Go,
        "java" => Language::Java,
        "cs" => Language::CSharp,
        _ => {
            // Try to detect from content
            if content.contains("function") || content.contains("const ") {
                Language::JavaScript
            } else if content.contains("def ") || content.contains("import ") {
                Language::Python
            } else if content.contains("fn ") || content.contains("impl ") {
                Language::Rust
            } else {
                Language::Unknown
            }
        }
    }
}

/// Main detection function with smart context analysis
pub fn detect_deception(content: &str, file_path: &str) -> DeceptionReport {
    let mut violations = Vec::new();
    let context = CodeContext::analyze(content, file_path);
    
    // Enhanced documentation file detection - skip validation for ALL documentation
    if is_documentation_file(file_path) {
        return DeceptionReport {
            is_deceptive: false,
            severity: DeceptionSeverity::None,
            violations: vec![],
            confidence: 0.0,
            recommendation: "Documentation file - validation skipped".to_string(),
        };
    }
    
    // For test files, we still check for deception but are more lenient
    // Only skip if it's clearly a test file with code examples in strings
    let is_test_with_examples = is_test_file_with_examples(file_path, content);
    
    // Skip ONLY if this is a proper test file with test framework code
    // and the suspicious code is inside string literals or test assertions
    if is_test_with_examples && has_code_only_in_test_contexts(content) {
        return DeceptionReport {
            is_deceptive: false,
            severity: DeceptionSeverity::None,
            violations: vec![],
            confidence: 0.0,
            recommendation: "Test file with code examples in strings/assertions - validation skipped".to_string(),
        };
    }
    
    // Always check for TODO patterns (including obfuscated ones)
    check_todo_patterns(content, &context, &mut violations);
    
    // Check for fake implementations
    check_fake_implementations(content, &context, &mut violations);
    
    // Always check for mock patterns
    // Mock/fake/stub code should be detected everywhere
    check_mock_patterns(content, file_path, &context, &mut violations);
    
    // Check for error hiding
    check_error_hiding(content, &context, &mut violations);
    
    // Check for hardcoded returns only if no real logic
    if !context.has_real_logic && context.function_count > 0 {
        check_hardcoded_returns(content, &context, &mut violations);
    }
    
    // Check for simulated work
    check_simulated_work(content, &context, &mut violations);
    
    // Calculate overall severity and confidence
    let (severity, confidence) = calculate_severity(&violations, &context);
    
    // Generate smart recommendation
    let recommendation = generate_recommendation(&violations, &severity, &context);
    
    DeceptionReport {
        is_deceptive: !violations.is_empty() && severity >= DeceptionSeverity::High,
        severity,
        violations,
        confidence,
        recommendation,
    }
}

// Precompiled regex patterns for performance
lazy_static! {
    // Legitimate TODO patterns - enhanced for better recognition
    static ref TODO_TICKET_REGEX: Regex = Regex::new(r"(?i)TODO\s*[:\(]\s*([A-Z]+-\d+|[A-Z]{2,}-\d+|JIRA-\d+)").expect("Valid regex");
    static ref TODO_ISSUE_REGEX: Regex = Regex::new(r"(?i)TODO\s*#\d+").expect("Valid regex");
    static ref TODO_DATE_REGEX: Regex = Regex::new(r"(?i)TODO\s*\((\d{4}-\d{2}-\d{2}|\d{2}/\d{2}/\d{4}|Q[1-4]\s+\d{4})\)").expect("Valid regex");
    static ref TODO_OPTIMIZE_REGEX: Regex = Regex::new(r"(?i)TODO.*(?:optimize|refactor|performance|cleanup|improve|enhance)").expect("Valid regex");
    static ref TODO_AUTHOR_REGEX: Regex = Regex::new(r"(?i)TODO\s*\(@[\w\-\.]+\)").expect("Valid regex");
    static ref TODO_VERSION_REGEX: Regex = Regex::new(r"(?i)TODO.*(?:v\d+\.\d+|version|after|when|once|migration|deployment|release)").expect("Valid regex");
    static ref TODO_SPECIFIC_REGEX: Regex = Regex::new(r"(?i)TODO.*(?:add|implement|create|update|fix|remove|replace|migrate).*(?:when|after|once|in|during)").expect("Valid regex");
    
    // Obfuscated patterns that indicate DELIBERATE deception attempts
    // FIXED: More precise regex that only catches real obfuscation, not accidental matches
    static ref OBFUSCATED_TODO_REGEX: Regex = Regex::new(r"(?i)\b(?:T0D0|TOD0|T-O-D-O|T\.O\.D\.O)\b").expect("Valid regex");
    static ref DISGUISED_TODO_REGEX: Regex = Regex::new(r"(?i)(?:fix|finish|complete)\s*(?:me|this|later|soon)").expect("Valid regex");
    
    // Spaced-out deception patterns with word boundaries to avoid false positives
    static ref SPACED_TODO_REGEX: Regex = Regex::new(r"(?i)\bT\s+O\s+D\s+O\b").expect("Valid regex");
    static ref SPACED_NOT_IMPL_REGEX: Regex = Regex::new(r"(?i)N\s+O\s+T\s+I\s+M\s+P\s+L\s+E\s+M\s+E\s+N\s+T").expect("Valid regex");
    static ref SPACED_FIXME_REGEX: Regex = Regex::new(r"(?i)F\s+I\s+X\s+M\s+E").expect("Valid regex");
    
    // Legitimate validator patterns
    static ref VALIDATOR_FUNC_REGEX: Regex = Regex::new(r"function\s+(?:is|has|can|should|check|validate)\w+").expect("Valid regex");
    static ref VALIDATOR_CONST_REGEX: Regex = Regex::new(r"const\s+(?:is|has|can|should|check|validate)\w+").expect("Valid regex");
    
    // Legitimate test/mock patterns
    static ref JEST_MOCK_REGEX: Regex = Regex::new(r"jest\.(?:fn|mock|spyOn)\(").expect("Valid regex");
    static ref TEST_MOCK_CLASS_REGEX: Regex = Regex::new(r"class\s+(?:Mock|Fake|Stub|Test)\w+").expect("Valid regex");
    static ref MOCK_FACTORY_REGEX: Regex = Regex::new(r"(?:create|make|get)(?:Mock|Fake|Stub)\w*").expect("Valid regex");
    
    // String literal detection patterns (for is_in_string_literal)
    static ref SINGLE_QUOTE_REGEX: Regex = Regex::new(r"'[^'\\]*(?:\\.[^'\\]*)*'").expect("Valid regex");
    static ref DOUBLE_QUOTE_REGEX: Regex = Regex::new(r#""[^"\\]*(?:\\.[^"\\]*)*""#).expect("Valid regex");
    static ref BACKTICK_REGEX: Regex = Regex::new(r"`[^`\\]*(?:\\.[^`\\]*)*`").expect("Valid regex");
    
    // AI-specific deception patterns
    static ref AI_SIMULATION_REGEX: Regex = Regex::new(r"(?i)(?:simulate|mock|fake|dummy|stub)\s*(?:data|response|result|value|api|call)?").expect("Valid regex");
    static ref IMPLEMENTATION_PENDING_REGEX: Regex = Regex::new(r"(?i)(?:implementation|logic|code)\s{0,10}(?:pending|coming|todo|tbd|needed|required|missing)").expect("Valid regex");
    static ref SIMULATED_DELAY_REGEX: Regex = Regex::new(r"(?:setTimeout|sleep|delay|wait)\s*\([^)]*\d{3,}[^)]*\)").expect("Valid regex");
    static ref FAKE_RANDOM_REGEX: Regex = Regex::new(r"Math\.random\(\)").expect("Valid regex");
    static ref SIMULATE_COMMENT_REGEX: Regex = Regex::new(r"(?i)//\s*(?:simulate|simulating|mock|fake|dummy)\s+(?:api|data|response|call|delay)").expect("Valid regex");
    
    // Mock patterns
    static ref MOCK_NAME_REGEX: Regex = Regex::new(r#"\b(mock|fake|stub|dummy)([A-Z]\w*|\b)"#).expect("Valid regex");
    
    // Test data patterns (for smart test detection)
    static ref TEST_DATA_STRING_REGEX: Regex = Regex::new(r#"(?i)['"]test(?:ing)?(?:\s+)?(?:data|value|string|user|email|password|token)['"]"#).expect("Valid regex");
    static ref TEST_DATA_VAR_REGEX: Regex = Regex::new(r#"(?i)test(?:Data|Value|String|User|Email|Password|Token)\s*[:=]"#).expect("Valid regex");
    static ref TEST_EMAIL_REGEX: Regex = Regex::new(r#"(?i)['"](?:user@test\.com|test@\w+\.com)['"]"#).expect("Valid regex");
    static ref TEST_CREDS_REGEX: Regex = Regex::new(r#"(?i)['"](?:password|secret|token)(?:123|test|demo)['"]"#).expect("Valid regex");
    static ref TEST_MODE_REGEX: Regex = Regex::new(r#"(?i)(?:testMode|isTest)\s*=\s*true"#).expect("Valid regex");
    
    // Legitimate test patterns (whitelist)
    static ref LEGIT_TEST_FUNC_REGEX: Regex = Regex::new(r#"function\s+test\w+"#).expect("Valid regex");
    static ref LEGIT_TEST_CONST_REGEX: Regex = Regex::new(r#"const\s+test\w+\s*="#).expect("Valid regex");
    static ref LEGIT_TEST_CALL_REGEX: Regex = Regex::new(r#"\.test\("#).expect("Valid regex");
    static ref LEGIT_DESCRIBE_REGEX: Regex = Regex::new(r#"describe\("#).expect("Valid regex");
    static ref LEGIT_IT_REGEX: Regex = Regex::new(r#"it\("#).expect("Valid regex");
    static ref LEGIT_EXPECT_REGEX: Regex = Regex::new(r#"expect\("#).expect("Valid regex");
}

/// Check if a TODO comment is legitimate (has proper context/tracking)
fn is_legitimate_todo(line: &str) -> bool {
    // Check against all legitimate TODO patterns
    TODO_TICKET_REGEX.is_match(line) ||
    TODO_ISSUE_REGEX.is_match(line) ||
    TODO_DATE_REGEX.is_match(line) ||
    TODO_OPTIMIZE_REGEX.is_match(line) ||
    TODO_AUTHOR_REGEX.is_match(line) ||
    TODO_VERSION_REGEX.is_match(line) ||
    TODO_SPECIFIC_REGEX.is_match(line)
}


/// Check if content is inside a comment or string literal
/// Check if a line is inside a multiline string literal (template literal)
/// Proper parsing with bounds checking and optimized character iteration
fn is_in_multiline_string(lines: &[&str], line_index: usize) -> bool {
    // Critical bounds check to prevent panic
    if line_index >= lines.len() {
        return false;
    }
    
    let mut in_template_literal = false;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    
    // Check from beginning of file up to current line
    for i in 0..=line_index {
        let line = lines[i];
        let mut chars = line.chars().peekable();
        
        // Parse each character in the line
        while let Some(ch) = chars.next() {
            // Handle escape sequences - only skip if escaping quotes or backslash
            if ch == '\\' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '\'' || next_ch == '"' || next_ch == '`' || next_ch == '\\' {
                        chars.next(); // Skip the escaped character
                        continue;
                    }
                }
            }
            
            // Track string literal states
            match ch {
                '\'' if !in_double_quote && !in_template_literal => {
                    in_single_quote = !in_single_quote;
                }
                '"' if !in_single_quote && !in_template_literal => {
                    in_double_quote = !in_double_quote;
                }
                '`' if !in_single_quote && !in_double_quote => {
                    in_template_literal = !in_template_literal;
                }
                _ => {}
            }
        }
        
        // Single-line strings reset at line end, template literals continue
        in_single_quote = false;
        in_double_quote = false;
    }
    
    in_template_literal
}

fn is_inside_comment_or_string(line: &str, lines: &[&str], line_index: usize) -> bool {
    let trimmed = line.trim();
    
    // Single-line comment
    if trimmed.starts_with("//") || trimmed.starts_with("#") || 
       trimmed.starts_with("*") || trimmed.starts_with("/*") {
        return true;
    }
    
    // Check if we're inside a multi-line comment
    if is_in_multiline_comment(lines, line_index) {
        return true;
    }
    
    // Check if we're inside a multiline string literal (template literal)
    if is_in_multiline_string(lines, line_index) {
        return true;
    }
    
    // Check if the problematic text is inside a single-line string literal
    if is_in_string_literal(line) {
        return true;
    }
    
    false
}

/// Check if current line is inside a multi-line comment block
fn is_in_multiline_comment(lines: &[&str], current_index: usize) -> bool {
    let mut in_comment = false;
    let mut in_code_block = false;
    
    // Check all lines before current to determine comment state
    for i in 0..=current_index {
        let line = lines[i];
        let trimmed = line.trim();
        
        // Check for markdown code blocks (```)
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            if i == current_index {
                return true; // The ``` line itself is part of comment
            }
        }
        
        // If we're in a code block, treat it as a comment
        if in_code_block {
            if i == current_index {
                return true;
            }
            continue;
        }
        
        // Check for /* ... */ style comments
        if line.contains("/*") {
            let start_pos = line.find("/*").unwrap();
            let end_pos = line.rfind("*/");
            
            if let Some(end) = end_pos {
                // Comment starts and ends on same line
                if end > start_pos {
                    // Check if current position would be inside this comment
                    // This is a simplification - we'd need char positions for accuracy
                    continue;
                }
            } else {
                // Comment starts but doesn't end on this line
                in_comment = true;
            }
        } else if line.contains("*/") && in_comment {
            in_comment = false;
        }
        
        // Check for Python docstring
        if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
            // Count occurrences to track if we're inside
            let triple_quotes = line.matches("\"\"\"").count() + line.matches("'''").count();
            if triple_quotes % 2 == 1 {
                in_comment = !in_comment;
            }
        }
    }
    
    in_comment || in_code_block
}

/// Helper function to check if any suspicious patterns are found in regex matches
/// Optimized for performance by converting to lowercase only once per match
fn check_suspicious_patterns_in_matches(regex: &Regex, line: &str) -> bool {
    for mat in regex.find_iter(line) {
        let content = mat.as_str().to_lowercase();
        // SUSPICIOUS_STRING_PATTERNS are already in lowercase for efficient comparison
        for &pattern in SUSPICIOUS_STRING_PATTERNS {
            if content.contains(pattern) {
                return true;
            }
        }
    }
    false
}

/// Check if problematic text is inside a string literal
/// Uses precompiled regex patterns and handles escape sequences properly
fn is_in_string_literal(line: &str) -> bool {
    // Check all string literal types using the helper function
    check_suspicious_patterns_in_matches(&SINGLE_QUOTE_REGEX, line) ||
    check_suspicious_patterns_in_matches(&DOUBLE_QUOTE_REGEX, line) ||
    check_suspicious_patterns_in_matches(&BACKTICK_REGEX, line)
}

fn check_todo_patterns(content: &str, context: &CodeContext, violations: &mut Vec<Violation>) {
    let lines: Vec<&str> = content.lines().collect();
    
    // Smart TODO detection - only flag if it indicates incomplete implementation
    let todo_regex = Regex::new(r#"(?i)\b(TODO|FIXME|HACK|XXX)\b"#).unwrap();
    
    for (i, line) in lines.iter().enumerate() {
        // Skip if we're in a README, documentation, or example file
        if context.is_config_file || context.is_type_definition {
            continue;
        }
        
        if todo_regex.is_match(line) {
            // CRITICAL FIX: Check legitimate TODOs FIRST before obfuscated patterns
            if is_legitimate_todo(line) {
                // But check if there's simulation nearby even with legitimate TODO
                if check_simulation_near_todo(&lines, i) {
                    violations.push(Violation {
                        category: ViolationCategory::SimulatedWork,
                        message: format!("Simulation detected near legitimate TODO at line {}", i + 1),
                        line_number: Some(i + 1),
                        confidence: 0.85,
                        context: line.trim().to_string(),
                    });
                }
                continue; // Skip legitimate TODOs
            }
            
            // NOW check for obfuscated patterns on non-legitimate TODOs
            if OBFUSCATED_TODO_REGEX.is_match(line) || DISGUISED_TODO_REGEX.is_match(line) ||
               SPACED_TODO_REGEX.is_match(line) || SPACED_NOT_IMPL_REGEX.is_match(line) || 
               SPACED_FIXME_REGEX.is_match(line) {
                // Only flag if NOT inside a string literal
                if is_in_string_literal(line) {
                    continue;
                }
                violations.push(Violation {
                    category: ViolationCategory::DeceptiveComment,
                    message: format!("Obfuscated or disguised TODO pattern at line {}", i + 1),
                    line_number: Some(i + 1),
                    confidence: 0.95,
                    context: line.trim().to_string(),
                });
                continue;
            }
            
            // Check if it's followed by actual implementation
            let has_impl_below = check_implementation_below(&lines, i);
            
            // Check the content of TODO for incompleteness indicators
            let line_lower = line.to_lowercase();
            let is_incomplete = line_lower.contains("implement") ||
                               line_lower.contains("finish") ||
                               line_lower.contains("complete") ||
                               line_lower.contains("add logic") ||
                               line_lower.contains("missing") ||
                               line_lower.contains("fill in") ||
                               line_lower.contains("add code here");
            
            if is_incomplete && !has_impl_below {
                violations.push(Violation {
                    category: ViolationCategory::IncompleteTodo,
                    message: format!("TODO indicates missing implementation at line {}", i + 1),
                    line_number: Some(i + 1),
                    confidence: 0.85,
                    context: line.trim().to_string(),
                });
            }
        }
        
        // Check for obfuscated patterns that don't match standard TODO format
        // This catches disguised patterns without TODO keyword
        else if SPACED_NOT_IMPL_REGEX.is_match(line) || SPACED_FIXME_REGEX.is_match(line) {
            // Only flag if NOT inside a string literal
            if is_in_string_literal(line) {
                continue;
            }
            violations.push(Violation {
                category: ViolationCategory::DeceptiveComment,
                message: format!("Spaced obfuscation pattern at line {}", i + 1),
                line_number: Some(i + 1),
                confidence: 0.95,
                context: line.trim().to_string(),
            });
        }
    }
}

/// Check if there's simulation code near a legitimate TODO
fn check_simulation_near_todo(lines: &[&str], todo_line: usize) -> bool {
    // Check next 5 lines for simulation patterns
    let end = (todo_line + 5).min(lines.len());
    
    for i in (todo_line + 1)..end {
        let line = lines[i].trim();
        
        // Check for simulation keywords
        if line.contains("simulate") || line.contains("mock") || 
           line.contains("fake") || line.contains("dummy") ||
           line.contains("delay") || line.contains("setTimeout") {
            // Check if it's actual simulation, not just a comment about future work
            if !line.starts_with("//") || line.contains("// simulate") || 
               line.contains("// mock") || line.contains("// fake") {
                return true;
            }
        }
        
        // Check for Math.random() used for IDs
        if line.contains("Math.random()") && (line.contains("id") || line.contains("Id")) {
            return true;
        }
    }
    
    false
}

fn check_implementation_below(lines: &[&str], todo_line: usize) -> bool {
    // Check next 10 lines for real implementation
    let end = (todo_line + 10).min(lines.len());
    let mut has_logic = false;
    
    for i in (todo_line + 1)..end {
        let line = lines[i].trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with("//") || line.starts_with("#") {
            continue;
        }
        
        // Check for return statements with logic
        if line.contains("return") && !line.contains("return true") && 
           !line.contains("return false") && !line.contains("return null") &&
           !line.contains("return {}") && !line.contains("return []") {
            has_logic = true;
            break;
        }
        
        // Check for actual logic
        if line.contains("if ") || line.contains("for ") || 
           line.contains("while ") || line.contains("await ") ||
           line.contains("fetch") || line.contains("query") {
            has_logic = true;
            break;
        }
    }
    
    has_logic
}

fn check_fake_implementations(content: &str, _context: &CodeContext, violations: &mut Vec<Violation>) {
    let lines: Vec<&str> = content.lines().collect();
    
    // Patterns that indicate fake implementation
    let fake_patterns = [
        (r#"(?i)\bnot\s+(?:yet\s+)?implemented\b"#, "Not implemented marker"),
        (r#"(?i)\bcoming\s+soon\b"#, "Coming soon placeholder"),
        (r#"(?i)\bwork\s+in\s+progress\b"#, "Work in progress marker"),
        (r#"(?i)\bplaceholder\b"#, "Placeholder marker"),
        (r#"(?i)console\.\w+\(['\"].*not\s+implemented"#, "Console log instead of implementation"),
        (r#"throw\s+new\s+Error\(['\"].*not\s+implemented"#, "Throwing not implemented error"),
    ];
    
    for (pattern, message) in &fake_patterns {
        let regex = Regex::new(pattern).unwrap();
        
        for (i, line) in lines.iter().enumerate() {
            // Skip if this line is inside a comment or string literal
            if is_inside_comment_or_string(line, &lines, i) {
                continue;
            }
            
            if regex.is_match(line) {
                violations.push(Violation {
                    category: ViolationCategory::FakeImplementation,
                    message: format!("{} at line {}", message, i + 1),
                    line_number: Some(i + 1),
                    confidence: 0.95,
                    context: line.trim().to_string(),
                });
            }
        }
    }
}

fn check_mock_patterns(content: &str, file_path: &str, context: &CodeContext, violations: &mut Vec<Violation>) {
    // Check filename first - cross-platform path handling
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path)
        .to_lowercase();
    
    // Only flag if explicitly mock/fake/stub in production code
    if filename.contains("mock") || filename.contains("fake") || filename.contains("stub") {
        if !context.is_test_file {
            violations.push(Violation {
                category: ViolationCategory::MockCode,
                message: "Mock/fake/stub file in production code".to_string(),
                line_number: None,
                confidence: 1.0,
                context: filename.clone(),
            });
        }
    }
    
    // Check for mock function/variable names but be smart about it
    let mock_name_regex = Regex::new(r#"\b(mock|fake|stub|dummy)([A-Z]\w*|\b)"#).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    
    for (i, line) in lines.iter().enumerate() {
        // Skip if this line is inside a comment or string literal
        if is_inside_comment_or_string(line, &lines, i) {
            continue;
        }
        
        if let Some(capture) = mock_name_regex.find(line) {
            let matched = capture.as_str();
            
            // Check if it's a legitimate use (like "mockito" in imports)
            let line_lower = line.to_lowercase();
            if line_lower.contains("import") || line_lower.contains("require") {
                continue; // Skip imports
            }
            
            // Check if it's defining a mock
            if line.contains("function") || line.contains("const ") || 
               line.contains("let ") || line.contains("var ") ||
               line.contains("class ") || line.contains("def ") {
                violations.push(Violation {
                    category: ViolationCategory::MockCode,
                    message: format!("Mock/fake identifier '{}' in production code at line {}", matched, i + 1),
                    line_number: Some(i + 1),
                    confidence: 0.85,
                    context: line.trim().to_string(),
                });
            }
        }
    }
    
    // Smart handling of "test" keyword - only flag suspicious test data in production
    if !context.is_test_file && !context.is_dev_file {
        check_test_patterns_smart(content, &lines, violations);
    }
}

/// Smart check for "test" patterns - distinguish legitimate from fake test data
fn check_test_patterns_smart(_content: &str, lines: &[&str], violations: &mut Vec<Violation>) {
    for (i, line) in lines.iter().enumerate() {
        // Skip if in string literal or comment
        if is_in_string_literal(line) || line.trim().starts_with("//") {
            continue;
        }
        
        // Check each test data pattern
        let mut found_issue = None;
        
        if TEST_DATA_STRING_REGEX.is_match(line) {
            found_issue = Some("Test data string");
        } else if TEST_DATA_VAR_REGEX.is_match(line) {
            found_issue = Some("Test data variable");
        } else if TEST_EMAIL_REGEX.is_match(line) {
            found_issue = Some("Test email address");
        } else if TEST_CREDS_REGEX.is_match(line) {
            found_issue = Some("Test credentials");
        } else if TEST_MODE_REGEX.is_match(line) {
            found_issue = Some("Test mode enabled in production");
        }
        
        if let Some(message) = found_issue {
            // Check if it's actually legitimate use
            if is_legitimate_test_use(line) {
                continue;
            }
            
            // Also check if it's in a configuration context (could be legitimate)
            let line_lower = line.to_lowercase();
            if line_lower.contains("config") || line_lower.contains("env") || 
               line_lower.contains("settings") || line_lower.contains("options") {
                // Check if it's checking for test environment (legitimate)
                if line_lower.contains("process.env") || line_lower.contains("node_env") {
                    continue;
                }
            }
            
            violations.push(Violation {
                category: ViolationCategory::MockCode,
                message: format!("{} in production code at line {}", message, i + 1),
                line_number: Some(i + 1),
                confidence: 0.75,
                context: line.trim().to_string(),
            });
        }
    }
}

/// Check if test-related code is legitimate
fn is_legitimate_test_use(line: &str) -> bool {
    LEGIT_TEST_FUNC_REGEX.is_match(line) ||
    LEGIT_TEST_CONST_REGEX.is_match(line) ||
    LEGIT_TEST_CALL_REGEX.is_match(line) ||
    LEGIT_DESCRIBE_REGEX.is_match(line) ||
    LEGIT_IT_REGEX.is_match(line) ||
    LEGIT_EXPECT_REGEX.is_match(line) ||
    line.contains("testId") ||
    line.contains("test-id") ||
    line.contains("testCase") ||
    line.contains("unittest") ||
    line.contains("pytest") ||
    line.contains("testing library") ||
    line.contains("@testing")
}

fn check_error_hiding(content: &str, context: &CodeContext, violations: &mut Vec<Violation>) {
    let lines: Vec<&str> = content.lines().collect();
    
    // Language-specific error hiding patterns
    match context.language {
        Language::JavaScript | Language::TypeScript => {
            // Check for empty catch blocks
            let empty_catch = Regex::new(r#"catch\s*\([^)]*\)\s*\{\s*(?://[^\n]*)?\s*\}"#).unwrap();
            
            for (i, line) in lines.iter().enumerate() {
                // Skip if this line is inside a comment or string literal
                if is_inside_comment_or_string(line, &lines, i) {
                    continue;
                }
                
                if empty_catch.is_match(line) {
                    // Check if next lines have any content
                    let mut is_empty = true;
                    if i + 1 < lines.len() {
                        let next_line = lines[i + 1].trim();
                        if !next_line.is_empty() && !next_line.starts_with("}") {
                            is_empty = false;
                        }
                    }
                    
                    if is_empty {
                        violations.push(Violation {
                            category: ViolationCategory::ErrorHiding,
                            message: format!("Empty catch block at line {}", i + 1),
                            line_number: Some(i + 1),
                            confidence: 0.9,
                            context: line.trim().to_string(),
                        });
                    }
                }
            }
        },
        Language::Python => {
            // Check for bare except with pass
            let bare_except = Regex::new(r#"except\s*:\s*$"#).unwrap();
            
            for (i, line) in lines.iter().enumerate() {
                if bare_except.is_match(line) {
                    if i + 1 < lines.len() && lines[i + 1].trim() == "pass" {
                        violations.push(Violation {
                            category: ViolationCategory::ErrorHiding,
                            message: format!("Bare except with pass at line {}", i + 1),
                            line_number: Some(i + 1),
                            confidence: 0.9,
                            context: format!("{}\n{}", line.trim(), lines[i + 1].trim()),
                        });
                    }
                }
            }
        },
        _ => {}
    }
}

fn check_hardcoded_returns(content: &str, _context: &CodeContext, violations: &mut Vec<Violation>) {
    let lines: Vec<&str> = content.lines().collect();
    
    // Only check for suspicious hardcoded returns in functions without logic
    let hardcoded_patterns = [
        (r#"return\s+true\s*;?\s*$"#, "true"),
        (r#"return\s+false\s*;?\s*$"#, "false"),
        (r#"return\s+null\s*;?\s*$"#, "null"),
        (r#"return\s+\{\s*\}\s*;?\s*$"#, "empty object"),
        (r#"return\s+\[\s*\]\s*;?\s*$"#, "empty array"),
    ];
    
    for (pattern, value_type) in &hardcoded_patterns {
        let regex = Regex::new(pattern).unwrap();
        
        for (i, line) in lines.iter().enumerate() {
            // Skip if this line is inside a comment or string literal
            if is_inside_comment_or_string(line, &lines, i) {
                continue;
            }
            
            if regex.is_match(line) {
                // Check if this is a one-liner function
                let is_oneliner = check_if_oneliner_function(&lines, i);
                
                // Check if there's logic above this return
                let has_logic_above = check_logic_above(&lines, i);
                
                if is_oneliner && !has_logic_above {
                    violations.push(Violation {
                        category: ViolationCategory::HardcodedLogic,
                        message: format!("Function returns hardcoded {} without logic at line {}", value_type, i + 1),
                        line_number: Some(i + 1),
                        confidence: 0.8,
                        context: line.trim().to_string(),
                    });
                }
            }
        }
    }
}

fn check_if_oneliner_function(lines: &[&str], return_line: usize) -> bool {
    // Look back up to 3 lines for function definition
    let start = if return_line > 3 { return_line - 3 } else { 0 };
    
    for i in start..return_line {
        let line = lines[i];
        if line.contains("function") || line.contains("=>") || 
           line.contains("def ") || line.contains("fn ") {
            // Found function definition, check if there's logic between it and return
            let mut has_logic = false;
            for j in (i + 1)..return_line {
                let check_line = lines[j].trim();
                if !check_line.is_empty() && !check_line.starts_with("//") && !check_line.starts_with("{") {
                    has_logic = true;
                    break;
                }
            }
            return !has_logic;
        }
    }
    
    false
}

fn check_logic_above(lines: &[&str], return_line: usize) -> bool {
    // Check previous 5 lines for logic
    let start = if return_line > 5 { return_line - 5 } else { 0 };
    
    for i in start..return_line {
        let line = lines[i].trim();
        if line.contains("if ") || line.contains("for ") || 
           line.contains("while ") || line.contains("switch ") ||
           line.contains("await ") || line.contains("try ") {
            return true;
        }
    }
    
    false
}

fn check_simulated_work(content: &str, context: &CodeContext, violations: &mut Vec<Violation>) {
    // Check for AI-specific simulation patterns first
    let lines: Vec<&str> = content.lines().collect();
    
    for (i, line) in lines.iter().enumerate() {
        // First check for simulation comments (they should be detected even if in comments)
        if SIMULATE_COMMENT_REGEX.is_match(line) {
            violations.push(Violation {
                category: ViolationCategory::SimulatedWork,
                message: format!("Simulation comment detected at line {}", i + 1),
                line_number: Some(i + 1),
                confidence: 0.95,
                context: line.trim().to_string(),
            });
            continue;
        }
        
        // Skip if in string literal only (not comment, we want to check code)
        if is_in_string_literal(line) {
            continue;
        }
        
        // Check for simulated delays
        if SIMULATED_DELAY_REGEX.is_match(line) {
            // Check if there's real logic around the delay
            let has_logic = check_logic_above(&lines, i) || check_implementation_below(&lines, i);
            if !has_logic {
                violations.push(Violation {
                    category: ViolationCategory::SimulatedWork,
                    message: format!("Simulated delay without real logic at line {}", i + 1),
                    line_number: Some(i + 1),
                    confidence: 0.9,
                    context: line.trim().to_string(),
                });
            }
        }
        
        // Check for fake random IDs (only in return statements or assignments)
        if FAKE_RANDOM_REGEX.is_match(line) && (line.contains("return") || line.contains("id:") || line.contains("id =")) {
            violations.push(Violation {
                category: ViolationCategory::SimulatedWork,
                message: format!("Using Math.random() for ID generation at line {}", i + 1),
                line_number: Some(i + 1),
                confidence: 0.85,
                context: line.trim().to_string(),
            });
        }
        
        // Check for AI simulation keywords (but allow in comments)
        if AI_SIMULATION_REGEX.is_match(line) && !context.is_test_file && !context.is_dev_file {
            // Check if it's in actual code, not just a comment
            let trimmed = line.trim();
            if !trimmed.starts_with("//") && !trimmed.starts_with("#") && !trimmed.starts_with("*") {
                violations.push(Violation {
                    category: ViolationCategory::SimulatedWork,
                    message: format!("AI simulation pattern detected at line {}", i + 1),
                    line_number: Some(i + 1),
                    confidence: 0.8,
                    context: line.trim().to_string(),
                });
            }
        }
        
        // Check for pending implementation markers
        if IMPLEMENTATION_PENDING_REGEX.is_match(line) {
            violations.push(Violation {
                category: ViolationCategory::FakeImplementation,
                message: format!("Implementation pending marker at line {}", i + 1),
                line_number: Some(i + 1),
                confidence: 0.95,
                context: line.trim().to_string(),
            });
        }
    }
    
    // Check for faker library usage - allow in dev/test files
    if !context.is_test_file && !context.is_dev_file {
        let faker_regex = Regex::new(r"faker\.").unwrap();
        
        for (i, line) in lines.iter().enumerate() {
            // Skip if this line is inside a comment or string literal
            if is_inside_comment_or_string(line, &lines, i) {
                continue;
            }
            
            if faker_regex.is_match(line) {
                violations.push(Violation {
                    category: ViolationCategory::SimulatedWork,
                    message: format!("Faker library in production code at line {}", i + 1),
                    line_number: Some(i + 1),
                    confidence: 0.85,
                    context: line.trim().to_string(),
                });
            }
        }
    }
}

fn calculate_severity(violations: &[Violation], context: &CodeContext) -> (DeceptionSeverity, f32) {
    if violations.is_empty() {
        return (DeceptionSeverity::None, 0.0);
    }
    
    let mut max_severity = DeceptionSeverity::None;
    let mut total_confidence = 0.0;
    
    for violation in violations {
        let severity = match violation.category {
            ViolationCategory::FakeImplementation => DeceptionSeverity::Critical,
            ViolationCategory::MockCode => {
                if context.is_test_file {
                    DeceptionSeverity::Low
                } else {
                    DeceptionSeverity::High
                }
            },
            ViolationCategory::ErrorHiding => DeceptionSeverity::High,
            ViolationCategory::IncompleteTodo => {
                if context.has_real_logic {
                    DeceptionSeverity::Medium
                } else {
                    DeceptionSeverity::High
                }
            },
            ViolationCategory::HardcodedLogic => {
                if context.has_real_logic {
                    DeceptionSeverity::Low
                } else {
                    DeceptionSeverity::High
                }
            },
            ViolationCategory::SimulatedWork => DeceptionSeverity::High, // Simulation is clear deception
            ViolationCategory::DeceptiveComment => DeceptionSeverity::High, // Obfuscated TODOs are serious deception attempts
        };
        
        if severity > max_severity {
            max_severity = severity;
        }
        
        total_confidence += violation.confidence;
    }
    
    let avg_confidence = total_confidence / violations.len() as f32;
    
    // Adjust severity based on context
    let adjusted_severity = if context.has_real_logic && context.has_error_handling {
        // If code has real logic and error handling, reduce severity
        match max_severity {
            DeceptionSeverity::Critical => DeceptionSeverity::High,
            DeceptionSeverity::High => DeceptionSeverity::Medium,
            other => other,
        }
    } else if !context.has_real_logic && violations.len() > 2 {
        // If no real logic and multiple violations, increase severity
        match max_severity {
            DeceptionSeverity::Medium => DeceptionSeverity::High,
            DeceptionSeverity::Low => DeceptionSeverity::Medium,
            other => other,
        }
    } else {
        max_severity
    };
    
    (adjusted_severity, avg_confidence)
}

fn generate_recommendation(violations: &[Violation], severity: &DeceptionSeverity, context: &CodeContext) -> String {
    if violations.is_empty() {
        return "Code appears to be legitimate implementation".to_string();
    }
    
    // Group violations by category
    let mut categories = HashSet::new();
    for v in violations {
        categories.insert(&v.category);
    }
    
    // Generate specific recommendations
    let mut recommendations = Vec::new();
    
    if categories.contains(&ViolationCategory::FakeImplementation) {
        recommendations.push("Remove 'not implemented' markers and add real implementation");
    }
    
    if categories.contains(&ViolationCategory::MockCode) && !context.is_test_file {
        recommendations.push("Replace mock/fake code with production implementation");
    }
    
    if categories.contains(&ViolationCategory::IncompleteTodo) {
        recommendations.push("Complete TODO items before marking as done");
    }
    
    if categories.contains(&ViolationCategory::ErrorHiding) {
        recommendations.push("Add proper error handling instead of swallowing exceptions");
    }
    
    if categories.contains(&ViolationCategory::HardcodedLogic) && !context.has_real_logic {
        recommendations.push("Add actual business logic instead of hardcoded returns");
    }
    
    // Generate final recommendation based on severity
    match severity {
        DeceptionSeverity::Critical => {
            format!("REJECT: Clear deception detected. {}", recommendations.join(". "))
        },
        DeceptionSeverity::High => {
            format!("REJECT: Multiple deception indicators. {}", recommendations.join(". "))
        },
        DeceptionSeverity::Medium => {
            format!("REVIEW: Suspicious patterns found. {}", recommendations.join(". "))
        },
        DeceptionSeverity::Low => {
            format!("CAUTION: Minor issues. {}", recommendations.join(". "))
        },
        DeceptionSeverity::None => {
            "Code appears legitimate".to_string()
        }
    }
}