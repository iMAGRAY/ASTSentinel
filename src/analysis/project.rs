use crate::analysis::metrics::{
    calculate_complexity_score, calculate_js_complexity, calculate_rust_complexity,
};
use crate::cache::project::{
    build_incremental_update, calculate_file_importance, compress_structure, count_lines_of_code,
    ComplexityDistribution, FileHash, LanguageStats, ProjectCache, ProjectMetrics,
};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Project structure representation for AI context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub root_path: String,
    pub files: Vec<ProjectFile>,
    pub directories: Vec<String>,
    pub total_files: usize,
    pub scan_timestamp: String,
}

/// Individual file information
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub fn scan_project_structure(root_path: &str, config: Option<ScanConfig>) -> Result<ProjectStructure> {
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

/// Scan project with caching and metrics calculation
pub fn scan_project_with_cache(
    root_path: &str,
    cache_path: Option<&Path>,
    config: Option<ScanConfig>,
) -> Result<(ProjectStructure, ProjectMetrics, Option<String>)> {
    let default_cache = PathBuf::from(".claude_project_cache.json");
    let cache_file = cache_path.unwrap_or(&default_cache);

    // Try to load cache
    let mut incremental_update = None;
    let (structure, from_cache) = if let Ok(Some(cache)) = ProjectCache::load(cache_file) {
        // Check for changes
        let changed_files = cache.get_changed_files(Path::new(root_path));

        if changed_files.is_empty() {
            // Use cached structure
            (cache.structure, true)
        } else if changed_files.len() < 10 {
            // Incremental update for small changes
            incremental_update = Some(build_incremental_update(&cache, changed_files)?);
            // Re-scan for now (could be optimized to only update changed files)
            (scan_project_structure(root_path, config)?, false)
        } else {
            // Too many changes, full re-scan
            (scan_project_structure(root_path, config)?, false)
        }
    } else {
        // No cache or expired, full scan
        (scan_project_structure(root_path, config)?, false)
    };

    // Calculate metrics if not from cache
    let metrics = if from_cache {
        // Load cached metrics
        if let Ok(Some(cache)) = ProjectCache::load(cache_file) {
            cache.metrics
        } else {
            calculate_project_metrics(&structure)?
        }
    } else {
        calculate_project_metrics(&structure)?
    };

    // Save to cache if we did a fresh scan
    if !from_cache {
        let file_hashes = build_file_hashes(&structure)?;
        let cache = ProjectCache {
            structure: structure.clone(),
            metrics: metrics.clone(),
            file_hashes,
            cache_timestamp: chrono::Local::now().timestamp(),
            last_modified: SystemTime::now(),
        };
        let _ = cache.save(cache_file); // Ignore save errors
    }

    Ok((structure, metrics, incremental_update))
}

/// Calculate comprehensive project metrics with AST-based complexity analysis
pub fn calculate_project_metrics(structure: &ProjectStructure) -> Result<ProjectMetrics> {
    // Use parallel processing for comprehensive file analysis
    let file_results: Vec<_> = structure
        .files
        .par_iter()
        .map(|file| {
            let importance = calculate_file_importance(file);
            let is_test = file.relative_path.contains("test") || file.relative_path.contains("spec");
            let is_doc = file.file_type == "md" || file.file_type == "rst";

            let mut complexity_metrics = None;
            let loc_stats = if file.is_code_file {
                let path = Path::new(&file.path);
                let loc = count_lines_of_code(path).ok();

                // Calculate complexity metrics based on file type
                let complexity = match file.file_type.as_str() {
                    "rs" => calculate_rust_complexity(path).ok(),
                    "js" | "ts" | "jsx" | "tsx" => {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            Some(calculate_js_complexity(&content))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                complexity_metrics = complexity;
                loc
            } else {
                None
            };

            (
                file.clone(),
                importance,
                is_test,
                is_doc,
                loc_stats,
                complexity_metrics,
            )
        })
        .collect();

    // Aggregate results with enhanced complexity tracking
    let mut total_loc = 0;
    let mut code_by_language: HashMap<String, LanguageStats> = HashMap::new();
    let mut file_importance_scores = HashMap::new();
    let mut test_files = 0;
    let mut doc_files = 0;

    // Complexity aggregation
    let mut all_complexity_scores = Vec::new();
    let mut total_cyclomatic = 0.0;
    let mut total_cognitive = 0.0;
    let mut max_cyclomatic = 0u32;
    let mut max_cognitive = 0u32;
    let mut high_complexity_files = 0;
    let mut complexity_count = 0;

    // Distribution counters
    let mut low_complexity = 0;
    let mut medium_complexity = 0;
    let mut high_complexity = 0;
    let mut extreme_complexity = 0;

    for (file, importance, is_test, is_doc, loc_stats, complexity_metrics) in file_results {
        file_importance_scores.insert(file.relative_path.clone(), importance);

        if is_test {
            test_files += 1;
        }
        if is_doc {
            doc_files += 1;
        }

        // Process complexity metrics
        if let Some(ref complexity) = complexity_metrics {
            let complexity_score = calculate_complexity_score(complexity);
            all_complexity_scores.push(complexity_score);

            total_cyclomatic += complexity.cyclomatic_complexity as f32;
            total_cognitive += complexity.cognitive_complexity as f32;
            max_cyclomatic = max_cyclomatic.max(complexity.cyclomatic_complexity);
            max_cognitive = max_cognitive.max(complexity.cognitive_complexity);
            complexity_count += 1;

            if complexity_score > 7.0 {
                high_complexity_files += 1;
            }

            // Complexity distribution
            match complexity_score {
                s if s <= 3.0 => low_complexity += 1,
                s if s <= 7.0 => medium_complexity += 1,
                s if s <= 10.0 => high_complexity += 1,
                _ => extreme_complexity += 1,
            }
        }

        if let Some((loc, comments, blanks)) = loc_stats {
            total_loc += loc;

            let stats = code_by_language
                .entry(file.file_type.clone())
                .or_insert(LanguageStats {
                    file_count: 0,
                    lines_of_code: 0,
                    lines_of_comments: 0,
                    blank_lines: 0,
                    average_file_size: 0,
                    complexity_estimate: 0.0,
                    // New complexity fields
                    average_cyclomatic: 0.0,
                    average_cognitive: 0.0,
                    max_cyclomatic: 0,
                    max_cognitive: 0,
                    total_functions: 0,
                    average_nesting_depth: 0.0,
                });

            stats.file_count += 1;
            stats.lines_of_code += loc;
            stats.lines_of_comments += comments;
            stats.blank_lines += blanks;

            // Add complexity data to language stats
            if let Some(ref complexity) = complexity_metrics {
                stats.average_cyclomatic += complexity.cyclomatic_complexity as f32;
                stats.average_cognitive += complexity.cognitive_complexity as f32;
                stats.max_cyclomatic = stats.max_cyclomatic.max(complexity.cyclomatic_complexity);
                stats.max_cognitive = stats.max_cognitive.max(complexity.cognitive_complexity);
                stats.total_functions += complexity.function_count;
                stats.average_nesting_depth += complexity.nesting_depth as f32;
            }
        }
    }

    // Calculate averages and finalize complexity metrics for each language
    for stats in code_by_language.values_mut() {
        if stats.file_count > 0 {
            stats.average_file_size = stats.lines_of_code / stats.file_count;
            stats.average_cyclomatic /= stats.file_count as f32;
            stats.average_cognitive /= stats.file_count as f32;
            stats.average_nesting_depth /= stats.file_count as f32;

            // Enhanced complexity estimate using AST metrics if available
            let comment_ratio = stats.lines_of_comments as f32 / (stats.lines_of_code as f32 + 1.0);
            let base_estimate = (stats.lines_of_code as f32 / 100.0) * (1.0 - comment_ratio.min(0.3));

            // Incorporate cyclomatic complexity if available
            if stats.average_cyclomatic > 0.0 {
                stats.complexity_estimate = (base_estimate * 0.3) + (stats.average_cyclomatic / 10.0 * 0.7);
            } else {
                stats.complexity_estimate = base_estimate;
            }
        }
    }

    // Calculate project-level scores
    let test_coverage_estimate = if structure.files.is_empty() {
        0.0
    } else {
        (test_files as f32 / structure.files.len() as f32).min(1.0)
    };

    let documentation_ratio = if structure.files.is_empty() {
        0.0
    } else {
        (doc_files as f32 / structure.files.len() as f32).min(1.0)
    };

    let project_complexity_score = if !all_complexity_scores.is_empty() {
        all_complexity_scores.iter().sum::<f32>() / all_complexity_scores.len() as f32
    } else {
        code_by_language
            .values()
            .map(|s| s.complexity_estimate)
            .sum::<f32>()
            / code_by_language.len().max(1) as f32
    };

    // Calculate average complexity metrics
    let average_cyclomatic_complexity = if complexity_count > 0 {
        total_cyclomatic / complexity_count as f32
    } else {
        0.0
    };

    let average_cognitive_complexity = if complexity_count > 0 {
        total_cognitive / complexity_count as f32
    } else {
        0.0
    };

    Ok(ProjectMetrics {
        total_lines_of_code: total_loc,
        code_by_language,
        file_importance_scores,
        project_complexity_score,
        test_coverage_estimate,
        documentation_ratio,
        // New complexity metrics
        average_cyclomatic_complexity,
        average_cognitive_complexity,
        max_cyclomatic_complexity: max_cyclomatic,
        max_cognitive_complexity: max_cognitive,
        high_complexity_files,
        complexity_distribution: ComplexityDistribution {
            low_complexity,
            medium_complexity,
            high_complexity,
            extreme_complexity,
        },
    })
}

/// Build file hashes for cache validation with content hashing for important files
fn build_file_hashes(structure: &ProjectStructure) -> Result<HashMap<String, FileHash>> {
    use std::io::Read;

    // Use parallel processing for hashing
    let hashes: Vec<_> = structure
        .files
        .par_iter()
        .filter_map(|file| {
            let path = Path::new(&file.path);
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    // Calculate content hash for important files (config, main files)
                    let content_hash = if file.relative_path == "Cargo.toml"
                        || file.relative_path == "package.json"
                        || file.relative_path.contains("main.")
                        || file.relative_path.contains("lib.")
                    {
                        if let Ok(mut file_handle) = fs::File::open(path) {
                            let mut hasher = Sha256::new();
                            let mut buffer = [0; 8192];
                            while let Ok(bytes_read) = file_handle.read(&mut buffer) {
                                if bytes_read == 0 {
                                    break;
                                }
                                hasher.update(&buffer[..bytes_read]);
                            }
                            Some(format!("{:x}", hasher.finalize()))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    Some((
                        file.relative_path.clone(),
                        FileHash {
                            path: file.relative_path.clone(),
                            modified_time: modified,
                            size: metadata.len(),
                            hash: content_hash,
                        },
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Ok(hashes.into_iter().collect())
}

/// Load ignore patterns from .gitignore and built-in excludes
fn load_ignore_patterns(root: &Path) -> Result<HashSet<String>> {
    let mut patterns = HashSet::new();

    // Built-in ignore patterns for common build/cache directories
    let builtin_patterns = [
        // Build outputs
        "target/",
        "build/",
        "dist/",
        "out/",
        "_build/",
        "bin/",
        "obj/",
        "coverage/",
        ".coverage/",
        ".next/",
        ".nuxt/",
        ".output/",
        ".vercel/",
        // Package managers
        "node_modules/",
        ".npm/",
        ".yarn/",
        ".pnpm/",
        ".cargo/",
        "vendor/",
        // Version control
        ".git/",
        ".svn/",
        ".hg/",
        ".bzr/",
        // IDEs and editors
        ".vscode/",
        ".idea/",
        "*.swp",
        "*.swo",
        "*~",
        ".DS_Store",
        "Thumbs.db",
        // Temporary files
        "tmp/",
        "temp/",
        ".tmp/",
        ".temp/",
        "*.tmp",
        "*.temp",
        "*.log",
        "*.bak",
        "*.backup",
        "*.cache",
        "*.pid",
        // Note: not filtering *.lock as package-lock.json and Cargo.lock are important

        // Language-specific
        "__pycache__/",
        "*.pyc",
        "*.pyo",
        ".pytest_cache/",
        "*.class",
        ".gradle/",
        ".maven/",
        ".nuget/",
        "packages/",
        // Compilation artifacts
        "*.o",
        "*.obj",
        "*.pdb",
        "*.exe",
        "*.dll",
        "*.so",
        "*.dylib",
        "*.a",
        "*.lib",
        "*.rlib",
        "*.rmeta",
        "*.wasm",
        "*.bc",
        // OS-specific
        ".Trash/",
        "$RECYCLE.BIN/",
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
                tracing::warn!(error=%e, "Could not read .gitignore");
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
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Check against all patterns
    for pattern in patterns {
        if pattern.ends_with('/') {
            // Directory pattern
            let dir_pattern = &pattern[..pattern.len() - 1];
            if path_str.starts_with(dir_pattern) || path_str.contains(&format!("/{dir_pattern}")) {
                return true;
            }
        } else if pattern.contains('*') {
            // Glob pattern (simple implementation)
            if matches_glob_pattern(&file_name, pattern) || matches_glob_pattern(&path_str, pattern) {
                return true;
            }
        } else {
            // Exact match
            if path_str == *pattern || file_name == *pattern || path_str.ends_with(&format!("/{}", pattern)) {
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
#[allow(clippy::too_many_arguments)]
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
            tracing::warn!(path=?current_path, error=%e, "Could not read directory");
            return Ok(());
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                tracing::warn!(error=%e, "Could not read directory entry");
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
            // Only include important files (code, configs, docs)
            if !is_important_file(&path) {
                continue;
            }

            if *file_count >= config.max_files {
                break;
            }

            // Add file to list
            if let Ok(relative_path) = path.strip_prefix(root_path) {
                let relative_path_str = relative_path.to_string_lossy().replace('\\', "/");
                let file_extension = path
                    .extension()
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

/// Determine if file should be included in project structure
fn is_important_file(path: &Path) -> bool {
    // Check extension first
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_str().unwrap_or("").to_lowercase();
        if is_code_file_extension(&ext_str) {
            return true;
        }
    }

    // Check for important files without extensions
    if let Some(file_name) = path.file_name() {
        let name = file_name.to_str().unwrap_or("").to_lowercase();
        matches!(
            name.as_str(),
            "dockerfile"
                | "makefile"
                | "rakefile"
                | "gemfile"
                | "procfile"
                | "guardfile"
                | "gruntfile"
                | "gulpfile"
                | ".gitignore"
                | ".dockerignore"
                | ".env"
                | ".env.example"
                | ".eslintrc"
                | ".prettierrc"
                | ".editorconfig"
                | "package-lock.json"
                | "yarn.lock"
                | "cargo.lock"
                | "pipfile"
                | "pipfile.lock"
                | "requirements.txt"
                | "go.mod"
                | "go.sum"
                | "cargo.toml"
                | "package.json"
        )
    } else {
        false
    }
}

/// Determine if file extension indicates a code file
fn is_code_file_extension(extension: &str) -> bool {
    matches!(
        extension,
        // Web technologies
        "js" | "jsx" | "ts" | "tsx" | "vue" | "svelte" |
        "html" | "htm" | "css" | "scss" | "sass" | "less" |
        // Programming languages
        "py" | "pyx" | "pyw" |
        "rs" |
        "java" | "kt" | "scala" |
        "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" |
        "cs" | "vb" | "fs" |
        "php" | "rb" | "go" | "swift" | "dart" |
        "pl" | "pm" | "r" | "julia" | "lua" |
        "clj" | "cljs" | "hs" | "elm" | "ml" |
        // Shell and scripting
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "cmd" | "bat" |
        // Configuration and data
        "json" | "yaml" | "yml" | "xml" | "ini" | "conf" | "toml" |
        "dockerfile" | "makefile" | "cmake" |
        "sql" | "graphql" | "proto" |
        // Documentation
        "md" | "rst" | "tex" | "adoc"
    )
}

/// Build machine-readable tree structure with clear hierarchy
fn build_machine_readable_tree(structure: &ProjectStructure) -> String {
    let mut output = String::new();
    let mut file_tree: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();

    // Group files by directory
    for file in &structure.files {
        // Split path into directory and filename
        let path = &file.relative_path;
        let (dir, filename) = if let Some(pos) = path.rfind(|c| ['/', '\\'].contains(&c)) {
            (&path[..pos], &path[pos + 1..])
        } else {
            (".", path.as_str())
        };

        file_tree
            .entry(dir.to_string())
            .or_default()
            .push(filename.to_string());
    }

    // Format as hierarchical structure
    for (dir, files) in file_tree.iter() {
        if dir == "." {
            // Root files
            for file in files {
                output.push_str(&format!("  /{}\n", file));
            }
        } else {
            // Directory and its files
            output.push_str(&format!("  /{}/\n", dir));
            for file in files {
                output.push_str(&format!("    {}\n", file));
            }
        }
    }

    output
}

/// Build complete project tree with files and directories
fn build_complete_project_tree(structure: &ProjectStructure) -> String {
    #[derive(Debug)]
    enum Node {
        File(String),
        Dir(String, Vec<Node>),
    }

    // Build tree from files and directories
    let mut root_nodes: Vec<Node> = Vec::new();

    // Process all files and build tree structure
    for file in &structure.files {
        let path_parts: Vec<&str> = file.relative_path.split(&['/', '\\'][..]).collect();
        insert_file_into_tree(&mut root_nodes, &path_parts);
    }

    // Format tree into compact string
    fn format_node(node: &Node) -> String {
        match node {
            Node::File(name) => name.clone(),
            Node::Dir(name, children) => {
                if children.is_empty() {
                    format!("{}[]", name)
                } else {
                    let children_str: Vec<String> = children.iter().map(format_node).collect();
                    format!("{}[{}]", name, children_str.join(","))
                }
            }
        }
    }

    // Insert file path into tree structure
    fn insert_file_into_tree(nodes: &mut Vec<Node>, path_parts: &[&str]) {
        if path_parts.is_empty() {
            return;
        }

        let name = path_parts[0].to_string();
        let remaining = &path_parts[1..];

        if remaining.is_empty() {
            // This is a file
            nodes.push(Node::File(name));
        } else {
            // This is a directory, find or create it
            let mut found = false;
            for node in nodes.iter_mut() {
                if let Node::Dir(dir_name, children) = node {
                    if *dir_name == name {
                        insert_file_into_tree(children, remaining);
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                let mut new_children = Vec::new();
                insert_file_into_tree(&mut new_children, remaining);
                nodes.push(Node::Dir(name, new_children));
            }
        }
    }

    let formatted_nodes: Vec<String> = root_nodes.iter().map(format_node).collect();

    formatted_nodes.join(",")
}

/// Build a nested tree structure from flat directory paths
/// Example: ["src", "src/bin", "tests", "tests/unit"] -> "src[bin],tests[unit]"
#[cfg(test)]
fn format_directory_tree(directories: &[String]) -> String {
    #[derive(Debug)]
    struct DirNode {
        name: String,
        children: Vec<DirNode>,
    }

    // Build tree structure from flat paths
    fn build_tree(paths: &[String]) -> Vec<DirNode> {
        let mut roots: Vec<DirNode> = Vec::new();

        // Sort paths to ensure parents come before children
        let mut sorted_paths = paths.to_vec();
        sorted_paths.sort();

        for path in sorted_paths {
            let parts: Vec<&str> = path.split('/').collect();
            insert_path(&mut roots, &parts);
        }

        roots
    }

    // Insert a path into the tree
    fn insert_path(nodes: &mut Vec<DirNode>, parts: &[&str]) {
        if parts.is_empty() {
            return;
        }

        let name = parts[0].to_string();
        let remaining = &parts[1..];

        // Find or create node
        let node = nodes.iter_mut().find(|n| n.name == name);

        if let Some(node) = node {
            if !remaining.is_empty() {
                insert_path(&mut node.children, remaining);
            }
        } else {
            let mut new_node = DirNode {
                name,
                children: Vec::new(),
            };

            if !remaining.is_empty() {
                insert_path(&mut new_node.children, remaining);
            }

            nodes.push(new_node);
        }
    }

    // Format tree to string recursively
    fn format_nodes(nodes: &[DirNode]) -> String {
        nodes
            .iter()
            .map(|node| {
                if node.children.is_empty() {
                    node.name.clone()
                } else {
                    format!("{}[{}]", node.name, format_nodes(&node.children))
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    let tree = build_tree(directories);
    format_nodes(&tree)
}

/// Format project structure with enhanced metrics and compression options
pub fn format_project_structure_for_ai_with_metrics(
    structure: &ProjectStructure,
    metrics: Option<&ProjectMetrics>,
    compress: bool,
) -> String {
    // Use compression if requested and metrics available
    if compress {
        if let Some(m) = metrics {
            let compressed = compress_structure(structure, m);
            return format!(
                "COMPRESSED_PROJECT[v{}][{}]\nMETRICS[{}]\nIMPORTANT[{}]\nTOKENS:{}",
                compressed.format_version,
                compressed.tree,
                compressed.metrics,
                compressed.important_files.join(","),
                compressed.token_estimate
            );
        }
    }

    // Standard format with enhanced metrics - machine-readable format
    let mut output = String::new();

    // Build machine-readable tree structure
    output.push_str("PROJECT_STRUCTURE:\n");
    let tree = build_machine_readable_tree(structure);
    output.push_str(&tree);

    // Add enhanced statistics if metrics available
    output.push_str("\nPROJECT_METRICS:\n");
    if let Some(metrics) = metrics {
        output.push_str(&format!(
            "  total_files: {}\n  total_dirs: {}\n  total_loc: {}\n",
            structure.total_files,
            structure.directories.len(),
            metrics.total_lines_of_code
        ));

        // Add language breakdown with LOC in machine-readable format
        if !metrics.code_by_language.is_empty() {
            output.push_str("  languages:\n");
            let mut sorted_langs: Vec<_> = metrics.code_by_language.iter().collect();
            sorted_langs.sort_by(|a, b| b.1.lines_of_code.cmp(&a.1.lines_of_code));

            for (lang, stats) in sorted_langs.iter() {
                output.push_str(&format!(
                    "    - {}: {} files, {} LOC\n",
                    lang, stats.file_count, stats.lines_of_code
                ));
            }
        }

        // Add project quality metrics
        output.push_str(&format!(
            "QUALITY: complexity:{:.1}, test_coverage:{:.0}%, docs:{:.0}%\n",
            metrics.project_complexity_score,
            metrics.test_coverage_estimate * 100.0,
            metrics.documentation_ratio * 100.0
        ));

        // Add top important files
        let mut important_files: Vec<_> = metrics.file_importance_scores.iter().collect();
        important_files.sort_by(|a, b| b.1.total_cmp(a.1));

        if !important_files.is_empty() {
            output.push_str("KEY_FILES:");
            for (i, (path, _score)) in important_files.iter().take(5).enumerate() {
                if i > 0 {
                    output.push(',');
                }
                output.push_str(path);
            }
            output.push('\n');
        }
    } else {
        // Fallback to basic stats
        output.push_str(&format!(
            "STATS: {} files, {} dirs\n",
            structure.total_files,
            structure.directories.len()
        ));
    }

    output
}

/// Format project structure as ultra-compact string for AI context (compatibility wrapper)
pub fn format_project_structure_for_ai(structure: &ProjectStructure, max_chars: usize) -> String {
    // For compatibility, calculate metrics on the fly if needed
    let metrics = calculate_project_metrics(structure).ok();

    // Use compression if output would be too large
    let compress = max_chars > 0 && max_chars < 2000;

    let _formatted = format_project_structure_for_ai_with_metrics(structure, metrics.as_ref(), compress);

    // Original compact format continues below for backwards compatibility
    let mut output = String::new();

    // Add summary statistics first so tests don't miss it on very long PROJECT trees
    if let Some(m) = metrics.as_ref() {
        output.push_str(&format!(
            "STATS: {} files, {} dirs, {} LOC\n",
            structure.total_files,
            structure.directories.len(),
            m.total_lines_of_code
        ));
    } else {
        output.push_str(&format!(
            "STATS: {} files, {} dirs\n",
            structure.total_files,
            structure.directories.len()
        ));
    }

    // Build complete tree structure with files and directories
    let tree = build_complete_project_tree(structure);
    output.push_str(&format!("PROJECT:[{}]\n", tree));

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
            if i > 0 {
                output.push(',');
            }
            output.push_str(&format!("{}:{}", ext, count));
        }
        output.push('\n');
    }

    // Key files - just names, no paths unless necessary
    let key_files = [
        "package.json",
        "cargo.toml",
        "requirements.txt",
        "go.mod",
        "dockerfile",
        "makefile",
        "readme.md",
        ".gitignore",
        "tsconfig.json",
        "pyproject.toml",
        "pom.xml",
    ];

    let mut found_keys = Vec::new();
    for file in &structure.files {
        let filename = file.relative_path.to_lowercase();
        for &pattern in &key_files {
            if filename.ends_with(pattern) || filename == pattern {
                // Just the filename if it's in root, otherwise minimal path
                let display_name = if file.relative_path.contains('/') || file.relative_path.contains('\\') {
                    // Include parent directory for context
                    let parts: Vec<&str> = file.relative_path.split(&['/', '\\'][..]).collect();
                    if parts.len() >= 2 {
                        format!("{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
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
        if found_keys.len() >= 8 {
            break;
        }
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
            if i > 0 {
                output.push(',');
            }
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
                    if sample_files.len() >= 30 {
                        break;
                    }
                }
            }
            if sample_files.len() >= 30 {
                break;
            }
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

    // Truncate only if max_chars is set (non-zero)
    if max_chars > 0 && output.len() > max_chars {
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
    fn test_format_directory_tree() {
        // Test basic nesting
        let dirs = vec![
            "src".to_string(),
            "src/bin".to_string(),
            "tests".to_string(),
            "tests/unit".to_string(),
            "tests/integration".to_string(),
        ];

        let result = format_directory_tree(&dirs);
        assert_eq!(result, "src[bin],tests[integration,unit]");

        // Test deeper nesting
        let dirs = vec![
            "src".to_string(),
            "src/bin".to_string(),
            "src/bin/tools".to_string(),
            "src/lib".to_string(),
            "src/lib/utils".to_string(),
            "src/lib/utils/helpers".to_string(),
        ];

        let result = format_directory_tree(&dirs);
        assert_eq!(result, "src[bin[tools],lib[utils[helpers]]]");

        // Test single directory
        let dirs = vec!["src".to_string()];
        let result = format_directory_tree(&dirs);
        assert_eq!(result, "src");

        // Test empty input
        let dirs: Vec<String> = vec![];
        let result = format_directory_tree(&dirs);
        assert_eq!(result, "");
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

        println!("Formatted output: {}", formatted);

        // Check that basic formatting structure works
        assert!(formatted.contains("PROJECT:")); // Project section exists
        assert!(formatted.contains("src")); // src directory is shown
        assert!(formatted.contains("lib.rs")); // lib.rs file is shown
        assert!(formatted.contains("bin[main.rs]")); // nested structure works
    }
}
