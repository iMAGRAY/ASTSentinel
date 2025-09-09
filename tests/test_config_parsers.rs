use rust_validation_hooks::analysis::ast::{AstQualityScorer, IssueSeverity, SupportedLanguage};
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;

#[test]
fn test_json_parse_error_reports_minor_issue() {
    let scorer = AstQualityScorer::new();
    let src = "{ invalid_json: true"; // malformed JSON
    let score = scorer.analyze(src, SupportedLanguage::Json).expect("analyze ok");
    assert!(
        score
            .concrete_issues
            .iter()
            .any(|i| matches!(i.severity, IssueSeverity::Minor) && i.rule_id == "CFG001"),
        "Expected Minor parse issue CFG001 in JSON"
    );
}

#[test]
fn test_yaml_hardcoded_credentials_detected() {
    let scorer = AstQualityScorer::new();
    let src = "password: secret123"; // overlay should flag creds
    let score = scorer.analyze(src, SupportedLanguage::Yaml).expect("analyze ok");
    assert!(
        score
            .concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::HardcodedCredentials) && matches!(i.severity, IssueSeverity::Critical)),
        "Expected Critical HardcodedCredentials in YAML"
    );
}

#[test]
fn test_toml_valid_no_issues() {
    let scorer = AstQualityScorer::new();
    let src = "[package]\nname = 'demo'\nversion = '0.1.0'";
    let score = scorer.analyze(src, SupportedLanguage::Toml).expect("analyze ok");
    assert!(
        score.concrete_issues.is_empty(),
        "Expected no issues for valid TOML without creds"
    );
}
