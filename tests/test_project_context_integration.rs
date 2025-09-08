use rust_validation_hooks::analysis::project::{
    format_project_structure_for_ai, scan_project_structure, ScanConfig,
};

#[test]
fn test_real_project_scanning() {
    println!("\n=== Testing project_context module on real project ===\n");

    // Get current directory
    let current_dir = std::env::current_dir().expect("Failed to get current directory");

    let current_dir_str = current_dir
        .to_str()
        .expect("Failed to convert path to string");

    println!("Scanning directory: {}\n", current_dir_str);

    // Create custom config for testing
    let config = ScanConfig {
        max_files: 100, // Limit for testing
        max_depth: 5,   // Don't go too deep
        include_hidden_files: false,
        follow_symlinks: false,
    };

    // Scan the project
    let structure =
        scan_project_structure(current_dir_str, Some(config)).expect("Failed to scan project");

    // Basic assertions
    assert!(
        !structure.root_path.is_empty(),
        "Root path should not be empty"
    );
    assert!(structure.total_files > 0, "Should find at least some files");
    assert!(
        !structure.scan_timestamp.is_empty(),
        "Timestamp should be set"
    );

    // Print results for manual verification
    println!("===== SCAN RESULTS =====");
    println!("Root path: {}", structure.root_path);
    println!("Total files found: {}", structure.total_files);
    println!("Total directories: {}", structure.directories.len());
    println!("Scan timestamp: {}", structure.scan_timestamp);

    // Show first 10 files
    println!("\n===== FIRST 10 FILES =====");
    for (i, file) in structure.files.iter().take(10).enumerate() {
        println!(
            "{}. {} ({}, {} bytes, code: {})",
            i + 1,
            file.relative_path,
            file.file_type,
            file.size_bytes,
            file.is_code_file
        );
    }

    // Test that target directory is ignored (it should be)
    let has_target_files = structure
        .files
        .iter()
        .any(|f| f.relative_path.starts_with("target/"));
    assert!(!has_target_files, "Target directory should be ignored");

    // Test that .git directory is ignored
    let has_git_files = structure
        .files
        .iter()
        .any(|f| f.relative_path.starts_with(".git/"));
    assert!(!has_git_files, ".git directory should be ignored");

    // Test AI formatting
    println!("\n===== AI CONTEXT FORMAT (max 1000 chars) =====");
    let ai_context = format_project_structure_for_ai(&structure, 1000);
    assert!(!ai_context.is_empty(), "AI context should not be empty");
    assert!(
        ai_context.len() <= 1000,
        "AI context should respect max length"
    );
    println!("{}", ai_context);

    // Check that we found some Rust files (since this is a Rust project)
    let rust_files: Vec<_> = structure
        .files
        .iter()
        .filter(|f| f.file_type == "rs")
        .collect();
    assert!(
        !rust_files.is_empty(),
        "Should find Rust files in a Rust project"
    );

    println!("\n✅ All assertions passed!");
}

#[test]
fn test_gitignore_respected() {
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // Create temporary directory with .gitignore
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create .gitignore
    let mut gitignore = fs::File::create(root.join(".gitignore")).unwrap();
    writeln!(gitignore, "secret.txt").unwrap();
    writeln!(gitignore, "*.log").unwrap();
    writeln!(gitignore, "ignored_dir/").unwrap();

    // Create files
    fs::write(root.join("included.rs"), "// code").unwrap();
    fs::write(root.join("secret.txt"), "secret data").unwrap();
    fs::write(root.join("debug.log"), "log data").unwrap();
    fs::create_dir(root.join("ignored_dir")).unwrap();
    fs::write(root.join("ignored_dir").join("file.txt"), "ignored").unwrap();

    // Scan
    let structure = scan_project_structure(root.to_str().unwrap(), None).expect("Failed to scan");

    // Verify only included.rs is found
    assert_eq!(structure.total_files, 1, "Should only find 1 file");
    assert_eq!(structure.files[0].relative_path, "included.rs");

    // Verify ignored files are not included
    let file_names: Vec<_> = structure.files.iter().map(|f| &f.relative_path).collect();

    assert!(!file_names.contains(&&"secret.txt".to_string()));
    assert!(!file_names.contains(&&"debug.log".to_string()));
    assert!(!file_names.iter().any(|f| f.starts_with("ignored_dir/")));

    println!("✅ .gitignore is properly respected");
}
