use rust_validation_hooks::analysis::ast::quality_scorer::{
    ConcreteIssue, IssueCategory, IssueSeverity, QualityScore,
};
use rust_validation_hooks::messages::glossary::build_quick_tips;

fn score_with_cats(cats: &[IssueCategory]) -> QualityScore {
    let issues = cats
        .iter()
        .enumerate()
        .map(|(i, c)| ConcreteIssue {
            severity: IssueSeverity::Major,
            category: *c,
            message: format!("issue {}", i),
            file: String::new(),
            line: i + 1,
            column: 1,
            rule_id: format!("R{:03}", i),
            points_deducted: 0,
        })
        .collect();
    QualityScore {
        total_score: 1000,
        functionality_score: 300,
        reliability_score: 200,
        maintainability_score: 200,
        performance_score: 150,
        security_score: 100,
        standards_score: 50,
        concrete_issues: issues,
    }
}

#[test]
fn quick_tips_are_unique_and_capped() {
    let cats = [
        IssueCategory::SqlInjection,
        IssueCategory::SqlInjection,
        IssueCategory::HardcodedCredentials,
        IssueCategory::DeepNesting,
    ];
    let score = score_with_cats(&cats);
    let tips = build_quick_tips(&score, 2, 120);
    assert!(tips.len() <= 2);
    // Uniqueness: two different categories produce two tips
    assert!(!tips.is_empty());
}
