use rust_validation_hooks::analysis::project::{scan_project_structure, format_project_structure_for_ai};

fn main() {
    println!("=== PROJECT CONTEXT AS SEEN BY AI ===\n");
    
    match scan_project_structure(".", None) {
        Ok(structure) => {
            // Show full formatted context (2000 chars limit as in posttooluse)
            let formatted = format_project_structure_for_ai(&structure, 2000);
            
            println!("This is exactly what gets sent to AI model:");
            println!("--------------------------------------------");
            println!("{}", formatted);
            println!("--------------------------------------------");
            println!("\nContext size: {} characters", formatted.len());
            
            // Show detailed breakdown
            println!("\n=== DETAILED BREAKDOWN ===");
            println!("Total files: {}", structure.total_files);
            println!("Total directories: {}", structure.directories.len());
            
            println!("\nDirectory structure:");
            for (i, dir) in structure.directories.iter().enumerate() {
                if i < 20 {
                    println!("  - {}", dir);
                } else if i == 20 {
                    println!("  ... and {} more directories", structure.directories.len() - 20);
                    break;
                }
            }
            
            println!("\nKey files found:");
            for (i, file) in structure.files.iter().enumerate() {
                let filename = file.relative_path.to_lowercase();
                if filename.ends_with("cargo.toml") || 
                   filename.ends_with("package.json") ||
                   filename.ends_with("readme.md") ||
                   filename.ends_with(".env") ||
                   filename.ends_with("claude.md") {
                    println!("  - {}", file.relative_path);
                }
                if i > 100 { break; }
            }
        }
        Err(e) => {
            eprintln!("Error scanning project: {}", e);
        }
    }
}
