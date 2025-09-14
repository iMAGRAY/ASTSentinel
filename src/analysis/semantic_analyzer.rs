/// Semantic analyzer for AST-based code understanding
/// Provides deeper context-aware analysis beyond string pattern matching
use crate::analysis::ast::languages::SupportedLanguage;
use crate::analysis::ast::quality_scorer::{ConcreteIssue, IssueCategory, IssueSeverity};
use crate::analysis::file_classifier::{FileCategory, FileClassifier};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

/// Semantic context for understanding code meaning
#[derive(Debug, Clone)]
pub struct SemanticContext {
    pub file_category: FileCategory,
    pub imports: BTreeSet<String>,
    pub function_calls: BTreeSet<String>,
    pub variable_assignments: BTreeMap<String, AssignmentType>,
    pub control_flow_depth: u32,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignmentType {
    LiteralString(String),
    FunctionCall(String),
    Variable(String),
    Computation,
    Unknown,
}

pub struct SemanticAnalyzer {
    classifier: FileClassifier,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            classifier: FileClassifier::new(),
        }
    }

    /// Perform semantic analysis on source code
    pub fn analyze(
        &self,
        source: &str,
        language: SupportedLanguage,
        file_path: Option<&Path>,
    ) -> Result<(SemanticContext, Vec<ConcreteIssue>), Box<dyn std::error::Error>> {
        let mut parser = Parser::new();
        let tree_sitter_lang = self.get_language_parser(language)?;
        parser.set_language(&tree_sitter_lang)?;

        let tree = parser.parse(source, None).ok_or("Failed to parse source code")?;

        let file_category = if let Some(path) = file_path {
            self.classifier.classify_file(path, Some(source))
        } else {
            crate::analysis::file_classifier::FileCategory::SourceCode {
                language: format!("{:?}", language).to_lowercase(),
                confidence: 0.8,
            }
        };

        let is_test_context = matches!(file_category, FileCategory::TestCode { .. });

        let mut context = SemanticContext {
            file_category,
            imports: BTreeSet::new(),
            function_calls: BTreeSet::new(),
            variable_assignments: BTreeMap::new(),
            control_flow_depth: 0,
            is_test_context,
        };

        let mut issues = Vec::new();

        self.traverse_ast(
            &tree.root_node(),
            source.as_bytes(),
            &mut context,
            &mut issues,
            language,
            0,
        );

        Ok((context, issues))
    }

    fn traverse_ast(
        &self,
        node: &Node,
        source_bytes: &[u8],
        context: &mut SemanticContext,
        issues: &mut Vec<ConcreteIssue>,
        language: SupportedLanguage,
        depth: u32,
    ) {
        let kind = node.kind();
        let mut cursor = node.walk();

        // Update control flow depth
        let is_control_flow = matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "while_statement"
                | "match_expression"
                | "switch_statement"
                | "try_statement"
                | "with_statement"
        );

        let new_depth = if is_control_flow { depth + 1 } else { depth };
        context.control_flow_depth = context.control_flow_depth.max(new_depth);

        // Analyze current node
        self.analyze_node(node, source_bytes, context, issues, language);

        // Recursively process children
        for child in node.children(&mut cursor) {
            self.traverse_ast(&child, source_bytes, context, issues, language, new_depth);
        }
    }

    fn analyze_node(
        &self,
        node: &Node,
        source_bytes: &[u8],
        context: &mut SemanticContext,
        issues: &mut Vec<ConcreteIssue>,
        language: SupportedLanguage,
    ) {
        let kind = node.kind();

        match language {
            SupportedLanguage::Python => {
                self.analyze_python_node(node, source_bytes, context, issues, kind);
            }
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                self.analyze_js_node(node, source_bytes, context, issues, kind);
            }
            SupportedLanguage::Rust => {
                self.analyze_rust_node(node, source_bytes, context, issues, kind);
            }
            SupportedLanguage::Go => {
                self.analyze_go_node(node, source_bytes, context, issues, kind);
            }
            _ => {
                // Generic analysis for other languages
                self.analyze_generic_node(node, source_bytes, context, issues, kind);
            }
        }
    }

    fn analyze_python_node(
        &self,
        node: &Node,
        source_bytes: &[u8],
        context: &mut SemanticContext,
        issues: &mut Vec<ConcreteIssue>,
        kind: &str,
    ) {
        match kind {
            "import_statement" | "import_from_statement" => {
                if let Ok(text) = node.utf8_text(source_bytes) {
                    context.imports.insert(text.trim().to_string());
                }
            }
            "assignment" => {
                self.analyze_assignment(node, source_bytes, context, issues);
            }
            "call" => {
                if let Ok(text) = node.utf8_text(source_bytes) {
                    context.function_calls.insert(text.trim().to_string());

                    // Check for dangerous function calls
                    if text.contains("eval(") || text.contains("exec(") {
                        issues.push(ConcreteIssue {
                            severity: IssueSeverity::Critical,
                            category: IssueCategory::HardcodedCredentials,
                            message: "Dangerous eval/exec call detected".to_string(),
                            file: String::new(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            rule_id: "SEC002".to_string(),
                            points_deducted: 100,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn analyze_js_node(
        &self,
        node: &Node,
        source_bytes: &[u8],
        context: &mut SemanticContext,
        issues: &mut Vec<ConcreteIssue>,
        kind: &str,
    ) {
        match kind {
            "import_statement" | "import_clause" => {
                if let Ok(text) = node.utf8_text(source_bytes) {
                    context.imports.insert(text.trim().to_string());
                }
            }
            "assignment_expression" | "variable_declarator" => {
                self.analyze_assignment(node, source_bytes, context, issues);
            }
            "call_expression" => {
                if let Ok(text) = node.utf8_text(source_bytes) {
                    context.function_calls.insert(text.trim().to_string());

                    // Check for dangerous patterns
                    if text.contains("eval(") || text.contains("Function(") {
                        issues.push(ConcreteIssue {
                            severity: IssueSeverity::Critical,
                            category: IssueCategory::HardcodedCredentials,
                            message: "Dangerous dynamic code execution".to_string(),
                            file: String::new(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            rule_id: "SEC003".to_string(),
                            points_deducted: 100,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn analyze_rust_node(
        &self,
        node: &Node,
        source_bytes: &[u8],
        context: &mut SemanticContext,
        issues: &mut Vec<ConcreteIssue>,
        kind: &str,
    ) {
        match kind {
            "use_declaration" => {
                if let Ok(text) = node.utf8_text(source_bytes) {
                    context.imports.insert(text.trim().to_string());
                }
            }
            "let_declaration" => {
                self.analyze_assignment(node, source_bytes, context, issues);
            }
            "call_expression" => {
                if let Ok(text) = node.utf8_text(source_bytes) {
                    context.function_calls.insert(text.trim().to_string());

                    // Check for unsafe patterns
                    if text.contains(".unwrap()") && !context.is_test_context {
                        issues.push(ConcreteIssue {
                            severity: IssueSeverity::Major,
                            category: IssueCategory::RobustnessAntipattern,
                            message: "Avoid unwrap() in production code".to_string(),
                            file: String::new(),
                            line: node.start_position().row + 1,
                            column: node.start_position().column + 1,
                            rule_id: "RUST001".to_string(),
                            points_deducted: 20,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn analyze_go_node(
        &self,
        node: &Node,
        source_bytes: &[u8],
        context: &mut SemanticContext,
        issues: &mut Vec<ConcreteIssue>,
        kind: &str,
    ) {
        match kind {
            "import_spec" | "import_declaration" => {
                if let Ok(text) = node.utf8_text(source_bytes) {
                    context.imports.insert(text.trim().to_string());
                }
            }
            "assignment_statement" | "var_declaration" => {
                self.analyze_assignment(node, source_bytes, context, issues);
            }
            "call_expression" => {
                if let Ok(text) = node.utf8_text(source_bytes) {
                    context.function_calls.insert(text.trim().to_string());
                }
            }
            _ => {}
        }
    }

    fn analyze_generic_node(
        &self,
        node: &Node,
        source_bytes: &[u8],
        context: &mut SemanticContext,
        _issues: &mut Vec<ConcreteIssue>,
        kind: &str,
    ) {
        // Generic patterns that work across languages
        if kind.contains("import") || kind.contains("include") {
            if let Ok(text) = node.utf8_text(source_bytes) {
                context.imports.insert(text.trim().to_string());
            }
        }

        if kind.contains("call") {
            if let Ok(text) = node.utf8_text(source_bytes) {
                context.function_calls.insert(text.trim().to_string());
            }
        }

        if kind.contains("assignment") || kind.contains("declaration") {
            self.analyze_assignment(node, source_bytes, context, _issues);
        }
    }

    fn analyze_assignment(
        &self,
        node: &Node,
        source_bytes: &[u8],
        context: &mut SemanticContext,
        issues: &mut Vec<ConcreteIssue>,
    ) {
        if let Ok(text) = node.utf8_text(source_bytes) {
            let assignment_type = self.classify_assignment(text);

            // Extract variable name (simplified)
            let var_name = self.extract_variable_name(text);

            if let Some(name) = var_name {
                context
                    .variable_assignments
                    .insert(name.clone(), assignment_type.clone());

                // Semantic analysis of credentials
                if self.is_credential_variable(&name) && !context.is_test_context {
                    match assignment_type {
                        AssignmentType::LiteralString(ref value) => {
                            // Only flag if it looks like a real credential (not placeholder)
                            if self.looks_like_real_credential(value) {
                                issues.push(ConcreteIssue {
                                    severity: IssueSeverity::Critical,
                                    category: IssueCategory::HardcodedCredentials,
                                    message: format!("Hardcoded credential in variable '{}'", name),
                                    file: String::new(),
                                    line: node.start_position().row + 1,
                                    column: node.start_position().column + 1,
                                    rule_id: "SEC001".to_string(),
                                    points_deducted: 50,
                                });
                            }
                        }
                        AssignmentType::FunctionCall(ref func) => {
                            // This is good - using function to get credential
                            if !func.contains("env") && !func.contains("config") {
                                issues.push(ConcreteIssue {
                                    severity: IssueSeverity::Minor,
                                    category: IssueCategory::HardcodedCredentials,
                                    message: format!("Consider using environment variables for '{}'", name),
                                    file: String::new(),
                                    line: node.start_position().row + 1,
                                    column: node.start_position().column + 1,
                                    rule_id: "SEC004".to_string(),
                                    points_deducted: 5,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn classify_assignment(&self, text: &str) -> AssignmentType {
        let text_lower = text.to_lowercase();

        // Check if it's a function call
        if text.contains('(') && text.contains(')') {
            // Extract function name
            if let Some(start) = text.find('=') {
                let rhs = &text[start + 1..].trim();
                if let Some(paren_pos) = rhs.find('(') {
                    let func_name = rhs[..paren_pos].trim();
                    return AssignmentType::FunctionCall(func_name.to_string());
                }
            }
        }

        // Check if it's a string literal
        if (text.contains('"') && text.matches('"').count() >= 2)
            || (text.contains('\'') && text.matches('\'').count() >= 2)
        {
            // Extract string value
            let quote_char = if text.contains('"') { '"' } else { '\'' };
            if let Some(start) = text.find(quote_char) {
                if let Some(end) = text.rfind(quote_char) {
                    if end > start {
                        let value = &text[start + 1..end];
                        return AssignmentType::LiteralString(value.to_string());
                    }
                }
            }
        }

        // Check if it's a variable reference
        if text.contains('=') && !text_lower.contains("input") && !text_lower.contains("scan") {
            if let Some(start) = text.find('=') {
                let rhs = &text[start + 1..].trim();
                if rhs.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') && !rhs.is_empty() {
                    return AssignmentType::Variable(rhs.to_string());
                }
            }
        }

        if text_lower.contains('+') || text_lower.contains('*') || text_lower.contains('/') {
            AssignmentType::Computation
        } else {
            AssignmentType::Unknown
        }
    }

    fn extract_variable_name(&self, text: &str) -> Option<String> {
        // Simple extraction - look for pattern: variable_name = value
        if let Some(eq_pos) = text.find('=') {
            let lhs = text[..eq_pos].trim();

            // Handle different patterns
            if lhs.contains("let ") {
                // Rust: let variable_name = value
                return lhs.strip_prefix("let ").map(|s| s.trim().to_string());
            } else if lhs.contains("var ") || lhs.contains("const ") {
                // JS: var/const variable_name = value
                let parts: Vec<&str> = lhs.split_whitespace().collect();
                if parts.len() >= 2 {
                    return Some(parts[1].to_string());
                }
            } else {
                // Python, simple assignment: variable_name = value
                return Some(lhs.to_string());
            }
        }
        None
    }

    fn is_credential_variable(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        name_lower.contains("password")
            || name_lower.contains("secret")
            || name_lower.contains("token")
            || name_lower.contains("api_key")
            || name_lower.contains("apikey")
            || name_lower.contains("auth")
            || name_lower.contains("key")
    }

    fn looks_like_real_credential(&self, value: &str) -> bool {
        // Skip obvious placeholders and test values
        let value_lower = value.to_lowercase();

        if value_lower.contains("placeholder")
            || value_lower.contains("example")
            || value_lower.contains("test")
            || value_lower.contains("dummy")
            || value_lower.contains("fake")
            || value_lower.contains("your_")
            || value_lower.contains("put_")
            || value.len() < 8
            || value == "password"
            || value == "secret"
            || value.chars().all(|c| c == 'x' || c == '1' || c == '0')
        {
            return false;
        }

        // Look for patterns that suggest real credentials
        value.len() >= 16
            && (value.chars().any(|c| c.is_ascii_uppercase())
                && value.chars().any(|c| c.is_ascii_lowercase())
                && value.chars().any(|c| c.is_ascii_digit()))
    }

    fn get_language_parser(&self, language: SupportedLanguage) -> Result<Language, &'static str> {
        match language {
            SupportedLanguage::Python => Ok(tree_sitter_python::LANGUAGE.into()),
            SupportedLanguage::JavaScript => Ok(tree_sitter_javascript::LANGUAGE.into()),
            SupportedLanguage::TypeScript => Ok(tree_sitter_typescript::language_typescript()),
            SupportedLanguage::Go => Ok(tree_sitter_go::language()),
            SupportedLanguage::Java => Ok(tree_sitter_java::language()),
            _ => Err("Unsupported language"),
        }
    }

    /// Get semantic score based on context analysis
    pub fn calculate_semantic_score(&self, context: &SemanticContext) -> u32 {
        let mut score = 1000; // Start with perfect score

        // Deduct points for high control flow depth
        if context.control_flow_depth > 5 {
            score -= (context.control_flow_depth - 5) * 20;
        }

        // Bonus for good import practices
        if !context.imports.is_empty() && context.imports.len() < 20 {
            score += 10;
        }

        // Analyze assignment patterns
        let mut credential_vars = 0;
        let mut safe_assignments = 0;

        for (name, assignment_type) in &context.variable_assignments {
            if self.is_credential_variable(name) {
                credential_vars += 1;
                match assignment_type {
                    AssignmentType::FunctionCall(func) if func.contains("env") || func.contains("config") => {
                        safe_assignments += 1;
                    }
                    AssignmentType::LiteralString(value) if !self.looks_like_real_credential(value) => {
                        safe_assignments += 1; // Placeholder is okay
                    }
                    _ => {}
                }
            }
        }

        // Bonus for safe credential handling
        if credential_vars > 0 {
            let safety_ratio = safe_assignments as f32 / credential_vars as f32;
            score += (safety_ratio * 50.0) as u32;
        }

        #[allow(clippy::unnecessary_min_or_max)]
        score.max(0)
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_credential_detection() {
        let analyzer = SemanticAnalyzer::new();
        let source = r#"
password = "real_secure_password_123ABC"
api_key = os.getenv("API_KEY")
test_password = "test123"
"#;

        let result = analyzer.analyze(source, SupportedLanguage::Python, None);
        assert!(result.is_ok());

        let (_context, issues) = result.unwrap();

        // Should detect hardcoded credential but not env var or test value
        let hardcoded_issues: Vec<_> = issues
            .iter()
            .filter(|i| matches!(i.category, IssueCategory::HardcodedCredentials))
            .collect();

        assert_eq!(hardcoded_issues.len(), 1);
        assert!(hardcoded_issues[0].message.contains("password"));
    }

    #[test]
    fn test_assignment_classification() {
        let analyzer = SemanticAnalyzer::new();

        // Function call
        let func_assignment = analyzer.classify_assignment("password = secrets.token_hex(32)");
        assert!(matches!(func_assignment, AssignmentType::FunctionCall(_)));

        // Literal string
        let literal_assignment = analyzer.classify_assignment("password = \"secret123\"");
        if let AssignmentType::LiteralString(value) = literal_assignment {
            assert_eq!(value, "secret123");
        } else {
            panic!("Expected LiteralString");
        }

        // Variable reference
        let var_assignment = analyzer.classify_assignment("password = other_var");
        assert!(matches!(var_assignment, AssignmentType::Variable(_)));
    }

    #[test]
    fn test_semantic_score() {
        let analyzer = SemanticAnalyzer::new();

        let mut context = SemanticContext {
            file_category: FileCategory::SourceCode {
                language: "python".to_string(),
                confidence: 0.9,
            },
            imports: BTreeSet::new(),
            function_calls: BTreeSet::new(),
            variable_assignments: BTreeMap::new(),
            control_flow_depth: 3,
            is_test_context: false,
        };

        // Test with safe credential handling
        context.variable_assignments.insert(
            "api_key".to_string(),
            AssignmentType::FunctionCall("os.getenv".to_string()),
        );

        let score = analyzer.calculate_semantic_score(&context);
        assert!(score > 1000); // Should have bonus for safe handling

        // Test with unsafe credential - clear previous assignments first
        context.variable_assignments.clear();
        context.variable_assignments.insert(
            "password".to_string(),
            AssignmentType::LiteralString("RealCredential123456789ABC".to_string()),
        );

        let score2 = analyzer.calculate_semantic_score(&context);

        // Verify credential detection is working correctly
        let test_val = "RealCredential123456789ABC";
        let is_real = analyzer.looks_like_real_credential(test_val);
        assert!(is_real, "The test credential should be detected as real");
        assert!(score2 < score); // Should be lower due to unsafe handling
    }
}
