use rust_validation_hooks::analysis::ast::{AstQualityScorer, SupportedLanguage};
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;

#[test]
fn rust_unreachable_after_return() {
    let code = r#"fn f()->i32{ return 1; let x = 2; }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in Rust after return");
}

#[test]
fn rust_too_many_params_and_deep_nesting() {
    let code = r#"fn g(a:i32,b:i32,c:i32,d:i32,e:i32,f:i32){ if true { if true { if true { if true { if true { }}}}} }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::TooManyParameters)),
        "expected TooManyParameters");
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::DeepNesting)),
        "expected DeepNesting");
}

#[test]
fn rust_unhandled_error_unwrap_and_panic() {
    let code = r#"fn h(){ let x = Some(1).unwrap(); panic!("boom"); }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnhandledError)),
        "expected UnhandledError (unwrap/panic)");
}

#[test]
fn rust_unhandled_error_macro_calls() {
    let code = r#"fn m(){ todo!("later"); unimplemented!(); panic!("boom"); }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnhandledError)),
        "expected UnhandledError for panic!/todo!/unimplemented! macro");
}

#[test]
fn rust_match_with_guards_good_code() {
    let code = r#"fn f(x:i32)->i32{ match x { n if n>0 => 1, _ => 0 } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for match with guards");
}

#[test]
fn rust_long_method_over_50_statements() {
    let mut body = String::from("fn h(){\n");
    for _ in 0..51 { body.push_str("let _x = 1;\n"); }
    body.push_str("}\n");
    let s = AstQualityScorer::new();
    let res = s.analyze(&body, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::LongMethod)),
        "expected LongMethod for >50 statements");
}

#[test]
fn rust_hardcoded_creds_and_sql() {
    let code = r#"fn k(){ let password = "secret"; let q = "SELECT * FROM T WHERE id=1"; }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::HardcodedCredentials)),
        "expected HardcodedCredentials");
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::SqlInjection)),
        "expected SqlInjection");
}

#[test]
fn rust_good_code_ok() {
    let code = r#"fn sum(a:i32,b:i32)->i32{ if a>0 && b>0 { a+b } else { 0 } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues");
}

#[test]
fn rust_complex_signature_too_many_params() {
    let code = r#"use std::fmt::Display;
fn fancy<'a, T: Display, const N: usize>(a:i32,b:i32,c:i32,d:i32,e:i32,f:i32)->i32 { a+b+c+d+e+f }
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::TooManyParameters)),
        "expected TooManyParameters for complex Rust generics/lifetimes/const signature (6 > 5)");
}

#[test]
fn rust_async_try_match_good_code() {
    let code = r#"async fn handle(x:i32)->i32 { match x { 1 => 1, 2 => 2, _ => 0 } }
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Rust).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for async + match good Rust code");
}
