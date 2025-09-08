use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"TEST WORKING"}}"#;
    
    // Method 1: Try println (will go to stderr in MSYS)
    println!("{}", json);
    
    // Method 2: Write to file for verification
    let mut file = File::create("direct_output.json")?;
    file.write_all(json.as_bytes())?;
    file.flush()?;
    
    Ok(())
}