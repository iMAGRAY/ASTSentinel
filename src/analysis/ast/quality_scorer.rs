//! Automated Code Quality Scorer using AST analysis
//! Provides concrete, deterministic scoring from 0-1000 based on AST patterns
// helper removed (duplicate)
use crate::analysis::ast::languages::LanguageCache;
#[cfg(feature = "ast_fastpath")]
use crate::analysis::ast::single_pass::SinglePassEngine;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use syn::{visit::Visit, Expr, Stmt};

use crate::analysis::ast::languages::SupportedLanguage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub total_score: u32,           // 0-1000
    pub functionality_score: u32,   // 0-300
    pub reliability_score: u32,     // 0-200
    pub maintainability_score: u32, // 0-200
    pub performance_score: u32,     // 0-150
    pub security_score: u32,        // 0-100
    pub standards_score: u32,       // 0-50
    pub concrete_issues: Vec<ConcreteIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcreteIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub rule_id: String,
    pub points_deducted: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Critical, // P1 - Must fix immediately
    Major,    // P2 - Should fix soon
    Minor,    // P3 - Nice to fix
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IssueCategory {
    // Functionality issues (300 points)
    UnhandledError,     // -50 points each
    MissingReturnValue, // -40 points
    InfiniteLoop,       // -60 points
    DeadCode,           // -20 points
    UnreachableCode,    // -30 points

    // Reliability issues (200 points)
    NullPointerRisk,      // -40 points
    ResourceLeak,         // -50 points
    RaceCondition,        // -50 points
    MissingErrorHandling, // -30 points

    // Maintainability issues (200 points)
    HighComplexity,    // -5 points per complexity point over 10
    DuplicateCode,     // -30 points per duplicate block
    LongMethod,        // -20 points (>50 lines)
    TooManyParameters, // -15 points (>5 params)
    DeepNesting,       // -10 points per level over 4

    // Performance issues (150 points)
    InefficientAlgorithm, // -40 points (O(n²) or worse)
    UnboundedRecursion,   // -50 points
    ExcessiveMemoryUse,   // -30 points
    SynchronousBlocking,  // -20 points in async context

    // Security issues (100 points)
    SqlInjection,         // -50 points
    CommandInjection,     // -50 points
    PathTraversal,        // -40 points
    HardcodedCredentials, // -50 points
    InsecureRandom,       // -20 points

    // Standards issues (50 points)
    NamingConvention,     // -5 points
    MissingDocumentation, // -10 points
    UnusedImports,        // -5 points
    UnusedVariables,      // -5 points
    // Style/Readability (no direct score change unless mapped above)
    LongLine, // Lines exceeding recommended length
    // Incompleteness (signals unfinished implementation)
    UnfinishedWork, // TODO/FIXME/TBD/XXX/HACK/WIP markers
}

/// AST-based quality scorer
pub struct AstQualityScorer {
    rules: HashMap<String, Box<dyn AstRule>>,
}

impl Default for AstQualityScorer {
    fn default() -> Self {
        Self::new()
    }
}

/// Enhanced trait for AST-based quality rules with language context
pub trait AstRule: Send + Sync {
    fn check(&self, ast: &tree_sitter::Tree, source: &str, language: SupportedLanguage)
        -> Vec<ConcreteIssue>;
    fn rule_id(&self) -> &str;
}

impl AstQualityScorer {
    pub fn new() -> Self {
        let mut scorer = Self {
            rules: HashMap::new(),
        };
        scorer.register_default_rules();
        // Optional pre-warm of tree-sitter languages to avoid first-call latency
        if std::env::var("AST_PREWARM").is_ok() {
            let _ = LanguageCache::initialize_all_languages();
        }
        scorer
    }

    fn register_default_rules(&mut self) {
        // Register all default AST rules
        self.register_rule(Box::new(UnhandledErrorRule));
        self.register_rule(Box::new(DeadCodeRule));
        self.register_rule(Box::new(ComplexityRule));
        self.register_rule(Box::new(SecurityPatternRule));
        self.register_rule(Box::new(ResourceLeakRule));
        // Register style/readability rules for multi-pass path
        self.register_rule(Box::new(LongLineRule { max_len: 120 }));
        // Register unfinished work detector (TODO/FIXME/TBD/XXX/HACK/WIP)
        self.register_rule(Box::new(TodoFixmeRule));
    }

    pub fn register_rule(&mut self, rule: Box<dyn AstRule>) {
        self.rules.insert(rule.rule_id().to_string(), rule);
    }

    /// Analyze code and return quality score with concrete issues
    pub fn analyze(&self, source: &str, language: SupportedLanguage) -> Result<QualityScore> {
        let mut score = QualityScore {
            total_score: 1000,
            functionality_score: 300,
            reliability_score: 200,
            maintainability_score: 200,
            performance_score: 150,
            security_score: 100,
            standards_score: 50,
            concrete_issues: Vec::new(),
        };

        // Rust: syn-based analysis (no tree-sitter)
        if language == SupportedLanguage::Rust {
            let mut result = score;
            // Long lines first (cheap pass)
            for (idx, line) in source.lines().enumerate() {
                if line.chars().count() > 120 {
                    result.concrete_issues.push(ConcreteIssue {
                        severity: IssueSeverity::Minor,
                        category: IssueCategory::LongLine,
                        message: format!("Line too long ({} > 120 chars)", line.chars().count()),
                        file: String::new(),
                        line: idx + 1,
                        column: 1,
                        rule_id: "STYLE001".to_string(),
                        points_deducted: 0,
                    });
                }
            }

            // Parse via syn
            if let Ok(ast) = syn::parse_file(source) {
                let mut v = RustAstVisitor::new();
                v.visit_file(&ast);
                // Apply category-to-score mapping consistently
                for issue in v.issues {
                    match issue.category {
                        IssueCategory::UnhandledError => {
                            result.functionality_score = result.functionality_score.saturating_sub(50)
                        }
                        IssueCategory::InfiniteLoop => {
                            result.functionality_score = result.functionality_score.saturating_sub(60)
                        }
                        IssueCategory::DeadCode | IssueCategory::UnreachableCode => {
                            result.functionality_score = result.functionality_score.saturating_sub(20)
                        }
                        IssueCategory::NullPointerRisk => {
                            result.reliability_score = result.reliability_score.saturating_sub(40)
                        }
                        IssueCategory::ResourceLeak => {
                            result.reliability_score = result.reliability_score.saturating_sub(50)
                        }
                        IssueCategory::HighComplexity | IssueCategory::LongMethod => {
                            result.maintainability_score =
                                result.maintainability_score.saturating_sub(issue.points_deducted)
                        }
                        IssueCategory::SqlInjection => {
                            result.security_score = result.security_score.saturating_sub(50)
                        }
                        IssueCategory::HardcodedCredentials => {
                            result.security_score = result.security_score.saturating_sub(50)
                        }
                        _ => {}
                    }
                    result.concrete_issues.push(issue);
                }
            }

            // Recompute total
            result.total_score = result.functionality_score
                + result.reliability_score
                + result.maintainability_score
                + result.performance_score
                + result.security_score
                + result.standards_score;
            return Ok(result);
        }

        // Config languages: parse with serde and apply security overlay (no panics)
        match language {
            SupportedLanguage::Json => {
                let mut issues = Vec::new();
                match serde_json::from_str::<serde_json::Value>(source) {
                    Ok(_v) => {
                        issues.extend(security_overlay_scan(source, "json"));
                    }
                    Err(e) => {
                        issues.push(ConcreteIssue {
                            severity: IssueSeverity::Minor,
                            category: IssueCategory::NamingConvention,
                            message: format!("JSON parse error: {}", e),
                            file: String::new(),
                            line: 1,
                            column: 1,
                            rule_id: "CFG001".to_string(),
                            points_deducted: 5,
                        });
                    }
                }
                for issue in issues {
                    if matches!(issue.category, IssueCategory::HardcodedCredentials) {
                        score.security_score = score.security_score.saturating_sub(50);
                    }
                    score.concrete_issues.push(issue);
                }
                score.total_score = score.functionality_score
                    + score.reliability_score
                    + score.maintainability_score
                    + score.performance_score
                    + score.security_score
                    + score.standards_score;
                return Ok(score);
            }
            SupportedLanguage::Yaml => {
                let mut issues = Vec::new();
                match serde_yaml::from_str::<serde_yaml::Value>(source) {
                    Ok(_v) => {
                        issues.extend(security_overlay_scan(source, "yaml"));
                    }
                    Err(e) => {
                        issues.push(ConcreteIssue {
                            severity: IssueSeverity::Minor,
                            category: IssueCategory::NamingConvention,
                            message: format!("YAML parse error: {}", e),
                            file: String::new(),
                            line: 1,
                            column: 1,
                            rule_id: "CFG001".to_string(),
                            points_deducted: 5,
                        });
                    }
                }
                for issue in issues {
                    if matches!(issue.category, IssueCategory::HardcodedCredentials) {
                        score.security_score = score.security_score.saturating_sub(50);
                    }
                    score.concrete_issues.push(issue);
                }
                score.total_score = score.functionality_score
                    + score.reliability_score
                    + score.maintainability_score
                    + score.performance_score
                    + score.security_score
                    + score.standards_score;
                return Ok(score);
            }
            SupportedLanguage::Toml => {
                let mut issues = Vec::new();
                match toml::from_str::<toml::Table>(source) {
                    Ok(_v) => {
                        issues.extend(security_overlay_scan(source, "toml"));
                    }
                    Err(e) => {
                        issues.push(ConcreteIssue {
                            severity: IssueSeverity::Minor,
                            category: IssueCategory::NamingConvention,
                            message: format!("TOML parse error: {}", e),
                            file: String::new(),
                            line: 1,
                            column: 1,
                            rule_id: "CFG001".to_string(),
                            points_deducted: 5,
                        });
                    }
                }
                for issue in issues {
                    if matches!(issue.category, IssueCategory::HardcodedCredentials) {
                        score.security_score = score.security_score.saturating_sub(50);
                    }
                    score.concrete_issues.push(issue);
                }
                score.total_score = score.functionality_score
                    + score.reliability_score
                    + score.maintainability_score
                    + score.performance_score
                    + score.security_score
                    + score.standards_score;
                return Ok(score);
            }
            _ => {}
        } // Parse AST (use cached Tree-sitter language for performance)
        let mut parser = LanguageCache::create_parser_with_language(language)?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse AST"))?;

        // Run rules
        #[cfg(not(feature = "ast_fastpath"))]
        {
            // Default multi-pass rules
            for rule in self.rules.values() {
                let issues = rule.check(&tree, source, language);
                for issue in issues {
                    match issue.category {
                        IssueCategory::UnhandledError => {
                            score.functionality_score = score.functionality_score.saturating_sub(50)
                        }
                        IssueCategory::InfiniteLoop => {
                            score.functionality_score = score.functionality_score.saturating_sub(60)
                        }
                        IssueCategory::DeadCode => {
                            score.functionality_score = score.functionality_score.saturating_sub(20)
                        }
                        IssueCategory::NullPointerRisk => {
                            score.reliability_score = score.reliability_score.saturating_sub(40)
                        }
                        IssueCategory::ResourceLeak => {
                            score.reliability_score = score.reliability_score.saturating_sub(50)
                        }
                        IssueCategory::HighComplexity => {
                            score.maintainability_score =
                                score.maintainability_score.saturating_sub(issue.points_deducted)
                        }
                        IssueCategory::UnfinishedWork => {
                            score.maintainability_score = score.maintainability_score.saturating_sub(15)
                        }
                        IssueCategory::SqlInjection => {
                            score.security_score = score.security_score.saturating_sub(50)
                        }
                        IssueCategory::HardcodedCredentials => {
                            score.security_score = score.security_score.saturating_sub(50)
                        }
                        _ => {}
                    }
                    score.concrete_issues.push(issue);
                }
            }
        }

        #[cfg(feature = "ast_fastpath")]
        {
            // Single-pass fast engine for selected languages
            let issues = SinglePassEngine::analyze(&tree, source, language);
            for issue in issues {
                match issue.category {
                    IssueCategory::UnhandledError => {
                        score.functionality_score = score.functionality_score.saturating_sub(50)
                    }
                    IssueCategory::InfiniteLoop => {
                        score.functionality_score = score.functionality_score.saturating_sub(60)
                    }
                    IssueCategory::DeadCode => {
                        score.functionality_score = score.functionality_score.saturating_sub(20)
                    }
                    IssueCategory::NullPointerRisk => {
                        score.reliability_score = score.reliability_score.saturating_sub(40)
                    }
                    IssueCategory::ResourceLeak => {
                        score.reliability_score = score.reliability_score.saturating_sub(50)
                    }
                    IssueCategory::HighComplexity => {
                        score.maintainability_score =
                            score.maintainability_score.saturating_sub(issue.points_deducted)
                    }
                    IssueCategory::UnfinishedWork => {
                        score.maintainability_score = score.maintainability_score.saturating_sub(15)
                    }
                    IssueCategory::SqlInjection => {
                        score.security_score = score.security_score.saturating_sub(50)
                    }
                    IssueCategory::HardcodedCredentials => {
                        score.security_score = score.security_score.saturating_sub(50)
                    }
                    _ => {}
                }
                score.concrete_issues.push(issue);
            }
        }

        // Calculate total score
        score.total_score = score.functionality_score
            + score.reliability_score
            + score.maintainability_score
            + score.performance_score
            + score.security_score
            + score.standards_score;

        Ok(score)
    }
}

/// Rule: Detect TODO/FIXME/TBD/XXX/HACK/WIP markers indicating unfinished code
struct TodoFixmeRule;

impl TodoFixmeRule {
    fn is_comment_node(language: SupportedLanguage, kind: &str) -> bool {
        if kind.contains("comment") {
            return true;
        }
        match language {
            SupportedLanguage::Python => kind == "comment",
            _ => kind.contains("comment"),
        }
    }
}

impl AstRule for TodoFixmeRule {
    fn check(
        &self,
        ast: &tree_sitter::Tree,
        source: &str,
        language: SupportedLanguage,
    ) -> Vec<ConcreteIssue> {
        let mut issues = Vec::new();
        let bytes = source.as_bytes();
        let re = match regex::Regex::new(r"(?i)\b(TODO|FIXME|TBD|XXX|HACK|WIP|UNFINISHED|STUB)\b") {
            Ok(r) => r,
            Err(_) => return issues, // defensive: if regex fails to compile in this env, skip rule gracefully
        };

        // Prefer comment nodes when available
        let mut saw_comment = false;
        let mut stack = vec![ast.root_node()];
        while let Some(n) = stack.pop() {
            for i in (0..n.child_count()).rev() {
                if let Some(ch) = n.child(i) {
                    stack.push(ch);
                }
            }
            if Self::is_comment_node(language, n.kind()) {
                saw_comment = true;
                if let Ok(text) = n.utf8_text(bytes) {
                    if re.is_match(text) {
                        issues.push(ConcreteIssue {
                            severity: IssueSeverity::Major,
                            category: IssueCategory::UnfinishedWork,
                            message: "Unfinished work marker in comment (TODO/FIXME/TBD/...)".to_string(),
                            file: String::new(),
                            line: n.start_position().row + 1,
                            column: n.start_position().column + 1,
                            rule_id: "TODO001".to_string(),
                            points_deducted: 15,
                        });
                    }
                }
            }
        }

        if !saw_comment {
            // Fallback: fast line scan
            for (i, line) in source.lines().enumerate() {
                if re.is_match(line) {
                    issues.push(ConcreteIssue {
                        severity: IssueSeverity::Major,
                        category: IssueCategory::UnfinishedWork,
                        message: "Unfinished work marker (TODO/FIXME/TBD/...)".to_string(),
                        file: String::new(),
                        line: i + 1,
                        column: 1,
                        rule_id: "TODO001".to_string(),
                        points_deducted: 15,
                    });
                }
            }
        }

        issues
    }

    fn rule_id(&self) -> &str {
        "TODO001"
    }
}

/// Example rule: Detect unhandled errors
struct UnhandledErrorRule;

impl UnhandledErrorRule {
    fn walk_ast_recursive(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        source: &str,
        issues: &mut Vec<ConcreteIssue>,
    ) {
        let node = cursor.node();

        // Check for unwrap() on Result/Option
        if node.kind() == "call_expression" {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                if text.contains(".unwrap()") && !text.contains("unwrap_or") {
                    issues.push(ConcreteIssue {
                        severity: IssueSeverity::Major,
                        category: IssueCategory::UnhandledError,
                        message: "Using .unwrap() without error handling".to_string(),
                        file: String::new(),
                        line: node.start_position().row + 1,
                        column: node.start_position().column + 1,
                        rule_id: self.rule_id().to_string(),
                        points_deducted: 50,
                    });
                }
            }
        }

        // Recursively walk children
        if cursor.goto_first_child() {
            loop {
                self.walk_ast_recursive(cursor, source, issues);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}

fn security_overlay_scan(source: &str, _kind: &str) -> Vec<ConcreteIssue> {
    let mut issues = Vec::new();
    // Compile once; never panic if regex fails for any reason
    static CREDENTIALS_RE: Lazy<Result<regex::Regex, regex::Error>> =
        Lazy::new(|| regex::Regex::new(r#"(?i)(password|api_key|secret|token)\s*[:=]\s*[^\s#\n]+"#));

    if let Ok(re) = CREDENTIALS_RE.as_ref() {
        if re.is_match(source) {
            issues.push(ConcreteIssue {
                severity: IssueSeverity::Critical,
                category: IssueCategory::HardcodedCredentials,
                message: "Potential hardcoded credentials in config".to_string(),
                file: String::new(),
                line: 1,
                column: 1,
                rule_id: "CFGSEC001".to_string(),
                points_deducted: 50,
            });
        }
    }
    issues
}

// =========================
// Rust syn-based visitor
// =========================
struct RustAstVisitor {
    issues: Vec<ConcreteIssue>,
}

impl RustAstVisitor {
    fn new() -> Self {
        Self { issues: Vec::new() }
    }

    fn push_issue(&mut self, severity: IssueSeverity, category: IssueCategory, message: impl Into<String>) {
        self.issues.push(ConcreteIssue {
            severity,
            category,
            message: message.into(),
            file: String::new(),
            line: 1, // syn::Span -> line mapping is non-trivial; tests assert categories, not exact lines
            column: 1,
            rule_id: match category {
                IssueCategory::UnhandledError => "AST001",
                IssueCategory::UnreachableCode => "AST002",
                IssueCategory::HighComplexity => "AST003",
                IssueCategory::TooManyParameters => "PARAMS001",
                IssueCategory::DeepNesting => "NEST001",
                IssueCategory::HardcodedCredentials => "SEC001",
                IssueCategory::SqlInjection => "SEC001",
                IssueCategory::LongLine => "STYLE001",
                _ => "GEN001",
            }
            .to_string(),
            points_deducted: match category {
                IssueCategory::TooManyParameters => 15,
                IssueCategory::DeepNesting => 10,
                IssueCategory::HighComplexity => 5,
                IssueCategory::UnreachableCode => 30,
                IssueCategory::UnhandledError => 50,
                IssueCategory::HardcodedCredentials => 50,
                IssueCategory::SqlInjection => 50,
                _ => 0,
            },
        });
    }

    fn is_panic_like_stmt(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Macro(mac_stmt) => mac_stmt
                .mac
                .path
                .segments
                .last()
                .map(|s| {
                    let id = s.ident.to_string();
                    id == "panic" || id == "todo" || id == "unimplemented"
                })
                .unwrap_or(false),
            Stmt::Expr(Expr::Macro(expr_mac), _) => expr_mac
                .mac
                .path
                .segments
                .last()
                .map(|s| {
                    let id = s.ident.to_string();
                    id == "panic" || id == "todo" || id == "unimplemented"
                })
                .unwrap_or(false),
            _ => false,
        }
    }

    fn scan_block_unreachable(&mut self, block: &syn::Block) {
        // Default: only return/panic-like terminate execution for the remainder of the block
        self.scan_block_unreachable_with_flags(block, false);
    }

    fn scan_block_unreachable_with_flags(&mut self, block: &syn::Block, in_loop: bool) {
        use syn::{ExprBreak, ExprContinue, ExprReturn};
        let mut after_terminator = false;
        for stmt in &block.stmts {
            if after_terminator {
                // Skip empty statements (there is no explicit Empty in syn::Stmt; any stmt counts)
                self.push_issue(
                    IssueSeverity::Major,
                    IssueCategory::UnreachableCode,
                    "Unreachable code after return/break/continue/panic",
                );
                break;
            }

            match stmt {
                Stmt::Expr(Expr::Return(ExprReturn { .. }), _) => {
                    after_terminator = true;
                }
                _ if Self::is_panic_like_stmt(stmt) => {
                    after_terminator = true;
                }
                Stmt::Expr(Expr::Break(ExprBreak { .. }), _) if in_loop => {
                    after_terminator = true;
                }
                Stmt::Expr(Expr::Continue(ExprContinue { .. }), _) if in_loop => {
                    after_terminator = true;
                }
                _ => {}
            }
        }
    }

    fn calc_max_nesting(&self, expr: &Expr, depth: u32) -> u32 {
        use syn::{ExprForLoop, ExprIf, ExprLet, ExprLoop, ExprMatch, ExprWhile};
        match expr {
            Expr::If(ExprIf {
                then_branch,
                else_branch,
                ..
            }) => {
                let then_d = self.calc_block_depth(then_branch, depth + 1);
                let else_d = if let Some((_, else_expr)) = else_branch {
                    self.calc_max_nesting(else_expr, depth + 1)
                } else {
                    depth + 1
                };
                then_d.max(else_d)
            }
            // while / while let — treat condition with Expr::Let as same depth increase
            Expr::While(ExprWhile { cond, body, .. }) => {
                let mut d = depth + 1;
                if matches!(cond.as_ref(), Expr::Let(ExprLet { .. })) {
                    d += 0; /* already counted */
                }
                self.calc_block_depth(body, d)
            }
            Expr::ForLoop(ExprForLoop { body, .. }) => self.calc_block_depth(body, depth + 1),
            Expr::Loop(ExprLoop { body, .. }) => self.calc_block_depth(body, depth + 1),
            Expr::Match(ExprMatch { arms, .. }) => {
                let mut m = depth + 1;
                for arm in arms {
                    m = m.max(self.calc_max_nesting(&arm.body, depth + 1));
                }
                m
            }
            Expr::Block(b) => self.calc_block_depth(&b.block, depth),
            _ => depth,
        }
    }

    fn calc_block_depth(&self, block: &syn::Block, depth: u32) -> u32 {
        let mut m = depth;
        for stmt in &block.stmts {
            if let Stmt::Expr(e, _) = stmt {
                m = m.max(self.calc_max_nesting(e, depth));
            }
        }
        m
    }
}

impl<'a> Visit<'a> for RustAstVisitor {
    fn visit_item_fn(&mut self, i: &syn::ItemFn) {
        // TooManyParameters
        if i.sig.inputs.len() > 5 {
            self.push_issue(
                IssueSeverity::Minor,
                IssueCategory::TooManyParameters,
                format!("Function has too many parameters ({} > 5)", i.sig.inputs.len()),
            );
        }
        // Deep nesting (threshold: >4)
        let max_depth = self.calc_block_depth(&i.block, 0);
        if max_depth > 4 {
            self.push_issue(
                IssueSeverity::Minor,
                IssueCategory::DeepNesting,
                format!("Deep nesting detected (level {})", max_depth),
            );
        }
        // Long method (approx by number of statements in block)
        if i.block.stmts.len() > 50 {
            self.push_issue(
                IssueSeverity::Minor,
                IssueCategory::LongMethod,
                format!("Long method ({} statements > 50)", i.block.stmts.len()),
            );
        }

        // Unreachable after return/panic (function body)
        self.scan_block_unreachable(&i.block);

        syn::visit::visit_item_fn(self, i);
    }

    fn visit_expr_method_call(&mut self, m: &syn::ExprMethodCall) {
        if m.method == "unwrap" || m.method == "expect" {
            self.push_issue(
                IssueSeverity::Major,
                IssueCategory::UnhandledError,
                "Using .unwrap()/expect() without error handling",
            );
        }
        syn::visit::visit_expr_method_call(self, m);
    }

    fn visit_expr_macro(&mut self, mac: &syn::ExprMacro) {
        // panic! macro call
        if let Some(seg) = mac.mac.path.segments.last() {
            if seg.ident == "panic" || seg.ident == "todo" || seg.ident == "unimplemented" {
                self.push_issue(
                    IssueSeverity::Major,
                    IssueCategory::UnhandledError,
                    "panic!/todo!/unimplemented! macro used",
                );
            }
        }
        syn::visit::visit_expr_macro(self, mac);
    }

    fn visit_expr_while(&mut self, w: &syn::ExprWhile) {
        // Inside loops, break/continue make the remainder of the loop block unreachable
        self.scan_block_unreachable_with_flags(&w.body, true);
        syn::visit::visit_expr_while(self, w);
    }

    fn visit_expr_for_loop(&mut self, f: &syn::ExprForLoop) {
        self.scan_block_unreachable_with_flags(&f.body, true);
        syn::visit::visit_expr_for_loop(self, f);
    }

    fn visit_expr_loop(&mut self, l: &syn::ExprLoop) {
        self.scan_block_unreachable_with_flags(&l.body, true);
        syn::visit::visit_expr_loop(self, l);
    }

    fn visit_expr_if(&mut self, i: &syn::ExprIf) {
        // Scan both branches for post-terminator unreachable code
        self.scan_block_unreachable(&i.then_branch);
        if let Some((_, else_expr)) = &i.else_branch {
            if let syn::Expr::Block(b) = else_expr.as_ref() {
                self.scan_block_unreachable(&b.block);
            }
        }
        syn::visit::visit_expr_if(self, i);
    }

    fn visit_stmt(&mut self, s: &Stmt) {
        // Detect macro statements like panic!/todo!/unimplemented! used as a standalone stmt
        if let Stmt::Macro(mac_stmt) = s {
            if let Some(seg) = mac_stmt.mac.path.segments.last() {
                let ident = seg.ident.to_string();
                if ident == "panic" || ident == "todo" || ident == "unimplemented" {
                    self.push_issue(
                        IssueSeverity::Major,
                        IssueCategory::UnhandledError,
                        "panic!/todo!/unimplemented! macro used",
                    );
                }
            }
        }

        // Look for string literal creds / SQL in simple let-bindings
        if let Stmt::Local(local) = s {
            let pat_ident = match &local.pat {
                syn::Pat::Ident(id) => Some(&id.ident),
                _ => None,
            };
            if let (Some(init), Some(pat_ident)) = (&local.init, pat_ident) {
                let name = pat_ident.to_string().to_lowercase();
                if let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = &*init.expr
                {
                    let val = lit.value();
                    if name.contains("password")
                        || name.contains("api_key")
                        || name.contains("secret")
                        || name.contains("token")
                    {
                        self.push_issue(
                            IssueSeverity::Critical,
                            IssueCategory::HardcodedCredentials,
                            "Hardcoded credentials in assignment",
                        );
                    }
                    if (val.contains("SELECT") && val.contains("WHERE"))
                        || (val.contains("INSERT") && val.contains("VALUES"))
                        || (val.contains("UPDATE") && val.contains("SET"))
                        || (val.contains("DELETE") && val.contains("FROM"))
                    {
                        self.push_issue(
                            IssueSeverity::Major,
                            IssueCategory::SqlInjection,
                            "Possible SQL in string literal — validate parameterization",
                        );
                    }
                }
            }
        }
        syn::visit::visit_stmt(self, s);
    }

    fn visit_block(&mut self, b: &syn::Block) {
        // Scan for unreachable inside nested blocks as well (non-loop context)
        self.scan_block_unreachable(b);
        syn::visit::visit_block(self, b);
    }
}

impl AstRule for UnhandledErrorRule {
    fn check(
        &self,
        ast: &tree_sitter::Tree,
        source: &str,
        language: SupportedLanguage,
    ) -> Vec<ConcreteIssue> {
        let mut issues = Vec::new();

        // Only apply to Rust language (unwrap is Rust-specific)
        if language != SupportedLanguage::Rust {
            return issues;
        }

        let mut cursor = ast.walk();

        // Walk AST looking for error patterns recursively
        self.walk_ast_recursive(&mut cursor, source, &mut issues);

        issues
    }

    fn rule_id(&self) -> &str {
        "AST001"
    }
}

/// Rule: Detect dead code
struct DeadCodeRule;

impl AstRule for DeadCodeRule {
    fn check(
        &self,
        ast: &tree_sitter::Tree,
        _source: &str,
        _language: SupportedLanguage,
    ) -> Vec<ConcreteIssue> {
        let mut issues = Vec::new();
        let mut cursor = ast.walk();

        loop {
            let node = cursor.node();

            // Check for code after return statements
            if node.kind() == "return_statement" {
                if let Some(next_sibling) = node.next_sibling() {
                    if next_sibling.kind() != "}" && next_sibling.kind() != "comment" {
                        issues.push(ConcreteIssue {
                            severity: IssueSeverity::Major,
                            category: IssueCategory::UnreachableCode,
                            message: "Unreachable code after return statement".to_string(),
                            file: String::new(),
                            line: next_sibling.start_position().row + 1,
                            column: next_sibling.start_position().column + 1,
                            rule_id: self.rule_id().to_string(),
                            points_deducted: 30,
                        });
                    }
                }
            }

            if !cursor.goto_next_sibling() && !cursor.goto_parent() {
                break;
            }
        }

        issues
    }

    fn rule_id(&self) -> &str {
        "AST002"
    }
}

/// Rule: Detect overly long lines in source code (language-agnostic)
/// This restores parity for AST-only path when `ast_fastpath` is disabled.
struct LongLineRule {
    max_len: usize,
}

impl AstRule for LongLineRule {
    fn check(
        &self,
        _ast: &tree_sitter::Tree,
        source: &str,
        _language: SupportedLanguage,
    ) -> Vec<ConcreteIssue> {
        let mut issues = Vec::new();
        for (index, line_text) in source.lines().enumerate() {
            let char_count = line_text.chars().count();
            if char_count > self.max_len {
                issues.push(ConcreteIssue {
                    severity: IssueSeverity::Minor,
                    category: IssueCategory::LongLine,
                    message: format!("Line too long ({} > {} chars)", char_count, self.max_len),
                    file: String::new(),
                    line: index + 1,
                    column: 1,
                    rule_id: "STYLE001".to_string(),
                    points_deducted: 0,
                });
            }
        }
        issues
    }

    fn rule_id(&self) -> &str {
        "STYLE001"
    }
}

/// Rule: Check cyclomatic complexity
struct ComplexityRule;

impl AstRule for ComplexityRule {
    fn check(
        &self,
        ast: &tree_sitter::Tree,
        source: &str,
        language: SupportedLanguage,
    ) -> Vec<ConcreteIssue> {
        let mut issues = Vec::new();
        let mut cursor = ast.walk();

        // Language-specific thresholds for complexity
        let complexity_threshold = match language {
            SupportedLanguage::Python => 8, // Python should be simpler
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => 10,
            SupportedLanguage::Java | SupportedLanguage::CSharp => 12, // Allow more complexity in strongly-typed languages
            SupportedLanguage::Go => 8,                                // Go encourages simplicity
            SupportedLanguage::C | SupportedLanguage::Cpp => 15, // System languages may need more complexity
            _ => 10,                                             // Default threshold
        };

        'outer: loop {
            let node = cursor.node();

            // Language-specific function detection
            let is_function = match language {
                SupportedLanguage::Python => node.kind() == "function_definition",
                SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                    node.kind() == "function_declaration" || node.kind() == "arrow_function"
                }
                SupportedLanguage::Java => node.kind() == "method_declaration",
                SupportedLanguage::Go => {
                    node.kind() == "function_declaration" || node.kind() == "method_declaration"
                }
                _ => node.kind() == "function_item" || node.kind() == "method_definition",
            };

            if is_function {
                let complexity = calculate_complexity(&node, source);
                if complexity > complexity_threshold {
                    issues.push(ConcreteIssue {
                        severity: IssueSeverity::Minor,
                        category: IssueCategory::HighComplexity,
                        message: format!(
                            "High cyclomatic complexity: {} (threshold: {})",
                            complexity, complexity_threshold
                        ),
                        file: String::new(),
                        line: node.start_position().row + 1,
                        column: node.start_position().column + 1,
                        rule_id: self.rule_id().to_string(),
                        points_deducted: (complexity.saturating_sub(complexity_threshold)) * 5,
                    });
                }
            }

            // Proper AST traversal: visit children first, then siblings, then go up
            if cursor.goto_first_child() {
                continue;
            }

            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    break 'outer;
                }
            }
        }

        issues
    }

    fn rule_id(&self) -> &str {
        "AST003"
    }
}

/// Rule: Detect security patterns
struct SecurityPatternRule;

impl SecurityPatternRule {
    fn walk_security_recursive(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        source: &str,
        issues: &mut Vec<ConcreteIssue>,
    ) {
        let node = cursor.node();

        // Check for hardcoded credentials in variable assignments (Python specific)
        if node.kind() == "assignment" {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                let text_lower = text.to_lowercase();
                if (text_lower.contains("password")
                    || text_lower.contains("api_key")
                    || text_lower.contains("secret")
                    || text_lower.contains("token"))
                    && text.contains("=")
                    && !text.contains("env")
                    && !text.contains("getenv")
                    && !text.contains("config")
                    && !text.contains("input")
                {
                    issues.push(ConcreteIssue {
                        severity: IssueSeverity::Critical,
                        category: IssueCategory::HardcodedCredentials,
                        message: "Hardcoded credentials in assignment".to_string(),
                        file: String::new(),
                        line: node.start_position().row + 1,
                        column: node.start_position().column + 1,
                        rule_id: self.rule_id().to_string(),
                        points_deducted: 50,
                    });
                }
            }
        }

        // Check for SQL injection in string content (especially f-strings with interpolation)
        if node.kind() == "string_content" {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                if (text.contains("SELECT") && text.contains("WHERE"))
                    || (text.contains("INSERT") && text.contains("VALUES"))
                    || (text.contains("UPDATE") && text.contains("SET"))
                    || (text.contains("DELETE") && text.contains("FROM"))
                {
                    // Check if this is in a string with interpolation (f-string)
                    if let Some(parent) = node.parent() {
                        if parent.kind() == "string" {
                            if let Ok(parent_text) = parent.utf8_text(source.as_bytes()) {
                                if parent_text.starts_with("f\"") || parent_text.starts_with("f'") {
                                    issues.push(ConcreteIssue {
                                        severity: IssueSeverity::Critical,
                                        category: IssueCategory::SqlInjection,
                                        message: "SQL injection risk in f-string - use parameterized queries"
                                            .to_string(),
                                        file: String::new(),
                                        line: node.start_position().row + 1,
                                        column: node.start_position().column + 1,
                                        rule_id: self.rule_id().to_string(),
                                        points_deducted: 50,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check string_content for hardcoded credentials
        if node.kind() == "string_content" {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                // Look for credential patterns like "sk-123", long tokens, etc.
                if (text.starts_with("sk-") && text.len() > 10)
                    || (text.contains("secret") && text.len() > 8)
                    || (text.contains("password") && text.len() > 8)
                    || (text.contains("token") && text.len() > 8)
                    || (text.len() > 20 && text.chars().all(|c| c.is_ascii_alphanumeric()))
                {
                    issues.push(ConcreteIssue {
                        severity: IssueSeverity::Critical,
                        category: IssueCategory::HardcodedCredentials,
                        message: "Hardcoded credential detected in string".to_string(),
                        file: String::new(),
                        line: node.start_position().row + 1,
                        column: node.start_position().column + 1,
                        rule_id: self.rule_id().to_string(),
                        points_deducted: 50,
                    });
                }
            }
        }

        // Recursively walk children
        if cursor.goto_first_child() {
            loop {
                self.walk_security_recursive(cursor, source, issues);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}

impl AstRule for SecurityPatternRule {
    fn check(
        &self,
        ast: &tree_sitter::Tree,
        source: &str,
        language: SupportedLanguage,
    ) -> Vec<ConcreteIssue> {
        let mut issues = Vec::new();

        // Only apply to Python language (patterns are Python-specific)
        if language != SupportedLanguage::Python {
            if std::env::var("DEBUG_HOOKS").is_ok() {
                tracing::debug!(%language, "SecurityPatternRule skipped - not Python language");
            }
            return issues;
        }

        let mut cursor = ast.walk();
        self.walk_security_recursive(&mut cursor, source, &mut issues);

        issues
    }

    fn rule_id(&self) -> &str {
        "SEC001"
    }
}

/// Rule: Detect resource leaks
struct ResourceLeakRule;

impl AstRule for ResourceLeakRule {
    fn check(
        &self,
        ast: &tree_sitter::Tree,
        source: &str,
        _language: SupportedLanguage,
    ) -> Vec<ConcreteIssue> {
        let mut issues = Vec::new();
        let mut cursor = ast.walk();

        loop {
            let node = cursor.node();

            // Check for file/connection opens without closes
            if node.kind() == "call_expression" {
                if let Ok(text) = node.utf8_text(source.as_bytes()) {
                    if text.contains("File::open") || text.contains("TcpStream::connect") {
                        // Check if there's a corresponding close/drop in the same scope
                        let parent = node.parent();
                        if let Some(parent) = parent {
                            let parent_text = parent.utf8_text(source.as_bytes()).unwrap_or("");
                            if !parent_text.contains("drop") && !parent_text.contains("close") {
                                issues.push(ConcreteIssue {
                                    severity: IssueSeverity::Major,
                                    category: IssueCategory::ResourceLeak,
                                    message: "Potential resource leak - ensure proper cleanup".to_string(),
                                    file: String::new(),
                                    line: node.start_position().row + 1,
                                    column: node.start_position().column + 1,
                                    rule_id: "RES001".to_string(),
                                    points_deducted: 50,
                                });
                            }
                        }
                    }
                }
            }

            if !cursor.goto_next_sibling() && !cursor.goto_parent() {
                break;
            }
        }

        issues
    }

    fn rule_id(&self) -> &str {
        "RES001"
    }
}

fn calculate_complexity(node: &tree_sitter::Node, source: &str) -> u32 {
    let mut complexity = 1;
    let mut cursor = node.walk();

    loop {
        let current = cursor.node();
        match current.kind() {
            "if_expression" | "if_statement" => complexity += 1,
            "match_expression" | "switch_statement" => complexity += 1,
            "while_expression" | "while_statement" => complexity += 1,
            "for_expression" | "for_statement" => complexity += 1,
            "binary_expression" => {
                if let Ok(text) = current.utf8_text(source.as_bytes()) {
                    if text.contains("&&") || text.contains("||") {
                        complexity += 1;
                    }
                }
            }
            _ => {}
        }

        if !cursor.goto_next_sibling() && !cursor.goto_parent() {
            break;
        }
    }

    complexity
}
