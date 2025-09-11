use rust_validation_hooks::analysis::ast::languages::SupportedLanguage;
use rust_validation_hooks::analysis::ast::quality_scorer::{AstQualityScorer, IssueCategory};

fn analyze(src: &str) -> Vec<(IssueCategory, String)> {
    let scorer = AstQualityScorer::new();
    let q = scorer.analyze(src, SupportedLanguage::Rust).expect("analyze");
    q.concrete_issues
        .into_iter()
        .map(|i| (i.category, i.message))
        .collect()
}

// unreachable after return — covered by existing engine; focus here on loop-specific cases

#[test]
fn rust_unreachable_after_break_continue_in_loop() {
    let src = r#"
fn f() {
    loop { break; let a = 1; }
    while let Some(x) = Some(1) { continue; let y = x + 1; }
}
"#;
    let issues = analyze(src);
    let mut found = 0;
    for (c, _) in &issues {
        if matches!(c, IssueCategory::UnreachableCode) {
            found += 1;
        }
    }
    assert!(
        found >= 2,
        "expected at least two unreachable issues, got {:?}",
        issues
    );
}

#[test]
fn rust_deep_nesting_with_while_let_and_loop() {
    let src = r#"
fn f(mut n: i32) {
    if n > 0 {
        while let Some(x) = Some(n) {
            if x > 0 {
                loop {
                    if x > 1 {
                        if x > 2 { return; }
                    }
                }
            }
        }
    }
}
"#;
    let issues = analyze(src);
    assert!(issues
        .iter()
        .any(|(c, _)| matches!(c, IssueCategory::DeepNesting)));
}
