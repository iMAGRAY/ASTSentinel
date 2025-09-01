use rust_validation_hooks::project_context::{
    scan_project_structure,
    format_project_structure_for_ai,
    ScanConfig,
};

fn main() {
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
            
            // Show what the old format looked like for comparison
            println!("\n{} OLD FORMAT EXAMPLE {}", "=".repeat(20), "=".repeat(20));
            println!("PROJECT STRUCTURE (C:\\Users\\1\\Documents\\GitHub\\ValidationCodeHook)");
            println!("Scanned: 2025-09-01 04:07:39 | Files: 11 | Directories: 9\n");
            println!("DIRECTORIES:");
            println!("  docs/");
            println!("  prompts/");
            println!("  src/");
            println!("  tests/");
            println!("  ... (truncated)\n");
            println!("CODE FILES BY TYPE:");
            println!("  .rs: 4 files");
            println!("  .md: 3 files");
            println!("  .toml: 1 files\n");
            println!("IMPORTANT FILES:");
            println!("  Cargo.toml");
            
            // Compare sizes
            println!("\n{} SIZE COMPARISON {}", "=".repeat(20), "=".repeat(20));
            let old_format_size = 350; // Approximate old format size
            let new_format_size = formatted.len();
            let savings = old_format_size - new_format_size;
            let percentage = (savings as f64 / old_format_size as f64) * 100.0;
            
            println!("Old format: ~{} chars", old_format_size);
            println!("New format: {} chars", new_format_size);
            println!("Savings: {} chars ({:.1}% reduction)", savings, percentage);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}