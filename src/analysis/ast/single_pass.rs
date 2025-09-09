//! Single-pass, fast AST analysis for selected languages (Python/JS/TS/Java/C#/Go)
use crate::analysis::ast::languages::SupportedLanguage;
use crate::analysis::ast::kind_ids;
use crate::analysis::ast::quality_scorer::{ConcreteIssue, IssueCategory, IssueSeverity};

pub struct SinglePassEngine;

impl SinglePassEngine {
    pub fn analyze(tree: &tree_sitter::Tree, source: &str, language: SupportedLanguage) -> Vec<ConcreteIssue> {
        let mut issues = Vec::new();
        let src_bytes = source.as_bytes();
        let kind_ids = kind_ids::get_for_language(language);
        for (i, line) in source.lines().enumerate() {
            if line.len() > 120 {
                issues.push(ConcreteIssue { severity: IssueSeverity::Minor, category: IssueCategory::NamingConvention, message: format!("Line exceeds 120 characters ({})", line.len()), file: String::new(), line: i + 1, column: 121, rule_id: "LINE001".to_string(), points_deducted: ((line.len() - 120) / 10) as u32 + 1, });
            }
        }
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            for i in (0..node.child_count()).rev() { if let Some(ch) = node.child(i) { stack.push(ch); } }
            if is_function_fast(language, &kind_ids, &node) {
                let params = count_params(language, &node);
                if params > 5 { issues.push(ConcreteIssue { severity: IssueSeverity::Minor, category: IssueCategory::TooManyParameters, message: format!("Function has too many parameters ({} > 5)", params), file: String::new(), line: node.start_position().row + 1, column: node.start_position().column + 1, rule_id: "PARAMS001".to_string(), points_deducted: 15, }); }
                let (complexity, max_depth) = scan_complexity(language, &node);
                let threshold = match language { SupportedLanguage::Python => 8, SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => 10, SupportedLanguage::Java | SupportedLanguage::CSharp => 12, SupportedLanguage::Go => 8, _ => 10 };
                if complexity > threshold { issues.push(ConcreteIssue { severity: IssueSeverity::Minor, category: IssueCategory::HighComplexity, message: format!("High cyclomatic complexity: {} (threshold: {})", complexity, threshold), file: String::new(), line: node.start_position().row + 1, column: node.start_position().column + 1, rule_id: "AST003".to_string(), points_deducted: (complexity - threshold) * 5, }); }
                if max_depth > 4 { issues.push(ConcreteIssue { severity: IssueSeverity::Minor, category: IssueCategory::DeepNesting, message: format!("Deep nesting detected (level {})", max_depth), file: String::new(), line: node.start_position().row + 1, column: node.start_position().column + 1, rule_id: "NEST001".to_string(), points_deducted: (max_depth - 4) * 10, }); }
            }

            // Unreachable code after return (generic)
            let is_return = if let Some(ids) = kind_ids {
                let k = node.kind_id();
                match language {
                    SupportedLanguage::Python => k == ids.py_return_statement,
                    SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => k == ids.js_return_statement,
                    SupportedLanguage::Java => k == ids.java_return_statement,
                    SupportedLanguage::CSharp => k == ids.cs_return_statement,
                    SupportedLanguage::Go => k == ids.go_return_statement,
                    SupportedLanguage::C => k == ids.c_return_statement,
                    SupportedLanguage::Cpp => k == ids.cpp_return_statement,
                    SupportedLanguage::Php => k == ids.php_return_statement,
                    SupportedLanguage::Ruby => k == ids.ruby_return,
                    _ => matches!(node.kind(), "return_statement" | "return_expression" | "return"),
                }
            } else {
                matches!(node.kind(), "return_statement" | "return_expression" | "return")
            };
            if is_return {
                if let Some(next) = node.next_sibling() {
                    let k = next.kind();
                    if k != "}" && k != "comment" {
                        issues.push(ConcreteIssue {
                            severity: IssueSeverity::Major,
                            category: IssueCategory::UnreachableCode,
                            message: "Unreachable code after return statement".to_string(),
                            file: String::new(),
                            line: next.start_position().row + 1,
                            column: next.start_position().column + 1,
                            rule_id: "AST002".to_string(),
                            points_deducted: 30,
                        });
                    }
                }
            }

            // Минимальные проверки безопасности для Python/C#/Go/Php
            let kind = node.kind();
            match language {
                SupportedLanguage::Python => {
    if kind == "assignment" {
        if let Ok(text) = node.utf8_text(src_bytes) {
            let text_lower = text.to_lowercase();
            if text_lower.contains("password") || text_lower.contains("api_key") || text_lower.contains("secret") || text_lower.contains("token")
                && text.contains("=")
                && !text_lower.contains("getenv")
                && !text_lower.contains("env")
                && !text_lower.contains("config")
                && !text_lower.contains("input")
            {
                issues.push(ConcreteIssue {
                    severity: IssueSeverity::Critical,
                    category: IssueCategory::HardcodedCredentials,
                    message: "Hardcoded credentials in assignment".to_string(),
                    file: String::new(),
                    line: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                    rule_id: "SEC001".to_string(),
                    points_deducted: 50,
                });
            }
        }
    }
    if kind == "string_content" {
        if let Ok(text) = node.utf8_text(src_bytes) {
            let sql_like = (text.contains("SELECT") && text.contains("WHERE"))
                || (text.contains("INSERT") && text.contains("VALUES"))
                || (text.contains("UPDATE") && text.contains("SET"))
                || (text.contains("DELETE") && text.contains("FROM"));
            if sql_like {
                if let Some(parent) = node.parent() {
                    if parent.kind() == "string" {
                        if let Ok(parent_text) = parent.utf8_text(src_bytes) {
                            if parent_text.starts_with("f\"") || parent_text.starts_with("f'") {
                                issues.push(ConcreteIssue {
                                    severity: IssueSeverity::Critical,
                                    category: IssueCategory::SqlInjection,
                                    message: "SQL injection risk in f-string - use parameterized queries".to_string(),
                                    file: String::new(),
                                    line: node.start_position().row + 1,
                                    column: node.start_position().column + 1,
                                    rule_id: "SEC001".to_string(),
                                    points_deducted: 50,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}SupportedLanguage::CSharp | SupportedLanguage::Go => {
                    let is_string = match (language, kind) {
                        (SupportedLanguage::CSharp, k) => k == "string_literal" || k.contains("interpolated_string") || k.contains("verbatim_string"),
                        (SupportedLanguage::Go, k) => k == "interpreted_string_literal" || k == "raw_string_literal" || k == "string_literal",
                        _ => false,
                    };
                    if is_string {
                        if let Ok(text) = node.utf8_text(src_bytes) {
                            let low = text.to_lowercase();
                            if (low.contains("password") || low.contains("api_key") || low.contains("secret") || low.contains("token"))
                                && likely_assignment_context(language, &node)
                            {
                                issues.push(ConcreteIssue { severity: IssueSeverity::Critical, category: IssueCategory::HardcodedCredentials, message: "Hardcoded credentials in string literal".to_string(), file: String::new(), line: node.start_position().row + 1, column: node.start_position().column + 1, rule_id: "SEC001".to_string(), points_deducted: 50 });
                            }
                            if (text.contains("SELECT") && text.contains("WHERE")) || (text.contains("INSERT") && text.contains("VALUES")) || (text.contains("UPDATE") && text.contains("SET")) || (text.contains("DELETE") && text.contains("FROM")) {
                                issues.push(ConcreteIssue { severity: IssueSeverity::Major, category: IssueCategory::SqlInjection, message: "Possible SQL in string literal — validate parameterization".to_string(), file: String::new(), line: node.start_position().row + 1, column: node.start_position().column + 1, rule_id: "SEC001".to_string(), points_deducted: 50 });
                            }
                        }
                    }
                }
                SupportedLanguage::Php => {
                    // Heuristics for PHP: detect creds both in assignments and string-like nodes
                    // 1) Assignment with sensitive variable name
                    if kind.contains("assign") {
                        if let Ok(text) = node.utf8_text(src_bytes) {
                            let low = text.to_lowercase();
                            if (low.contains("password") || low.contains("api_key") || low.contains("secret") || low.contains("token"))
                                && text.contains("=")
                                && !low.contains("getenv")
                                && !low.contains("env")
                                && !low.contains("config")
                                && !low.contains("input")
                            {
                                issues.push(ConcreteIssue { severity: IssueSeverity::Critical, category: IssueCategory::HardcodedCredentials, message: "Hardcoded credentials in assignment".to_string(), file: String::new(), line: node.start_position().row + 1, column: node.start_position().column + 1, rule_id: "SEC001".to_string(), points_deducted: 50 });
                            }
                        }
                    }
                    // 2) String-like node with suspicious content
                    if kind.contains("string") {
                        if let Ok(text) = node.utf8_text(src_bytes) {
                            let low = text.to_lowercase();
                            if low.contains("password") || low.contains("api_key") || low.contains("secret") || low.contains("token") {
                                issues.push(ConcreteIssue { severity: IssueSeverity::Critical, category: IssueCategory::HardcodedCredentials, message: "Hardcoded credentials in string literal".to_string(), file: String::new(), line: node.start_position().row + 1, column: node.start_position().column + 1, rule_id: "SEC001".to_string(), points_deducted: 50 });
                            }
                            if (text.contains("SELECT") && text.contains("WHERE")) || (text.contains("INSERT") && text.contains("VALUES")) || (text.contains("UPDATE") && text.contains("SET")) || (text.contains("DELETE") && text.contains("FROM")) {
                                issues.push(ConcreteIssue { severity: IssueSeverity::Major, category: IssueCategory::SqlInjection, message: "Possible SQL in string literal — validate parameterization".to_string(), file: String::new(), line: node.start_position().row + 1, column: node.start_position().column + 1, rule_id: "SEC001".to_string(), points_deducted: 50 });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        issues
    }
}

fn likely_assignment_context(lang: SupportedLanguage, node: &tree_sitter::Node) -> bool {
    let check = |n: &tree_sitter::Node| -> bool {
        let k = n.kind();
        match lang {
            SupportedLanguage::CSharp => k.contains("assignment") || k.contains("declarator") || k.contains("variable_declaration"),
            SupportedLanguage::Go => k.contains("assignment") || k.contains("declaration") || k.contains("short_var_declaration"),
            _ => false,
        }
    };
    if let Some(p) = node.parent() {
        if check(&p) { return true; }
        if let Some(gp) = p.parent() { if check(&gp) { return true; } }
    }
    false
}

fn is_function(lang: SupportedLanguage, kind: &str) -> bool {
    match lang {
        SupportedLanguage::Python => matches!(kind, "function_definition" | "async_function_definition"),
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => matches!(kind, "function_declaration" | "function_expression" | "method_definition" | "arrow_function"),
        SupportedLanguage::Java => matches!(kind, "method_declaration"),
        SupportedLanguage::CSharp => matches!(kind, "method_declaration"),
        SupportedLanguage::Go => matches!(kind, "function_declaration" | "method_declaration"),
        SupportedLanguage::C | SupportedLanguage::Cpp => matches!(kind, "function_definition"),
        SupportedLanguage::Php => matches!(kind, "function_definition" | "method_declaration"),
        _ => false,
    }
}

fn is_function_fast(lang: SupportedLanguage, ids: &Option<kind_ids::KindIds>, node: &tree_sitter::Node) -> bool {
    if let Some(ids) = ids {
        let k = node.kind_id();
        let by_id = match lang {
            SupportedLanguage::Python => k == ids.py_function_definition || k == ids.py_async_function_definition,
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                k == ids.js_function_declaration || k == ids.js_function_expression || k == ids.js_method_definition
            }
            SupportedLanguage::Java => k == ids.java_method_declaration,
            SupportedLanguage::CSharp => k == ids.cs_method_declaration,
            SupportedLanguage::Go => k == ids.go_function_declaration || k == ids.go_method_declaration,
            SupportedLanguage::C => k == ids.c_function_definition,
            SupportedLanguage::Cpp => k == ids.cpp_function_definition,
            SupportedLanguage::Php => k == ids.php_function_definition || k == ids.php_method_declaration,
            SupportedLanguage::Ruby => false,
            _ => false,
        };
        if by_id { return true; }
    }
    // Fallback to string-based
    is_function(lang, node.kind())
}

fn count_params(lang: SupportedLanguage, func: &tree_sitter::Node) -> u32 {
    let mut cur = func.walk();
    if cur.goto_first_child() {
        loop {
            let n = cur.node();
            let is_list = match lang {
                SupportedLanguage::Python => matches!(n.kind(), "parameters" | "parameter_list"),
                SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => matches!(n.kind(), "formal_parameters" | "parameters"),
                SupportedLanguage::Java => matches!(n.kind(), "formal_parameters"),
                SupportedLanguage::CSharp => matches!(n.kind(), "parameter_list"),
                SupportedLanguage::Go => matches!(n.kind(), "parameter_list"),
                _ => false,
            };
            if is_list { return count_param_nodes(lang, &n); }
            if !cur.goto_next_sibling() { break; }
        }
    }
    0
}

fn count_param_nodes(lang: SupportedLanguage, list: &tree_sitter::Node) -> u32 {
    let mut cnt = 0;
    let mut c = list.walk();
    if c.goto_first_child() {
        loop {
            let k = c.node().kind();
            let is_param = match lang {
                SupportedLanguage::Python => matches!(k, "identifier" | "typed_parameter" | "default_parameter" | "list_splat_pattern"),
                SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => matches!(k, "identifier" | "formal_parameter" | "rest_parameter" | "object_pattern" | "array_pattern"),
                SupportedLanguage::Java => matches!(k, "formal_parameter" | "receiver_parameter" | "spread_parameter"),
                SupportedLanguage::CSharp => matches!(k, "parameter" | "parameter_array"),
                SupportedLanguage::Go => matches!(k, "parameter_declaration" | "variadic_parameter_declaration"),
                _ => false,
            };
            if is_param { cnt += 1; }
            if !c.goto_next_sibling() { break; }
        }
    }
    cnt
}

fn scan_complexity(lang: SupportedLanguage, func: &tree_sitter::Node) -> (u32, u32) {
    let mut complexity: u32 = 1;
    let mut max_depth: u32 = 0;
    let mut depth: u32 = 0;
    let mut stack = vec![*func];
    while let Some(n) = stack.pop() {
        for i in (0..n.child_count()).rev() { if let Some(ch) = n.child(i) { stack.push(ch); } }
        let k = n.kind();
        let (is_decision, enters_scope, exits_scope) = match lang {
            SupportedLanguage::Python => (
                matches!(k, "if_statement" | "elif_clause" | "while_statement" | "for_statement" | "try_statement" | "except_clause"),
                matches!(k, "if_statement" | "elif_clause" | "while_statement" | "for_statement" | "try_statement" | "class_definition"),
                false,
            ),
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => (
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "switch_statement" | "catch_clause"),
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "switch_statement" | "try_statement" | "class_declaration" | "statement_block"),
                k == "statement_block",
            ),
            SupportedLanguage::Java => (
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "switch_expression" | "catch_clause"),
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "switch_expression" | "try_statement" | "block" | "class_declaration"),
                k == "block",
            ),
            SupportedLanguage::CSharp => (
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "foreach_statement" | "switch_statement" | "catch_clause"),
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "foreach_statement" | "switch_statement" | "try_statement" | "block" | "class_declaration"),
                k == "block",
            ),
            SupportedLanguage::Go => (
                matches!(k, "if_statement" | "for_statement" | "switch_statement" | "select_statement"),
                matches!(k, "if_statement" | "for_statement" | "switch_statement" | "select_statement" | "block" | "type_declaration"),
                k == "block",
            ),
            SupportedLanguage::C | SupportedLanguage::Cpp => (
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "switch_statement"),
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "switch_statement" | "compound_statement"),
                k == "compound_statement",
            ),
            SupportedLanguage::Php => (
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "switch_statement"),
                matches!(k, "if_statement" | "for_statement" | "while_statement" | "switch_statement" | "compound_statement"),
                k == "compound_statement",
            ),
            _ => (false, false, false),
        };
        if is_decision { complexity += 1; }
        if enters_scope { depth += 1; if depth > max_depth { max_depth = depth; } }
        if exits_scope && depth > 0 { depth -= 1; }
    }
    (complexity, max_depth)
}






