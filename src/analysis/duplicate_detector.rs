use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub hash: String,
    pub lines: usize,
    pub modified: std::time::SystemTime,
}

#[derive(Debug)]
pub struct DuplicateGroup {
    pub pattern: String,
    pub files: Vec<FileInfo>,
    pub conflict_type: ConflictType,
}

#[derive(Debug, PartialEq)]
pub enum ConflictType {
    ExactDuplicate,      // Same content, different names
    SimilarName,         // test.js, test2.js, test_old.js
    BackupFile,          // .bak, .old, .backup, ~
    VersionConflict,     // v1, v2, _new, _old
    TempFile,            // .tmp, .temp, .swp
}

pub struct DuplicateDetector {
    files: Vec<FileInfo>,
}

impl DuplicateDetector {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn scan_directory(&mut self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        self.scan_recursive(dir, 0)?;
        Ok(())
    }

    fn scan_recursive(&mut self, dir: &Path, depth: usize) -> Result<(), Box<dyn std::error::Error>> {
        if depth > 10 { return Ok(()); } // Prevent infinite recursion
        
        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // Skip common non-source directories
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') || 
                   name_str == "node_modules" || 
                   name_str == "target" ||
                   name_str == "dist" ||
                   name_str == "build" {
                    continue;
                }
            }
            
            if path.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.len() > 0 && metadata.len() < 10_000_000 { // Skip huge files
                        if let Ok(content) = fs::read(&path) {
                            let hash = format!("{:x}", Sha256::digest(&content));
                            let lines = content.iter().filter(|&&b| b == b'\n').count() + 1;
                            
                            self.files.push(FileInfo {
                                path: path.clone(),
                                size: metadata.len(),
                                hash,
                                lines,
                                modified: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
                            });
                        }
                    }
                }
            } else if path.is_dir() {
                self.scan_recursive(&path, depth + 1)?;
            }
        }
        Ok(())
    }

    pub fn find_duplicates(&self) -> Vec<DuplicateGroup> {
        let mut groups = Vec::new();
        
        // 1. Find exact content duplicates
        let mut hash_map: HashMap<String, Vec<&FileInfo>> = HashMap::new();
        for file in &self.files {
            hash_map.entry(file.hash.clone()).or_insert_with(Vec::new).push(file);
        }
        
        for (hash, files) in hash_map {
            if files.len() > 1 {
                groups.push(DuplicateGroup {
                    pattern: format!("Content hash: {}", &hash[..8]),
                    files: files.into_iter().cloned().collect(),
                    conflict_type: ConflictType::ExactDuplicate,
                });
            }
        }
        
        // 2. Find similar named files (potential versions)
        let mut name_groups: HashMap<String, Vec<&FileInfo>> = HashMap::new();
        for file in &self.files {
            if let Some(stem) = file.path.file_stem() {
                let stem_str = stem.to_string_lossy().to_lowercase();
                let clean_stem = Self::clean_filename(&stem_str);
                name_groups.entry(clean_stem).or_insert_with(Vec::new).push(file);
            }
        }
        
        for (pattern, files) in name_groups {
            if files.len() > 1 {
                // Check if they're actually different versions
                let unique_hashes: std::collections::HashSet<_> = 
                    files.iter().map(|f| &f.hash).collect();
                
                if unique_hashes.len() > 1 {
                    let conflict_type = Self::detect_conflict_type(&files);
                    groups.push(DuplicateGroup {
                        pattern: pattern.clone(),
                        files: files.into_iter().cloned().collect(),
                        conflict_type,
                    });
                }
            }
        }
        
        groups
    }

    fn clean_filename(name: &str) -> String {
        // Remove common version indicators
        name.replace("_old", "")
            .replace("_new", "")
            .replace("_backup", "")
            .replace("_copy", "")
            .replace("_temp", "")
            .replace("_tmp", "")
            .replace(".backup", "")
            .replace(".old", "")
            .replace(".bak", "")
            .replace("~", "")
            .replace(char::is_numeric, "")
            .replace("v", "")
            .trim_matches('_')
            .trim_matches('-')
            .to_string()
    }

    fn detect_conflict_type(files: &[&FileInfo]) -> ConflictType {
        let names: Vec<String> = files.iter()
            .filter_map(|f| f.path.file_name())
            .map(|n| n.to_string_lossy().to_lowercase())
            .collect();
        
        // Check for backup patterns
        if names.iter().any(|n| n.contains(".bak") || n.contains(".old") || 
                                 n.contains("backup") || n.ends_with('~')) {
            return ConflictType::BackupFile;
        }
        
        // Check for temp files
        if names.iter().any(|n| n.contains(".tmp") || n.contains(".temp") || 
                                 n.contains(".swp")) {
            return ConflictType::TempFile;
        }
        
        // Check for version patterns
        if names.iter().any(|n| n.contains("_v") || n.contains("_new") || 
                                 n.contains("_old") || n.contains("copy")) {
            return ConflictType::VersionConflict;
        }
        
        ConflictType::SimilarName
    }

    pub fn format_report(&self, groups: &[DuplicateGroup]) -> String {
        if groups.is_empty() {
            return String::new();
        }
        
        let mut report = String::from("\n🔴 **КРИТИЧНО: Обнаружены дубликаты/конфликты файлов**\n");
        
        for group in groups {
            let conflict_icon = match group.conflict_type {
                ConflictType::ExactDuplicate => "🔁",
                ConflictType::BackupFile => "💾",
                ConflictType::TempFile => "🗑️",
                ConflictType::VersionConflict => "⚠️",
                ConflictType::SimilarName => "📝",
            };
            
            report.push_str(&format!("\n{} **{:?}** ({})\n", 
                conflict_icon, group.conflict_type, group.pattern));
            
            // Sort files by size (largest first) and modification time
            let mut sorted_files = group.files.clone();
            sorted_files.sort_by(|a, b| {
                b.size.cmp(&a.size)
                    .then_with(|| b.modified.cmp(&a.modified))
            });
            
            for (i, file) in sorted_files.iter().enumerate() {
                let path_str = file.path.display().to_string();
                let relative_path = path_str.split("ValidationCodeHook").last()
                    .or_else(|| path_str.split("GitHub").last())
                    .unwrap_or(&path_str);
                
                let is_likely_main = i == 0; // Largest and newest is likely the main one
                let marker = if is_likely_main { "→ ОСНОВНОЙ" } else { "  " };
                
                report.push_str(&format!(
                    "  {} {} | {}B | {}L | {}\n",
                    marker,
                    relative_path,
                    file.size,
                    file.lines,
                    &file.hash[..8]
                ));
            }
            
            // Add recommendation
            if group.conflict_type == ConflictType::ExactDuplicate {
                report.push_str("  💡 Удалить дубликаты, оставить один файл\n");
            } else if group.conflict_type == ConflictType::BackupFile || 
                      group.conflict_type == ConflictType::TempFile {
                report.push_str("  💡 Удалить backup/temp файлы\n");
            } else {
                report.push_str("  💡 Объединить изменения в один файл\n");
            }
        }
        
        report
    }
}