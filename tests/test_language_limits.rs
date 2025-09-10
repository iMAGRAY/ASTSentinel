use rust_validation_hooks::analysis::ast::{MultiLanguageAnalyzer, SupportedLanguage};

#[test]
fn analyze_with_tree_sitter_empty_source_errors() {
    let res = MultiLanguageAnalyzer::analyze_with_tree_sitter("", SupportedLanguage::Python);
    assert!(res.is_err(), "Empty source should error");
}

#[test]
fn analyze_with_tree_sitter_huge_source_errors_fast() {
    // >10MB input should be rejected without heavy work
    let huge = "a".repeat(10_000_001);
    let res = MultiLanguageAnalyzer::analyze_with_tree_sitter(&huge, SupportedLanguage::JavaScript);
    assert!(res.is_err(), "Huge input should error early");
}

