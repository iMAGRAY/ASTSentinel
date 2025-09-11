use thiserror::Error;

#[derive(Debug, Error)]
pub enum AstError {
    #[error("source code cannot be empty")]
    EmptySource,

    #[error("source code too large ({0} bytes), potential DoS risk")]
    SourceTooLarge(usize),

    #[error("rust analysis should use syn crate, not Tree-sitter")]
    RustShouldUseSyn,

    #[error("failed to parse {0} source code - syntax may be invalid")]
    ParseFailed(String),

    #[error("source code contains syntax errors that prevent analysis")]
    SyntaxError,

    #[error("analysis timeout: {language} code analysis exceeded {timeout_secs}s timeout")]
    AnalysisTimeout { language: String, timeout_secs: u64 },

    #[error("analysis thread failed unexpectedly for {0} code")]
    AnalysisThreadFailed(String),

    #[error("unsupported language for this operation: {0}")]
    UnsupportedLanguage(&'static str),
}
