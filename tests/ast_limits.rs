use rust_validation_hooks::analysis::ast::error::AstError;
use rust_validation_hooks::analysis::ast::{languages::MultiLanguageAnalyzer, SupportedLanguage};

#[test]
fn ast_source_too_large_is_rejected_fast() {
    // Construct >10MB string without allocating too many intermediate buffers
    let chunk = "a".repeat(1024 * 1024); // 1MB
    let mut code = String::with_capacity(10_500_000);
    for _ in 0..10 {
        code.push_str(&chunk);
    }
    code.push_str("extra"); // >10MB
    let res = MultiLanguageAnalyzer::analyze_with_tree_sitter(&code, SupportedLanguage::Python);
    assert!(res.is_err());
    let err = res.unwrap_err();
    let ast = err.downcast_ref::<AstError>().expect("AstError expected");
    match ast {
        AstError::SourceTooLarge(sz) => assert!(*sz > 10_000_000),
        _ => panic!("unexpected error: {ast:?}"),
    }
}
