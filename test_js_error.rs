use rust_validation_hooks::analysis::ast::{MultiLanguageAnalyzer, SupportedLanguage};

fn main() {
    let invalid_js = "function broken {";
    
    match MultiLanguageAnalyzer::analyze_with_tree_sitter(invalid_js, SupportedLanguage::JavaScript) {
        Ok(metrics) => {
            println!("Success! Metrics: {:?}", metrics);
            println!("This should NOT happen for invalid JS!");
        },
        Err(e) => {
            println!("Error (expected): {}", e);
        }
    }
}