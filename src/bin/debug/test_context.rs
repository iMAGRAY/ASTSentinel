use rust_validation_hooks::analysis::project::{scan_project_structure, format_project_structure_for_ai};

fn main() {
    println!("Testing project context scanning...");
    
    match scan_project_structure(".", None) {
        Ok(structure) => {
            println!("Successfully scanned project:");
            println!("  - Total files: {}", structure.total_files);
            println!("  - Total directories: {}", structure.directories.len());
            
            let formatted = format_project_structure_for_ai(&structure, 1000);
            println!("\nFormatted context for AI:");
            println!("{}", formatted);
        }
        Err(e) => {
            eprintln!("Error scanning project: {}", e);
        }
    }
}
