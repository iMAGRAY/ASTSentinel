#[cfg(test)]
mod integration_tests {
    use rust_validation_hooks::validation::diff_formatter::*;

    #[test]
    fn test_actual_full_context_output() {
        let original = "function test() {\n    console.log(\"line 1\");\n    console.log(\"line 2\");\n    return true;\n}";
        let modified = "function test() {\n    console.log(\"line 1\");\n    console.log(\"MODIFIED LINE\");\n    return true;\n}";

        let result = format_full_file_with_changes("test.js", Some(original), Some(modified));

        // Print actual output for verification
        println!("\n=== ACTUAL OUTPUT ===");
        println!("{}", result);
        println!("=== END OUTPUT ===\n");

        // Verify it contains full file
        assert!(
            result.contains("function test()"),
            "Should contain function declaration"
        );
        assert!(
            result.contains("console.log(\"line 1\")"),
            "Should contain unchanged line"
        );
        assert!(
            result.contains("-     console.log(\"line 2\")"),
            "Should show removed line"
        );
        assert!(
            result.contains("+     console.log(\"MODIFIED LINE\")"),
            "Should show added line"
        );
        assert!(
            result.contains("return true"),
            "Should contain return statement"
        );
    }

    #[test]
    fn test_edit_full_context_actual() {
        let file_content = "function hello() {\n    console.log(\"Hello\");\n    console.log(\"World\");\n    return true;\n}";
        let old_string = "console.log(\"Hello\");";
        let new_string = "console.log(\"Hello, Modified!\");";

        let result =
            format_edit_full_context("test.js", Some(file_content), old_string, new_string);

        println!("\n=== EDIT FULL CONTEXT OUTPUT ===");
        println!("{}", result);
        println!("=== END ===\n");

        // Check that full file is shown
        assert!(result.contains("function hello()"));
        assert!(result.contains("console.log(\"World\")"));
        assert!(result.contains("return true"));
        // Check that change is visible
        assert!(result.contains("Hello, Modified!") || result.contains("+ "));
    }

    #[test]
    fn test_large_file_truncation() {
        // Test file larger than 100KB
        let large_content = "x".repeat(150_000);
        let small = "small";

        let result = format_full_file_with_changes("large.txt", Some(&large_content), Some(small));

        println!("\n=== TRUNCATION TEST ===");
        println!("Content length: {}", large_content.len());
        println!(
            "Result contains truncation warning: {}",
            result.contains("truncated")
        );

        assert!(result.contains("truncated") || result.contains("⚠️"));
    }
}
