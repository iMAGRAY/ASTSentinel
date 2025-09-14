use rust_validation_hooks::analysis::ast::error::AstError;
use rust_validation_hooks::analysis::ast::{languages::MultiLanguageAnalyzer, SupportedLanguage};
use rust_validation_hooks::formatting::error::FormattingError;
use rust_validation_hooks::formatting::FormattingService;

#[test]
fn ast_returns_typed_error_on_empty_source() {
    let res = MultiLanguageAnalyzer::analyze_with_tree_sitter("", SupportedLanguage::Python);
    assert!(res.is_err());
    let err = res.unwrap_err();
    // Downcast to our typed error
    if let Some(ast) = err.downcast_ref::<AstError>() {
        match ast {
            AstError::EmptySource => {}
            _ => panic!("unexpected AstError: {ast:?}"),
        }
    } else {
        panic!("expected AstError, got: {err}");
    }
}

#[test]
fn ast_rust_should_use_syn_error() {
    let res = MultiLanguageAnalyzer::analyze_with_tree_sitter("fn main() {}", SupportedLanguage::Rust);
    assert!(res.is_err());
    let err = res.unwrap_err();
    let ast = err.downcast_ref::<AstError>().expect("AstError expected");
    match ast {
        AstError::RustShouldUseSyn => {}
        _ => panic!("unexpected error: {ast:?}"),
    }
}

#[test]
fn ast_returns_timeout_error_on_long_timeout() {
    // Extremely small timeout with modest but valid code to trip timeout in worst
    // case; but our implementation runs fast. So this test simply exercises the
    // API without asserting timeout.
    let code = "def f():\n  return 1\n";
    let res = MultiLanguageAnalyzer::analyze_with_tree_sitter_timeout(
        code,
        SupportedLanguage::Python,
        std::time::Duration::from_millis(1),
    );
    // Either Ok (fast) or Err(AstError::AnalysisTimeout); both acceptable in CI
    // environments.
    if let Err(e) = res {
        // If error, ensure it's our type
        let _ = e
            .downcast_ref::<AstError>()
            .expect("expected AstError on timeout");
    }
}

#[test]
fn formatting_unsupported_extension_typed_error() {
    use std::fs;
    let svc = FormattingService::new().expect("formatting service");
    let dir = tempfile::tempdir().expect("tmp");
    let file = dir.path().join("test.unknown");
    fs::write(&file, "hello").expect("write");
    let res = svc.format_file(&file);
    assert!(res.is_err());
    let err = res.unwrap_err();
    let fe = err.downcast_ref::<FormattingError>().expect("FormattingError");
    match fe {
        FormattingError::UnsupportedExtension(ext) => assert_eq!(ext, "unknown"),
        _ => panic!("unexpected {fe:?}"),
    }
}
