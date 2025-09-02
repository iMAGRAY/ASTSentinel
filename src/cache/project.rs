use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cache entry for project structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectCache {
    pub structure: crate::analysis::project::ProjectStructure,
    pub metrics: ProjectMetrics,
    pub file_hashes: HashMap<String, FileHash>,
    pub cache_timestamp: i64,
    pub last_modified: SystemTime,
}

/// File hash information for incremental updates
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileHash {
    pub path: String,
    pub modified_time: SystemTime,
    pub size: u64,
    pub hash: Option<String>, // Optional content hash for important files
}

/// Project metrics and statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectMetrics {
    pub total_lines_of_code: usize,
    pub code_by_language: HashMap<String, LanguageStats>,
    pub file_importance_scores: HashMap<String, f32>,
    pub project_complexity_score: f32,
    pub test_coverage_estimate: f32,
    pub documentation_ratio: f32,
    // Advanced complexity metrics
    pub average_cyclomatic_complexity: f32,
    pub average_cognitive_complexity: f32,
    pub max_cyclomatic_complexity: u32,
    pub max_cognitive_complexity: u32,
    pub high_complexity_files: usize,  // Files with complexity score > 7
    pub complexity_distribution: ComplexityDistribution,
}

/// Language-specific statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LanguageStats {
    pub file_count: usize,
    pub lines_of_code: usize,
    pub lines_of_comments: usize,
    pub blank_lines: usize,
    pub average_file_size: usize,
    pub complexity_estimate: f32,
    // Enhanced complexity metrics
    pub average_cyclomatic: f32,
    pub average_cognitive: f32,
    pub max_cyclomatic: u32,
    pub max_cognitive: u32,
    pub total_functions: u32,
    pub average_nesting_depth: f32,
}

/// Distribution of complexity across the project
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplexityDistribution {
    pub low_complexity: usize,    // 0-3 complexity score
    pub medium_complexity: usize, // 4-7 complexity score
    pub high_complexity: usize,   // 8-10 complexity score
    pub extreme_complexity: usize, // >10 complexity score
}

/// Compressed structure format for token efficiency
#[derive(Debug, Serialize, Deserialize)]
pub struct CompressedStructure {
    pub format_version: u8,
    pub tree: String,           // Compressed tree representation
    pub metrics: String,         // Compressed metrics
    pub important_files: Vec<String>, // Top priority files
    pub token_estimate: usize,   // Estimated token count
}

impl ProjectCache {
    /// Load cache from disk
    pub fn load(cache_path: &Path) -> Result<Option<Self>> {
        if !cache_path.exists() {
            return Ok(None);
        }
        
        let contents = fs::read_to_string(cache_path)?;
        let cache: ProjectCache = serde_json::from_str(&contents)?;
        
        // Check if cache is still valid (default: 5 minutes)
        let now = chrono::Local::now().timestamp();
        if now - cache.cache_timestamp > 300 {
            return Ok(None);
        }
        
        Ok(Some(cache))
    }
    
    /// Save cache to disk
    pub fn save(&self, cache_path: &Path) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(cache_path, contents)?;
        Ok(())
    }
    
    /// Check if specific file needs update with content hash verification
    pub fn needs_update(&self, file_path: &str) -> bool {
        use std::io::Read;
        use sha2::{Sha256, Digest};
        
        let path = Path::new(file_path);
        
        if let Some(cached_hash) = self.file_hashes.get(file_path) {
            if let Ok(metadata) = fs::metadata(path) {
                // Quick check: size and modification time
                if metadata.len() != cached_hash.size {
                    return true;
                }
                
                if let Ok(modified) = metadata.modified() {
                    if modified != cached_hash.modified_time {
                        // If we have content hash, verify it
                        if let Some(ref old_hash) = cached_hash.hash {
                            if let Ok(mut file) = fs::File::open(path) {
                                let mut hasher = Sha256::new();
                                let mut buffer = [0; 8192];
                                while let Ok(bytes_read) = file.read(&mut buffer) {
                                    if bytes_read == 0 { break; }
                                    hasher.update(&buffer[..bytes_read]);
                                }
                                let new_hash = format!("{:x}", hasher.finalize());
                                return &new_hash != old_hash;
                            }
                        }
                        return true;
                    }
                }
            }
        }
        
        true // If we can't determine, assume it needs update
    }
    
    /// Get files that have changed since cache was created
    pub fn get_changed_files(&self, root_path: &Path) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        
        for (file_path, hash) in &self.file_hashes {
            let full_path = root_path.join(file_path);
            if let Ok(metadata) = fs::metadata(&full_path) {
                if let Ok(modified) = metadata.modified() {
                    if modified > hash.modified_time || metadata.len() != hash.size {
                        changed.push(full_path);
                    }
                }
            }
        }
        
        changed
    }
}

/// Enhanced LOC counter with better multi-line comment handling
pub fn count_lines_of_code(file_path: &Path) -> Result<(usize, usize, usize)> {
    let content = fs::read_to_string(file_path)?;
    let mut loc = 0;
    let mut comments = 0;
    let mut blanks = 0;
    
    let extension = file_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    // Language-specific comment patterns
    let comment_config = match extension {
        "rs" => Some(CommentConfig {
            single: "//",
            multi_start: "/*",
            multi_end: "*/",
            doc_single: Some("///"),
            doc_multi_start: Some("/**"),
        }),
        "js" | "ts" | "jsx" | "tsx" => Some(CommentConfig {
            single: "//",
            multi_start: "/*",
            multi_end: "*/",
            doc_single: None,
            doc_multi_start: Some("/**"),
        }),
        "py" => Some(CommentConfig {
            single: "#",
            multi_start: "\"\"\"",
            multi_end: "\"\"\"",
            doc_single: None,
            doc_multi_start: None,
        }),
        "java" | "c" | "cpp" | "go" => Some(CommentConfig {
            single: "//",
            multi_start: "/*",
            multi_end: "*/",
            doc_single: None,
            doc_multi_start: Some("/**"),
        }),
        "rb" => Some(CommentConfig {
            single: "#",
            multi_start: "=begin",
            multi_end: "=end",
            doc_single: None,
            doc_multi_start: None,
        }),
        "html" | "xml" => Some(CommentConfig {
            single: "",
            multi_start: "<!--",
            multi_end: "-->",
            doc_single: None,
            doc_multi_start: None,
        }),
        _ => None,
    };
    
    if let Some(config) = comment_config {
        let mut in_multi_comment = false;
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed.is_empty() {
                blanks += 1;
                continue;
            }
            
            // Handle multi-line comments
            if in_multi_comment {
                comments += 1;
                if trimmed.contains(config.multi_end) {
                    in_multi_comment = false;
                }
                continue;
            }
            
            // Check for multi-line comment start
            if !config.multi_start.is_empty() && trimmed.contains(config.multi_start) {
                in_multi_comment = !trimmed.contains(config.multi_end);
                comments += 1;
                continue;
            }
            
            // Check for single-line comments
            if !config.single.is_empty() && trimmed.starts_with(config.single) {
                comments += 1;
                continue;
            }
            
            // Check for doc comments
            if let Some(doc_single) = config.doc_single {
                if trimmed.starts_with(doc_single) {
                    comments += 1;
                    continue;
                }
            }
            
            // Otherwise it's code
            loc += 1;
        }
    } else {
        // Fallback for unknown file types
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                blanks += 1;
            } else {
                loc += 1;
            }
        }
    }
    
    Ok((loc, comments, blanks))
}

// Helper struct for comment configuration
struct CommentConfig {
    single: &'static str,
    multi_start: &'static str,
    multi_end: &'static str,
    doc_single: Option<&'static str>,
    #[allow(dead_code)]
    doc_multi_start: Option<&'static str>,
}

/// Calculate importance score for a file
pub fn calculate_file_importance(file: &crate::analysis::project::ProjectFile) -> f32 {
    let mut score: f32 = 0.0;
    
    // Base score by file type
    score += match file.file_type.as_str() {
        // Core application files
        "rs" | "go" | "java" | "cpp" => 1.0,
        "js" | "ts" | "py" | "rb" => 0.9,
        
        // Configuration and setup
        "toml" | "yaml" | "json" if file.relative_path.contains("config") => 0.8,
        "toml" if file.relative_path == "Cargo.toml" => 1.0,
        "json" if file.relative_path == "package.json" => 1.0,
        
        // Tests
        _ if file.relative_path.contains("test") => 0.7,
        
        // Documentation
        "md" if file.relative_path == "README.md" => 0.8,
        "md" => 0.5,
        
        _ => 0.3,
    };
    
    // Boost for main/index files
    if file.relative_path.contains("main.") || 
       file.relative_path.contains("index.") ||
       file.relative_path.contains("app.") {
        score += 0.3;
    }
    
    // Boost for src directory files
    if file.relative_path.starts_with("src/") {
        score += 0.2;
    }
    
    // Penalize generated or vendor files
    if file.relative_path.contains("vendor/") ||
       file.relative_path.contains("generated/") ||
       file.relative_path.contains(".min.") {
        score *= 0.3;
    }
    
    score.min(1.0) // Cap at 1.0
}

/// Compress project structure for efficient token usage with improved abbreviations
pub fn compress_structure(
    structure: &crate::analysis::project::ProjectStructure,
    metrics: &ProjectMetrics,
) -> CompressedStructure {
    // Use abbreviated directory names
    let abbreviations: HashMap<&str, &str> = [
        ("src", "s"),
        ("tests", "t"),
        ("docs", "d"),
        ("node_modules", "nm"),
        ("target", "tg"),
        ("bin", "b"),
        ("lib", "l"),
        ("examples", "ex"),
        ("fixtures", "fx"),
        ("components", "c"),
    ].iter().cloned().collect();
    
    // Build compressed tree using abbreviations
    let mut tree_parts = Vec::new();
    
    // Group files by directory and abbreviate extensions
    let mut dir_files: HashMap<String, Vec<String>> = HashMap::new();
    for file in &structure.files {
        let parts: Vec<&str> = file.relative_path.split('/').collect();
        if parts.len() > 1 {
            let dir = parts[..parts.len()-1].iter()
                .map(|d| abbreviations.get(d as &&str).unwrap_or(d))
                .cloned()
                .collect::<Vec<_>>()
                .join("/");
            
            // Abbreviate common file extensions
            let filename = parts[parts.len()-1];
            let short_name = if filename.ends_with(".js") {
                filename.trim_end_matches(".js").to_string() + ":j"
            } else if filename.ends_with(".rs") {
                filename.trim_end_matches(".rs").to_string() + ":r"
            } else if filename.ends_with(".py") {
                filename.trim_end_matches(".py").to_string() + ":p"
            } else if filename.ends_with(".ts") {
                filename.trim_end_matches(".ts").to_string() + ":t"
            } else if filename.ends_with(".json") {
                filename.trim_end_matches(".json").to_string() + ":jn"
            } else {
                filename.to_string()
            };
            
            dir_files.entry(dir).or_insert_with(Vec::new).push(short_name);
        } else {
            dir_files.entry(String::new()).or_insert_with(Vec::new).push(file.relative_path.clone());
        }
    }
    
    // Build compressed representation with sorted directories
    let mut sorted_dirs: Vec<_> = dir_files.iter().collect();
    sorted_dirs.sort_by_key(|(dir, _)| dir.as_str());
    
    for (dir, files) in sorted_dirs {
        if dir.is_empty() {
            tree_parts.push(files.join(","));
        } else {
            tree_parts.push(format!("{}[{}]", dir, files.join(",")));
        }
    }
    
    let tree = tree_parts.join(";");
    
    // Ultra-compress metrics using abbreviations
    let lang_abbrev: HashMap<&str, &str> = [
        ("rs", "r"),
        ("js", "j"),
        ("py", "p"),
        ("ts", "t"),
        ("java", "jv"),
        ("cpp", "c+"),
        ("go", "g"),
        ("rb", "rb"),
    ].iter().cloned().collect();
    
    let metrics_str = format!(
        "L{},{}",
        metrics.total_lines_of_code,
        metrics.code_by_language.iter()
            .map(|(lang, stats)| {
                let short = lang_abbrev.get(lang.as_str())
                    .map(|s| *s)
                    .unwrap_or(lang.as_str());
                format!("{}:{}/{}/{:.1}/{:.1}", 
                    short, 
                    stats.lines_of_code, 
                    stats.file_count,
                    stats.average_cyclomatic,
                    stats.average_cognitive
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    );
    
    // Enhanced quality metrics with complexity distribution
    let quality_str = format!(
        "Q{:.0}/{:.0}/{:.0}",
        metrics.project_complexity_score * 10.0,
        metrics.test_coverage_estimate * 100.0,
        metrics.documentation_ratio * 100.0
    );
    
    // Complexity metrics in ultra-compressed format
    let complexity_str = format!(
        "C{:.1}/{:.1}/{}/{}/{}+{}+{}+{}",
        metrics.average_cyclomatic_complexity,
        metrics.average_cognitive_complexity,
        metrics.max_cyclomatic_complexity,
        metrics.max_cognitive_complexity,
        metrics.complexity_distribution.low_complexity,
        metrics.complexity_distribution.medium_complexity,
        metrics.complexity_distribution.high_complexity,
        metrics.complexity_distribution.extreme_complexity
    );
    
    // Get top 5 important files (abbreviated)
    let mut important: Vec<_> = metrics.file_importance_scores.iter()
        .map(|(path, score)| (path.clone(), *score))
        .collect();
    important.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    let important_files: Vec<String> = important.iter()
        .take(5)
        .map(|(path, _)| {
            // Abbreviate path
            path.split('/').last().unwrap_or(path).to_string()
        })
        .collect();
    
    // More accurate token estimate with complexity data
    let full_output = format!("{};{};{};{}", tree, metrics_str, quality_str, complexity_str);
    let token_estimate = (full_output.len() + 2) / 3; // Better approximation
    
    CompressedStructure {
        format_version: 3, // Version 3 with complexity metrics
        tree,
        metrics: format!("{};{};{}", metrics_str, quality_str, complexity_str),
        important_files,
        token_estimate,
    }
}

/// Build incremental update from cache
pub fn build_incremental_update(
    cache: &ProjectCache,
    changed_files: Vec<PathBuf>,
) -> Result<String> {
    let mut updates = Vec::new();
    
    for file_path in changed_files {
        if let Ok(metadata) = fs::metadata(&file_path) {
            let relative = file_path.strip_prefix(&cache.structure.root_path)
                .unwrap_or(&file_path)
                .to_string_lossy()
                .replace('\\', "/");
            
            updates.push(format!(
                "MOD:{}:{}b",
                relative,
                metadata.len()
            ));
        }
    }
    
    Ok(format!("INCREMENTAL[{}]", updates.join(",")))
}