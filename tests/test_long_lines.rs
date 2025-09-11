#![cfg(not(feature = "ast_fastpath"))]
// Multi-pass path: LongLineRule should trigger for over-120-char lines
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;
use rust_validation_hooks::analysis::ast::{AstQualityScorer, SupportedLanguage};

#[test]
fn long_line_detected_in_python() {
    let scorer = AstQualityScorer::new();
    let long = "x".repeat(130);
    let src = format!("{}\nprint('ok')\n", long);
    let score = scorer.analyze(&src, SupportedLanguage::Python).unwrap();
    assert!(score
        .concrete_issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::LongLine)));
}
