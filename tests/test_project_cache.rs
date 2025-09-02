use rust_validation_hooks::cache::project::*;
use rust_validation_hooks::analysis::project::*;
use std::fs;
use tempfile::TempDir;
use std::collections::HashMap;

// Helper function to create test metrics with all required fields
fn create_test_metrics() -> ProjectMetrics {
    ProjectMetrics {
        total_lines_of_code: 100,
        code_by_language: HashMap::new(),
        file_importance_scores: HashMap::new(),
        project_complexity_score: 5.0,
        test_coverage_estimate: 0.5,
        documentation_ratio: 0.2,
        average_cyclomatic_complexity: 3.5,
        average_cognitive_complexity: 4.2,
        max_cyclomatic_complexity: 12,
        max_cognitive_complexity: 15,
        high_complexity_files: 2,
        complexity_distribution: ComplexityDistribution {
            low_complexity: 8,
            medium_complexity: 5,
            high_complexity: 2,
            extreme_complexity: 0,
        },
    }
}

// Helper function to create test language stats
fn create_test_language_stats() -> LanguageStats {
    LanguageStats {
        file_count: 10,
        lines_of_code: 500,
        lines_of_comments: 100,
        blank_lines: 50,
        average_file_size: 50,
        complexity_estimate: 3.5,
        average_cyclomatic: 4.2,
        average_cognitive: 5.1,
        max_cyclomatic: 12,
        max_cognitive: 15,
        total_functions: 25,
        average_nesting_depth: 2.3,
    }
}

#[test]
fn test_count_lines_of_code_rust() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    
    fs::write(&file_path, r#"
// This is a comment
/// This is a doc comment
fn main() {
    /* Multi-line
       comment */
    println!("Hello, world!"); // inline comment
    
    let x = 42;
}
"#).unwrap();
    
    let (loc, comments, blanks) = count_lines_of_code(&file_path).unwrap();
    assert_eq!(loc, 4); // fn main, println, let x, closing brace
    assert_eq!(comments, 4); // single, doc, multi-line (2 lines)
    assert_eq!(blanks, 2); // two empty lines
}

#[test]
fn test_count_lines_of_code_javascript() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.js");
    
    fs::write(&file_path, r#"
// Single line comment
function test() {
    /* Multi-line
       comment */
    console.log("test");
    /** JSDoc comment
     * @param {string} x
     */
    return 42;
}
"#).unwrap();
    
    let (loc, comments, blanks) = count_lines_of_code(&file_path).unwrap();
    assert_eq!(loc, 4); // function, console.log, return, closing brace
    assert_eq!(comments, 5); // single + multi (2) + jsdoc (3)
    assert_eq!(blanks, 1);
}

#[test]
fn test_count_lines_of_code_python() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");
    
    fs::write(&file_path, r#"
# This is a comment
def main():
    """
    Docstring
    """
    print("Hello")  # inline comment
    x = 42
    
    return x
"#).unwrap();
    
    let (loc, comments, blanks) = count_lines_of_code(&file_path).unwrap();
    assert_eq!(loc, 4); // def, print, x =, return
    assert_eq!(comments, 4); // comment + docstring (3 lines)
    assert_eq!(blanks, 2);
}

#[test]
fn test_file_importance_scoring() {
    use rust_validation_hooks::analysis::project::ProjectFile;
    
    let main_file = ProjectFile {
        path: "src/main.rs".to_string(),
        relative_path: "src/main.rs".to_string(),
        file_type: "rs".to_string(),
        size_bytes: 1000,
        is_code_file: true,
    };
    
    let test_file = ProjectFile {
        path: "tests/test.rs".to_string(),
        relative_path: "tests/test.rs".to_string(),
        file_type: "rs".to_string(),
        size_bytes: 500,
        is_code_file: true,
    };
    
    let doc_file = ProjectFile {
        path: "README.md".to_string(),
        relative_path: "README.md".to_string(),
        file_type: "md".to_string(),
        size_bytes: 2000,
        is_code_file: false,
    };
    
    let main_score = calculate_file_importance(&main_file);
    let test_score = calculate_file_importance(&test_file);
    let doc_score = calculate_file_importance(&doc_file);
    
    // Main file should have highest importance
    assert!(main_score > test_score);
    assert!(main_score > doc_score);
    
    // Test file should have moderate importance
    assert_eq!(test_score, 0.7);
    
    // README should have good importance
    assert_eq!(doc_score, 0.8);
}

#[test]
fn test_cache_loading_and_saving() {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("test_cache.json");
    
    // Create a test cache
    let cache = ProjectCache {
        structure: ProjectStructure {
            root_path: temp_dir.path().to_string_lossy().to_string(),
            files: vec![],
            directories: vec![],
            total_files: 0,
            scan_timestamp: "2024-01-01 00:00:00".to_string(),
        },
        metrics: create_test_metrics(),
        file_hashes: std::collections::HashMap::new(),
        cache_timestamp: chrono::Local::now().timestamp(),
        last_modified: std::time::SystemTime::now(),
    };
    
    // Save cache
    cache.save(&cache_path).unwrap();
    assert!(cache_path.exists());
    
    // Load cache
    let loaded = ProjectCache::load(&cache_path).unwrap();
    assert!(loaded.is_some());
    
    let loaded_cache = loaded.unwrap();
    assert_eq!(loaded_cache.metrics.total_lines_of_code, 100);
    assert_eq!(loaded_cache.metrics.project_complexity_score, 5.0);
}

#[test]
fn test_cache_expiration() {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("expired_cache.json");
    
    // Create an expired cache
    let mut cache = ProjectCache {
        structure: ProjectStructure {
            root_path: temp_dir.path().to_string_lossy().to_string(),
            files: vec![],
            directories: vec![],
            total_files: 0,
            scan_timestamp: "2024-01-01 00:00:00".to_string(),
        },
        metrics: create_test_metrics(),
        file_hashes: std::collections::HashMap::new(),
        cache_timestamp: chrono::Local::now().timestamp() - 7200, // 2 hours ago
        last_modified: std::time::SystemTime::now(),
    };
    
    cache.save(&cache_path).unwrap();
    
    // Try to load with 1 hour TTL - should fail
    let loaded = ProjectCache::load(&cache_path).unwrap();
    assert!(loaded.is_none());
    
    // Try to load with 3 hour TTL - should succeed
    let loaded = ProjectCache::load(&cache_path).unwrap();
    assert!(loaded.is_some());
}

#[test]
fn test_parallel_metrics_calculation() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("test{}.rs", i));
        fs::write(&file_path, format!("fn test{}() {{ println!(\"test\"); }}", i)).unwrap();
    }
    
    // Create test structure
    let mut files = vec![];
    for i in 0..10 {
        files.push(ProjectFile {
            path: temp_dir.path().join(format!("test{}.rs", i)).to_string_lossy().to_string(),
            relative_path: format!("test{}.rs", i),
            file_type: "rs".to_string(),
            size_bytes: 50,
            is_code_file: true,
        });
    }
    
    let structure = ProjectStructure {
        root_path: temp_dir.path().to_string_lossy().to_string(),
        files,
        directories: vec![],
        total_files: 10,
        scan_timestamp: "2024-01-01 00:00:00".to_string(),
    };
    
    // Calculate metrics (uses parallel processing)
    let metrics = calculate_project_metrics(&structure).unwrap();
    
    assert_eq!(metrics.total_lines_of_code, 10); // 1 line per file
    assert!(metrics.code_by_language.contains_key("rs"));
    assert_eq!(metrics.code_by_language["rs"].file_count, 10);
}

#[test]
fn test_compressed_structure_format() {
    let structure = ProjectStructure {
        root_path: "/test".to_string(),
        files: vec![
            ProjectFile {
                path: "/test/src/main.rs".to_string(),
                relative_path: "src/main.rs".to_string(),
                file_type: "rs".to_string(),
                size_bytes: 1000,
                is_code_file: true,
            },
            ProjectFile {
                path: "/test/src/lib.rs".to_string(),
                relative_path: "src/lib.rs".to_string(),
                file_type: "rs".to_string(),
                size_bytes: 2000,
                is_code_file: true,
            },
        ],
        directories: vec!["src".to_string()],
        total_files: 2,
        scan_timestamp: "2024-01-01 00:00:00".to_string(),
    };
    
    let mut lang_stats = std::collections::HashMap::new();
    lang_stats.insert("rs".to_string(), create_test_language_stats());
    
    let mut importance_scores = std::collections::HashMap::new();
    importance_scores.insert("src/main.rs".to_string(), 1.0);
    importance_scores.insert("src/lib.rs".to_string(), 0.9);
    
    let mut metrics = create_test_metrics();
    metrics.code_by_language = lang_stats;
    metrics.file_importance_scores = importance_scores;
    metrics.project_complexity_score = 3.0;
    
    let compressed = compress_structure(&structure, &metrics);
    
    assert_eq!(compressed.format_version, 3);
    assert!(compressed.tree.contains("src[main.rs,lib.rs]"));
    assert!(compressed.metrics.contains("LOC:100"));
    assert!(compressed.metrics.contains("rs:100"));
    assert_eq!(compressed.important_files.len(), 2);
    assert!(compressed.token_estimate > 0);
}