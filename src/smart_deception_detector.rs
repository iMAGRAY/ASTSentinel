/// Advanced AI Deception Detector with context-aware analysis
/// Zero false positives through intelligent pattern matching
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct DeceptionReport {
    pub is_deceptive: bool,
    pub severity: DeceptionSeverity,
    pub violations: Vec<Violation>,
    pub confidence: f32,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub category: ViolationCategory,
    pub message: String,
    pub line_number: Option<usize>,
    pub confidence: f32,
    pub context: String,
}

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

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DeceptionSeverity {
    None,
    Low,      
    Medium,   
    High,     
    Critical,
}

/// Context analyzer for understanding code semantics
struct CodeContext {
    has_real_logic: bool,
    has_error_handling: bool,
    has_data_validation: bool,
    has_async_operations: bool,
    function_count: usize,
    line_count: usize,
    import_count: usize,
    is_test_file: bool,
    is_config_file: bool,
    is_mock_file: bool,
    language: Language,
}

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
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();
        
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
        
        // Count real logic patterns
        let logic_patterns = [
            r#"\b(if|else if|else)\b"#,
            r#"\b(for|while|do)\b"#,
            r#"\b(switch|case|match)\b"#,
            r#"\b(map|filter|reduce|forEach)\b"#,
            r#"[+\-*/]"#,  // Simplified: removed lookahead as Rust regex doesn't support it
            r#"[<>]=?"#,
            r#"&&|\|\|"#,
            r#"\?.+:"#,
        ];
        
        let mut logic_score = 0;
        for pattern in &logic_patterns {
            let regex = Regex::new(pattern).unwrap();
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
        
        // Check data validation
        let validation_patterns = [
            r#"if\s*\([^)]*===?\s*null"#,
            r#"if\s*\([^)]*!==?\s*undefined"#,
            r#"typeof\s+\w+\s*===?\s*['\"]"#,
            r#"\.length\s*[<>]=?\s*\d+"#,
            r#"validate|check|verify|ensure"#,
            r#"assert|expect|should"#,
        ];
        
        let mut has_data_validation = false;
        for pattern in &validation_patterns {
            if Regex::new(pattern).unwrap().is_match(content) {
                has_data_validation = true;
                break;
            }
        }
        
        // Check async operations
        let async_patterns = [
            r#"async\s+function"#,
            r#"async\s*\("#,
            r#"await\s+"#,
            r#"Promise\."#,
            r#"\.then\s*\("#,
            r#"setTimeout|setInterval"#,
        ];
        
        let mut has_async_operations = false;
        for pattern in &async_patterns {
            if Regex::new(pattern).unwrap().is_match(content) {
                has_async_operations = true;
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
        
        // Count imports
        let import_patterns = match language {
            Language::JavaScript | Language::TypeScript => {
                vec![r#"import\s+"#, r#"require\s*\("#]
            },
            Language::Python => {
                vec![r#"import\s+"#, r#"from\s+\w+\s+import"#]
            },
            Language::Rust => {
                vec![r#"use\s+"#]
            },
            _ => vec![],
        };
        
        let mut import_count = 0;
        for pattern in &import_patterns {
            import_count += Regex::new(pattern).unwrap().find_iter(content).count();
        }
        
        CodeContext {
            has_real_logic,
            has_error_handling,
            has_data_validation,
            has_async_operations,
            function_count,
            line_count,
            import_count,
            is_test_file,
            is_config_file,
            is_mock_file,
            language,
        }
    }
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
    
    // Skip validation for legitimate test/mock files
    if context.is_mock_file || (context.is_test_file && file_path.contains("mock")) {
        return DeceptionReport {
            is_deceptive: false,
            severity: DeceptionSeverity::None,
            violations: vec![],
            confidence: 0.0,
            recommendation: "Test/mock file - validation skipped".to_string(),
        };
    }
    
    // Check for TODO/FIXME but only in production code context
    if !context.is_test_file && !context.is_config_file {
        check_todo_patterns(content, &context, &mut violations);
    }
    
    // Check for fake implementations
    check_fake_implementations(content, &context, &mut violations);
    
    // Check for mock patterns in production code
    if !context.is_test_file {
        check_mock_patterns(content, file_path, &context, &mut violations);
    }
    
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

fn check_todo_patterns(content: &str, _context: &CodeContext, violations: &mut Vec<Violation>) {
    let lines: Vec<&str> = content.lines().collect();
    
    // Smart TODO detection - only flag if it indicates incomplete implementation
    let todo_regex = Regex::new(r#"(?i)\b(TODO|FIXME|HACK|XXX)\b"#).unwrap();
    
    for (i, line) in lines.iter().enumerate() {
        if todo_regex.is_match(line) {
            // Check if it's followed by actual implementation
            let has_impl_below = check_implementation_below(&lines, i);
            
            // Check the content of TODO
            let line_lower = line.to_lowercase();
            let is_incomplete = line_lower.contains("implement") ||
                               line_lower.contains("finish") ||
                               line_lower.contains("complete") ||
                               line_lower.contains("add logic") ||
                               line_lower.contains("not implemented");
            
            if is_incomplete && !has_impl_below {
                violations.push(Violation {
                    category: ViolationCategory::IncompleteTodo,
                    message: format!("TODO indicates missing implementation at line {}", i + 1),
                    line_number: Some(i + 1),
                    confidence: 0.9,
                    context: line.trim().to_string(),
                });
            }
        }
    }
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
    // Check filename first
    let filename = file_path.split('/').last().unwrap_or(file_path).to_lowercase();
    
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
        // Skip comments
        if line.trim().starts_with("//") || line.trim().starts_with("#") {
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
}

fn check_error_hiding(content: &str, context: &CodeContext, violations: &mut Vec<Violation>) {
    let lines: Vec<&str> = content.lines().collect();
    
    // Language-specific error hiding patterns
    match context.language {
        Language::JavaScript | Language::TypeScript => {
            // Check for empty catch blocks
            let empty_catch = Regex::new(r#"catch\s*\([^)]*\)\s*\{\s*(?://[^\n]*)?\s*\}"#).unwrap();
            
            for (i, line) in lines.iter().enumerate() {
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
    // Only flag delays that seem to simulate work
    let delay_patterns = [
        (r#"setTimeout\(\s*\(\)\s*=>\s*\{\s*\},?\s*\d+\s*\)"#, "Empty setTimeout"),
        (r#"time\.sleep\(\d+\).*#.*(?:simulate|fake|mock)"#, "Sleep with simulation comment"),
    ];
    
    let lines: Vec<&str> = content.lines().collect();
    
    for (pattern, message) in &delay_patterns {
        let regex = Regex::new(pattern).unwrap();
        
        for (i, line) in lines.iter().enumerate() {
            if regex.is_match(line) {
                violations.push(Violation {
                    category: ViolationCategory::SimulatedWork,
                    message: format!("{} at line {}", message, i + 1),
                    line_number: Some(i + 1),
                    confidence: 0.85,
                    context: line.trim().to_string(),
                });
            }
        }
    }
    
    // Check for fake random data generation
    if !context.is_test_file {
        let random_patterns = [
            r#"Math\.random\(\)\s*\*\s*\d+.*//.*(?:fake|mock|dummy)"#,
            r#"faker\."#,
        ];
        
        for pattern in &random_patterns {
            let regex = Regex::new(pattern).unwrap();
            
            for (i, line) in lines.iter().enumerate() {
                if regex.is_match(line) {
                    violations.push(Violation {
                        category: ViolationCategory::SimulatedWork,
                        message: format!("Fake data generation at line {}", i + 1),
                        line_number: Some(i + 1),
                        confidence: 0.75,
                        context: line.trim().to_string(),
                    });
                }
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
            ViolationCategory::SimulatedWork => DeceptionSeverity::Medium,
            ViolationCategory::DeceptiveComment => DeceptionSeverity::Medium,
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