/// Multi-language AST analysis using Tree-sitter and specialized parsers
use anyhow::Result;
use tree_sitter::{Language, Parser};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use lazy_static::lazy_static;

use crate::analysis::ast::visitor::ComplexityVisitor;
use crate::analysis::metrics::ComplexityMetrics;

/// Supported languages for AST analysis
/// Note: Rust uses syn crate for superior macro and procedural parsing,
/// while other languages use Tree-sitter for consistent cross-language support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    /// Rust - handled by syn crate (not Tree-sitter)
    Rust,
    /// Python - Tree-sitter based
    Python,
    /// JavaScript - Tree-sitter based  
    JavaScript,
    /// TypeScript - Tree-sitter based
    TypeScript,
    /// Java - Tree-sitter based
    Java,
    /// C# - Tree-sitter based
    CSharp,
    /// Go - Tree-sitter based
    Go,
    /// C - Tree-sitter based
    C,
    /// C++ - Tree-sitter based
    Cpp,
    /// PHP - Tree-sitter based
    Php,
    /// Ruby - Tree-sitter based
    Ruby,
}

impl SupportedLanguage {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "js" | "mjs" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "jsx" => Some(Self::JavaScript),
            "java" => Some(Self::Java),
            "cs" => Some(Self::CSharp),
            "go" => Some(Self::Go),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Self::Cpp),
            "php" => Some(Self::Php),
            "rb" => Some(Self::Ruby),
            _ => None,
        }
    }

    pub fn get_tree_sitter_language(self) -> Result<Language> {
        match self {
            Self::Rust => anyhow::bail!("Rust uses syn crate, not tree-sitter"),
            Self::Python => Ok(tree_sitter_python::language()),
            Self::JavaScript => Ok(tree_sitter_javascript::language()),
            Self::TypeScript => Ok(tree_sitter_typescript::language_typescript()),
            Self::Java => Ok(tree_sitter_java::language()),
            Self::CSharp => Ok(tree_sitter_c_sharp::language()),
            Self::Go => Ok(tree_sitter_go::language()),
            Self::C => Ok(tree_sitter_c::language()),
            Self::Cpp => Ok(tree_sitter_cpp::language()),
            Self::Php => Ok(tree_sitter_php::language_php()),
            Self::Ruby => Ok(tree_sitter_ruby::language()),
        }
    }
}

impl std::fmt::Display for SupportedLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "Rust"),
            Self::Python => write!(f, "Python"),
            Self::JavaScript => write!(f, "JavaScript"),
            Self::TypeScript => write!(f, "TypeScript"),
            Self::Java => write!(f, "Java"),
            Self::CSharp => write!(f, "C#"),
            Self::Go => write!(f, "Go"),
            Self::C => write!(f, "C"),
            Self::Cpp => write!(f, "C++"),
            Self::Php => write!(f, "PHP"),
            Self::Ruby => write!(f, "Ruby"),
        }
    }
}

// Thread-safe language cache to avoid recreating Tree-sitter languages
// Languages are expensive to create and can be safely shared between parsers
lazy_static! {
    static ref LANGUAGE_CACHE: Arc<RwLock<HashMap<SupportedLanguage, Language>>> = 
        Arc::new(RwLock::new(HashMap::new()));
}

/// Language cache manager for efficient Tree-sitter language reuse
/// 
/// This cache stores Tree-sitter Language objects for reuse across multiple parser creations.
/// Languages are expensive to create and can be safely shared between parsers.
/// We use a thread-safe cache to avoid recreating languages in multi-threaded scenarios.
pub struct LanguageCache;

impl LanguageCache {
    /// Get or create a Tree-sitter language for the given language type
    /// 
    /// This method first checks the cache for an existing language. If not found,
    /// it creates a new language and adds it to the cache for future use.
    /// Returns the Language object that can be used to configure a Parser.
    pub fn get_or_create_language(language: SupportedLanguage) -> Result<Language> {
        // Try to get existing language from cache (read lock scope)
        {
            let cache = LANGUAGE_CACHE.read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock on language cache: {}", e))?;
            if let Some(lang) = cache.get(&language) {
                // Languages need to be cloned (Tree-sitter Language doesn't implement Copy)
                return Ok(lang.clone());
            }
        }
        
        // Language not in cache, create new one
        let tree_sitter_lang = language.get_tree_sitter_language()?;
        
        // Add to cache (write lock scope)
        {
            let mut cache = LANGUAGE_CACHE.write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock on language cache: {}", e))?;
            // Double-check pattern to avoid race conditions
            if !cache.contains_key(&language) {
                cache.insert(language, tree_sitter_lang.clone());
            }
        }
        
        Ok(tree_sitter_lang)
    }
    
    /// Create a configured parser for the given language
    /// 
    /// This is a convenience method that combines language caching with parser creation.
    /// It uses the cached language if available, otherwise creates and caches it.
    pub fn create_parser_with_language(language: SupportedLanguage) -> Result<Parser> {
        let tree_sitter_lang = Self::get_or_create_language(language)?;
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_lang)
            .map_err(|e| anyhow::anyhow!("Failed to set parser language for {}: {}", language, e))?;
        Ok(parser)
    }
    
    /// Clear the language cache (useful for tests or memory management)
    pub fn clear_cache() -> Result<()> {
        let mut cache = LANGUAGE_CACHE.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
        cache.clear();
        Ok(())
    }
    
    /// Get cache size for monitoring and debugging
    pub fn cache_size() -> Result<usize> {
        let cache = LANGUAGE_CACHE.read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
        Ok(cache.len())
    }
    
    /// Initialize cache with all supported languages
    /// 
    /// This pre-populates the cache to avoid initialization delays during analysis.
    /// Useful for applications that need predictable performance.
    pub fn initialize_all_languages() -> Result<()> {
        let languages = [
            SupportedLanguage::Python,
            SupportedLanguage::JavaScript,
            SupportedLanguage::TypeScript,
            SupportedLanguage::Java,
            SupportedLanguage::CSharp,
            SupportedLanguage::Go,
            SupportedLanguage::C,
            SupportedLanguage::Cpp,
            SupportedLanguage::Php,
            SupportedLanguage::Ruby,
        ];
        
        for &lang in &languages {
            // This will create and cache each language
            Self::get_or_create_language(lang)?;
        }
        
        Ok(())
    }
}

/// Multi-language AST analyzer using Tree-sitter
pub struct MultiLanguageAnalyzer;

impl MultiLanguageAnalyzer {
    /// Analyze source code with Tree-sitter and return complexity metrics
    /// Includes timeout protection to prevent hanging on malformed code
    pub fn analyze_with_tree_sitter(
        source_code: &str,
        language: SupportedLanguage,
    ) -> Result<ComplexityMetrics> {
        Self::analyze_with_tree_sitter_timeout(source_code, language, std::time::Duration::from_secs(5))
    }
    
    /// Analyze source code with Tree-sitter with configurable timeout
    /// This prevents hanging on extremely complex or malformed code
    pub fn analyze_with_tree_sitter_timeout(
        source_code: &str,
        language: SupportedLanguage,
        timeout: std::time::Duration,
    ) -> Result<ComplexityMetrics> {
        use std::sync::mpsc;
        use std::thread;
        
        // Input validation
        if source_code.is_empty() {
            return Err(anyhow::anyhow!("Source code cannot be empty"));
        }

        // Additional validation for extremely long input to prevent resource exhaustion
        if source_code.len() > 10_000_000 {
            // 10MB limit
            return Err(anyhow::anyhow!(
                "Source code too large (>10MB), potential DoS risk"
            ));
        }

        // Rust should use syn crate, not Tree-sitter
        if language == SupportedLanguage::Rust {
            return Err(anyhow::anyhow!(
                "Rust analysis should use syn crate, not Tree-sitter"
            ));
        }
        
        // Clone source code for thread safety
        let source_code_clone = source_code.to_string();
        
        // Create channel for result communication
        let (tx, rx) = mpsc::channel();
        
        // Spawn analysis thread
        let handle = thread::spawn(move || {
            // Perform the actual analysis in a separate thread
            let result = (|| -> Result<ComplexityMetrics> {
                // Use the new LanguageCache for efficient parser creation
                let mut parser = LanguageCache::create_parser_with_language(language)?;

                // Parse source code with error handling for malformed syntax
                let tree = parser.parse(&source_code_clone, None).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Failed to parse {} source code - syntax may be invalid",
                        language
                    )
                })?;

                // Validate tree structure to prevent potential crashes
                let root_node = tree.root_node();
                if root_node.has_error() {
                    return Err(anyhow::anyhow!(
                        "Source code contains syntax errors that prevent analysis"
                    ));
                }

                // Create visitor and analyze AST
                let mut visitor = ComplexityVisitor::new(&source_code_clone, language);
                visitor.visit_node(&root_node)?;

                Ok(visitor.build_metrics())
            })();
            
            // Send result back to main thread
            let _ = tx.send(result);
        });
        
        // Wait for result with timeout
        match rx.recv_timeout(timeout) {
            Ok(result) => {
                // Make sure to join the thread to avoid resource leaks
                let _ = handle.join();
                result
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Analysis took too long
                // Note: The spawned thread will continue running but we return an error
                Err(anyhow::anyhow!(
                    "Analysis timeout: {} code analysis exceeded {:?} timeout",
                    language,
                    timeout
                ))
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Thread panicked or channel was dropped
                Err(anyhow::anyhow!(
                    "Analysis thread failed unexpectedly for {} code",
                    language
                ))
            }
        }
    }

    /// Analyze multiple files concurrently using cached parsers
    pub fn analyze_files_concurrent(
        files: &[(String, SupportedLanguage)],
    ) -> Vec<Result<ComplexityMetrics>> {
        use rayon::prelude::*;
        
        files.par_iter()
            .map(|(content, lang)| Self::analyze_with_tree_sitter(content, *lang))
            .collect()
    }

    /// Analyze files from paths concurrently with automatic language detection
    pub fn analyze_file_paths_concurrent(
        file_paths: &[std::path::PathBuf],
    ) -> Vec<(std::path::PathBuf, Result<ComplexityMetrics>)> {
        use rayon::prelude::*;
        use std::fs;
        
        file_paths.par_iter()
            .map(|path| {
                let result = (|| -> Result<ComplexityMetrics> {
                    // Detect language from extension
                    let extension = path.extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("");
                    
                    let language = SupportedLanguage::from_extension(extension)
                        .ok_or_else(|| anyhow::anyhow!("Unsupported file extension: {}", extension))?;
                    
                    // Skip Rust files as they use syn crate
                    if language == SupportedLanguage::Rust {
                        return Err(anyhow::anyhow!("Rust files should use syn crate analysis"));
                    }
                    
                    // Read file content
                    let content = fs::read_to_string(path)
                        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))?;
                    
                    Self::analyze_with_tree_sitter(&content, language)
                })();
                
                (path.clone(), result)
            })
            .collect()
    }

    /// Analyze project directory concurrently with filtering
    /// Returns successful results and collected errors separately
    /// Uses crossbeam-channel for efficient multi-producer multi-consumer communication
    pub fn analyze_project_concurrent(
        project_root: &std::path::Path,
        max_depth: Option<usize>,
        exclude_patterns: &[&str],
    ) -> Result<(Vec<(std::path::PathBuf, ComplexityMetrics)>, Vec<(std::path::PathBuf, anyhow::Error)>)> {
        use rayon::prelude::*;
        use crossbeam_channel::bounded;
        use std::thread;
        
        // Collect all relevant files
        let mut files = Vec::new();
        Self::collect_files_recursive(project_root, &mut files, max_depth.unwrap_or(10), 0, exclude_patterns)?;
        
        #[cfg(test)]
        {
            eprintln!("DEBUG: Collected {} files from {}", files.len(), project_root.display());
            for file in &files {
                eprintln!("  - {}", file.display());
            }
        }
        
        // Early return if no files to process
        if files.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        
        // Create bounded channels for better backpressure control
        // Bounded channels prevent memory exhaustion with large file counts
        let (results_tx, results_rx) = bounded::<(std::path::PathBuf, ComplexityMetrics)>(100);
        let (errors_tx, errors_rx) = bounded::<(std::path::PathBuf, anyhow::Error)>(100);
        
        // Spawn collector thread with efficient channel handling
        let total_files = files.len();
        let collector_handle = thread::spawn(move || {
            let mut final_results = Vec::new();
            let mut final_errors = Vec::new();
            let mut processed_count = 0;
            
            // Use crossbeam's select! macro for efficient multi-channel receiving
            // This eliminates busy-wait and provides optimal performance
            use crossbeam_channel::{select, RecvError};
            
            loop {
                // Exit when we've collected all expected results
                if processed_count >= total_files {
                    break;
                }
                
                // Use select! to efficiently wait on multiple channels
                select! {
                    recv(results_rx) -> msg => {
                        match msg {
                            Ok(result) => {
                                final_results.push(result);
                                processed_count += 1;
                            },
                            Err(RecvError) => {
                                // Channel disconnected, drain remaining messages
                                while let Ok(result) = results_rx.try_recv() {
                                    final_results.push(result);
                                    processed_count += 1;
                                }
                                // Continue to check errors channel
                            }
                        }
                    },
                    recv(errors_rx) -> msg => {
                        match msg {
                            Ok(error) => {
                                final_errors.push(error);
                                processed_count += 1;
                            },
                            Err(RecvError) => {
                                // Channel disconnected, drain remaining messages
                                while let Ok(error) = errors_rx.try_recv() {
                                    final_errors.push(error);
                                    processed_count += 1;
                                }
                                // Continue to check results channel
                            }
                        }
                    },
                    default(std::time::Duration::from_millis(100)) => {
                        // Timeout after 100ms of no activity
                        // Check if both channels are disconnected
                        if results_rx.is_empty() && errors_rx.is_empty() {
                            // Try one more drain to be sure
                            while let Ok(result) = results_rx.try_recv() {
                                final_results.push(result);
                                processed_count += 1;
                            }
                            while let Ok(error) = errors_rx.try_recv() {
                                final_errors.push(error);
                                processed_count += 1;
                            }
                            
                            // If still no progress and channels are disconnected, exit
                            if results_rx.try_recv().is_err() && errors_rx.try_recv().is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            
            (final_results, final_errors)
        });
        
        // Process files concurrently without mutex contention
        files.par_iter().for_each(|path| {
            match Self::analyze_file_paths_concurrent(&[path.clone()]).into_iter().next() {
                Some((_, Ok(metrics))) => {
                    if let Err(e) = results_tx.send((path.clone(), metrics)) {
                        eprintln!("Warning: Failed to send analysis result: {}", e);
                    }
                },
                Some((p, Err(e))) => {
                    #[cfg(test)]
                    eprintln!("DEBUG: Analysis error for {}: {}", p.display(), e);
                    
                    if let Err(send_err) = errors_tx.send((p.clone(), e)) {
                        eprintln!("Warning: Failed to send analysis error: {}", send_err);
                    }
                },
                None => {
                    let error = anyhow::anyhow!("No analysis result returned for {}", path.display());
                    if let Err(send_err) = errors_tx.send((path.clone(), error)) {
                        eprintln!("Warning: Failed to send empty result error: {}", send_err);
                    }
                }
            }
        });
        
        // Drop senders to signal completion to collector
        drop(results_tx);
        drop(errors_tx);
        
        // Wait for collector to finish and return results
        collector_handle.join()
            .map_err(|_| anyhow::anyhow!("Failed to join collector thread"))
    }

    /// Strict version that fails on any error
    pub fn analyze_project_concurrent_strict(
        project_root: &std::path::Path,
        max_depth: Option<usize>,
        exclude_patterns: &[&str],
    ) -> Result<Vec<(std::path::PathBuf, ComplexityMetrics)>> {
        let (results, errors) = Self::analyze_project_concurrent(project_root, max_depth, exclude_patterns)?;
        
        if !errors.is_empty() {
            let error_summary = errors.iter()
                .map(|(path, err)| format!("{}: {}", path.display(), err))
                .collect::<Vec<_>>()
                .join("\n");
            
            return Err(anyhow::anyhow!(
                "Analysis failed for {} files:\n{}", 
                errors.len(), 
                error_summary
            ));
        }
        
        Ok(results)
    }

    /// Helper function to recursively collect files
    fn collect_files_recursive(
        dir: &std::path::Path,
        files: &mut Vec<std::path::PathBuf>,
        max_depth: usize,
        current_depth: usize,
        exclude_patterns: &[&str],
    ) -> Result<()> {
        use std::fs;
        
        if current_depth > max_depth {
            return Ok(());
        }
        
        let entries = fs::read_dir(dir)
            .map_err(|e| anyhow::anyhow!("Failed to read directory {}: {}", dir.display(), e))?;
        
        for entry in entries {
            let entry = entry
                .map_err(|e| anyhow::anyhow!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            // Check exclude patterns
            let path_str = path.to_string_lossy();
            if exclude_patterns.iter().any(|pattern| path_str.contains(pattern)) {
                continue;
            }
            
            if path.is_dir() {
                Self::collect_files_recursive(&path, files, max_depth, current_depth + 1, exclude_patterns)?;
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                // Only collect files with supported extensions (except Rust)
                if SupportedLanguage::from_extension(ext).is_some() && ext != "rs" {
                    files.push(path);
                }
            }
        }
        
        Ok(())
    }

    /// Batch analyze with resource management and progress reporting
    pub fn analyze_batch_with_progress<F>(
        files: &[(String, SupportedLanguage)],
        batch_size: usize,
        progress_callback: F,
    ) -> Result<Vec<ComplexityMetrics>>
    where
        F: Fn(usize, usize) + Send + Sync,
    {
        use rayon::prelude::*;
        use std::sync::Arc;
        
        let callback = Arc::new(progress_callback);
        let total_files = files.len();
        let mut results = Vec::with_capacity(total_files);
        
        // Process in batches to control memory usage
        for (batch_idx, batch) in files.chunks(batch_size).enumerate() {
            let batch_results: Vec<Result<ComplexityMetrics>> = batch.par_iter()
                .map(|(content, lang)| {
                    Self::analyze_with_tree_sitter(content, *lang)
                })
                .collect();
            
            // Collect successful results only
            for (file_idx, result) in batch_results.into_iter().enumerate() {
                match result {
                    Ok(metrics) => results.push(metrics),
                    Err(e) => {
                        eprintln!("Warning: Failed to analyze file in batch {}, index {}: {}", 
                                batch_idx, file_idx, e);
                        // Push default metrics for failed analysis
                        results.push(ComplexityMetrics::default());
                    }
                }
            }
            
            // Report progress
            let processed = (batch_idx + 1) * batch_size.min(total_files - batch_idx * batch_size);
            callback(processed, total_files);
        }
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_language_from_extension() {
        assert_eq!(
            SupportedLanguage::from_extension("py"),
            Some(SupportedLanguage::Python)
        );
        assert_eq!(
            SupportedLanguage::from_extension("js"),
            Some(SupportedLanguage::JavaScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("ts"),
            Some(SupportedLanguage::TypeScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("java"),
            Some(SupportedLanguage::Java)
        );
        assert_eq!(
            SupportedLanguage::from_extension("rs"),
            Some(SupportedLanguage::Rust)
        );
        assert_eq!(SupportedLanguage::from_extension("unknown"), None);
    }

    #[test]
    fn test_tree_sitter_language_creation() {
        // Test that we can create Tree-sitter languages for supported languages
        assert!(SupportedLanguage::Python.get_tree_sitter_language().is_ok());
        assert!(SupportedLanguage::JavaScript
            .get_tree_sitter_language()
            .is_ok());
        assert!(SupportedLanguage::Java.get_tree_sitter_language().is_ok());

        // Rust should fail as it uses syn crate
        assert!(SupportedLanguage::Rust.get_tree_sitter_language().is_err());
    }

    #[test]
    fn test_analyze_empty_code() {
        let result = MultiLanguageAnalyzer::analyze_with_tree_sitter("", SupportedLanguage::Python);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_analyze_rust_rejection() {
        let result = MultiLanguageAnalyzer::analyze_with_tree_sitter(
            "fn main() {}",
            SupportedLanguage::Rust,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("syn crate"));
    }

    #[test]
    fn test_analyze_simple_python() {
        let python_code = "def hello():\n    return 'world'";
        let result =
            MultiLanguageAnalyzer::analyze_with_tree_sitter(python_code, SupportedLanguage::Python);
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert!(metrics.function_count >= 1);
        assert!(metrics.line_count >= 2);
    }

    #[test]
    fn test_analyze_simple_javascript() {
        let js_code = "function hello() { return 'world'; }";
        let result =
            MultiLanguageAnalyzer::analyze_with_tree_sitter(js_code, SupportedLanguage::JavaScript);
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert!(metrics.function_count >= 1);
    }

    #[test]
    fn test_language_cache() {
        // Clear cache to start fresh
        let _ = LanguageCache::clear_cache();
        
        let python_code = "def test(): pass";
        
        // First analysis should create and cache language
        let result1 = MultiLanguageAnalyzer::analyze_with_tree_sitter(
            python_code, SupportedLanguage::Python
        );
        assert!(result1.is_ok());
        
        // Check cache has at least one entry (Python)
        let cache_size = LanguageCache::cache_size().unwrap_or(0);
        assert!(cache_size >= 1, "Cache should have at least Python language cached");
        
        // Second analysis should reuse cached language
        let result2 = MultiLanguageAnalyzer::analyze_with_tree_sitter(
            python_code, SupportedLanguage::Python
        );
        assert!(result2.is_ok());
        
        // Cache size should be the same or have grown
        let cache_size_after = LanguageCache::cache_size().unwrap_or(0);
        assert!(cache_size_after >= cache_size, "Cache should not shrink after reuse");
    }
    
    #[test]
    fn test_language_cache_direct() {
        // Test direct language cache methods in isolation
        let _ = LanguageCache::clear_cache();
        
        // Verify cache starts empty
        let initial_size = LanguageCache::cache_size().unwrap();
        assert_eq!(initial_size, 0, "Cache should start empty after clear");
        
        // Test getting a language
        let lang = LanguageCache::get_or_create_language(SupportedLanguage::JavaScript);
        assert!(lang.is_ok(), "Should be able to create JavaScript language");
        
        // Test cache size after adding one language
        let size = LanguageCache::cache_size();
        assert!(size.is_ok());
        assert_eq!(size.unwrap(), 1, "Cache should have exactly one entry after adding JavaScript");
        
        // Test getting the same language again (should use cache)
        let lang2 = LanguageCache::get_or_create_language(SupportedLanguage::JavaScript);
        assert!(lang2.is_ok(), "Should reuse cached JavaScript language");
        
        // Cache size should still be 1
        let size2 = LanguageCache::cache_size().unwrap();
        assert_eq!(size2, 1, "Cache size should remain 1 after reusing language");
        
        // Test creating parser with cached language
        let parser = LanguageCache::create_parser_with_language(SupportedLanguage::JavaScript);
        assert!(parser.is_ok(), "Should create parser with cached language");
        
        // Add a different language to test cache growth
        let lang3 = LanguageCache::get_or_create_language(SupportedLanguage::Python);
        assert!(lang3.is_ok(), "Should be able to create Python language");
        
        let final_size = LanguageCache::cache_size().unwrap();
        assert!(final_size >= 2, "Cache should have at least 2 languages after adding Python (actual: {})", final_size);
    }
    
    #[test]
    fn test_language_cache_initialization() {
        let _ = LanguageCache::clear_cache();
        
        // Test initializing all languages
        let result = LanguageCache::initialize_all_languages();
        assert!(result.is_ok());
        
        // Cache should contain all supported languages (except Rust)
        let cache_size = LanguageCache::cache_size().unwrap();
        assert_eq!(cache_size, 10); // All languages except Rust
    }

    #[test]
    fn test_concurrent_analysis() {
        let files = vec![
            ("def func1(): pass".to_string(), SupportedLanguage::Python),
            ("function func2() {}".to_string(), SupportedLanguage::JavaScript),
            ("public void func3() {}".to_string(), SupportedLanguage::Java),
        ];
        
        let results = MultiLanguageAnalyzer::analyze_files_concurrent(&files);
        assert_eq!(results.len(), 3);
        
        for result in results {
            assert!(result.is_ok());
        }
    }
    
    #[test]
    fn test_thread_safety() {
        let python_code = "def test(): pass";
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let code = python_code.to_string();
                std::thread::spawn(move || {
                    MultiLanguageAnalyzer::analyze_with_tree_sitter(
                        &code, SupportedLanguage::Python
                    )
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.is_ok());
        }
    }
    
    /// Test concurrent analysis of multiple files with different languages
    #[test]
    fn test_analyze_files_concurrent_mixed() {
        let files = vec![
            ("def func1(): pass".to_string(), SupportedLanguage::Python),
            ("function func2() { return 42; }".to_string(), SupportedLanguage::JavaScript),
            ("public class Test { public void func3() {} }".to_string(), SupportedLanguage::Java),
            ("func func4() {}".to_string(), SupportedLanguage::Go),
        ];
        
        let results = MultiLanguageAnalyzer::analyze_files_concurrent(&files);
        assert_eq!(results.len(), 4);
        
        for result in results {
            assert!(result.is_ok(), "Analysis should succeed for valid code");
            let metrics = result.unwrap();
            assert!(metrics.function_count >= 1, "Should detect at least one function");
        }
    }
    
    /// Test concurrent analysis with some invalid files
    #[test]
    fn test_analyze_files_concurrent_with_errors() {
        let files = vec![
            ("def valid(): pass".to_string(), SupportedLanguage::Python),
            ("invalid { syntax".to_string(), SupportedLanguage::JavaScript),
            ("public class Valid {}".to_string(), SupportedLanguage::Java),
        ];
        
        let results = MultiLanguageAnalyzer::analyze_files_concurrent(&files);
        assert_eq!(results.len(), 3);
        
        // Should have mix of success and failure
        let successes = results.iter().filter(|r| r.is_ok()).count();
        let failures = results.iter().filter(|r| r.is_err()).count();
        
        assert!(successes >= 2, "At least 2 files should succeed");
        assert!(failures <= 1, "At most 1 file should fail");
    }
    
    /// Test file path analysis with temporary files
    #[test]
    fn test_analyze_file_paths_concurrent() {
        use tempfile::TempDir;
        use std::fs;
        
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        let py_file = temp_dir.path().join("test.py");
        fs::write(&py_file, "def hello():\n    return 'world'").unwrap();
        
        let js_file = temp_dir.path().join("test.js");
        fs::write(&js_file, "function hello() { return 'world'; }").unwrap();
        
        let paths = vec![py_file.clone(), js_file.clone()];
        let results = MultiLanguageAnalyzer::analyze_file_paths_concurrent(&paths);
        
        assert_eq!(results.len(), 2);
        
        for (path, result) in results {
            assert!(result.is_ok(), "Analysis should succeed for {:?}", path);
            let metrics = result.unwrap();
            assert!(metrics.function_count >= 1, "Should detect at least 1 function in {:?}", path);
            assert!(metrics.line_count >= 1);
        }
    }
    
    /// Test project directory analysis
    #[test]
    fn test_analyze_project_concurrent() {
        use tempfile::TempDir;
        use std::fs;
        
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        
        // Create nested directory structure
        fs::create_dir_all(project_root.join("src")).unwrap();
        fs::create_dir_all(project_root.join("tests")).unwrap();
        
        // Create test files
        fs::write(project_root.join("src/main.py"), "def main(): pass").unwrap();
        fs::write(project_root.join("src/utils.js"), "function util() {}").unwrap();
        fs::write(project_root.join("tests/test.py"), "def test_main(): assert True").unwrap();
        
        // Exclude tests directory
        let exclude_patterns = vec!["tests"];
        let result = MultiLanguageAnalyzer::analyze_project_concurrent(
            project_root, 
            Some(5), 
            &exclude_patterns
        );
        
        assert!(result.is_ok());
        let (results, errors) = result.unwrap();
        
        // Should have 2 files (excluding tests directory)
        assert_eq!(results.len(), 2, "Should analyze 2 files (excluding tests)");
        assert!(errors.is_empty(), "Should have no errors for valid files");
        
        // Verify all results have valid metrics
        for (path, metrics) in results {
            assert!(metrics.function_count >= 1, "File {:?} should have functions", path);
        }
    }
    
    /// Test strict project analysis that fails on any error
    #[test]
    fn test_analyze_project_concurrent_strict() {
        use tempfile::TempDir;
        use std::fs;
        
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        
        // Create test files with one invalid
        fs::create_dir_all(project_root.join("src")).unwrap();
        fs::write(project_root.join("src/valid.py"), "def func(): pass").unwrap();
        fs::write(project_root.join("src/invalid.js"), "function broken {").unwrap();
        
        let result = MultiLanguageAnalyzer::analyze_project_concurrent_strict(
            project_root, 
            None, 
            &[]
        );
        
        // Debug output to understand what's happening
        match &result {
            Ok(results) => {
                println!("Unexpected success with {} results:", results.len());
                for (path, metrics) in results {
                    println!("  - {}: functions={}", path.display(), metrics.function_count);
                }
            },
            Err(e) => println!("Got expected error: {}", e),
        }
        
        // Should fail because of invalid.js
        assert!(result.is_err(), "Strict analysis should fail with invalid files");
        
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Analysis failed"), "Error should indicate analysis failure");
    }
    
    /// Test batch analysis with progress reporting
    #[test]
    fn test_analyze_batch_with_progress() {
        use std::sync::{Arc, Mutex};
        
        let files = vec![
            ("def func1(): pass".to_string(), SupportedLanguage::Python),
            ("function func2() {}".to_string(), SupportedLanguage::JavaScript),
            ("public void func3() {}".to_string(), SupportedLanguage::Java),
            ("func func4() {}".to_string(), SupportedLanguage::Go),
        ];
        
        // Track progress calls
        let progress_calls = Arc::new(Mutex::new(Vec::new()));
        let progress_calls_clone = Arc::clone(&progress_calls);
        
        let progress_callback = move |processed: usize, total: usize| {
            if let Ok(mut calls) = progress_calls_clone.lock() {
                calls.push((processed, total));
            }
        };
        
        let batch_size = 2;
        let result = MultiLanguageAnalyzer::analyze_batch_with_progress(
            &files, 
            batch_size, 
            progress_callback
        );
        
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 4);
        
        // Verify progress was reported
        let calls = progress_calls.lock().unwrap();
        assert!(!calls.is_empty(), "Progress callback should be called");
        
        // Last call should be for all files
        if let Some((processed, total)) = calls.last() {
            assert_eq!(*total, 4);
            assert!(*processed <= 4);
        }
    }
    
    /// Test empty project handling
    #[test]
    fn test_analyze_empty_project() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let empty_project = temp_dir.path();
        
        let result = MultiLanguageAnalyzer::analyze_project_concurrent(
            empty_project, 
            None, 
            &[]
        );
        
        assert!(result.is_ok());
        let (results, errors) = result.unwrap();
        assert!(results.is_empty(), "Empty project should return no results");
        assert!(errors.is_empty(), "Empty project should have no errors");
    }
    
    /// Test invalid JavaScript detection
    #[test]
    fn test_invalid_javascript_detection() {
        let invalid_js = "function broken {";
        let result = MultiLanguageAnalyzer::analyze_with_tree_sitter(
            invalid_js, 
            SupportedLanguage::JavaScript
        );
        
        // This should return an error for invalid syntax
        println!("Result for invalid JS: {:?}", result);
        
        // If this assertion fails, we need to fix error detection
        assert!(result.is_err(), "Invalid JavaScript should return an error");
    }
    
    /// Test that parser works without caching (caching not implemented yet)
    #[test]
    fn test_parser_without_cache() {
        // Test that multiple calls work even without caching
        let python_code = "def test(): pass";
        
        let result1 = MultiLanguageAnalyzer::analyze_with_tree_sitter(
            python_code, 
            SupportedLanguage::Python
        );
        assert!(result1.is_ok());
        
        let result2 = MultiLanguageAnalyzer::analyze_with_tree_sitter(
            python_code, 
            SupportedLanguage::Python
        );
        assert!(result2.is_ok());
        
        // Results should be consistent
        let metrics1 = result1.unwrap();
        let metrics2 = result2.unwrap();
        assert_eq!(metrics1.function_count, metrics2.function_count);
    }
}
