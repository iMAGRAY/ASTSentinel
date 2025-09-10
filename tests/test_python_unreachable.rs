use rust_validation_hooks::analysis::ast::languages::{LanguageCache, SupportedLanguage};
use rust_validation_hooks::analysis::ast::single_pass::SinglePassEngine;
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;

#[test]
fn py_unreachable_after_raise_in_try_block() {
    let code = r#"def f():
    try:
        raise ValueError("x")
        x = 1
    except Exception as e:
        y = 2
    return 0
"#;
    let mut parser = LanguageCache::create_parser_with_language(SupportedLanguage::Python).unwrap();
    let tree = parser.parse(code, None).unwrap();
    let issues = SinglePassEngine::analyze(&tree, code, SupportedLanguage::Python);
    assert!(issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)));
}

#[test]
fn py_unreachable_after_break_and_continue() {
    let code = r#"def f():
    while True:
        break
        x = 1
    for i in [1]:
        continue
        y = 2
    return 0
"#;
    let mut parser = LanguageCache::create_parser_with_language(SupportedLanguage::Python).unwrap();
    let tree = parser.parse(code, None).unwrap();
    let issues = SinglePassEngine::analyze(&tree, code, SupportedLanguage::Python);
    let cnt = issues.iter().filter(|i| matches!(i.category, IssueCategory::UnreachableCode)).count();
    assert!(cnt >= 2, "expected >=2 unreachable, got {} (issues: {:?})", cnt, issues);
}

