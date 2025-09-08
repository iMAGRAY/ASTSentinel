use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test simple stdout write
    eprintln!("This goes to stderr");
    
    // Try different methods to write to stdout
    println!("Method 1: println!");
    
    let json = r#"{"test": "method2"}"#;
    std::io::stdout().write_all(json.as_bytes())?;
    std::io::stdout().flush()?;
    
    eprintln!("This also goes to stderr");
    
    Ok(())
}