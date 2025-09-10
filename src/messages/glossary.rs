use crate::analysis::ast::quality_scorer::{ConcreteIssue, IssueCategory, QualityScore};
use std::collections::HashSet;

// Return a concise, action-oriented tip for an issue category (<= 120 chars per G1)
pub fn tip_for_category(cat: &IssueCategory) -> &'static str {
    match cat {
        IssueCategory::HardcodedCredentials => "Never hardcode secrets; use env vars or a secret manager.",
        IssueCategory::SqlInjection => "Use parameterized queries; avoid string-concatenated SQL.",
        IssueCategory::CommandInjection => "Avoid shell concatenation; prefer exec APIs with arg arrays.",
        IssueCategory::PathTraversal => "Join + normalize paths; validate against allowed roots.",
        IssueCategory::UnhandledError => "Handle Result/Exception; replace unwrap/panic with proper errors.",
        IssueCategory::UnreachableCode => "Remove code after early-returns/raise/break; simplify control flow.",
        IssueCategory::TooManyParameters => "Reduce params (>5); group into struct/object to simplify calls.",
        IssueCategory::DeepNesting => "Flatten nesting (>4): early return/guard clauses + extract helpers.",
        IssueCategory::HighComplexity => "Split large functions; extract pure helpers; simplify branches.",
        IssueCategory::LongMethod => "Shorten method (>50 stmts); extract cohesive blocks into helpers.",
        IssueCategory::NamingConvention => "Use consistent, descriptive names; follow project naming rules.",
        IssueCategory::MissingDocumentation => "Document public APIs briefly (what, params, returns).",
        IssueCategory::UnusedImports => "Remove unused imports; keep build/lint fast and clean.",
        IssueCategory::UnusedVariables => "Remove or underscore unused variables for clarity.",
        IssueCategory::LongLine => "Wrap lines >120 chars; extract expressions; split format strings.",
        _ => "Refactor for clarity; keep functions short and single-purpose.",
    }
}

// Aggregate unique tips (by category) for a score, respecting caps and length
pub fn build_quick_tips(score: &QualityScore, max_tips: usize, max_line_chars: usize) -> Vec<String> {
    let mut seen: HashSet<&'static str> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    // Deterministic: order by severity -> line -> rule_id already in callers; here keep stable iteration
    for i in &score.concrete_issues {
        let tip = tip_for_category(&i.category);
        if seen.insert(tip) {
            let clipped = if tip.chars().count() > max_line_chars {
                let mut s = tip.chars().take(max_line_chars.saturating_sub(1)).collect::<String>();
                s.push('…'); s
            } else { tip.to_string() };
            out.push(clipped);
            if out.len() >= max_tips { break; }
        }
    }
    out
}

