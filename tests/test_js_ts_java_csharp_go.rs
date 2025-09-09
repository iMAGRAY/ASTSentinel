#![cfg(feature = "ast_fastpath")] // These tests rely on single-pass engine parity
use rust_validation_hooks::analysis::ast::{AstQualityScorer, SupportedLanguage, IssueSeverity};
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;

#[test]
fn js_unreachable_after_return() {
    let code = "function f(){ return 1; var x = 2; }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::JavaScript).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in JavaScript after return");
}

#[test]
fn ts_unreachable_after_return() {
    let code = "function f(){ return 1; const x = 2; }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in TypeScript after return");
}

#[test]
fn java_unreachable_after_return() {
    let code = r#"class X { int f(){ return 1; int x = 2; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Java).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in Java after return");
}

#[test]
fn csharp_hardcoded_credentials_and_sql() {
    // Assignment context for credentials + a SQL-looking string
    let code = r#"class X { void f(){ var password = "secret"; var q = "SELECT * FROM T WHERE id=1"; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::CSharp).unwrap();
    // Hardcoded creds should be Critical
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::HardcodedCredentials) && matches!(i.severity, IssueSeverity::Critical)),
        "expected Critical HardcodedCredentials in C# assignment");
    // Possible SQL in string literal as a Major warning
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::SqlInjection)),
        "expected SqlInjection warning in C# string literal");
}

#[test]
fn go_hardcoded_credentials_assignment() {
    let code = r#"package main
func f(){ password := "p@ss"; query := "SELECT * FROM users WHERE id=1" }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Go).unwrap();
    assert!(
        res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::HardcodedCredentials) && matches!(i.severity, IssueSeverity::Critical))
        || res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::SqlInjection)),
        "expected HardcodedCredentials (Critical) or SqlInjection warning in Go"
    );
}

#[test]
fn js_good_code_has_no_issues() {
    let code = "function sum(a,b){ if(a>0 && b>0){ return a+b; } return 0 }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::JavaScript).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for simple JS good code");
}

#[test]
fn java_good_code_has_no_issues() {
    let code = r#"class X { int sum(int a, int b){ if(a>0 && b>0){ return a+b; } return 0; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Java).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for simple Java good code");
}

#[test]
fn ts_good_code_has_no_issues() {
    let code = "function sum(a: number,b: number): number { if(a>0 && b>0){ return a+b; } return 0; }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for simple TS good code");
}

#[test]
fn csharp_good_code_has_no_issues() {
    let code = r#"class X { int Sum(int a, int b){ if(a>0 && b>0){ return a+b; } return 0; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::CSharp).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for simple C# good code");
}

#[test]
fn go_good_code_has_no_issues() {
    let code = r#"package main
func sum(a int, b int) int { if a>0 && b>0 { return a+b } ; return 0 }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Go).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for simple Go good code");
}
