#[cfg(test)]
mod tests {
    use rust_validation_hooks::validation::diff_formatter::*;

    #[test]
    fn test_full_file_context() {
        let original = "line 1\nline 2\nline 3";
        let modified = "line 1\nline 2 modified\nline 3\nline 4";

        let result = format_full_file_with_changes("test.js", Some(original), Some(modified));

        println!("Full file diff output:");
        println!("{}", result);

        assert!(result.contains("=== Full file: test.js ==="));
        assert!(result.contains("   1   line 1"));
        assert!(result.contains("   2 - line 2"));
        assert!(result.contains("   2 + line 2 modified"));
        assert!(result.contains("   3   line 3"));
        assert!(result.contains("   4 + line 4"));
    }

    #[test]
    fn test_edit_full_context() {
        let file_content = "function hello() {\n    console.log(\"Hello\");\n    return true;\n}";
        let old_string = "console.log(\"Hello\");";
        let new_string = "console.log(\"Hello, World!\");";

        let result =
            format_edit_full_context("test.js", Some(file_content), old_string, new_string);

        println!("Edit full context output:");
        println!("{}", result);

        assert!(result.contains("=== Full file with Edit changes"));
        assert!(result.contains("console.log(\"Hello, World!\")"));
    }
}
