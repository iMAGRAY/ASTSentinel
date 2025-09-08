use anyhow::{Context, Result};
use chrono;
use serde::Deserialize;
use serde_json;
use std::io::{self, Read};
use tokio;
use uuid;
// Removed unused import: quick_xml::se::to_string
use lazy_static::lazy_static;
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::num::NonZeroUsize;
use std::sync::Mutex;

// Use universal AI client for multi-provider support
use rust_validation_hooks::providers::ai::UniversalAIClient;
use rust_validation_hooks::validation_constants::*;
use rust_validation_hooks::{sanitize_zero_width_chars, truncate_utf8_safe, Config};

/// Configuration constants for memory organizer
mod config {
    use lazy_static::lazy_static;
    use std::env;

    /// Average characters per token for GPT-5-nano based on OpenAI documentation (2025)
    /// Used for estimating token counts to stay within API limits
    pub const AVG_CHARS_PER_TOKEN: f64 = 3.7;

    /// Average tokens per word for English text in GPT models
    /// Used as secondary metric for token estimation accuracy
    pub const AVG_TOKENS_PER_WORD: f64 = 1.3;

    /// Default maximum stdin input size in bytes (50KB)
    /// Prevents DoS attacks while allowing reasonable hook input sizes
    pub const DEFAULT_MAX_STDIN_SIZE: usize = 51_200;

    /// Minimum allowed stdin size (1KB) for validation
    /// Used to ensure input is not too small to be useful
    pub const MIN_STDIN_SIZE: usize = 1_024;

    /// Maximum allowed stdin size (1MB) for validation
    /// Upper bound to prevent excessive memory usage
    pub const MAX_STDIN_SIZE: usize = 1_048_576;

    lazy_static! {
        /// Cached maximum stdin size parsed from environment variable
        /// Avoids repeated environment variable lookups for performance
        static ref CACHED_MAX_STDIN_SIZE: usize = parse_max_stdin_size();
    }

    /// Parse maximum stdin size from environment variable with validation
    /// Returns validated size or DEFAULT_MAX_STDIN_SIZE if invalid/unset
    /// Logs warnings for invalid values
    ///
    /// Note: Trims whitespace from environment variable for better usability
    fn parse_max_stdin_size() -> usize {
        match env::var("MEMORY_MAX_STDIN_SIZE") {
            Ok(val) => {
                let trimmed = val.trim(); // Trim whitespace for better user experience

                // Handle empty/whitespace-only values explicitly
                if trimmed.is_empty() {
                    log_warning(&format!(
                        "MEMORY_MAX_STDIN_SIZE is empty or whitespace, using default {}",
                        DEFAULT_MAX_STDIN_SIZE
                    ));
                    return DEFAULT_MAX_STDIN_SIZE;
                }

                match trimmed.parse::<usize>() {
                    Ok(size) => validate_stdin_size(size),
                    Err(_) => {
                        log_warning(&format!(
                            "Invalid MEMORY_MAX_STDIN_SIZE='{}', using default {}",
                            trimmed, DEFAULT_MAX_STDIN_SIZE
                        ));
                        DEFAULT_MAX_STDIN_SIZE
                    }
                }
            }
            Err(_) => DEFAULT_MAX_STDIN_SIZE,
        }
    }

    /// Validate parsed stdin size against allowed bounds
    /// Separated for better testability and maintainability
    fn validate_stdin_size(size: usize) -> usize {
        if size >= MIN_STDIN_SIZE && size <= MAX_STDIN_SIZE {
            size
        } else {
            log_warning(&format!(
                "MEMORY_MAX_STDIN_SIZE={} is outside valid range ({}-{}), using default {}",
                size, MIN_STDIN_SIZE, MAX_STDIN_SIZE, DEFAULT_MAX_STDIN_SIZE
            ));
            DEFAULT_MAX_STDIN_SIZE
        }
    }

    /// Centralized logging function for warnings
    /// Makes it easier to control log output and levels
    fn log_warning(message: &str) {
        if std::env::var("DEBUG_HOOKS").unwrap_or_default() == "true" {
            eprintln!("Warning: {}", message);
        }
    }

    /// Get maximum stdin size from cached environment variable parsing
    ///
    /// # Returns
    /// Validated stdin size in bytes, guaranteed to be within safe limits
    pub fn max_stdin_size() -> usize {
        *CACHED_MAX_STDIN_SIZE
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::env;

        /// Test helper to isolate environment variable changes
        struct EnvGuard {
            key: &'static str,
            original: Option<String>,
        }

        impl EnvGuard {
            fn new(key: &'static str) -> Self {
                let original = env::var(key).ok();
                Self { key, original }
            }

            fn set(&self, value: &str) {
                env::set_var(self.key, value);
            }
        }

        impl Drop for EnvGuard {
            fn drop(&mut self) {
                if let Some(ref original) = self.original {
                    env::set_var(self.key, original);
                } else {
                    env::remove_var(self.key);
                }
            }
        }

        #[test]
        fn test_max_stdin_size_valid() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("2048");
            assert_eq!(parse_max_stdin_size(), 2048);
        }

        #[test]
        fn test_max_stdin_size_too_small() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("512");
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_too_large() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("2000000");
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_invalid() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("invalid");
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_empty_string() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("");
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_zero() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("0");
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_negative() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("-1024");
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_whitespace() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("  2048  ");
            // Test validates that whitespace trimming works correctly:
            // Input "  2048  " should trim to "2048" and parse successfully
            // This ensures user-friendly behavior when whitespace is accidentally included
            assert_eq!(parse_max_stdin_size(), 2048);
        }

        #[test]
        fn test_max_stdin_size_whitespace_only() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("   "); // Only whitespace
                               // Empty string after trim would fail parsing, fallback to default
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_very_large_number() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("99999999999999999999999999");
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_boundary_min() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set(&MIN_STDIN_SIZE.to_string());
            assert_eq!(parse_max_stdin_size(), MIN_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_boundary_max() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set(&MAX_STDIN_SIZE.to_string());
            assert_eq!(parse_max_stdin_size(), MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_unset() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            env::remove_var("MEMORY_MAX_STDIN_SIZE");
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_non_ascii() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("2048€"); // Non-ASCII character
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_unicode_digits() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("２０４８"); // Unicode full-width digits
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_max_stdin_size_mixed_characters() {
            let _guard = EnvGuard::new("MEMORY_MAX_STDIN_SIZE");
            _guard.set("20abc48"); // Mixed ASCII letters and numbers
            assert_eq!(parse_max_stdin_size(), DEFAULT_MAX_STDIN_SIZE);
        }

        #[test]
        fn test_env_guard_isolation() {
            // Test that EnvGuard properly isolates changes
            const TEST_VAR: &str = "MEMORY_MAX_STDIN_SIZE";

            // Set initial value
            env::set_var(TEST_VAR, "initial");

            {
                let _guard1 = EnvGuard::new(TEST_VAR);
                _guard1.set("value1");
                assert_eq!(env::var(TEST_VAR).unwrap(), "value1");

                {
                    let _guard2 = EnvGuard::new(TEST_VAR);
                    _guard2.set("value2");
                    assert_eq!(env::var(TEST_VAR).unwrap(), "value2");
                } // guard2 drops, should restore to value1

                assert_eq!(env::var(TEST_VAR).unwrap(), "value1");
            } // guard1 drops, should restore to initial

            assert_eq!(env::var(TEST_VAR).unwrap(), "initial");

            // Clean up
            env::remove_var(TEST_VAR);
        }
    }
}

/// Memory organization hook for conversation stop event
/// Simplified input to handle any Stop event format from Claude Code
#[derive(Debug, Deserialize)]
struct StopEventInput {
    // Make all fields optional to handle various Stop event formats
    #[serde(default)]
    cwd: Option<String>,
}

// For Stop events, Claude Code expects an empty JSON object
// We'll log messages to stderr instead of returning them in JSON

// All legacy structures have been removed - using direct JSON to XML conversion

// Removed dead code: transform_path_to_project_name function

/// Determine project directory based on input
fn determine_project_dir(input: &StopEventInput) -> Result<String> {
    // Use the project's actual working directory instead of creating a separate structure
    if let Some(cwd) = &input.cwd {
        // Validate the path
        if cwd.is_empty() {
            anyhow::bail!("Empty cwd path");
        }

        // Basic path validation to prevent path traversal attacks
        if cwd.contains("..") {
            anyhow::bail!("Invalid cwd path contains '..' components: {}", cwd);
        }

        // Validate path characters to prevent injection
        if cwd
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            anyhow::bail!("Invalid cwd path contains control characters");
        }

        // Return the project's working directory directly
        return Ok(cwd.clone());
    }

    anyhow::bail!("Cannot determine project directory from cwd")
}

/// Construct the path to CLAUDE.md in the project directory with validation
fn construct_project_claude_md_path(hook_input: &StopEventInput) -> Result<String> {
    if let Some(cwd) = &hook_input.cwd {
        // Validate the path
        if cwd.is_empty() {
            anyhow::bail!("Empty cwd path");
        }

        // Check for path traversal attacks - CRITICAL SECURITY CHECK
        if cwd.contains("..") {
            anyhow::bail!("Invalid path contains '..' components: {}", cwd);
        }

        // Limit path length to prevent DoS attacks (use Unicode char count)
        if cwd.chars().count() > MAX_PATH_LENGTH {
            anyhow::bail!(
                "Path too long: {} chars > {} limit",
                cwd.chars().count(),
                MAX_PATH_LENGTH
            );
        }

        // Validate path characters to prevent injection
        if cwd
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            anyhow::bail!("Invalid cwd path contains control characters");
        }

        // Early check for absolute path requirement
        use std::path::Path;
        if !Path::new(cwd).is_absolute() {
            anyhow::bail!("Path must be absolute: {}", cwd);
        }

        // Use project path for canonicalization
        let project_path = Path::new(cwd);

        // Canonicalize path to resolve any symlinks and normalize it
        let canonical_path = std::fs::canonicalize(project_path)
            .with_context(|| format!("Failed to canonicalize path: {}", cwd))?;

        // Double-check that it's still absolute after canonicalization
        if !canonical_path.is_absolute() {
            anyhow::bail!("Canonicalized path is not absolute: {:?}", canonical_path);
        }

        // Ensure it's a directory
        if !canonical_path.is_dir() {
            anyhow::bail!("Path is not a directory: {:?}", canonical_path);
        }

        // Safely join with CLAUDE.md
        let claude_md_path = canonical_path.join("CLAUDE.md");

        // Convert back to string, handling potential Unicode issues
        claude_md_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid Unicode in path: {:?}", claude_md_path))
            .map(|s| s.to_string())
    } else {
        anyhow::bail!("Cannot construct CLAUDE.md path: missing cwd")
    }
}

/// Find the latest transcript file in the project directory
async fn find_latest_transcript(project_dir: &str) -> Result<String> {
    use tokio::fs;

    let mut entries = fs::read_dir(project_dir)
        .await
        .with_context(|| format!("Failed to read project directory: {}", project_dir))?;

    let mut latest_file: Option<(String, std::time::SystemTime)> = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            let metadata = entry.metadata().await?;
            let modified = metadata.modified()?;

            if let Some((_, last_modified)) = &latest_file {
                if modified > *last_modified {
                    latest_file = Some((path.to_string_lossy().to_string(), modified));
                }
            } else {
                latest_file = Some((path.to_string_lossy().to_string(), modified));
            }
        }
    }

    latest_file
        .map(|(path, _)| path)
        .ok_or_else(|| anyhow::anyhow!("No transcript files found in project directory"))
}

/// Read transcript from JSONL file
async fn read_transcript(path: &str) -> Result<String> {
    use tokio::fs;
    use tokio::io::AsyncBufReadExt;

    let file = fs::File::open(path)
        .await
        .with_context(|| format!("Failed to open transcript file: {}", path))?;
    let reader = tokio::io::BufReader::new(file);
    let mut lines = reader.lines();

    let mut transcript = String::new();
    while let Some(line) = lines.next_line().await? {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            // Extract relevant content from Claude Code's conversation JSON format
            if let Some(message) = json.get("message") {
                if let Some(content) = message.get("content") {
                    // Handle both array and string content formats
                    if let Some(content_array) = content.as_array() {
                        for item in content_array {
                            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                transcript.push_str(text);
                                transcript.push_str("\n\n");
                            }
                        }
                    } else if let Some(text) = content.as_str() {
                        transcript.push_str(text);
                        transcript.push_str("\n\n");
                    }
                }
            }
        }
    }

    Ok(transcript)
}

/// Read existing memory file if it exists
async fn read_existing_memory(path: &str) -> Result<String> {
    use tokio::fs;

    match fs::read_to_string(path).await {
        Ok(content) => Ok(content),
        Err(_) => Ok(String::new()), // Return empty string if file doesn't exist
    }
}

/// Load memory optimization prompt from file
async fn load_memory_prompt() -> Result<String> {
    use tokio::fs;

    let prompt_path = "prompts/memory_optimization.txt";
    fs::read_to_string(prompt_path).await.with_context(|| {
        format!(
            "Failed to load memory optimization prompt from {}",
            prompt_path
        )
    })
}

// Use thread-safe global LRU cache for token counting
lazy_static! {
    static ref TOKEN_CACHE: Mutex<LruCache<[u8; 32], usize>> = {
        let cache_size = NonZeroUsize::new(1000).unwrap();
        Mutex::new(LruCache::new(cache_size))
    };
}

/// Hash text to use as cache key (saves memory for large texts)
fn hash_text(text: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Convert JSON response to XML format optimized for Claude Code
fn convert_json_to_xml(json_value: serde_json::Value) -> Result<String> {
    // Build XML structure with descriptive element names for Claude Code
    let xml_content = json_to_claude_xml(json_value)?;

    let xml_header = r#"<?xml version="1.0" encoding="UTF-8"?>
"#;

    Ok(format!("{}{}", xml_header, xml_content))
}

/// Convert JSON structure to Claude Code optimized XML with validation
fn json_to_claude_xml(json_value: serde_json::Value) -> Result<String> {
    // Validate input structure
    if !json_value.is_object() {
        anyhow::bail!(
            "Expected JSON object for memory structure, got {}",
            json_value.to_string()
        );
    }

    let map = json_value.as_object().unwrap();
    let mut xml_parts = Vec::new();

    // Handle ai_dynamic_context specially
    if let Some(context_value) = map.get("ai_dynamic_context") {
        xml_parts.push("<claude-context>".to_string());
        xml_parts.push(format_ai_dynamic_context(context_value)?);

        // Add other top-level elements
        if let Some(project_info) = map.get("project_info") {
            xml_parts.push(format_project_info(project_info)?);
        }
        if let Some(api_config) = map.get("api_configuration") {
            xml_parts.push(format_api_configuration(api_config)?);
        }

        xml_parts.push("</claude-context>".to_string());
    } else {
        // If no ai_dynamic_context, assume this IS the context
        xml_parts.push("<claude-context>".to_string());
        xml_parts.push(format_ai_dynamic_context(&json_value)?);
        xml_parts.push("</claude-context>".to_string());
    }

    Ok(xml_parts.join("\n"))
}

// XML writing configuration constants
mod xml_config {
    pub const DEFAULT_MAX_XML_SIZE: usize = 50 * 1024 * 1024; // 50MB
    pub const MIN_XML_SIZE: usize = 100; // Minimum reasonable XML size
    pub const TEMP_FILE_EXTENSION: &str = "tmp";
}

/// Safely write XML memory content to file with validation, security checks, and atomic writes
async fn write_xml_memory_file(memory_file: &std::path::PathBuf, xml_content: &str) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    // std::env import not needed here - using module level imports

    // Validate XML content size with detailed error handling and logging
    validate_xml_size(xml_content)?;

    // Enhanced security: Validate path with stricter checks
    let canonical_path =
        validate_secure_path(memory_file).with_context(|| "Path security validation failed")?;

    // Validate XML with proper parser before writing
    validate_xml_with_parser(xml_content).with_context(|| "XML content validation failed")?;

    // Ensure parent directory exists
    if let Some(parent_dir) = canonical_path.parent() {
        tokio::fs::create_dir_all(parent_dir)
            .await
            .with_context(|| "Failed to create parent directory for memory file")?;
    }

    // Atomic write using unique temporary file to prevent corruption and conflicts
    let temp_file = create_unique_temp_file(&canonical_path)?;

    // Write to temporary file first
    {
        let file = tokio::fs::File::create(&temp_file)
            .await
            .with_context(|| "Failed to create temporary memory file")?;

        let mut writer = tokio::io::BufWriter::new(file);
        writer
            .write_all(xml_content.as_bytes())
            .await
            .with_context(|| "Failed to write XML content to temporary file")?;

        writer
            .flush()
            .await
            .with_context(|| "Failed to flush XML content to temporary file")?;
    } // File handle is dropped here, ensuring it's closed

    // Atomically move temporary file to final location
    tokio::fs::rename(&temp_file, &canonical_path)
        .await
        .with_context(|| "Failed to atomically move temporary file to final location")?;

    Ok(())
}

/// Parse XML size limit from environment with caching
fn parse_xml_size_limit() -> usize {
    lazy_static! {
        static ref XML_SIZE_LIMIT: usize = {
            use std::env;

            match env::var("MEMORY_MAX_XML_SIZE") {
                Ok(val) => parse_xml_size_value(&val),
                Err(_) => xml_config::DEFAULT_MAX_XML_SIZE,
            }
        };
    }
    *XML_SIZE_LIMIT
}

/// Parse XML size value with comprehensive validation
fn parse_xml_size_value(val: &str) -> usize {
    let trimmed = val.trim();

    // Check for empty or whitespace-only values
    if trimmed.is_empty() {
        eprintln!(
            "Warning: MEMORY_MAX_XML_SIZE is empty, using default {}",
            xml_config::DEFAULT_MAX_XML_SIZE
        );
        return xml_config::DEFAULT_MAX_XML_SIZE;
    }

    // Enhanced parsing with overflow protection
    match trimmed.parse::<i64>() {
        Ok(size) if size < 0 => {
            eprintln!(
                "Warning: MEMORY_MAX_XML_SIZE ({}) is negative, using default {}",
                size,
                xml_config::DEFAULT_MAX_XML_SIZE
            );
            xml_config::DEFAULT_MAX_XML_SIZE
        }
        Ok(size) if size > usize::MAX as i64 => {
            eprintln!("Warning: MEMORY_MAX_XML_SIZE ({}) exceeds usize::MAX on this platform, using default {}", size, xml_config::DEFAULT_MAX_XML_SIZE);
            xml_config::DEFAULT_MAX_XML_SIZE
        }
        Ok(size) => {
            // Safe casting - overflow already checked in match guard
            let size_usize = size as usize;

            if size_usize < xml_config::MIN_XML_SIZE {
                eprintln!(
                    "Warning: MEMORY_MAX_XML_SIZE ({}) is too small, minimum is {}, using default",
                    size_usize,
                    xml_config::MIN_XML_SIZE
                );
                xml_config::DEFAULT_MAX_XML_SIZE
            } else {
                size_usize
            }
        }
        Err(_) => {
            eprintln!("Warning: Invalid MEMORY_MAX_XML_SIZE value: '{}'. Must be a positive integer, using default {}", 
                trimmed, xml_config::DEFAULT_MAX_XML_SIZE);
            xml_config::DEFAULT_MAX_XML_SIZE
        }
    }
}

/// Validate XML content size with detailed error handling and logging
fn validate_xml_size(xml_content: &str) -> Result<()> {
    let max_xml_size = parse_xml_size_limit();

    // Validate XML content size with contextual logging
    let content_size = xml_content.len();

    if content_size > max_xml_size {
        eprintln!(
            "XML size validation failed: {} bytes exceeds limit of {} bytes",
            content_size, max_xml_size
        );
        anyhow::bail!(
            "XML content size ({} bytes) exceeds maximum allowed size ({} bytes)",
            content_size,
            max_xml_size
        );
    }

    if content_size < xml_config::MIN_XML_SIZE {
        eprintln!(
            "XML size validation failed: {} bytes is below minimum of {} bytes",
            content_size,
            xml_config::MIN_XML_SIZE
        );
        anyhow::bail!(
            "XML content size ({} bytes) is too small, minimum is {}",
            content_size,
            xml_config::MIN_XML_SIZE
        );
    }

    Ok(())
}

/// Enhanced path security validation to prevent directory traversal and symlink attacks
fn validate_secure_path(memory_file: &std::path::PathBuf) -> Result<std::path::PathBuf> {
    // Check for dangerous path components
    for component in memory_file.components() {
        match component {
            std::path::Component::ParentDir => {
                anyhow::bail!(
                    "Path contains '..' component, potential directory traversal: {:?}",
                    memory_file
                );
            }
            std::path::Component::CurDir => {
                // '.' is generally safe but we log it
                if std::env::var("DEBUG_HOOKS").unwrap_or_default() == "true" {
                    eprintln!("Warning: Path contains '.' component: {:?}", memory_file);
                }
            }
            _ => {} // Normal components are OK
        }
    }

    // Canonicalize path with enhanced error handling
    let canonical_path = memory_file
        .canonicalize()
        .or_else(|_| {
            // If file doesn't exist, canonicalize parent and join filename
            if let Some(parent) = memory_file.parent() {
                if let Some(filename) = memory_file.file_name() {
                    parent.canonicalize().map(|p| p.join(filename))
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid memory file path: no filename",
                    ))
                }
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid memory file path: no parent directory",
                ))
            }
        })
        .with_context(|| format!("Failed to canonicalize memory file path: {:?}", memory_file))?;

    // Enhanced symlink detection for security
    if let Ok(metadata) = std::fs::symlink_metadata(&canonical_path) {
        if metadata.file_type().is_symlink() {
            // Always log symlink detection for security - not just in debug mode
            eprintln!(
                "Security Warning: Path points to a symlink: {:?}",
                canonical_path
            );
            anyhow::bail!(
                "Path points to a symlink, potential security risk: {:?}",
                canonical_path
            );
        }
    }

    // Check if path was normalized due to symlinks in parent directories
    // Only check if the canonical path exists (memory_file existence already handled above)
    if canonical_path.exists() {
        // Attempt to canonicalize the original memory_file for comparison
        match memory_file.canonicalize() {
            Ok(memory_canonical) => {
                if canonical_path != memory_canonical {
                    // This indicates symlink resolution happened in parent directories
                    eprintln!(
                        "Security Info: Path was resolved through symlinks: {:?} -> {:?}",
                        memory_file, canonical_path
                    );
                }
            }
            Err(_) => {
                // If we can't canonicalize the original, just log this fact
                eprintln!(
                    "Security Info: Could not verify path normalization for: {:?}",
                    memory_file
                );
            }
        }
    }

    Ok(canonical_path)
}

/// Create unique temporary file in same directory as target to ensure atomic rename
fn create_unique_temp_file(target_path: &std::path::PathBuf) -> Result<std::path::PathBuf> {
    use uuid::Uuid;

    let parent_dir = target_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Target path has no parent directory: {:?}", target_path))?;

    let filename = target_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid target filename: {:?}", target_path))?;

    // Create unique temporary filename with UUID for collision avoidance
    let uuid = Uuid::new_v4();
    let temp_filename = format!(
        "{}.{}.{}",
        filename,
        uuid.simple(),
        xml_config::TEMP_FILE_EXTENSION
    );
    let temp_path = parent_dir.join(temp_filename);

    // Validate temp file is in same directory (cross-filesystem safety)
    if temp_path.parent() != target_path.parent() {
        anyhow::bail!("Temporary file not in same directory as target, atomic rename may fail");
    }

    Ok(temp_path)
}

/// Validate XML with proper parser for structural correctness
fn validate_xml_with_parser(xml_content: &str) -> Result<()> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    // Basic content validation
    if xml_content.trim().is_empty() {
        anyhow::bail!("XML content is empty");
    }

    // Use quick-xml for proper XML validation
    let mut reader = Reader::from_str(xml_content);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut depth = 0;
    let mut root_element_found = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                // Check for expected root element
                if depth == 1 {
                    if name != "claude-context" {
                        anyhow::bail!("Expected root element 'claude-context', found '{}'", name);
                    }
                    root_element_found = true;
                }
            }
            Ok(Event::End(_)) => {
                depth -= 1;
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                // Check for expected root element (self-closing)
                if depth == 0 && name == "claude-context" {
                    root_element_found = true;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                anyhow::bail!("XML parsing error: {}", e);
            }
            _ => {} // Other events are OK
        }

        buf.clear();
    }

    if !root_element_found {
        anyhow::bail!("XML must contain 'claude-context' root element");
    }

    if depth != 0 {
        anyhow::bail!("XML has unbalanced tags, depth at end: {}", depth);
    }

    Ok(())
}

/// Log memory operation success with consistent formatting and security
fn log_memory_operation_success(memory_file: &std::path::PathBuf, operation_type: &str) {
    if std::env::var("DEBUG_HOOKS").unwrap_or_default() == "true" {
        // In debug mode, log filename for diagnostics
        match memory_file.file_name().and_then(|name| name.to_str()) {
            Some(filename) => {
                eprintln!(
                    "Memory organizer: Successfully saved {} to {}",
                    operation_type, filename
                );
            }
            None => {
                eprintln!(
                    "Memory organizer: Successfully saved {} (filename not UTF-8)",
                    operation_type
                );
            }
        }
    } else {
        // In production mode, log without filename for security
        eprintln!("Memory organizer: Successfully saved {}", operation_type);
    }
}

/// Format ai_dynamic_context section
fn format_ai_dynamic_context(context: &serde_json::Value) -> Result<String> {
    let mut xml = String::from("  <ai-dynamic-context>\n");

    if let Some(current_session) = context.get("current_session") {
        xml.push_str(&format_current_session(current_session, 4)?);
    }

    if let Some(metadata) = context.get("metadata") {
        xml.push_str(&format_metadata(metadata, 4)?);
    }

    if let Some(solutions) = context.get("solutions_archive") {
        xml.push_str(&format_solutions_archive(solutions, 4)?);
    }

    xml.push_str("  </ai-dynamic-context>\n");
    Ok(xml)
}

/// Format current session with proper indentation
fn format_current_session(session: &serde_json::Value, indent: usize) -> Result<String> {
    let indent_str = " ".repeat(indent);
    let mut xml = format!("{}  <current-session>\n", indent_str);

    // Active context
    if let Some(active_context) = session.get("active_context") {
        xml.push_str(&format!("{}    <active-context>\n", indent_str));

        if let Some(task) = active_context.get("current_task") {
            xml.push_str(&format!(
                "{}      <current-task>{}</current-task>\n",
                indent_str,
                xml_escape(task.as_str().unwrap_or(""))
            ));
        }

        if let Some(action) = active_context.get("last_action") {
            xml.push_str(&format!(
                "{}      <last-action>{}</last-action>\n",
                indent_str,
                xml_escape(action.as_str().unwrap_or(""))
            ));
        }

        if let Some(steps) = active_context.get("next_steps") {
            xml.push_str(&format!("{}      <next-steps>\n", indent_str));
            if let Some(array) = steps.as_array() {
                for step in array {
                    xml.push_str(&format!(
                        "{}        <step>{}</step>\n",
                        indent_str,
                        xml_escape(step.as_str().unwrap_or(""))
                    ));
                }
            }
            xml.push_str(&format!("{}      </next-steps>\n", indent_str));
        }

        if let Some(status) = active_context.get("status") {
            xml.push_str(&format!(
                "{}      <status>{}</status>\n",
                indent_str,
                xml_escape(status.as_str().unwrap_or(""))
            ));
        }

        if let Some(files) = active_context.get("working_files") {
            xml.push_str(&format!("{}      <working-files>\n", indent_str));
            if let Some(array) = files.as_array() {
                for file in array {
                    xml.push_str(&format!(
                        "{}        <file>{}</file>\n",
                        indent_str,
                        xml_escape(file.as_str().unwrap_or(""))
                    ));
                }
            }
            xml.push_str(&format!("{}      </working-files>\n", indent_str));
        }

        xml.push_str(&format!("{}    </active-context>\n", indent_str));
    }

    // Key insights
    if let Some(insights) = session.get("key_insights") {
        xml.push_str(&format!("{}    <key-insights>\n", indent_str));
        if let Some(array) = insights.as_array() {
            for insight in array {
                xml.push_str(&format!(
                    "{}      <insight>{}</insight>\n",
                    indent_str,
                    xml_escape(insight.as_str().unwrap_or(""))
                ));
            }
        }
        xml.push_str(&format!("{}    </key-insights>\n", indent_str));
    }

    // Technical state
    if let Some(tech_state) = session.get("technical_state") {
        xml.push_str(&format!("{}    <technical-state>\n", indent_str));
        if let Some(array) = tech_state.as_array() {
            for item in array {
                xml.push_str(&format!("{}      <technical-detail>\n", indent_str));

                if let Some(content) = item.get("content") {
                    xml.push_str(&format!(
                        "{}        <content>{}</content>\n",
                        indent_str,
                        xml_escape(content.as_str().unwrap_or(""))
                    ));
                }
                if let Some(location) = item.get("location") {
                    xml.push_str(&format!(
                        "{}        <location>{}</location>\n",
                        indent_str,
                        xml_escape(location.as_str().unwrap_or(""))
                    ));
                }
                if let Some(status) = item.get("status") {
                    xml.push_str(&format!(
                        "{}        <status>{}</status>\n",
                        indent_str,
                        xml_escape(status.as_str().unwrap_or(""))
                    ));
                }
                if let Some(type_val) = item.get("type") {
                    xml.push_str(&format!(
                        "{}        <type>{}</type>\n",
                        indent_str,
                        xml_escape(type_val.as_str().unwrap_or(""))
                    ));
                }

                xml.push_str(&format!("{}      </technical-detail>\n", indent_str));
            }
        }
        xml.push_str(&format!("{}    </technical-state>\n", indent_str));
    }

    xml.push_str(&format!("{}  </current-session>\n", indent_str));
    Ok(xml)
}

/// Format metadata section
fn format_metadata(metadata: &serde_json::Value, indent: usize) -> Result<String> {
    let indent_str = " ".repeat(indent);
    let mut xml = format!("{}  <metadata>\n", indent_str);

    if let Some(updated) = metadata.get("last_updated") {
        xml.push_str(&format!(
            "{}    <last-updated>{}</last-updated>\n",
            indent_str,
            xml_escape(updated.as_str().unwrap_or(""))
        ));
    }

    if let Some(stats) = metadata.get("optimization_stats") {
        xml.push_str(&format!("{}    <optimization-stats>\n", indent_str));

        if let Some(count) = stats.get("memories_count") {
            xml.push_str(&format!(
                "{}      <memories-count>{}</memories-count>\n",
                indent_str,
                count.as_u64().unwrap_or(0)
            ));
        }
        if let Some(ratio) = stats.get("reduction_ratio") {
            xml.push_str(&format!(
                "{}      <reduction-ratio>{}</reduction-ratio>\n",
                indent_str,
                ratio.as_f64().unwrap_or(0.0)
            ));
        }
        if let Some(tokens) = stats.get("total_tokens") {
            xml.push_str(&format!(
                "{}      <total-tokens>{}</total-tokens>\n",
                indent_str,
                tokens.as_u64().unwrap_or(0)
            ));
        }

        xml.push_str(&format!("{}    </optimization-stats>\n", indent_str));
    }

    if let Some(source) = metadata.get("source") {
        xml.push_str(&format!(
            "{}    <source>{}</source>\n",
            indent_str,
            xml_escape(source.as_str().unwrap_or(""))
        ));
    }

    xml.push_str(&format!("{}  </metadata>\n", indent_str));
    Ok(xml)
}

/// Format solutions archive
fn format_solutions_archive(solutions: &serde_json::Value, indent: usize) -> Result<String> {
    let indent_str = " ".repeat(indent);
    let mut xml = format!("{}  <solutions-archive>\n", indent_str);

    if let Some(array) = solutions.as_array() {
        for solution in array {
            xml.push_str(&format!("{}    <solution>\n", indent_str));

            if let Some(problem) = solution.get("problem") {
                xml.push_str(&format!(
                    "{}      <problem>{}</problem>\n",
                    indent_str,
                    xml_escape(problem.as_str().unwrap_or(""))
                ));
            }
            if let Some(sol_text) = solution.get("solution") {
                xml.push_str(&format!(
                    "{}      <solution-text>{}</solution-text>\n",
                    indent_str,
                    xml_escape(sol_text.as_str().unwrap_or(""))
                ));
            }
            if let Some(files) = solution.get("files_changed") {
                xml.push_str(&format!("{}      <files-changed>\n", indent_str));
                if let Some(files_array) = files.as_array() {
                    for file in files_array {
                        xml.push_str(&format!(
                            "{}        <file>{}</file>\n",
                            indent_str,
                            xml_escape(file.as_str().unwrap_or(""))
                        ));
                    }
                }
                xml.push_str(&format!("{}      </files-changed>\n", indent_str));
            }

            xml.push_str(&format!("{}    </solution>\n", indent_str));
        }
    }

    xml.push_str(&format!("{}  </solutions-archive>\n", indent_str));
    Ok(xml)
}

/// Format project info section
fn format_project_info(project: &serde_json::Value) -> Result<String> {
    let mut xml = String::from("  <project-info>\n");

    if let Some(name) = project.get("name") {
        xml.push_str(&format!(
            "    <name>{}</name>\n",
            xml_escape(name.as_str().unwrap_or(""))
        ));
    }
    if let Some(desc) = project.get("description") {
        xml.push_str(&format!(
            "    <description>{}</description>\n",
            xml_escape(desc.as_str().unwrap_or(""))
        ));
    }
    if let Some(created) = project.get("created_at") {
        xml.push_str(&format!(
            "    <created-at>{}</created-at>\n",
            xml_escape(created.as_str().unwrap_or(""))
        ));
    }

    xml.push_str("  </project-info>\n");
    Ok(xml)
}

/// Format API configuration section
fn format_api_configuration(config: &serde_json::Value) -> Result<String> {
    let mut xml = String::from("  <api-configuration>\n");

    if let Some(gpt5) = config.get("gpt5_settings") {
        xml.push_str("    <gpt5-settings>\n");
        if let Some(model) = gpt5.get("model") {
            xml.push_str(&format!(
                "      <model>{}</model>\n",
                xml_escape(model.as_str().unwrap_or(""))
            ));
        }
        if let Some(endpoint) = gpt5.get("endpoint") {
            xml.push_str(&format!(
                "      <endpoint>{}</endpoint>\n",
                xml_escape(endpoint.as_str().unwrap_or(""))
            ));
        }
        xml.push_str("    </gpt5-settings>\n");
    }

    xml.push_str("  </api-configuration>\n");
    Ok(xml)
}

/// Escape special XML characters for all content including numbers
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Token calculation metrics for logging
#[derive(Debug)]
struct TokenMetrics {
    original_tokens: usize,
    optimized_tokens: usize,
    tokens_reduced: usize,
    reduction_percentage: f64,
}

/// Process memory optimization with retry logic and validation
async fn process_memory_optimization(
    transcript: &str,
    existing_memory: &str,
    context_window: usize,
) -> Result<String> {
    const MAX_RETRIES: usize = 3;
    const RETRY_DELAY_MS: u64 = 1000;

    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
        match optimize_memories_with_validation(transcript, existing_memory, context_window).await {
            Ok(xml) => {
                if attempt > 1 {
                    eprintln!("Memory optimization succeeded on attempt {}", attempt);
                }
                return Ok(xml);
            }
            Err(e) => {
                eprintln!("Memory optimization attempt {} failed: {}", attempt, e);
                last_error = Some(e);

                if attempt < MAX_RETRIES {
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        RETRY_DELAY_MS * attempt as u64,
                    ))
                    .await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
}

/// Optimize memories with GPT-5 and validate response
async fn optimize_memories_with_validation(
    transcript: &str,
    existing_memory: &str,
    context_window: usize,
) -> Result<String> {
    // Get JSON response from GPT-5
    let optimization_json = optimize_memories(transcript, existing_memory, context_window).await?;

    // Validate JSON schema before processing
    let json_value = validate_gpt5_response(&optimization_json)?;

    // Convert to XML with validation
    let optimization_xml = convert_json_to_xml(json_value)
        .with_context(|| "Failed to convert validated JSON to XML format")?;

    // Final XML validation
    validate_xml_output(&optimization_xml)?;

    Ok(optimization_xml)
}

/// Validate GPT-5 JSON response against expected schema
fn validate_gpt5_response(json_str: &str) -> Result<serde_json::Value> {
    let json_value: serde_json::Value =
        serde_json::from_str(json_str).with_context(|| "Failed to parse GPT-5 JSON response")?;

    // Basic structure validation
    if !json_value.is_object() {
        anyhow::bail!("GPT-5 response is not a JSON object");
    }

    // Check for required top-level fields
    let obj = json_value.as_object().unwrap();

    // Validate that we have either ai_dynamic_context or the fields directly
    if !obj.contains_key("ai_dynamic_context")
        && !obj.contains_key("current_task")
        && !obj.contains_key("active_context")
    {
        anyhow::bail!("GPT-5 response missing required memory structure fields");
    }

    Ok(json_value)
}

/// Validate final XML output for well-formedness
fn validate_xml_output(xml_str: &str) -> Result<()> {
    // Basic XML structure checks
    if !xml_str.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>") {
        anyhow::bail!("XML output missing proper XML declaration");
    }

    if !xml_str.contains("<claude-context>") || !xml_str.contains("</claude-context>") {
        anyhow::bail!("XML output missing required claude-context root element");
    }

    // Check for balanced tags (basic validation)
    let open_tags = xml_str.matches('<').count() - xml_str.matches("</").count();
    let close_tags = xml_str.matches("</").count();

    // Should have roughly equal open and close tags (accounting for self-closing)
    if (open_tags as i32 - close_tags as i32).abs() > 2 {
        anyhow::bail!("XML output appears to have unbalanced tags");
    }

    Ok(())
}

/// Calculate token metrics for performance monitoring
fn calculate_token_metrics(
    transcript: &str,
    existing_memory: &str,
    optimization_xml: &str,
) -> Result<TokenMetrics> {
    let original_tokens = count_tokens(transcript)? + count_tokens(existing_memory)?;
    let optimized_tokens = count_tokens(optimization_xml)?;
    let tokens_reduced = original_tokens.saturating_sub(optimized_tokens);

    let reduction_percentage = if original_tokens > 0 {
        (tokens_reduced as f64 / original_tokens as f64) * 100.0
    } else {
        0.0
    };

    Ok(TokenMetrics {
        original_tokens,
        optimized_tokens,
        tokens_reduced,
        reduction_percentage,
    })
}

/// Log token metrics for debugging and monitoring
fn log_token_metrics(metrics: &TokenMetrics) {
    eprintln!(
        "Token analysis: Original: {}, Optimized (XML): {}, Reduced: {} ({:.1}%)",
        metrics.original_tokens,
        metrics.optimized_tokens,
        metrics.tokens_reduced,
        metrics.reduction_percentage
    );
}

/// Build full memory structure from GPT-5 response with proper validation
fn build_full_memory_structure(json_value: serde_json::Value) -> Result<serde_json::Value> {
    // Check if GPT-5 already returned full structure
    if json_value.get("ai_dynamic_context").is_some() {
        // GPT-5 returned full structure, just return it as-is
        return Ok(json_value);
    }

    // Otherwise, validate that we have at least the required fields for simplified structure
    let _active_context = json_value
        .get("active_context")
        .ok_or_else(|| anyhow::anyhow!("Missing 'active_context' in GPT-5 response"))?;

    // Extract values with safe defaults
    let reduction_ratio = json_value
        .get("reduction_ratio")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.75);

    let total_tokens = json_value
        .get("total_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let solutions_archive = json_value
        .get("solutions_archive")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    let solutions_count = if let Some(arr) = solutions_archive.as_array() {
        arr.len()
    } else {
        0
    };

    // Calculate actual memories count from optimized_memories or key_insights
    let memories_count = json_value
        .get("optimized_memories")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .or_else(|| {
            json_value
                .get("key_insights")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
        })
        .unwrap_or(0);

    // Build full structure according to prompts/memory_optimization.txt
    let full_structure = serde_json::json!({
        "ai_dynamic_context": {
            "current_session": json_value,
            "metadata": {
                "last_updated": chrono::Utc::now().to_rfc3339(),
                "optimization_stats": {
                    "memories_count": memories_count,
                    "reduction_ratio": reduction_ratio,
                    "solutions_count": solutions_count,
                    "total_tokens": total_tokens
                },
                "source": "memory_organizer_hook",
                "version": "1.0.0"
            },
            "solutions_archive": solutions_archive
        },
        "project_info": {
            "name": "ValidationCodeHook",
            "description": "AI-driven validation hooks for Claude Code",
            "created_at": "2025-09-06T13:42:16Z"
        },
        "api_configuration": {
            "gpt5_settings": {
                "model": "gpt-5-nano",
                "endpoint": "https://api.openai.com/v1/responses",
                "max_output_tokens": 12000,
                "reasoning_effort": "medium"
            },
            "error_handling": {
                "gpt5_401": "Check OPENAI_API_KEY starts with 'sk-'",
                "path_validation": "Use hook_input.cwd for safe path construction"
            },
            "environment_variables": {
                "OPENAI_API_KEY": "Required for GPT-5 memory optimization",
                "DEBUG_HOOKS": "Set to 'true' for detailed logging"
            }
        }
    });

    Ok(full_structure)
}

/// Count tokens in a string using simple estimation for GPT-5-nano
/// Uses combination of character and word count for better accuracy
fn count_tokens(text: &str) -> Result<usize> {
    // Empty text shortcut
    if text.is_empty() {
        return Ok(0);
    }

    // Check text size limit
    let text_len = text.len();
    if text_len > MAX_TRANSCRIPT_SIZE {
        return Err(anyhow::anyhow!(
            "Text too large: {} bytes exceeds {} byte limit",
            text_len,
            MAX_TRANSCRIPT_SIZE
        ));
    }

    // Use simpler hash for performance - just text length and first/last chars
    let cache_key = if text_len < 100 {
        // For small texts, hash the whole thing
        hash_text(text)
    } else {
        // For larger texts, use a faster approximation
        let mut hasher = Sha256::new();
        hasher.update(&text_len.to_le_bytes());
        hasher.update(&text[..100].as_bytes()); // First 100 bytes
        hasher.update(&text[text_len.saturating_sub(100)..].as_bytes()); // Last 100 bytes
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    };

    // Check cache first
    {
        let mut cache = TOKEN_CACHE
            .lock()
            .map_err(|e| anyhow::anyhow!("Cache lock poisoned: {}", e))?;
        if let Some(&count) = cache.get(&cache_key) {
            return Ok(count);
        }
    }

    // Simple token estimation for GPT-5-nano
    let chars = text.chars().count();
    let words = text.split_whitespace().count();

    // Use combination of character and word count for better estimation
    let char_estimate = (chars as f64 / config::AVG_CHARS_PER_TOKEN) as usize;
    let word_estimate = (words as f64 * config::AVG_TOKENS_PER_WORD) as usize;

    // Take weighted average (60% char-based, 40% word-based)
    let count = (char_estimate * 6 + word_estimate * 4) / 10;

    // Store in LRU cache
    {
        let mut cache = TOKEN_CACHE
            .lock()
            .map_err(|e| anyhow::anyhow!("Cache lock poisoned: {}", e))?;
        cache.put(cache_key, count);
    }

    Ok(count)
}

/// Optimize memories using AI
async fn optimize_memories(
    transcript: &str,
    existing_memory: &str,
    context_window: usize,
) -> Result<String> {
    // Load configuration
    let config = Config::from_env()?;

    // Create AI client for GPT-5 calls
    let ai_client = UniversalAIClient::new(config.clone())?;

    // Load memory optimization prompt
    let prompt = load_memory_prompt().await?;

    // Limit transcript size to prevent API errors
    let max_transcript_size = MAX_TRANSCRIPT_SIZE;
    let truncated_transcript = if transcript.chars().count() > max_transcript_size {
        eprintln!(
            "Truncating transcript from {} to {} chars to prevent API errors",
            transcript.chars().count(),
            max_transcript_size
        );
        // Safely truncate at character boundary to avoid Unicode panic
        let truncated: String = transcript.chars().take(max_transcript_size).collect();
        format!("{}...[TRUNCATED DUE TO SIZE]", truncated)
    } else {
        transcript.to_string()
    };

    // Calculate accurate token counts with global caching
    let prompt_tokens = count_tokens(&prompt)?;
    let existing_memory_tokens = count_tokens(&existing_memory)?;
    let transcript_tokens = count_tokens(&truncated_transcript)?;

    // Prepare context for AI - validate size
    let context = format!(
        "CONTEXT WINDOW LIMIT: {} tokens
        \nEXISTING MEMORIES ({} tokens):
{}

NEW CONVERSATION TRANSCRIPT ({} tokens):
{}

Optimize and merge these memories, keeping only the most relevant information within the token limit.",
        context_window,
        existing_memory_tokens,
        if existing_memory.is_empty() { "No existing memories" } else { existing_memory },
        transcript_tokens,
        if truncated_transcript.is_empty() { "No new transcript" } else { &truncated_transcript }
    );

    // Calculate exact total input tokens
    let context_tokens = count_tokens(&context)?;
    let total_input_tokens = prompt_tokens + context_tokens;

    if total_input_tokens > context_window {
        eprintln!(
            "Warning: Input exceeds context window ({} tokens > {} limit)",
            total_input_tokens, context_window
        );
        // Truncate if necessary
        if total_input_tokens > context_window * 2 {
            anyhow::bail!(
                "Input too large: {} tokens exceeds double the context window",
                total_input_tokens
            );
        }
    }

    // Use GPT-5-nano for memory optimization as per docs
    let model = "gpt-5-nano";

    // Call GPT-5 using UniversalAIClient
    let response = ai_client
        .optimize_memory_gpt5(&context, &prompt, model)
        .await;

    match response {
        Ok(json_value) => {
            // Build full structure according to prompts/memory_optimization.txt
            let full_structure = build_full_memory_structure(json_value)?;

            let json_string = serde_json::to_string_pretty(&full_structure)
                .context("Failed to serialize full memory structure")?;
            Ok(json_string)
        }
        Err(e) => {
            // When AI is unavailable, don't do anything - just return error
            eprintln!("AI optimization failed: {}. Memory not optimized.", e);
            Err(e)
        }
    }
}

/// Safely read and validate input from stdin with size limits and error handling
async fn read_and_validate_stdin_input() -> Result<StopEventInput> {
    // Read from stdin with size limits to prevent DoS attacks
    let mut buffer = String::new();
    let stdin_reader = std::io::BufReader::new(io::stdin().take(config::max_stdin_size() as u64));
    let mut handle = stdin_reader;

    // Handle potential UTF-8 encoding errors gracefully
    match handle.read_to_string(&mut buffer) {
        Ok(0) => anyhow::bail!("No input received from stdin"),
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::InvalidData => {
            anyhow::bail!("Invalid UTF-8 sequence in stdin input: {}", e);
        }
        Err(e) => anyhow::bail!("Failed to read from stdin: {}", e),
    }

    // Sanitize input to prevent zero-width character obfuscation attacks
    let sanitized_buffer = sanitize_zero_width_chars(&buffer);

    // Validate JSON structure before parsing
    validate_json_structure(&sanitized_buffer)?;

    // Parse the input with flexible structure for Stop events
    let hook_input: StopEventInput =
        serde_json::from_str(&sanitized_buffer).with_context(|| {
            format!(
                "Failed to parse input JSON. Input preview: {}",
                truncate_utf8_safe(&sanitized_buffer, 100)
            )
        })?;

    Ok(hook_input)
}

/// Validate JSON structure before deserialization to prevent malformed input
fn validate_json_structure(input: &str) -> Result<()> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        anyhow::bail!("Empty input received");
    }

    // Perform basic JSON validation using serde_json
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(_) => Ok(()),
        Err(e) => anyhow::bail!("Invalid JSON structure: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Read and validate input from stdin
    let hook_input = match read_and_validate_stdin_input().await {
        Ok(input) => input,
        Err(e) => {
            eprintln!("Memory organizer: Failed to read or validate input: {}", e);
            println!(r#"{{"continue":true}}"#); // Continue without blocking
            return Ok(());
        }
    };

    // For Stop events, we always process (no need to check event name)
    // Claude Code calls this hook only for Stop events as configured in settings.json

    // Construct safe path to CLAUDE.md in project directory
    let memory_file = match construct_project_claude_md_path(&hook_input) {
        Ok(path) => path,
        Err(e) => {
            eprintln!(
                "Memory organizer: Failed to construct CLAUDE.md path: {}",
                e
            );
            println!(r#"{{"continue":true}}"#); // Continue without blocking
            return Ok(());
        }
    };

    // Determine project directory for transcript search (keeping for compatibility)
    let project_dir = match determine_project_dir(&hook_input) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!(
                "Memory organizer: Failed to determine project directory: {}",
                e
            );
            println!(r#"{{"continue":true}}"#); // Continue without blocking
            return Ok(());
        }
    };
    let context_window = DEFAULT_CONTEXT_WINDOW; // Default context window

    eprintln!("Memory organizer: Processing project at {}", project_dir);

    // Find latest transcript file in project directory
    let transcript_path = match find_latest_transcript(&project_dir).await {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Memory organizer: No transcript found: {}", e);
            println!(r#"{{"continue":true}}"#); // Continue without blocking
            return Ok(());
        }
    };

    eprintln!("Using transcript: {}", transcript_path);

    // Read transcript and existing memory
    let transcript = read_transcript(&transcript_path).await?;
    let existing_memory = read_existing_memory(&memory_file).await?;

    eprintln!(
        "Transcript size: {} chars, Memory size: {} chars",
        transcript.len(),
        existing_memory.len()
    );

    // Get optimized memory with retry logic
    let optimization_xml = match process_memory_optimization(
        &transcript,
        &existing_memory,
        context_window,
    )
    .await
    {
        Ok(xml) => xml,
        Err(api_error) => {
            eprintln!("Memory optimization failed after retries: {}. Cannot continue without AI optimization.", api_error);
            println!(r#"{{"continue":true}}"#);
            return Ok(());
        }
    };

    // Calculate and log token metrics
    let token_metrics = calculate_token_metrics(&transcript, &existing_memory, &optimization_xml)?;
    log_token_metrics(&token_metrics);

    // Save XML with robust error handling and validation
    let memory_path = std::path::PathBuf::from(&memory_file);
    write_xml_memory_file(&memory_path, &optimization_xml).await?;

    // Log success using helper function
    log_memory_operation_success(&memory_path, "XML memory");

    println!(r#"{{"continue":true}}"#); // Continue without blocking

    Ok(())
}
