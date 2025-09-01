use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Project structure representation for AI context
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub root_path: String,
    pub files: Vec<ProjectFile>,
    pub directories: Vec<String>,
    pub total_files: usize,
    pub scan_timestamp: String,
}

/// Individual file information
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    pub path: String,
    pub relative_path: String,
    pub file_type: String,
    pub size_bytes: u64,
    pub is_code_file: bool,
}

/// Configuration for project scanning
pub struct ScanConfig {
    pub max_files: usize,
    pub max_depth: usize,
    pub include_hidden_files: bool,
    pub follow_symlinks: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_files: 1000,
            max_depth: 10,
            include_hidden_files: false,
            follow_symlinks: false,
        }
    }
}

/// Scan project directory and build structure representation
pub fn scan_project_structure(
    root_path: &str,
    config: Option<ScanConfig>,
) -> Result<ProjectStructure> {
    let config = config.unwrap_or_default();
    let root = Path::new(root_path);
    
    if !root.exists() {
        anyhow::bail!("Root path does not exist: {}", root_path);
    }
    
    if !root.is_dir() {
        anyhow::bail!("Root path is not a directory: {}", root_path);
    }

    // Load ignore patterns
    let ignore_patterns = load_ignore_patterns(root)?;
    
    // Scan directory recursively
    let mut files = Vec::new();
    let mut directories = Vec::new();
    let mut file_count = 0;
    
    scan_directory_recursive(
        root,
        root,
        &ignore_patterns,
        &config,
        &mut files,
        &mut directories,
        &mut file_count,
        0,
    )?;

    // Sort for consistent output
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    directories.sort();

    Ok(ProjectStructure {
        root_path: root_path.to_string(),
        files,
        directories,
        total_files: file_count,
        scan_timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    })
}

/// Load ignore patterns from .gitignore and built-in excludes
fn load_ignore_patterns(root: &Path) -> Result<HashSet<String>> {
    let mut patterns = HashSet::new();
    
    // Built-in ignore patterns for common build/cache directories
    let builtin_patterns = [
        // Build outputs
        "target/", "build/", "dist/", "out/", "_build/",
        "bin/", "obj/",
        
        // Package managers
        "node_modules/", ".npm/", ".yarn/", ".pnpm/",
        ".cargo/", "vendor/",
        
        // Version control
        ".git/", ".svn/", ".hg/", ".bzr/",
        
        // IDEs and editors
        ".vscode/", ".idea/", "*.swp", "*.swo", "*~",
        ".DS_Store", "Thumbs.db",
        
        // Temporary files
        "tmp/", "temp/", ".tmp/", ".temp/",
        "*.tmp", "*.temp", "*.log",
        
        // Language-specific
        "__pycache__/", "*.pyc", "*.pyo", ".pytest_cache/",
        "*.class", ".gradle/", ".maven/",
        ".nuget/", "packages/",
        
        // OS-specific
        ".Trash/", "$RECYCLE.BIN/",
    ];
    
    for pattern in builtin_patterns {
        patterns.insert(pattern.to_string());
    }
    
    // Load .gitignore if exists
    let gitignore_path = root.join(".gitignore");
    if gitignore_path.exists() {
        match fs::read_to_string(&gitignore_path) {
            Ok(content) => {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        patterns.insert(line.to_string());
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Could not read .gitignore: {}", e);
            }
        }
    }
    
    Ok(patterns)
}

/// Check if path should be ignored based on patterns
fn should_ignore(path: &Path, root: &Path, patterns: &HashSet<String>) -> bool {
    // Get relative path from root
    let relative_path = match path.strip_prefix(root) {
        Ok(rel) => rel,
        Err(_) => return true, // If we can't get relative path, ignore
    };
    
    let path_str = relative_path.to_string_lossy().replace('\\', "/");
    let file_name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    
    // Check against all patterns
    for pattern in patterns {
        if pattern.ends_with('/') {
            // Directory pattern
            let dir_pattern = &pattern[..pattern.len()-1];
            if path_str.starts_with(dir_pattern) || 
               path_str.contains(&format!("/{}", dir_pattern)) {
                return true;
            }
        } else if pattern.contains('*') {
            // Glob pattern (simple implementation)
            if matches_glob_pattern(&file_name, pattern) ||
               matches_glob_pattern(&path_str, pattern) {
                return true;
            }
        } else {
            // Exact match
            if path_str == *pattern || 
               file_name == *pattern ||
               path_str.ends_with(&format!("/{}", pattern)) {
                return true;
            }
        }
    }
    
    false
}

/// Simple glob pattern matching (supports * wildcard)
fn matches_glob_pattern(text: &str, pattern: &str) -> bool {
    if !pattern.contains('*') {
        return text == pattern;
    }
    
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.is_empty() {
        return true;
    }
    
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        
        if i == 0 {
            // First part must match from start
            if !text.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
            // Last part must match to end
            if !text[pos..].ends_with(part) {
                return false;
            }
        } else {
            // Middle part must exist somewhere after current position
            if let Some(found) = text[pos..].find(part) {
                pos += found + part.len();
            } else {
                return false;
            }
        }
    }
    
    true
}

/// Recursively scan directory
fn scan_directory_recursive(
    current_path: &Path,
    root_path: &Path,
    ignore_patterns: &HashSet<String>,
    config: &ScanConfig,
    files: &mut Vec<ProjectFile>,
    directories: &mut Vec<String>,
    file_count: &mut usize,
    depth: usize,
) -> Result<()> {
    if depth >= config.max_depth || *file_count >= config.max_files {
        return Ok(());
    }
    
    let entries = match fs::read_dir(current_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Warning: Could not read directory {:?}: {}", current_path, e);
            return Ok(());
        }
    };
    
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("Warning: Could not read directory entry: {}", e);
                continue;
            }
        };
        
        let path = entry.path();
        
        // Skip hidden files unless configured to include them
        if !config.include_hidden_files {
            if let Some(file_name) = path.file_name() {
                if file_name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }
        }
        
        // Check if should be ignored
        if should_ignore(&path, root_path, ignore_patterns) {
            continue;
        }
        
        // Handle symlinks
        let metadata = if config.follow_symlinks {
            match fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            }
        } else {
            match fs::symlink_metadata(&path) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() {
                        continue; // Skip symlinks
                    }
                    metadata
                }
                Err(_) => continue,
            }
        };
        
        if metadata.is_dir() {
            // Add directory to list
            if let Ok(relative_path) = path.strip_prefix(root_path) {
                let dir_path = relative_path.to_string_lossy().replace('\\', "/");
                directories.push(dir_path);
            }
            
            // Recurse into directory
            scan_directory_recursive(
                &path,
                root_path,
                ignore_patterns,
                config,
                files,
                directories,
                file_count,
                depth + 1,
            )?;
        } else if metadata.is_file() {
            if *file_count >= config.max_files {
                break;
            }
            
            // Add file to list
            if let Ok(relative_path) = path.strip_prefix(root_path) {
                let relative_path_str = relative_path.to_string_lossy().replace('\\', "/");
                let file_extension = path.extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                
                let is_code_file = is_code_file_extension(&file_extension);
                
                files.push(ProjectFile {
                    path: path.to_string_lossy().to_string(),
                    relative_path: relative_path_str,
                    file_type: if file_extension.is_empty() {
                        "none".to_string()
                    } else {
                        file_extension
                    },
                    size_bytes: metadata.len(),
                    is_code_file,
                });
                
                *file_count += 1;
            }
        }
    }
    
    Ok(())
}

/// Determine if file extension indicates a code file
fn is_code_file_extension(extension: &str) -> bool {
    matches!(extension,
        // Web technologies
        "js" | "jsx" | "ts" | "tsx" | "vue" | "svelte" |
        "html" | "htm" | "css" | "scss" | "sass" | "less" |
        
        // Programming languages
        "py" | "pyx" | "pyw" |
        "rs" | "toml" |
        "java" | "kt" | "scala" |
        "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" |
        "cs" | "vb" | "fs" |
        "php" | "rb" | "go" | "swift" | "dart" |
        "pl" | "pm" | "r" | "julia" | "lua" |
        "clj" | "cljs" | "hs" | "elm" | "ml" |
        
        // Shell and scripting
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "cmd" | "bat" |
        
        // Configuration and data
        "json" | "yaml" | "yml" | "xml" | "toml" | "ini" | "conf" |
        "dockerfile" | "makefile" | "cmake" |
        "sql" | "graphql" | "proto" |
        
        // Documentation
        "md" | "rst" | "tex" | "adoc"
    )
}

/// Format project structure as ultra-compact string for AI context
pub fn format_project_structure_for_ai(structure: &ProjectStructure, max_chars: usize) -> String {
    let mut output = String::new();
    
    // Ultra-compact header - just essential stats
    output.push_str(&format!("FILES:{} DIRS:{}\n", 
        structure.total_files,
        structure.directories.len()
    ));
    
    // Compact directory tree with grouped subdirectories to avoid repetition
    if !structure.directories.is_empty() {
        output.push_str("D:");
        
        // Sort directories to ensure parent dirs come before children
        let mut sorted_dirs = structure.directories.clone();
        sorted_dirs.sort();
        
        // Group directories by parent
        let mut dir_groups: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
        let mut root_dirs = Vec::new();
        
        for dir in &sorted_dirs {
            if let Some(slash_pos) = dir.rfind('/') {
                // Has parent directory
                let parent = &dir[..slash_pos];
                let child = &dir[slash_pos + 1..];
                dir_groups.entry(parent.to_string())
                    .or_insert_with(Vec::new)
                    .push(child.to_string());
            } else {
                // Root level directory
                root_dirs.push(dir.clone());
            }
        }
        
        // Format output with grouped subdirectories
        let mut formatted_dirs = Vec::new();
        let mut processed = std::collections::HashSet::new();
        
        for root_dir in &root_dirs {
            if let Some(children) = dir_groups.get(root_dir) {
                // Has subdirectories - format as "parent/[child1,child2,...]"
                formatted_dirs.push(format!("{}/[{}]", root_dir, children.join(",")));
                processed.insert(root_dir.clone());
                for child in children {
                    processed.insert(format!("{}/{}", root_dir, child));
                }
            } else {
                // No subdirectories - just the directory name
                formatted_dirs.push(root_dir.clone());
                processed.insert(root_dir.clone());
            }
        }
        
        // Add any remaining nested directories not yet processed
        for (parent, children) in &dir_groups {
            if !processed.contains(parent) {
                // This is a nested directory not at root level
                if children.len() > 1 {
                    formatted_dirs.push(format!("{}/[{}]", parent, children.join(",")));
                } else if children.len() == 1 {
                    formatted_dirs.push(format!("{}/{}", parent, children[0]));
                }
                for child in children {
                    processed.insert(format!("{}/{}", parent, child));
                }
            }
        }
        
        // Output formatted directories
        output.push_str(&formatted_dirs.join(","));
        
        if sorted_dirs.len() > processed.len() {
            output.push_str(&format!(",+{}", sorted_dirs.len() - processed.len()));
        }
        output.push('\n');
    }
    
    // File type statistics - ultra compact
    let mut file_types = std::collections::HashMap::new();
    for file in &structure.files {
        if file.is_code_file {
            *file_types.entry(&file.file_type).or_insert(0) += 1;
        }
    }
    
    if !file_types.is_empty() {
        output.push_str("T:");
        let mut sorted_types: Vec<_> = file_types.iter().collect();
        sorted_types.sort_by(|a, b| b.1.cmp(a.1));
        
        for (i, (ext, count)) in sorted_types.iter().enumerate() {
            if i > 0 { output.push(','); }
            output.push_str(&format!("{}:{}", ext, count));
        }
        output.push('\n');
    }
    
    // Key files - just names, no paths unless necessary
    let key_files = [
        "package.json", "cargo.toml", "requirements.txt", "go.mod",
        "dockerfile", "makefile", "readme.md", ".gitignore",
        "tsconfig.json", "pyproject.toml", "pom.xml"
    ];
    
    let mut found_keys = Vec::new();
    for file in &structure.files {
        let filename = file.relative_path.to_lowercase();
        for &pattern in &key_files {
            if filename.ends_with(pattern) || filename == pattern {
                // Just the filename if it's in root, otherwise minimal path
                let display_name = if file.relative_path.contains('/') || file.relative_path.contains('\\') {
                    // Include parent directory for context
                    let parts: Vec<&str> = file.relative_path.split(|c| c == '/' || c == '\\').collect();
                    if parts.len() >= 2 {
                        format!("{}/{}", parts[parts.len()-2], parts[parts.len()-1])
                    } else {
                        file.relative_path.clone()
                    }
                } else {
                    file.relative_path.clone()
                };
                found_keys.push(display_name);
                break;
            }
        }
        if found_keys.len() >= 8 { break; }
    }
    
    if !found_keys.is_empty() {
        output.push_str("K:");
        output.push_str(&found_keys.join(","));
        output.push('\n');
    }
    
    // All files list - ultra compact paths
    if structure.files.len() <= 50 {
        // If small project, list all files
        output.push_str("F:");
        for (i, file) in structure.files.iter().enumerate() {
            if i > 0 { output.push(','); }
            output.push_str(&file.relative_path);
        }
        output.push('\n');
    } else {
        // For large projects, sample important directories
        let mut sample_files = Vec::new();
        let important_dirs = ["src", "lib", "app", "components", "pages", "api", "core", "utils"];
        
        for dir in important_dirs {
            for file in &structure.files {
                if file.is_code_file && file.relative_path.starts_with(dir) {
                    sample_files.push(file.relative_path.clone());
                    if sample_files.len() >= 30 { break; }
                }
            }
            if sample_files.len() >= 30 { break; }
        }
        
        if !sample_files.is_empty() {
            output.push_str("F:");
            output.push_str(&sample_files.join(","));
            if structure.files.len() > sample_files.len() {
                output.push_str(&format!(",+{}", structure.files.len() - sample_files.len()));
            }
            output.push('\n');
        }
    }
    
    // Truncate if still too long
    if output.len() > max_chars {
        output.truncate(max_chars - 4);
        output.push_str("...");
    }
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;
    
    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let structure = scan_project_structure(temp_dir.path().to_str().unwrap(), None).unwrap();
        
        assert_eq!(structure.files.len(), 0);
        assert_eq!(structure.directories.len(), 0);
        assert_eq!(structure.total_files, 0);
    }
    
    #[test]
    fn test_scan_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create test files
        fs::write(root.join("test.js"), "console.log('hello');").unwrap();
        fs::write(root.join("readme.md"), "# Test Project").unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
        
        let structure = scan_project_structure(root.to_str().unwrap(), None).unwrap();
        
        assert_eq!(structure.total_files, 3);
        assert_eq!(structure.directories.len(), 1);
        assert_eq!(structure.files.len(), 3);
        
        // Check code file detection
        let code_files: Vec<_> = structure.files.iter().filter(|f| f.is_code_file).collect();
        assert_eq!(code_files.len(), 3); // js, md, rs are all code files
    }
    
    #[test]
    fn test_gitignore_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create .gitignore
        let mut gitignore = fs::File::create(root.join(".gitignore")).unwrap();
        writeln!(gitignore, "*.log").unwrap();
        writeln!(gitignore, "node_modules/").unwrap();
        writeln!(gitignore, "# Comment").unwrap();
        writeln!(gitignore, "secret.key").unwrap();
        
        // Create files (some should be ignored)
        fs::write(root.join("app.js"), "code").unwrap();
        fs::write(root.join("debug.log"), "log content").unwrap();
        fs::write(root.join("secret.key"), "secret").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules").join("package.json"), "{}").unwrap();
        
        let structure = scan_project_structure(root.to_str().unwrap(), None).unwrap();
        
        // Should only include app.js (others ignored by gitignore)
        assert_eq!(structure.total_files, 1);
        assert_eq!(structure.files[0].relative_path, "app.js");
    }
    
    #[test]
    fn test_glob_pattern_matching() {
        assert!(matches_glob_pattern("test.log", "*.log"));
        assert!(matches_glob_pattern("debug.log", "*.log"));
        assert!(!matches_glob_pattern("test.txt", "*.log"));
        
        assert!(matches_glob_pattern("test_file.tmp", "*_file.*"));
        assert!(!matches_glob_pattern("testfile.tmp", "*_file.*"));
    }
    
    #[test]
    fn test_format_with_nested_directories() {
        let structure = ProjectStructure {
            root_path: "/test/project".to_string(),
            files: vec![
                ProjectFile {
                    path: "/test/project/src/lib.rs".to_string(),
                    relative_path: "src/lib.rs".to_string(),
                    file_type: "rs".to_string(),
                    size_bytes: 1024,
                    is_code_file: true,
                },
                ProjectFile {
                    path: "/test/project/src/bin/main.rs".to_string(),
                    relative_path: "src/bin/main.rs".to_string(),
                    file_type: "rs".to_string(),
                    size_bytes: 2048,
                    is_code_file: true,
                },
            ],
            directories: vec![
                "src".to_string(),
                "src/bin".to_string(),
                "tests".to_string(),
                "tests/unit".to_string(),
                "tests/integration".to_string(),
            ],
            total_files: 2,
            scan_timestamp: chrono::Utc::now().to_rfc3339(),
        };
        
        let formatted = format_project_structure_for_ai(&structure, 500);
        
        // Check that nested directories are grouped to avoid repetition
        // Now format is: tests/[integration,unit] instead of tests/integration,tests/unit
        assert!(formatted.contains("D:"));
        assert!(formatted.contains("src/[bin]")); // bin grouped under src
        assert!(formatted.contains("tests/[integration,unit]")); // subdirs grouped under tests
    }
}