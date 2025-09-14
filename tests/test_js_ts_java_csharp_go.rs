#![cfg(feature = "ast_fastpath")] // These tests rely on single-pass engine parity
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;
use rust_validation_hooks::analysis::ast::{AstQualityScorer, IssueSeverity, SupportedLanguage};

#[test]
fn js_unreachable_after_return() {
    let code = "function f(){ return 1; var x = 2; }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::JavaScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in JavaScript after return"
    );
}

#[test]
fn ts_unreachable_after_return() {
    let code = "function f(){ return 1; const x = 2; }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in TypeScript after return"
    );
}

#[test]
fn ts_unreachable_in_try_catch() {
    let code = r#"function f(){ try { let x=1; return x; const y=2; } catch(e) { return 0; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode after return inside try (TS)"
    );
}

#[test]
fn js_unreachable_in_catch_block() {
    let code = r#"function f(){ try { throw 1; } catch(e) { return 0; var z = 1; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::JavaScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode after return inside catch (JS)"
    );
}

#[test]
fn ts_async_try_catch_switch_good_code() {
    // Async + try/catch + switch should parse and remain under thresholds
    let code = r#"async function handle(x: number): Promise<string> {
  try {
    switch (x) {
      case 1:
        return "one";
      case 2:
        return "two";
      default:
        return "other";
    }
  } catch (e) {
    return "error";
  }
}
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for TS async/try-catch/switch good code"
    );
}

#[test]
fn ts_too_many_parameters_typed() {
    // 6 typed parameters (>5) should trigger TooManyParameters in TS
    let code =
        "function f(a: number,b: number,c: number,d: number,e: number,f: number): number { return 1; }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::TooManyParameters)),
        "expected TooManyParameters in TypeScript (6 > 5)"
    );
}

#[test]
fn ts_arrow_destructured_and_rest_params_too_many() {
    // Arrow function with multiple params (6+): two destructured + identifiers +
    // rest
    let code = r#"const f = ({a,b},{c,d},{e,f}, g, h, ...rest) => { return 1 }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::TooManyParameters)),
        "expected TooManyParameters for arrow with destructured + rest params"
    );
}

#[test]
fn ts_too_many_parameters_complex_signature() {
    // Optional and typed parameters; 6 total should trigger TooManyParameters
    let code = "function f(a?: number,b?: number,c?: number,d?: number,e?: number,f?: number){ return 1 }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::TooManyParameters)),
        "expected TooManyParameters in TS complex signature (6 > 5)"
    );
}

#[test]
fn java_unreachable_after_return() {
    let code = r#"class X { int f(){ return 1; int x = 2; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Java).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in Java after return"
    );
}

#[test]
fn csharp_hardcoded_credentials_and_sql() {
    // Assignment context for credentials + a SQL-looking string
    let code = r#"class X { void f(){ var password = "secret"; var q = "SELECT * FROM T WHERE id=1"; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::CSharp).unwrap();
    // Hardcoded creds should be Critical
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::HardcodedCredentials)
                && matches!(i.severity, IssueSeverity::Critical)),
        "expected Critical HardcodedCredentials in C# assignment"
    );
    // Possible SQL in string literal as a Major warning
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::SqlInjection)),
        "expected SqlInjection warning in C# string literal"
    );
}

#[test]
fn csharp_async_try_catch_switch_good_code() {
    let code = r#"using System; using System.Threading.Tasks;
class X { 
  public async Task<int> F(int x){
    try {
      switch(x){
        case 1: return 1;
        case 2: return 2;
        default: return 3;
      }
    } catch (Exception) {
      return -1;
    }
  }
}
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::CSharp).unwrap();
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for C# async/try-catch/switch good code"
    );
}

#[test]
fn csharp_too_many_parameters_complex_signature() {
    let code = r#"class X { 
  public async System.Threading.Tasks.Task<int> G<T>(int a, int b, int c, int d, int e, int f){ return a+b+c+d+e+f; }
}
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::CSharp).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::TooManyParameters)),
        "expected TooManyParameters in C# complex signature (6 > 5)"
    );
}

#[test]
fn go_hardcoded_credentials_assignment() {
    let code = r#"package main
func f(){ password := "p@ss"; query := "SELECT * FROM users WHERE id=1" }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Go).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::HardcodedCredentials)
                && matches!(i.severity, IssueSeverity::Critical))
            || res
                .concrete_issues
                .iter()
                .any(|i| matches!(i.category, IssueCategory::SqlInjection)),
        "expected HardcodedCredentials (Critical) or SqlInjection warning in Go"
    );
}

#[test]
fn go_switch_good_code() {
    // Good switch with returns inside cases — should not trigger unreachable on
    // case boundaries
    let code = r#"package main
func pick(x int) string {
  switch x {
    case 1: return "one"
    case 2: return "two"
    default: return "other"
  }
}
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Go).unwrap();
    if !res.concrete_issues.is_empty() {
        eprintln!(
            "Go switch produced issues: {:?}",
            res.concrete_issues
                .iter()
                .map(|i| (format!("{:?}", i.category), i.message.clone(), i.line, i.column))
                .collect::<Vec<_>>()
        );
    }
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for Go switch good code"
    );
}

#[test]
fn go_deep_nesting_with_switch_bad_code() {
    let code = r#"package main
func bad(x int) int {
  if x > 0 {
    if x > 1 {
      if x > 2 {
        if x > 3 {
          switch x { case 4: return 4; default: return 0 }
        }
      }
    }
  }
  return 1
}
"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Go).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::DeepNesting)),
        "expected DeepNesting in Go with nested if/switch"
    );
}

#[test]
fn js_good_code_has_no_issues() {
    let code = "function sum(a,b){ if(a>0 && b>0){ return a+b; } return 0 }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::JavaScript).unwrap();
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for simple JS good code"
    );
}

#[test]
fn java_good_code_has_no_issues() {
    let code = r#"class X { int sum(int a, int b){ if(a>0 && b>0){ return a+b; } return 0; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Java).unwrap();
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for simple Java good code"
    );
}

#[test]
fn ts_good_code_has_no_issues() {
    let code = "function sum(a: number,b: number): number { if(a>0 && b>0){ return a+b; } return 0; }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for simple TS good code"
    );
}

#[test]
fn ts_async_try_finally_await_good_code() {
    // Proper async/await with try/finally should not create false positives
    let code = r#"async function f(x:number){
  try {
    await g();
    if (x) return 1;
    return 0;
  } finally {
    await h();
  }
}"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for async try/finally with await"
    );
}

#[test]
fn ts_class_method_too_many_parameters() {
    let code = r#"class X { m(a:number,b:number,c:number,d:number,e:number,f:number){ return a+b; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::TooManyParameters)),
        "expected TooManyParameters for TS class method with 6 params"
    );
}

#[test]
fn ts_switch_fallthrough_good_code() {
    let code = r#"function f(x:number){
  switch(x){
    case 1:
    case 2:
      return 2;
    default:
      return 0;
  }
}"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for fallthrough switch TS"
    );
}

#[test]
fn ts_unreachable_in_try_block_bad_code() {
    let code = r#"function f(){ try { return 1; const y = 2; } catch(e){ return 0; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in TS for code after return inside try block"
    );
}

#[test]
fn csharp_good_code_has_no_issues() {
    let code = r#"class X { int Sum(int a, int b){ if(a>0 && b>0){ return a+b; } return 0; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::CSharp).unwrap();
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for simple C# good code"
    );
}

#[test]
fn csharp_unreachable_in_try_block_bad_code() {
    let code = r#"class X { int F(){ try { return 1; var y = 2; } catch(System.Exception) { return 0; } } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::CSharp).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in C# for code after return inside try block"
    );
}

#[test]
fn go_good_code_has_no_issues() {
    let code = r#"package main
func sum(a int, b int) int { if a>0 && b>0 { return a+b } ; return 0 }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Go).unwrap();
    assert!(
        res.concrete_issues.is_empty(),
        "expected no issues for simple Go good code"
    );
}

#[test]
fn go_too_many_parameters() {
    let code = r#"package main
func f(a int, b int, c int, d int, e int, f int) int { return 1 }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Go).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::TooManyParameters)),
        "expected TooManyParameters in Go (6 > 5)"
    );
}

#[test]
fn ts_deep_nesting() {
    let code = "function f(){ if(1){ if(1){ if(1){ if(1){ if(1){ return 1; }}}}} }";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::TypeScript).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::DeepNesting)),
        "expected DeepNesting in TS"
    );
}

#[test]
fn csharp_deep_nesting() {
    let code = r#"class X { int f(){ if(true){ if(true){ if(true){ if(true){ if(true){ return 1; }}}}} return 0; } }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::CSharp).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::DeepNesting)),
        "expected DeepNesting in C#"
    );
}

#[test]
fn go_deep_nesting() {
    let code = r#"package main
func f() int { if true { if true { if true { if true { if true { return 1 }}}}} ; return 0 }"#;
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Go).unwrap();
    assert!(
        res.concrete_issues
            .iter()
            .any(|i| matches!(i.category, IssueCategory::DeepNesting)),
        "expected DeepNesting in Go"
    );
}
