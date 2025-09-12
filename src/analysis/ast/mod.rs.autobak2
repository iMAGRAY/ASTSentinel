pub mod error;
pub mod kind_ids;
/// AST analysis modules for multi-language support
pub mod languages;
pub mod quality_scorer;
#[cfg(feature = "ast_fastpath")]
pub mod single_pass;
pub mod visitor;

// Re-export main types for convenience
pub use languages::{MultiLanguageAnalyzer, SupportedLanguage};
pub use quality_scorer::{AstQualityScorer, ConcreteIssue, IssueSeverity, QualityScore};
pub use visitor::ComplexityVisitor;
