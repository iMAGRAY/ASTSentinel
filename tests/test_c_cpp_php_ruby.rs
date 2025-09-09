#![cfg(feature = "ast_fastpath")] // These assertions rely on fastpath single-pass engine parity
// NOTE: Multi-pass legacy path does not guarantee identical rule coverage for these languages.
use rust_validation_hooks::analysis::ast::{AstQualityScorer, IssueSeverity, SupportedLanguage};
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;

#[test]
fn c_unreachable_after_return() {
    let code = r#"int f(){ return 0; int x = 1; }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::C).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in C after return");
}

#[test]
fn cpp_unreachable_after_return() {
    let code = r#"int f(){ return 0; auto x = 1; }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Cpp).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in C++ after return");
}

#[test]
fn php_unreachable_after_return_and_creds() {
    let code = r#"<?php
function foo(){
  $password = "s3cr3t";
  return 1;
  $x = 2; // unreachable
}
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Php).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in PHP after return");
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::HardcodedCredentials) && matches!(i.severity, IssueSeverity::Critical)),
        "expected Critical HardcodedCredentials in PHP");
}

#[test]
fn ruby_unreachable_after_return() {
    let code = r#"def f
  return 1
  x = 2
end
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Ruby).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in Ruby after return");
}
