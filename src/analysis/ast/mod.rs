/// AST analysis modules for multi-language support
pub mod languages;
pub mod visitor;
pub mod quality_scorer;

// Re-export main types for convenience
pub use languages::{MultiLanguageAnalyzer, SupportedLanguage};
pub use visitor::ComplexityVisitor;
pub use quality_scorer::{AstQualityScorer, QualityScore, ConcreteIssue, IssueSeverity};
