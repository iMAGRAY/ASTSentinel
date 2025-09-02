/// Validation modules for code and file checks

pub mod test_files;
pub mod diff_formatter;

// Re-export validation functions and types
pub use test_files::{
    TestFileConfig,
    TestFileValidation,
    validate_test_file,
    detect_test_content,
};