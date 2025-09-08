use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("This goes to stderr");
    
    // Try Windows-specific stdout using raw handle
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use std::os::windows::io::FromRawHandle;
        
        unsafe {
            let stdout_handle = std::io::stdout().as_raw_handle();
            let mut file = std::fs::File::from_raw_handle(stdout_handle);
            file.write_all(b"Windows specific stdout\n")?;
            file.flush()?;
        }
    }
    
    // Also try using libc directly
    #[cfg(unix)]
    {
        use std::ffi::CString;
        let msg = CString::new("Unix specific stdout\n").unwrap();
        unsafe {
            libc::write(libc::STDOUT_FILENO, msg.as_ptr() as *const _, msg.as_bytes().len());
        }
    }
    
    Ok(())
}