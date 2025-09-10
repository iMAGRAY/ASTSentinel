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
fn cpp_switch_good_code_has_no_issues() {
    let code = r#"int pick(int x){
  switch(x){
    case 1: return 1;
    case 2: return 2;
    default: return 0;
  }
}
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Cpp).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for simple C++ switch good code");
}

#[test]
fn cpp_try_catch_good_code_has_no_issues() {
    // Tree-sitter C++ may not count try/catch towards complexity; ensure it parses cleanly
    let code = r#"#include <stdexcept>
int f(int x){
  try {
    if (x == 0) throw std::runtime_error("err");
    return x;
  } catch (const std::exception&){
    return -1;
  }
}
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Cpp).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for C++ try/catch good code");
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
