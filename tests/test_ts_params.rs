use rust_validation_hooks::analysis::ast::languages::{LanguageCache, SupportedLanguage};
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;
use rust_validation_hooks::analysis::ast::single_pass::SinglePassEngine;

#[test]
fn ts_this_parameter_is_ignored_in_count() {
    let code = r#"function f(this: HTMLElement, a: number, b?: string, ...rest: any[]){ return 1 }"#;
    let mut parser = LanguageCache::create_parser_with_language(SupportedLanguage::TypeScript).unwrap();
    let tree = parser.parse(code, None).unwrap();
    let issues = SinglePassEngine::analyze(&tree, code, SupportedLanguage::TypeScript);
    // Only 2+rest => total 3 params; below threshold (<=5) => no TooManyParameters
    assert!(issues
        .iter()
        .all(|i| !matches!(i.category, IssueCategory::TooManyParameters)));
}

#[test]
fn ts_complex_params_counted_properly() {
    let code = r#"function g({x,y}:{x:number,y:number}, [a,b]:[number,number], opt?: string, def: number = 1, ...rest: any[]){ return a }"#;
    let mut parser = LanguageCache::create_parser_with_language(SupportedLanguage::TypeScript).unwrap();
    let tree = parser.parse(code, None).unwrap();
    let issues = SinglePassEngine::analyze(&tree, code, SupportedLanguage::TypeScript);
    // 5 parameters → threshold is 5, so TooManyParameters should NOT trigger
    assert!(issues
        .iter()
        .all(|i| !matches!(i.category, IssueCategory::TooManyParameters)));
}

#[test]
fn ts_too_many_params_still_flags_over_threshold() {
    let code = r#"function h(a:number,b:number,c:number,d:number,e:number,f:number){ return a }"#;
    let mut parser = LanguageCache::create_parser_with_language(SupportedLanguage::TypeScript).unwrap();
    let tree = parser.parse(code, None).unwrap();
    let issues = SinglePassEngine::analyze(&tree, code, SupportedLanguage::TypeScript);
    assert!(issues
        .iter()
        .any(|i| matches!(i.category, IssueCategory::TooManyParameters)));
}
