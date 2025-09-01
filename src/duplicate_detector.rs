/// Smart Duplicate File Detector - Prevents AI from creating unnecessary file copies
/// Blocks: demo, example, v2, v3, _new, _simple, _copy, _backup, _old, _temp etc.
use std::path::Path;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct DuplicateAnalysis {
    pub is_duplicate: bool,
    pub original_file: Option<String>,
    pub duplicate_type: DuplicateType,
    pub confidence: f32,
    pub reason: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DuplicateType {
    NotDuplicate,
    VersionVariant,     // _v2, _v3, _version2
    DemoExample,        // _demo, _example, _sample
    SimpleVariant,      // _simple, _basic, _minimal
    BackupCopy,         // _backup, _copy, _old, _bak
    TempFile,           // _temp, _tmp, _test
    NumericSuffix,      // file2.js, file3.js when file.js exists
    Experimental,       // _new, _updated, _improved
    Alternative,        // _alt, _alternative, _other
}

/// Check if the new file is likely a duplicate of an existing file
pub fn detect_duplicate(
    new_file_path: &str,
    existing_files: &[String],
    user_context: Option<&str>,
) -> DuplicateAnalysis {
    let new_path = Path::new(new_file_path);
    let new_filename = new_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    // Check if user explicitly requested a version/example/demo
    if let Some(context) = user_context {
        if is_explicit_duplicate_request(context) {
            return DuplicateAnalysis {
                is_duplicate: false,
                original_file: None,
                duplicate_type: DuplicateType::NotDuplicate,
                confidence: 0.0,
                reason: "User explicitly requested this file variant".to_string(),
                recommendation: "Allow - user requested".to_string(),
            };
        }
    }
    
    // Extract base name and extension
    let (base_name, extension) = split_filename(new_filename);
    
    // Check for duplicate patterns
    let duplicate_pattern = detect_duplicate_pattern(&base_name);
    
    if duplicate_pattern != DuplicateType::NotDuplicate {
        // Try to find the original file
        let original = find_original_file(
            &base_name,
            &extension,
            new_path.parent(),
            existing_files,
            &duplicate_pattern
        );
        
        if let Some(original_file) = original {
            return DuplicateAnalysis {
                is_duplicate: true,
                original_file: Some(original_file.clone()),
                duplicate_type: duplicate_pattern,
                confidence: 0.95,
                reason: format!("This appears to be a {} variant of '{}'", 
                    format_duplicate_type(&duplicate_pattern), original_file),
                recommendation: format!(
                    "REJECT: Edit '{}' instead of creating a duplicate. Use Edit/MultiEdit tool.",
                    original_file
                ),
            };
        }
    }
    
    // Check for numeric suffix pattern (file2.js when file.js exists)
    if let Some(original) = find_numeric_duplicate(&base_name, &extension, existing_files) {
        return DuplicateAnalysis {
            is_duplicate: true,
            original_file: Some(original.clone()),
            duplicate_type: DuplicateType::NumericSuffix,
            confidence: 0.9,
            reason: format!("Numeric suffix suggests this is a copy of '{}'", original),
            recommendation: format!(
                "REJECT: Edit '{}' instead of creating numbered copies.",
                original
            ),
        };
    }
    
    // Check for similar names in same directory
    if let Some(similar) = find_similar_file(new_filename, new_path.parent(), existing_files) {
        let similarity = calculate_similarity(new_filename, &similar);
        if similarity > 0.8 {
            return DuplicateAnalysis {
                is_duplicate: true,
                original_file: Some(similar.clone()),
                duplicate_type: DuplicateType::Alternative,
                confidence: similarity,
                reason: format!("Very similar name to existing file '{}'", similar),
                recommendation: format!(
                    "REJECT: Edit '{}' instead. The names are {}% similar.",
                    similar, (similarity * 100.0) as i32
                ),
            };
        }
    }
    
    // Not a duplicate
    DuplicateAnalysis {
        is_duplicate: false,
        original_file: None,
        duplicate_type: DuplicateType::NotDuplicate,
        confidence: 0.0,
        reason: "File appears to be unique".to_string(),
        recommendation: "Allow - not a duplicate".to_string(),
    }
}

/// Check if user explicitly requested a version/demo/example
fn is_explicit_duplicate_request(context: &str) -> bool {
    let context_lower = context.to_lowercase();
    
    // Explicit requests patterns
    let explicit_patterns = [
        "create.*demo",
        "create.*example",
        "make.*copy",
        "create.*version",
        "create.*v2",
        "create.*v3",
        "create.*backup",
        "create.*alternative",
        "save.*as.*new",
        "duplicate.*as",
        "create.*simple.*version",
        "create.*minimal.*example",
        "want.*separate.*file",
        "don't.*edit.*original",
        "keep.*original",
        "new.*file.*called",
    ];
    
    for pattern in &explicit_patterns {
        let regex = Regex::new(pattern).unwrap();
        if regex.is_match(&context_lower) {
            return true;
        }
    }
    
    false
}

/// Detect duplicate pattern in filename
fn detect_duplicate_pattern(base_name: &str) -> DuplicateType {
    let name_lower = base_name.to_lowercase();
    
    // Version patterns
    if name_lower.contains("_v2") || name_lower.contains("_v3") || 
       name_lower.contains("_version") || name_lower.contains("-v2") ||
       name_lower.contains("-v3") || name_lower.ends_with("v2") ||
       name_lower.ends_with("v3") {
        return DuplicateType::VersionVariant;
    }
    
    // Demo/Example patterns
    if name_lower.contains("demo") || name_lower.contains("example") ||
       name_lower.contains("sample") || name_lower.contains("test") {
        return DuplicateType::DemoExample;
    }
    
    // Simple/Basic patterns
    if name_lower.contains("simple") || name_lower.contains("basic") ||
       name_lower.contains("minimal") || name_lower.contains("lite") {
        return DuplicateType::SimpleVariant;
    }
    
    // Backup/Copy patterns
    if name_lower.contains("backup") || name_lower.contains("copy") ||
       name_lower.contains("_old") || name_lower.contains("_bak") ||
       name_lower.contains("_orig") || name_lower.ends_with(".bak") {
        return DuplicateType::BackupCopy;
    }
    
    // Temp patterns
    if name_lower.contains("temp") || name_lower.contains("tmp") ||
       name_lower.starts_with("~") || name_lower.starts_with(".~") {
        return DuplicateType::TempFile;
    }
    
    // New/Updated patterns
    if name_lower.contains("_new") || name_lower.contains("_updated") ||
       name_lower.contains("_improved") || name_lower.contains("_fixed") ||
       name_lower.contains("_refactored") {
        return DuplicateType::Experimental;
    }
    
    // Alternative patterns
    if name_lower.contains("_alt") || name_lower.contains("alternative") ||
       name_lower.contains("_other") || name_lower.contains("_variant") {
        return DuplicateType::Alternative;
    }
    
    DuplicateType::NotDuplicate
}

/// Helper to extract filename from path string
fn extract_filename(file_path: &str) -> Option<&str> {
    Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
}

/// Check if file is in the specified directory
fn is_file_in_directory(file_path: &str, directory: &Path) -> bool {
    let file_path = Path::new(file_path);
    
    // Try to canonicalize paths for accurate comparison
    // If canonicalization fails, fall back to string comparison
    let canonical_file = file_path.canonicalize().ok();
    let canonical_dir = directory.canonicalize().ok();
    
    // If we have canonical paths, use them for comparison
    if let (Some(can_file), Some(can_dir)) = (&canonical_file, &canonical_dir) {
        if let Some(file_parent) = can_file.parent() {
            // Check exact match or if file is under directory
            return file_parent == can_dir || file_parent.starts_with(can_dir);
        }
    }
    
    // Fallback to non-canonical comparison if canonicalization failed
    if let Some(file_parent) = file_path.parent() {
        // Normalize paths for comparison
        let file_parent_normalized = normalize_path_separators(file_parent);
        let dir_normalized = normalize_path_separators(directory);
        
        // Check exact match
        if file_parent_normalized == dir_normalized {
            return true;
        }
        
        // Check if file_parent is under directory (for nested paths)
        // Use normalized paths to handle different separators
        if file_parent_normalized.starts_with(&dir_normalized) {
            // Ensure it's a proper subdirectory (not just a prefix match)
            let remainder = &file_parent_normalized[dir_normalized.len()..];
            if remainder.is_empty() || remainder.starts_with('/') || remainder.starts_with('\\') {
                return true;
            }
        }
    }
    
    false
}

/// Normalize path separators to forward slashes for consistent comparison
fn normalize_path_separators(path: &Path) -> String {
    path.to_str()
        .unwrap_or("")
        .replace('\\', "/")
}

/// Find the original file that this might be a duplicate of
fn find_original_file(
    base_name: &str,
    extension: &str,
    directory: Option<&Path>,
    existing_files: &[String],
    duplicate_type: &DuplicateType,
) -> Option<String> {
    // Remove duplicate indicators to get probable original name
    let probable_original = remove_duplicate_indicators(base_name, duplicate_type);
    
    // Build expected original filename
    let original_with_ext = if !extension.is_empty() {
        format!("{}.{}", probable_original, extension)
    } else {
        probable_original.clone()
    };
    
    // Pre-process existing files to extract filenames once
    let files_with_names: Vec<(&String, Option<&str>)> = existing_files
        .iter()
        .map(|f| (f, extract_filename(f)))
        .collect();
    
    // Check in same directory first for exact match
    if let Some(dir) = directory {
        for (file_path, file_name_opt) in &files_with_names {
            if let Some(file_name) = file_name_opt {
                // Check for exact filename match (not just suffix)
                if *file_name == original_with_ext {
                    // Verify it's in the same directory using proper path comparison
                    if is_file_in_directory(file_path, dir) {
                        return Some((*file_path).clone());
                    }
                }
            }
        }
    }
    
    // Check globally if not found in same directory, but still require exact filename match
    // Sort by path depth to prefer files in shallower directories
    let mut exact_matches = Vec::new();
    for (file_path, file_name_opt) in &files_with_names {
        if let Some(file_name) = file_name_opt {
            // Check for exact filename match (not just suffix)
            if *file_name == original_with_ext {
                let depth = file_path.chars().filter(|&c| c == '/' || c == '\\').count();
                exact_matches.push(((*file_path).clone(), depth));
            }
        }
    }
    
    // Return the match with the shortest path (likely the most relevant)
    if !exact_matches.is_empty() {
        exact_matches.sort_by_key(|(_, depth)| *depth);
        return Some(exact_matches[0].0.clone());
    }
    
    // Fallback: check for similar names if exact match not found
    // This handles cases where the original might have slightly different naming
    let probable_lower = probable_original.to_lowercase();
    let mut candidates: Vec<(&String, usize)> = Vec::new();
    
    for (file_path, file_name_opt) in &files_with_names {
        if let Some(file_name) = file_name_opt {
            let file_base = file_name.split('.').next().unwrap_or("");
            
            // Check if the base names are very similar
            if file_base.to_lowercase() == probable_lower {
                // Calculate priority (shorter paths are preferred)
                let path_depth = file_path.matches('/').count() + file_path.matches('\\').count();
                candidates.push((file_path, path_depth));
            }
        }
    }
    
    // If multiple candidates found, choose the one with shortest path (likely the original)
    if !candidates.is_empty() {
        candidates.sort_by_key(|(_, depth)| *depth);
        return Some(candidates[0].0.clone());
    }
    
    None
}

/// Remove duplicate indicators from filename
fn remove_duplicate_indicators(name: &str, duplicate_type: &DuplicateType) -> String {
    let mut clean_name = name.to_string();
    
    match duplicate_type {
        DuplicateType::VersionVariant => {
            // Remove version suffixes
            clean_name = Regex::new(r"[_-]?v\d+$").unwrap()
                .replace(&clean_name, "").to_string();
            clean_name = clean_name.replace("_version2", "")
                .replace("_version3", "")
                .replace("-version2", "")
                .replace("-version3", "");
        },
        DuplicateType::DemoExample => {
            clean_name = clean_name.replace("_demo", "")
                .replace("-demo", "")
                .replace("demo_", "")
                .replace("_example", "")
                .replace("-example", "")
                .replace("example_", "")
                .replace("_sample", "")
                .replace("-sample", "");
        },
        DuplicateType::SimpleVariant => {
            clean_name = clean_name.replace("_simple", "")
                .replace("-simple", "")
                .replace("simple_", "")
                .replace("_basic", "")
                .replace("-basic", "")
                .replace("_minimal", "")
                .replace("-minimal", "");
        },
        DuplicateType::BackupCopy => {
            clean_name = clean_name.replace("_backup", "")
                .replace("-backup", "")
                .replace("_copy", "")
                .replace("-copy", "")
                .replace("_old", "")
                .replace("-old", "")
                .replace("_bak", "")
                .replace("_orig", "");
        },
        DuplicateType::Experimental => {
            clean_name = clean_name.replace("_new", "")
                .replace("-new", "")
                .replace("_updated", "")
                .replace("-updated", "")
                .replace("_improved", "")
                .replace("-improved", "")
                .replace("_fixed", "")
                .replace("_refactored", "");
        },
        DuplicateType::Alternative => {
            clean_name = clean_name.replace("_alt", "")
                .replace("-alt", "")
                .replace("_alternative", "")
                .replace("-alternative", "")
                .replace("_other", "")
                .replace("_variant", "");
        },
        _ => {}
    }
    
    clean_name
}

/// Find files with numeric suffix pattern
fn find_numeric_duplicate(
    base_name: &str,
    extension: &str,
    existing_files: &[String],
) -> Option<String> {
    // Check if name ends with a number
    let regex = Regex::new(r"^(.+?)(\d+)$").unwrap();
    
    if let Some(captures) = regex.captures(base_name) {
        let base = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let number = captures.get(2).map(|m| m.as_str()).unwrap_or("");
        
        // Only consider it duplicate if number > 1
        if let Ok(num) = number.parse::<i32>() {
            if num > 1 {
                // Look for the original without number or with lower number
                let original_name = if !extension.is_empty() {
                    format!("{}.{}", base, extension)
                } else {
                    base.to_string()
                };
                
                for file in existing_files {
                    if file.ends_with(&original_name) {
                        return Some(file.clone());
                    }
                }
                
                // Also check for file1 if we're file2+
                if num > 2 {
                    let prev_name = if !extension.is_empty() {
                        format!("{}1.{}", base, extension)
                    } else {
                        format!("{}1", base)
                    };
                    
                    for file in existing_files {
                        if file.ends_with(&prev_name) {
                            return Some(file.clone());
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Find similar files in the same directory
fn find_similar_file(
    new_filename: &str,
    directory: Option<&Path>,
    existing_files: &[String],
) -> Option<String> {
    let mut best_match = None;
    let mut best_similarity = 0.0;
    
    let dir_str = directory.and_then(|d| d.to_str()).unwrap_or("");
    
    for file in existing_files {
        // Only check files in same directory
        if !dir_str.is_empty() && !file.contains(dir_str) {
            continue;
        }
        
        if let Some(existing_filename) = Path::new(file).file_name().and_then(|n| n.to_str()) {
            let similarity = calculate_similarity(new_filename, existing_filename);
            
            if similarity > best_similarity && similarity > 0.7 {
                best_similarity = similarity;
                best_match = Some(file.clone());
            }
        }
    }
    
    best_match
}

/// Calculate similarity between two filenames (0.0 to 1.0)
fn calculate_similarity(name1: &str, name2: &str) -> f32 {
    if name1 == name2 {
        return 1.0;
    }
    
    let name1_lower = name1.to_lowercase();
    let name2_lower = name2.to_lowercase();
    
    // Use Levenshtein distance
    let distance = levenshtein_distance(&name1_lower, &name2_lower);
    let max_len = name1_lower.len().max(name2_lower.len()) as f32;
    
    if max_len == 0.0 {
        return 0.0;
    }
    
    1.0 - (distance as f32 / max_len)
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
    
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }
    
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
    
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }
    
    for (i, c1) in s1.chars().enumerate() {
        for (j, c2) in s2.chars().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }
    
    matrix[len1][len2]
}

/// Split filename into base name and extension
fn split_filename(filename: &str) -> (String, String) {
    if let Some(dot_pos) = filename.rfind('.') {
        if dot_pos > 0 {
            let base = filename[..dot_pos].to_string();
            let ext = filename[dot_pos + 1..].to_string();
            return (base, ext);
        }
    }
    (filename.to_string(), String::new())
}

/// Format duplicate type for user message
fn format_duplicate_type(dtype: &DuplicateType) -> &str {
    match dtype {
        DuplicateType::VersionVariant => "version",
        DuplicateType::DemoExample => "demo/example",
        DuplicateType::SimpleVariant => "simplified",
        DuplicateType::BackupCopy => "backup/copy",
        DuplicateType::TempFile => "temporary",
        DuplicateType::NumericSuffix => "numbered copy",
        DuplicateType::Experimental => "experimental",
        DuplicateType::Alternative => "alternative",
        DuplicateType::NotDuplicate => "unique",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_version_duplicate() {
        let existing = vec!["src/service.js".to_string()];
        let result = detect_duplicate("src/service_v2.js", &existing, None);
        assert!(result.is_duplicate);
        assert_eq!(result.duplicate_type, DuplicateType::VersionVariant);
    }
    
    #[test]
    fn test_detect_demo_duplicate() {
        let existing = vec!["app.py".to_string()];
        let result = detect_duplicate("app_demo.py", &existing, None);
        assert!(result.is_duplicate);
        assert_eq!(result.duplicate_type, DuplicateType::DemoExample);
    }
    
    #[test]
    fn test_detect_numeric_duplicate() {
        let existing = vec!["config.json".to_string()];
        let result = detect_duplicate("config2.json", &existing, None);
        assert!(result.is_duplicate);
        assert_eq!(result.duplicate_type, DuplicateType::NumericSuffix);
    }
    
    #[test]
    fn test_allow_explicit_request() {
        let existing = vec!["main.rs".to_string()];
        let context = "create a demo version called main_demo.rs";
        let result = detect_duplicate("main_demo.rs", &existing, Some(context));
        assert!(!result.is_duplicate);
    }
    
    #[test]
    fn test_similar_file_detection() {
        let existing = vec!["user_service.js".to_string()];
        let result = detect_duplicate("userservice.js", &existing, None);
        assert!(result.is_duplicate);
        assert_eq!(result.duplicate_type, DuplicateType::Alternative);
    }
}