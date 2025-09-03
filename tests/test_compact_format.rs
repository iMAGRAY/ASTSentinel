use rust_validation_hooks::analysis::project::{
    scan_project_structure,
    format_project_structure_for_ai,
    ScanConfig,
};

#[test]
fn test_compact_format() {
    println!("Testing ultra-compact project structure format\n");
    
    // Test with current directory
    let config = ScanConfig {
        max_files: 800,
        max_depth: 5, 
        include_hidden_files: false,
        follow_symlinks: false,
    };
    
    match scan_project_structure(".", Some(config)) {
        Ok(structure) => {
            println!("Project stats:");
            println!("  Total files: {}", structure.total_files);
            println!("  Total directories: {}", structure.directories.len());
            
            // Test different format sizes
            let sizes = [100, 250, 500, 1000, 2000];
            
            for size in sizes {
                println!("\n{} Format (max {} chars) {}", "=".repeat(20), size, "=".repeat(20));
                let formatted = format_project_structure_for_ai(&structure, size);
                println!("{}", formatted);
                println!("Actual size: {} chars", formatted.len());
            }
            
            // Verify format is compact - test with larger size to include STATS
            let formatted = format_project_structure_for_ai(&structure, 2000);
            assert!(formatted.contains("PROJECT:"), "Should contain PROJECT: section");
            assert!(formatted.contains("STATS:"), "Should contain STATS: section in full format");
            assert!(!formatted.contains("PROJECT STRUCTURE"), "Should not contain verbose title");
            assert!(!formatted.contains("Scanned:"), "Should not contain verbose scan info");
            
            // Test small format only has PROJECT
            let small_formatted = format_project_structure_for_ai(&structure, 500);
            assert!(small_formatted.contains("PROJECT:"), "Small format should have PROJECT:");
            assert!(small_formatted.len() <= 500, "Should be truncated to max size");
            
            println!("\nTest passed! Format is ultra-compact.");
        }
        Err(e) => {
            panic!("Error: {}", e);
        }
    }
}