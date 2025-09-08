/// Language-specific formatter implementations
/// 
/// This module provides secure, sandboxed access to external formatting tools
/// with comprehensive security measures and resource limits.

pub mod rust;
pub mod python;  
pub mod javascript;
pub mod typescript;
pub mod java;
pub mod csharp;
pub mod go;
pub mod c;
pub mod cpp;
pub mod php;
pub mod ruby;

use std::process::{Command, Stdio};
use std::time::Duration;
use std::collections::HashSet;
use anyhow::Result;
use std::io::Write;

/// Security configuration for command execution
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum execution time in seconds
    pub max_execution_time: u64,
    /// Maximum memory usage in bytes (if supported by OS)
    pub max_memory_bytes: u64,
    /// Maximum input size in bytes (separate from output limit)
    pub max_input_bytes: usize,
    /// Maximum output size in bytes
    pub max_output_bytes: usize,
    /// Allowed commands whitelist
    pub allowed_commands: HashSet<String>,
    /// Working directory restrictions
    pub allowed_working_dirs: HashSet<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let mut allowed_commands = HashSet::new();
        // Whitelist only known, safe formatting commands
        allowed_commands.insert("rustfmt".to_string());
        allowed_commands.insert("black".to_string());
        allowed_commands.insert("prettier".to_string());
        allowed_commands.insert("gofmt".to_string());
        allowed_commands.insert("clang-format".to_string());
        allowed_commands.insert("dotnet".to_string());
        allowed_commands.insert("java".to_string());
        allowed_commands.insert("php-cs-fixer".to_string());
        allowed_commands.insert("rubocop".to_string());
        
        Self {
            max_execution_time: 30, // 30 seconds max
            max_memory_bytes: 100 * 1024 * 1024, // 100MB max
            max_input_bytes: 5 * 1024 * 1024, // 5MB max input
            max_output_bytes: 10 * 1024 * 1024, // 10MB max output
            allowed_commands,
            allowed_working_dirs: HashSet::new(), // Will be set dynamically
        }
    }
}

/// Secure command executor with sandboxing and resource limits
pub struct SecureCommandExecutor {
    config: SecurityConfig,
}

impl SecureCommandExecutor {
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    /// Check if a command exists on the system (safe check only)
    pub fn command_exists(&self, command: &str) -> bool {
        // Validate command against whitelist first
        if !self.config.allowed_commands.contains(command) {
            return false;
        }

        // Use safe method to check command existence
        self.safe_which_check(command)
    }

    /// Safe implementation of 'which' command check
    fn safe_which_check(&self, command: &str) -> bool {
        let which_cmd = if cfg!(windows) { "where" } else { "which" };
        
        let output = Command::new(which_cmd)
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        match output {
            Ok(result) => result.status.success(),
            Err(_) => false,
        }
    }

    /// Execute a formatter command with full security measures
    pub fn execute_formatter(&self, command: &str, args: &[String], input: Option<&str>) -> Result<String> {
        // Security validation
        self.validate_command_security(command, args)?;

        // Create secure command
        let cmd = self.create_secure_command(command, args)?;

        // Execute with timeout and resource limits
        self.execute_with_security_limits(cmd, input)
    }

    fn validate_command_security(&self, command: &str, args: &[String]) -> Result<()> {
        // Check command whitelist
        if !self.config.allowed_commands.contains(command) {
            anyhow::bail!("Command '{}' not in allowed whitelist", command);
        }

        // Validate arguments for injection attempts
        for arg in args {
            if self.contains_dangerous_patterns(arg) {
                anyhow::bail!("Argument contains potentially dangerous patterns: {}", arg);
            }
        }

        Ok(())
    }

    fn contains_dangerous_patterns(&self, input: &str) -> bool {
        let dangerous_patterns = [
            ";", "&&", "||", "|", "`", "$", 
            "$(", "${", "../", "~/", "/etc/",
            "&", ">", "<", ">>", "<<",
            "\n", "\r", "\0"
        ];

        for pattern in &dangerous_patterns {
            if input.contains(pattern) {
                return true;
            }
        }

        // Check for suspicious executables
        let suspicious_exts = [".exe", ".bat", ".cmd", ".sh", ".ps1"];
        for ext in &suspicious_exts {
            if input.to_lowercase().ends_with(ext) {
                return true;
            }
        }

        false
    }

    fn create_secure_command(&self, command: &str, args: &[String]) -> Result<Command> {
        let mut cmd = Command::new(command);
        
        // Add validated arguments
        for arg in args {
            cmd.arg(arg);
        }

        // Secure the process environment
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Set resource limits if on Unix-like system
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            
            unsafe {
                cmd.pre_exec(|| {
                    // Set CPU time limit (SIGXCPU after soft limit)
                    libc::setrlimit(libc::RLIMIT_CPU, &libc::rlimit {
                        rlim_cur: 30, // 30 seconds
                        rlim_max: 60, // 60 seconds hard limit
                    });
                    
                    // Set memory limit (if available)
                    #[cfg(target_os = "linux")]
                    {
                        libc::setrlimit(libc::RLIMIT_AS, &libc::rlimit {
                            rlim_cur: 100 * 1024 * 1024, // 100MB
                            rlim_max: 200 * 1024 * 1024, // 200MB hard limit
                        });
                    }
                    
                    Ok(())
                });
            }
        }

        Ok(cmd)
    }

    fn execute_with_security_limits(&self, mut cmd: Command, input: Option<&str>) -> Result<String> {
        use std::sync::mpsc;
        use std::thread;
        use std::time::Instant;

        let start_time = Instant::now();
        let timeout = Duration::from_secs(self.config.max_execution_time);

        // Spawn the process
        let mut child = cmd.spawn()?;

        // Handle input if provided
        if let Some(stdin_data) = input {
            if let Some(mut stdin) = child.stdin.take() {
                // Validate input size
                if stdin_data.len() > self.config.max_input_bytes {
                    let _ = child.kill();
                    anyhow::bail!("Input too large: {} bytes (max: {})", 
                                stdin_data.len(), self.config.max_input_bytes);
                }

                // Write input in a separate thread to avoid blocking
                let data = stdin_data.to_string();
                thread::spawn(move || {
                    let _ = stdin.write_all(data.as_bytes());
                    let _ = stdin.flush();
                });
            }
        }

        // Wait for completion with timeout
        let (tx, rx) = mpsc::channel();
        let child_id = child.id();
        
        thread::spawn(move || {
            let result = child.wait_with_output();
            let _ = tx.send(result);
        });

        // Wait with timeout
        match rx.recv_timeout(timeout) {
            Ok(Ok(output)) => {
                let elapsed = start_time.elapsed();
                
                // Check execution time
                if elapsed > timeout {
                    anyhow::bail!("Command execution timeout: {:?}", elapsed);
                }

                // Check output size
                if output.stdout.len() > self.config.max_output_bytes {
                    anyhow::bail!("Output too large: {} bytes (max: {})", 
                                output.stdout.len(), self.config.max_output_bytes);
                }

                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .map_err(|e| anyhow::anyhow!("Output encoding error: {}", e))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("Formatter failed: {}", stderr);
                }
            },
            Ok(Err(e)) => {
                anyhow::bail!("Process execution error: {}", e)
            },
            Err(_) => {
                // Timeout - try to kill the process
                #[cfg(unix)]
                {
                    unsafe {
                        libc::kill(child_id as i32, libc::SIGTERM);
                        thread::sleep(Duration::from_millis(100));
                        libc::kill(child_id as i32, libc::SIGKILL);
                    }
                }
                
                #[cfg(windows)]
                {
                    let _ = Command::new("taskkill")
                        .args(&["/PID", &child_id.to_string(), "/F"])
                        .output();
                }
                
                anyhow::bail!("Command timeout after {:?}", timeout)
            }
        }
    }

    /// Get version info for a formatter (safe, limited execution)
    pub fn get_formatter_version(&self, command: &str) -> String {
        if !self.command_exists(command) {
            return format!("{} (not available)", command);
        }

        let version_args = self.get_version_args(command);
        
        match self.execute_formatter(command, &version_args, None) {
            Ok(output) => {
                // Clean and truncate output
                let clean_output: String = output
                    .lines()
                    .next() // Take only first line
                    .unwrap_or("unknown version")
                    .trim()
                    .chars()
                    .take(200) // Limit length
                    .collect();
                
                format!("{} {}", command, clean_output)
            },
            Err(_) => format!("{} (version check failed)", command)
        }
    }

    fn get_version_args(&self, command: &str) -> Vec<String> {
        match command {
            "rustfmt" => vec!["--version".to_string()],
            "black" => vec!["--version".to_string()],
            "prettier" => vec!["--version".to_string()],
            "gofmt" => vec![], // gofmt doesn't have --version
            "clang-format" => vec!["--version".to_string()],
            "java" => vec!["--version".to_string()],
            "dotnet" => vec!["--version".to_string()],
            "php-cs-fixer" => vec!["--version".to_string()],
            "rubocop" => vec!["--version".to_string()],
            _ => vec!["--version".to_string()],
        }
    }
}

impl Default for SecureCommandExecutor {
    fn default() -> Self {
        Self::new(SecurityConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.allowed_commands.contains("rustfmt"));
        assert!(config.allowed_commands.contains("black"));
        assert_eq!(config.max_execution_time, 30);
        assert!(config.max_output_bytes > 0);
    }

    #[test]
    fn test_dangerous_patterns_detection() {
        let executor = SecureCommandExecutor::default();
        
        assert!(executor.contains_dangerous_patterns("file.txt; rm -rf /"));
        assert!(executor.contains_dangerous_patterns("$(malicious)"));
        assert!(executor.contains_dangerous_patterns("file && rm file"));
        assert!(executor.contains_dangerous_patterns("../../../etc/passwd"));
        assert!(executor.contains_dangerous_patterns("file.exe"));
        
        assert!(!executor.contains_dangerous_patterns("normal_file.rs"));
        assert!(!executor.contains_dangerous_patterns("--indent=4"));
        assert!(!executor.contains_dangerous_patterns("output.json"));
    }

    #[test]
    fn test_command_whitelist_validation() {
        let executor = SecureCommandExecutor::default();
        
        assert!(executor.execute_formatter("rm", &["-rf".to_string(), "/".to_string()], None).is_err());
        assert!(executor.execute_formatter("curl", &["http://evil.com".to_string()], None).is_err());
        assert!(executor.execute_formatter("bash", &["-c".to_string(), "echo hi".to_string()], None).is_err());
    }

    #[test]
    fn test_argument_injection_protection() {
        let executor = SecureCommandExecutor::default();
        
        let dangerous_args = vec![
            "file.txt; rm -rf /".to_string(),
            "--option=$(malicious)".to_string(),
            "file && rm file".to_string(),
        ];
        
        assert!(executor.execute_formatter("rustfmt", &dangerous_args, None).is_err());
    }

    #[test]
    fn test_safe_which_check() {
        let executor = SecureCommandExecutor::default();
        
        // Test that the check doesn't throw errors or panic
        let _exists = executor.safe_which_check("rustfmt");
        let _not_exists = executor.safe_which_check("definitely-not-a-real-command-12345");
    }
}