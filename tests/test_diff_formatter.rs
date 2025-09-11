#[cfg(test)]
mod tests {
    use rust_validation_hooks::validation::diff_formatter::*;

    #[test]
    fn test_full_file_with_empty_content() {
        // Test with empty files
        let result = format_full_file_with_changes("test.js", Some(""), Some(""));
        assert!(result.contains("=== Full file: test.js ==="));

        // Test None cases
        let result = format_full_file_with_changes("test.js", None, None);
        assert!(result.contains("(No content)"));

        // Test new empty file
        let result = format_full_file_with_changes("test.js", None, Some(""));
        assert!(result.contains("(New empty file)"));

        // Test deleted empty file
        let result = format_full_file_with_changes("test.js", Some(""), None);
        assert!(result.contains("(Empty file deleted)"));
    }

    #[test]
    fn test_full_file_with_simple_changes() {
        let original = "line 1\nline 2\nline 3";
        let modified = "line 1\nline 2 modified\nline 3\nline 4";

        let result = format_full_file_with_changes("test.js", Some(original), Some(modified));

        // Check structure
        assert!(result.contains("=== Full file: test.js ==="));
        assert!(result.contains("=== End of test.js ==="));

        // Check specific lines
        assert!(result.contains("   1   line 1")); // Unchanged
        assert!(result.contains("   2 - line 2")); // Removed
        assert!(result.contains("   2 + line 2 modified")); // Added
        assert!(result.contains("   3   line 3")); // Unchanged
        assert!(result.contains("   4 + line 4")); // New line
    }

    #[test]
    fn test_edit_full_context_basic() {
        let file_content = "function hello() {\n    console.log(\"Hello\");\n    return true;\n}";
        let old_string = "console.log(\"Hello\");";
        let new_string = "console.log(\"Hello, World!\");";

        let result = format_edit_full_context("test.js", Some(file_content), old_string, new_string);

        assert!(result.contains("=== Full file with Edit changes: test.js ==="));
        assert!(result.contains("function hello()"));
        assert!(result.contains("console.log(\"Hello, World!\")"));
        assert!(result.contains("return true"));
    }

    #[test]
    fn test_edit_full_context_not_found() {
        let file_content = "function test() { return 42; }";
        let old_string = "nonexistent";
        let new_string = "replacement";

        let result = format_edit_full_context("test.js", Some(file_content), old_string, new_string);

        assert!(result.contains("Edit target not found"));
        assert!(result.contains("function test()"));
    }

    #[test]
    fn test_large_file_truncation() {
        // Create a large content string (> 100KB)
        let large_content: String = "x".repeat(150_000);
        let small_content = "small";

        let result = format_full_file_with_changes("large.js", Some(&large_content), Some(small_content));

        // Should contain truncation warning
        assert!(result.contains("File truncated for display"));
    }

    #[test]
    fn test_utf8_content() {
        let original = "Hello 世界\n🚀 Rust\n😀 Test";
        let modified = "Hello World\n🚀 Rust\n😀 Test Modified";

        let result = format_full_file_with_changes("utf8.txt", Some(original), Some(modified));

        assert!(result.contains("Hello 世界"));
        assert!(result.contains("Hello World"));
        assert!(result.contains("🚀 Rust"));
        assert!(result.contains("😀 Test Modified"));
    }

    #[test]
    fn test_multiline_edit() {
        let original = "line 1\nline 2\nline 3\nline 4\nline 5";
        let modified = "line 1\nmodified 2\nmodified 3\nline 4\nline 5";

        let result = format_full_file_with_changes("multi.txt", Some(original), Some(modified));

        // Check that multiple consecutive changes are shown
        assert!(result.contains("   2 - line 2"));
        assert!(result.contains("   2 + modified 2"));
        assert!(result.contains("   3 - line 3"));
        assert!(result.contains("   3 + modified 3"));
        assert!(result.contains("   4   line 4")); // Unchanged
    }

    #[test]
    fn test_file_deletion() {
        let original = "content to delete\nline 2\nline 3";

        let result = format_full_file_with_changes("deleted.js", Some(original), None);

        assert!(result.contains("(File deleted)"));
        assert!(result.contains("   1 - content to delete"));
        assert!(result.contains("   2 - line 2"));
        assert!(result.contains("   3 - line 3"));
    }

    #[test]
    fn test_new_file_creation() {
        let new_content = "new file\nwith content\nlines";

        let result = format_full_file_with_changes("new.js", None, Some(new_content));

        assert!(result.contains("(New file)"));
        assert!(result.contains("   1 + new file"));
        assert!(result.contains("   2 + with content"));
        assert!(result.contains("   3 + lines"));
    }
}
