use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ConflictType {
    ExactDuplicate,  // Same content, different names
    SimilarName,     // test.js, test2.js, test_old.js
    BackupFile,      // .bak, .old, .backup, ~
    VersionConflict, // v1, v2, _new, _old
    TempFile,        // .tmp, .temp, .swp
}

pub struct DuplicateDetector {
    files: Vec<FileInfo>,
}

impl Default for DuplicateDetector {
    fn default() -> Self {
        Self::new()
    }
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
        if depth > 10 {
            return Ok(());
        } // Prevent infinite recursion

        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Skip common non-source directories
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.')
                    || name_str == "node_modules"
                    || name_str == "target"
                    || name_str == "dist"
                    || name_str == "build"
                {
                    continue;
                }
            }

            if path.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.len() > 0 && metadata.len() < 10_000_000 {
                        // Skip huge files
                        if let Ok(content) = fs::read(&path) {
                            let hash = format!("{:x}", Sha256::digest(&content));

                            // Correct line counting - convert to string and count lines properly
                            let lines = if let Ok(content_str) = std::str::from_utf8(&content) {
                                if content_str.is_empty() {
                                    0
                                } else {
                                    content_str.lines().count()
                                }
                            } else {
                                // Fallback for binary files - count \n bytes
                                content.iter().filter(|&&b| b == b'\n').count()
                            };

                            // Better error handling for modified time
                            let modified = metadata.modified().unwrap_or_else(|e| {
                                tracing::warn!(path=%path.display(), error=%e, "Failed to get modified time");
                                std::time::UNIX_EPOCH
                            });

                            self.files.push(FileInfo {
                                path: path.clone(),
                                size: metadata.len(),
                                hash,
                                lines,
                                modified,
                            });
                        } else {
                            tracing::warn!(path=%path.display(), "Failed to read file content");
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
            hash_map.entry(file.hash.clone()).or_default().push(file);
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

                // Skip standard Rust module files - these are expected to exist in multiple directories
                if Self::is_standard_filename(&stem_str) {
                    continue;
                }

                let clean_stem = Self::clean_filename(&stem_str);

                // Include parent directory in the grouping key to avoid false positives
                // for files with same name in different directories serving different purposes
                let parent_dir = file
                    .path
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|p| p.to_string_lossy().to_lowercase())
                    .unwrap_or_else(|| "root".to_string());

                let grouping_key = format!("{parent_dir}::{clean_stem}");

                name_groups.entry(grouping_key).or_default().push(file);
            }
        }

        for (pattern, files) in name_groups {
            if files.len() > 1 {
                // Check if they're actually different versions
                let unique_hashes: std::collections::HashSet<_> = files.iter().map(|f| &f.hash).collect();

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

        // Deterministic ordering: by conflict type priority, then pattern asc
        fn prio(t: &ConflictType) -> u8 {
            match t {
                ConflictType::ExactDuplicate => 0,
                ConflictType::VersionConflict => 1,
                ConflictType::BackupFile => 2,
                ConflictType::TempFile => 3,
                ConflictType::SimilarName => 4,
            }
        }

        let sum = |g: &DuplicateGroup| -> u64 { g.files.iter().map(|f| f.size).sum() };

        groups.sort_by(|a, b| {
            prio(&a.conflict_type)
                .cmp(&prio(&b.conflict_type))
                .then_with(|| sum(b).cmp(&sum(a))) // larger groups first within same type
                .then_with(|| a.pattern.cmp(&b.pattern))
        });
        groups
    }

    /// Check if a filename is a standard file that's expected to exist in multiple directories
    fn is_standard_filename(name: &str) -> bool {
        matches!(
            name,
            "mod" |           // Rust module files
            "lib" |           // Library root files
            "main" |          // Main entry point files
            "index" |         // Index files (common in web projects)
            "readme" |        // README files
            "__init__" |      // Python package files
            "makefile" |      // Build files
            "dockerfile" |    // Docker files
            "package" |       // Package manifest files (package.json, etc.)
            "cargo" |         // Cargo manifest files
            "pyproject" |     // Python project files
            "setup" |         // Setup/configuration files
            "config" |        // Configuration files
            "test" |          // Generic test files
            "tests" |         // Test suite files
            "spec" // Specification files
        )
    }

    fn clean_filename(name: &str) -> String {
        // Only remove clear version/backup indicators, preserve meaningful name parts
        // Don't strip numbers or 'v' indiscriminately as they may be part of the actual name
        let cleaned = name
            .replace("_old", "")
            .replace("_new", "")
            .replace("_backup", "")
            .replace("_copy", "")
            .replace("_temp", "")
            .replace("_tmp", "")
            .replace(".backup", "")
            .replace(".old", "")
            .replace(".bak", "")
            .replace("~", "");

        // Only trim underscores/dashes if they're at edges
        cleaned.trim_matches('_').trim_matches('-').to_string()
    }

    fn detect_conflict_type(files: &[&FileInfo]) -> ConflictType {
        let names: Vec<String> = files
            .iter()
            .filter_map(|f| f.path.file_name())
            .map(|n| n.to_string_lossy().to_lowercase())
            .collect();

        // Check for backup patterns
        if names
            .iter()
            .any(|n| n.contains(".bak") || n.contains(".old") || n.contains("backup") || n.ends_with('~'))
        {
            return ConflictType::BackupFile;
        }

        // Check for temp files
        if names
            .iter()
            .any(|n| n.contains(".tmp") || n.contains(".temp") || n.contains(".swp"))
        {
            return ConflictType::TempFile;
        }

        // Check for version patterns
        if names
            .iter()
            .any(|n| n.contains("_v") || n.contains("_new") || n.contains("_old") || n.contains("copy"))
        {
            return ConflictType::VersionConflict;
        }

        ConflictType::SimilarName
    }

    pub fn format_report(&self, groups: &[DuplicateGroup]) -> String {
        if groups.is_empty() {
            return String::new();
        }

        // Soft caps to keep report compact
        let max_groups: usize = std::env::var("DUP_REPORT_MAX_GROUPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20)
            .clamp(1, 200);
        let max_files: usize = std::env::var("DUP_REPORT_MAX_FILES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10)
            .clamp(1, 200);

        let mut report = String::from("\n🔴 **КРИТИЧНО: Обнаружены дубликаты/конфликты файлов**\n");

        // Summary per conflict type
        let mut counts: std::collections::HashMap<ConflictType, usize> = std::collections::HashMap::new();
        for g in groups {
            *counts.entry(g.conflict_type.clone()).or_insert(0) += 1;
        }
        report.push_str("Сводка по типам: ");
        let mut parts = Vec::new();
        let pushp = |t: ConflictType, name: &str, v: &mut Vec<String>| {
            if let Some(c) = counts.get(&t) {
                v.push(format!("{name}:{c}"));
            }
        };
        pushp(ConflictType::ExactDuplicate, "Exact", &mut parts);
        pushp(ConflictType::VersionConflict, "Version", &mut parts);
        pushp(ConflictType::BackupFile, "Backup", &mut parts);
        pushp(ConflictType::TempFile, "Temp", &mut parts);
        pushp(ConflictType::SimilarName, "Similar", &mut parts);
        report.push_str(&parts.join(", "));
        report.push('\n');
        // Grand totals
        let total_groups = groups.len();
        let mut total_files = 0usize;
        let mut total_bytes: u64 = 0;
        for g in groups {
            total_files += g.files.len();
            total_bytes += g.files.iter().map(|f| f.size).sum::<u64>();
        }
        let kb = (total_bytes as f64 / 1024.0).round() as u64;
        report.push_str(&format!("Итого: групп {total_groups} , файлов {total_files} (~{kb} KB)\n"));

        // Optional per-directory summary (top-K)
        let top_dirs: usize = std::env::var("DUP_REPORT_TOP_DIRS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3)
            .clamp(0, 20);
        if top_dirs > 0 {
            use std::collections::HashMap as Map;
            let mut dir_counts: Map<String, usize> = Map::new();
            for g in groups {
                for f in &g.files {
                    if let Some(parent) = f.path.parent().and_then(|p| p.to_str()) {
                        *dir_counts.entry(parent.to_string()).or_insert(0) += 1;
                    }
                }
            }
            let mut dir_list: Vec<(String, usize)> = dir_counts.into_iter().collect();
            dir_list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let mut parts = Vec::new();
            for (i, (d, c)) in dir_list.into_iter().enumerate() {
                if i >= top_dirs {
                    break;
                }
                parts.push(format!("{d}: {c}"));
            }
            if !parts.is_empty() {
                report.push_str("Топ директорий: ");
                report.push_str(&parts.join(", "));
                report.push('\n');
            }
        }

        let shown_groups = groups.iter().take(max_groups);
        let hidden_groups = groups.len().saturating_sub(max_groups);

        for group in shown_groups {
            let conflict_icon = match group.conflict_type {
                ConflictType::ExactDuplicate => "🔁",
                ConflictType::BackupFile => "💾",
                ConflictType::TempFile => "🗑️",
                ConflictType::VersionConflict => "⚠️",
                ConflictType::SimilarName => "📝",
            };

            report.push_str(&format!(
                "\n{} **{:?}** ({})\n",
                conflict_icon, group.conflict_type, group.pattern
            ));

            // Sort files by size (largest first) and modification time
            let mut sorted_files = group.files.clone();
            sorted_files.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| b.modified.cmp(&a.modified)));

            if sorted_files.len() > max_files {
                sorted_files.truncate(max_files);
            }
            let hidden_files = group.files.len().saturating_sub(max_files);

            for (i, file) in sorted_files.iter().enumerate() {
                let path_str = file.path.display().to_string();
                let relative_path = if let Ok(cwd) = std::env::current_dir() {
                    let cwd_s = cwd.display().to_string();
                    if path_str.starts_with(&cwd_s) {
                        let mut rel = path_str[cwd_s.len()..].to_string();
                        if rel.starts_with('/') || rel.starts_with('\\') {
                            let _ = rel.remove(0);
                        }
                        rel
                    } else {
                        path_str.clone()
                    }
                } else {
                    path_str.clone()
                };

                let is_likely_main = i == 0; // Largest and newest is likely the main one
                let marker = if is_likely_main {
                    "→ ОСНОВНОЙ"
                } else {
                    "  "
                };

                report.push_str(&format!("  {marker} {relative_path} | {size}B | {lines}L | {hash}\n", marker=marker, relative_path=relative_path, size=file.size, lines=file.lines, hash=&file.hash[..8]));
            }

            if hidden_files > 0 {
                report.push_str(&format!("  ... и ещё {hidden_files} файлов скрыто по лимиту\n"));
            }

            // Add recommendation
            if group.conflict_type == ConflictType::ExactDuplicate {
                report.push_str("  💡 Удалить дубликаты, оставить один файл\n");
            } else if group.conflict_type == ConflictType::BackupFile
                || group.conflict_type == ConflictType::TempFile
            {
                report.push_str("  💡 Удалить backup/temp файлы\n");
            } else {
                report.push_str("  💡 Объединить изменения в один файл\n");
            }
        }

        if hidden_groups > 0 {
            report.push_str(&format!("\n... и ещё {hidden_groups} групп скрыто по лимиту\n"));
        }

        report
    }
}

