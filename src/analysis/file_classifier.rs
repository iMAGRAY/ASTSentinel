use serde::{Deserialize, Serialize};
/// Smart file classifier for distinguishing test files from source code
/// Reduces false positives in metrics and issue detection
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileCategory {
    SourceCode {
        language: String,
        confidence: f32,
    },
    TestCode {
        framework: TestFramework,
        confidence: f32,
    },
    Vendor {
        package_manager: PackageManager,
        confidence: f32,
    },
    Backup {
        original: Option<String>,
        confidence: f32,
    },
    Generated {
        generator: Option<String>,
        confidence: f32,
    },
    Documentation {
        format: DocFormat,
        confidence: f32,
    },
    Configuration {
        config_type: String,
        confidence: f32,
    },
    Unknown {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestFramework {
    Jest,
    Pytest,
    RustTest,
    Go,
    JUnit,
    Mocha,
    RSpec,
    PHPUnit,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageManager {
    NPM,
    Cargo,
    Composer,
    Go,
    Python,
    Maven,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocFormat {
    Markdown,
    HTML,
    Text,
    Unknown(String),
}

pub struct FileClassifier {
    // Patterns for different file types - ordered for deterministic results
    test_patterns: Vec<TestPattern>,
    vendor_patterns: Vec<VendorPattern>,
    backup_patterns: Vec<BackupPattern>,
    generated_patterns: Vec<GeneratedPattern>,
}

#[derive(Debug, Clone)]
struct TestPattern {
    path_contains: Vec<&'static str>,
    filename_patterns: Vec<&'static str>,
    content_indicators: Vec<&'static str>,
    framework: TestFramework,
    weight: f32,
}

#[derive(Debug, Clone)]
struct VendorPattern {
    path_exact: Vec<&'static str>,
    path_contains: Vec<&'static str>,
    package_manager: PackageManager,
    weight: f32,
}

#[derive(Debug, Clone)]
struct BackupPattern {
    extensions: Vec<&'static str>,
    suffixes: Vec<&'static str>,
    patterns: Vec<&'static str>,
    weight: f32,
}

#[derive(Debug, Clone)]
struct GeneratedPattern {
    content_markers: Vec<&'static str>,
    filename_patterns: Vec<&'static str>,
    generator: Option<&'static str>,
    weight: f32,
}

impl FileClassifier {
    pub fn new() -> Self {
        Self {
            test_patterns: vec![
                TestPattern {
                    path_contains: vec!["/tests/", "\\tests\\", "/test/", "\\test\\", "/__tests__/"],
                    filename_patterns: vec!["test_", "_test.", ".test.", "test.py", "test.js", "test.rs"],
                    content_indicators: vec![
                        "#[test]",
                        "describe(",
                        "it(",
                        "def test_",
                        "class Test",
                        "@Test",
                    ],
                    framework: TestFramework::Unknown("generic".to_string()),
                    weight: 0.8,
                },
                TestPattern {
                    path_contains: vec!["/spec/", "\\spec\\"],
                    filename_patterns: vec!["_spec.", ".spec."],
                    content_indicators: vec!["describe ", "context ", "it "],
                    framework: TestFramework::RSpec,
                    weight: 0.9,
                },
                TestPattern {
                    path_contains: vec![],
                    filename_patterns: vec!["pytest_", "test_*.py"],
                    content_indicators: vec!["import pytest", "def test_", "@pytest."],
                    framework: TestFramework::Pytest,
                    weight: 0.85,
                },
                TestPattern {
                    path_contains: vec![],
                    filename_patterns: vec!["*.test.js", "*.test.ts", "*.spec.js"],
                    content_indicators: vec!["describe(", "test(", "expect(", "jest."],
                    framework: TestFramework::Jest,
                    weight: 0.85,
                },
            ],
            vendor_patterns: vec![
                VendorPattern {
                    path_exact: vec!["node_modules", "vendor", "target/debug", "target/release"],
                    path_contains: vec!["/node_modules/", "\\node_modules\\", "/vendor/", "\\vendor\\"],
                    package_manager: PackageManager::NPM,
                    weight: 1.0,
                },
                VendorPattern {
                    path_exact: vec!["target"],
                    path_contains: vec!["/target/", "\\target\\"],
                    package_manager: PackageManager::Cargo,
                    weight: 0.95,
                },
            ],
            backup_patterns: vec![BackupPattern {
                extensions: vec![
                    ".bak", ".backup", ".old", ".orig", ".tmp", ".temp", ".swp", ".swo",
                ],
                suffixes: vec!["~", ".autobak", ".safebak", ".prepatchbak", ".manualbak"],
                patterns: vec!["*_backup*", "*backup*", "*_old*", "*copy*"],
                weight: 0.95,
            }],
            generated_patterns: vec![GeneratedPattern {
                content_markers: vec![
                    "// This file is generated",
                    "# This file is generated",
                    "/* This file is generated",
                    "// Code generated by",
                    "# Generated by",
                    "@generated",
                ],
                filename_patterns: vec!["*.gen.*", "*_gen.*", "generated_*"],
                generator: None,
                weight: 0.9,
            }],
        }
    }

    /// Classify a file based on path and optionally content
    pub fn classify_file(&self, file_path: &Path, content: Option<&str>) -> FileCategory {
        let path_str = file_path.to_string_lossy().to_lowercase();
        let filename = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // Check backup files first (highest priority)
        if let Some(confidence) = self.check_backup_patterns(&path_str, &filename) {
            return FileCategory::Backup {
                original: self.guess_original_name(file_path),
                confidence,
            };
        }

        // Check vendor files
        if let Some((pm, confidence)) = self.check_vendor_patterns(&path_str) {
            return FileCategory::Vendor {
                package_manager: pm,
                confidence,
            };
        }

        // Check generated files
        if let Some((gen, confidence)) = self.check_generated_patterns(&path_str, &filename, content) {
            return FileCategory::Generated {
                generator: gen.map(|s| s.to_string()),
                confidence,
            };
        }

        // Check test files
        if let Some((framework, confidence)) = self.check_test_patterns(&path_str, &filename, content) {
            return FileCategory::TestCode {
                framework,
                confidence,
            };
        }

        // Check documentation
        if let Some((format, confidence)) = self.check_doc_patterns(&path_str, &filename) {
            return FileCategory::Documentation { format, confidence };
        }

        // Check configuration
        if let Some((config_type, confidence)) = self.check_config_patterns(&path_str, &filename) {
            return FileCategory::Configuration {
                config_type,
                confidence,
            };
        }

        // Default: assume source code
        if let Some(lang) = self.detect_language(file_path) {
            FileCategory::SourceCode {
                language: lang,
                confidence: 0.7,
            }
        } else {
            FileCategory::Unknown {
                reason: "Could not determine file type".to_string(),
            }
        }
    }

    /// Quick classification without reading file content
    pub fn classify_file_fast(&self, file_path: &Path) -> FileCategory {
        self.classify_file(file_path, None)
    }

    fn check_backup_patterns(&self, _path: &str, filename: &str) -> Option<f32> {
        let mut max_confidence: f32 = 0.0;

        for pattern in &self.backup_patterns {
            let mut confidence: f32 = 0.0;

            // Check extensions
            for ext in &pattern.extensions {
                if filename.ends_with(ext) {
                    confidence = confidence.max(pattern.weight);
                }
            }

            // Check suffixes
            for suffix in &pattern.suffixes {
                if filename.ends_with(suffix) {
                    confidence = confidence.max(pattern.weight);
                }
            }

            // Check patterns
            for pat in &pattern.patterns {
                let clean_pattern = pat.replace('*', "");
                if filename.contains(&clean_pattern) {
                    confidence = confidence.max(pattern.weight * 0.8);
                }
            }

            max_confidence = max_confidence.max(confidence);
        }

        if max_confidence > 0.5 {
            Some(max_confidence)
        } else {
            None
        }
    }

    fn check_vendor_patterns(&self, path: &str) -> Option<(PackageManager, f32)> {
        for pattern in &self.vendor_patterns {
            // Check exact paths
            for exact in &pattern.path_exact {
                if path.contains(exact)
                    && (path.ends_with(exact)
                        || path.contains(&format!("{}/", exact))
                        || path.contains(&format!("{}\\", exact)))
                {
                    return Some((pattern.package_manager.clone(), pattern.weight));
                }
            }

            // Check path contains
            for contains in &pattern.path_contains {
                if path.contains(contains) {
                    return Some((pattern.package_manager.clone(), pattern.weight * 0.9));
                }
            }
        }
        None
    }

    fn check_generated_patterns(
        &self,
        _path: &str,
        filename: &str,
        content: Option<&str>,
    ) -> Option<(Option<&'static str>, f32)> {
        for pattern in &self.generated_patterns {
            let mut confidence: f32 = 0.0;

            // Check content markers if available
            if let Some(content) = content {
                for marker in &pattern.content_markers {
                    if content.contains(marker) {
                        confidence = confidence.max(pattern.weight);
                    }
                }
            }

            // Check filename patterns
            for pat in &pattern.filename_patterns {
                let clean_pattern = pat.replace('*', "");
                if filename.contains(&clean_pattern) {
                    confidence = confidence.max(pattern.weight * 0.7);
                }
            }

            if confidence > 0.6 {
                return Some((pattern.generator, confidence));
            }
        }
        None
    }

    fn check_test_patterns(
        &self,
        path: &str,
        filename: &str,
        content: Option<&str>,
    ) -> Option<(TestFramework, f32)> {
        let mut best_match: Option<(TestFramework, f32)> = None;

        for pattern in &self.test_patterns {
            let mut confidence: f32 = 0.0;

            // Check path contains
            for path_part in &pattern.path_contains {
                if path.contains(path_part) {
                    confidence = confidence.max(pattern.weight);
                }
            }

            // Check filename patterns
            for file_pattern in &pattern.filename_patterns {
                if file_pattern.contains('*') {
                    let clean_pattern = file_pattern.replace('*', "");
                    if filename.contains(&clean_pattern) {
                        confidence = confidence.max(pattern.weight * 0.8);
                    }
                } else if filename.contains(file_pattern) {
                    confidence = confidence.max(pattern.weight * 0.9);
                }
            }

            // Check content indicators if available
            if let Some(content) = content {
                for indicator in &pattern.content_indicators {
                    if content.contains(indicator) {
                        confidence = confidence.max(pattern.weight * 0.95);
                    }
                }
            }

            // Update best match if this is better
            if confidence > 0.6
                && (best_match.is_none() || confidence > best_match.as_ref().map_or(0.0, |(_, c)| *c))
            {
                best_match = Some((pattern.framework.clone(), confidence));
            }
        }

        best_match
    }

    fn check_doc_patterns(&self, _path: &str, filename: &str) -> Option<(DocFormat, f32)> {
        if filename.ends_with(".md") || filename.ends_with(".markdown") {
            Some((DocFormat::Markdown, 0.9))
        } else if filename.ends_with(".html") || filename.ends_with(".htm") {
            Some((DocFormat::HTML, 0.9))
        } else if filename.ends_with(".txt") || filename.ends_with(".text") {
            Some((DocFormat::Text, 0.8))
        } else if filename == "readme" || filename.starts_with("readme.") {
            Some((DocFormat::Unknown("readme".to_string()), 0.85))
        } else {
            None
        }
    }

    fn check_config_patterns(&self, _path: &str, filename: &str) -> Option<(String, f32)> {
        let config_files = [
            ("package.json", 0.95),
            ("cargo.toml", 0.95),
            ("pyproject.toml", 0.95),
            (".gitignore", 0.9),
            (".env", 0.9),
            ("config.yaml", 0.85),
            ("config.yml", 0.85),
            ("config.json", 0.85),
            ("tsconfig.json", 0.9),
        ];

        for (config_name, confidence) in config_files {
            if filename == config_name {
                return Some((config_name.to_string(), confidence));
            }
        }

        // Check patterns
        if filename.ends_with(".toml") {
            Some(("toml_config".to_string(), 0.7))
        } else if filename.ends_with(".yaml") || filename.ends_with(".yml") {
            Some(("yaml_config".to_string(), 0.7))
        } else if filename.ends_with(".json") && !filename.contains("test") {
            Some(("json_config".to_string(), 0.6))
        } else {
            None
        }
    }

    fn detect_language(&self, file_path: &Path) -> Option<String> {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| {
                let lang = match ext.to_lowercase().as_str() {
                    "rs" => "rust",
                    "py" => "python",
                    "js" | "mjs" => "javascript",
                    "ts" => "typescript",
                    "go" => "go",
                    "java" => "java",
                    "c" => "c",
                    "cpp" | "cc" | "cxx" => "cpp",
                    "php" => "php",
                    "rb" => "ruby",
                    "sh" => "shell",
                    _ => return None,
                };
                Some(lang.to_string())
            })
    }

    fn guess_original_name(&self, backup_path: &Path) -> Option<String> {
        let filename = backup_path.file_name()?.to_string_lossy();

        // Remove common backup extensions/suffixes
        let cleaned = filename
            .strip_suffix(".bak")
            .unwrap_or(&filename)
            .strip_suffix(".backup")
            .unwrap_or(&filename)
            .strip_suffix(".old")
            .unwrap_or(&filename)
            .strip_suffix(".orig")
            .unwrap_or(&filename)
            .strip_suffix("~")
            .unwrap_or(&filename)
            .strip_suffix(".autobak")
            .unwrap_or(&filename)
            .strip_suffix(".safebak")
            .unwrap_or(&filename)
            .strip_suffix(".prepatchbak")
            .unwrap_or(&filename)
            .strip_suffix(".manualbak")
            .unwrap_or(&filename);

        if cleaned != filename {
            Some(cleaned.to_string())
        } else {
            None
        }
    }

    /// Check if file should be excluded from metrics/analysis
    pub fn should_exclude_from_analysis(&self, category: &FileCategory) -> bool {
        matches!(
            category,
            FileCategory::TestCode { .. }
                | FileCategory::Vendor { .. }
                | FileCategory::Backup { .. }
                | FileCategory::Generated { .. }
                | FileCategory::Configuration { .. }
        )
    }

    /// Get confidence score for classification
    pub fn get_confidence(&self, category: &FileCategory) -> f32 {
        match category {
            FileCategory::SourceCode { confidence, .. } => *confidence,
            FileCategory::TestCode { confidence, .. } => *confidence,
            FileCategory::Vendor { confidence, .. } => *confidence,
            FileCategory::Backup { confidence, .. } => *confidence,
            FileCategory::Generated { confidence, .. } => *confidence,
            FileCategory::Documentation { confidence, .. } => *confidence,
            FileCategory::Configuration { confidence, .. } => *confidence,
            FileCategory::Unknown { .. } => 0.0,
        }
    }
}

impl Default for FileClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_backup_file_detection() {
        let classifier = FileClassifier::new();

        let backup_files = [
            "src/main.rs.bak",
            "config.yaml.backup",
            "test.py~",
            "file.old",
            "data.json.autobak",
            "script.sh.prepatchbak",
        ];

        for backup_file in backup_files {
            let path = PathBuf::from(backup_file);
            let category = classifier.classify_file_fast(&path);
            assert!(
                matches!(category, FileCategory::Backup { .. }),
                "Failed to detect {} as backup",
                backup_file
            );
        }
    }

    #[test]
    fn test_test_file_detection() {
        let classifier = FileClassifier::new();

        let test_files = [
            "tests/test_main.py",
            "src/lib.test.js",
            "__tests__/component.test.tsx",
            "spec/user_spec.rb",
        ];

        for test_file in test_files {
            let path = PathBuf::from(test_file);
            let category = classifier.classify_file_fast(&path);
            assert!(
                matches!(category, FileCategory::TestCode { .. }),
                "Failed to detect {} as test",
                test_file
            );
        }
    }

    #[test]
    fn test_source_file_detection() {
        let classifier = FileClassifier::new();

        let source_files = ["src/main.rs", "lib/utils.py", "components/Header.js"];

        for source_file in source_files {
            let path = PathBuf::from(source_file);
            let category = classifier.classify_file_fast(&path);
            assert!(
                matches!(category, FileCategory::SourceCode { .. }),
                "Failed to detect {} as source",
                source_file
            );
        }
    }

    #[test]
    fn test_vendor_file_detection() {
        let classifier = FileClassifier::new();

        let vendor_files = [
            "node_modules/react/index.js",
            "vendor/autoload.php",
            "target/release/app",
        ];

        for vendor_file in vendor_files {
            let path = PathBuf::from(vendor_file);
            let category = classifier.classify_file_fast(&path);
            assert!(
                matches!(category, FileCategory::Vendor { .. }),
                "Failed to detect {} as vendor",
                vendor_file
            );
        }
    }
}
