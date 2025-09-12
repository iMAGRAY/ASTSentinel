use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormattingError {
    #[error("formatter not implemented: {0}")]
    NotImplemented(&'static str),

    #[error("unsupported file extension: {0}")]
    UnsupportedExtension(String),

    #[error("no formatter available for language: {0}")]
    FormatterUnavailable(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
