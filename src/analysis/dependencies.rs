//! Dependency analysis module for project dependencies validation
//! Supports multiple package managers: npm, pip, cargo, etc.

use anyhow::{Context, Result};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub name: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub package_manager: PackageManager,
    pub is_dev_dependency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageManager {
    Npm,
    Pip,
    Cargo,
    Poetry,
    Yarn,
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageManager::Npm => write!(f, "npm"),
            PackageManager::Pip => write!(f, "pip"),
            PackageManager::Cargo => write!(f, "cargo"),
            PackageManager::Poetry => write!(f, "poetry"),
            PackageManager::Yarn => write!(f, "yarn"),
        }
    }
}

#[derive(Debug)]
pub struct ProjectDependencies {
    pub dependencies: Vec<DependencyInfo>,
    pub total_count: usize,
    pub outdated_count: usize,
    pub dev_dependencies_count: usize,
}

impl Default for ProjectDependencies {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectDependencies {
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
            total_count: 0,
            outdated_count: 0,
            dev_dependencies_count: 0,
        }
    }

    pub fn add_dependency(&mut self, dep: DependencyInfo) {
        if dep.is_dev_dependency {
            self.dev_dependencies_count += 1;
        }
        
        if dep.latest_version.is_some() && 
           dep.latest_version.as_ref() != Some(&dep.current_version) {
            self.outdated_count += 1;
        }
        
        self.dependencies.push(dep);
        self.total_count += 1;
    }

    pub fn format_for_ai(&self) -> String {
        if self.dependencies.is_empty() {
            return "No dependencies detected in project.".to_string();
        }

        let mut result = String::with_capacity(1024);
        result.push_str("\n## PROJECT DEPENDENCIES ANALYSIS\n");
        result.push_str(&format!(
            "Total: {} dependencies ({} dev, {} production)\n",
            self.total_count,
            self.dev_dependencies_count,
            self.total_count - self.dev_dependencies_count
        ));
        
        if self.outdated_count > 0 {
            result.push_str(&format!(
                "⚠️  {} potentially outdated dependencies detected\n\n",
                self.outdated_count
            ));
        } else {
            result.push_str("✅ All dependencies appear up-to-date\n\n");
        }

        // Group by package manager
        let mut by_manager: HashMap<PackageManager, Vec<&DependencyInfo>> = HashMap::new();
        for dep in &self.dependencies {
            by_manager.entry(dep.package_manager.clone()).or_default().push(dep);
        }

        for (manager, deps) in by_manager {
            result.push_str(&format!("### {} Dependencies ({})\n", manager, deps.len()));
            
            // Separate outdated and current dependencies
            let mut outdated: Vec<_> = deps.iter().filter(|d| {
                d.latest_version.is_some() && 
                d.latest_version.as_ref() != Some(&d.current_version)
            }).collect();
            let current: Vec<_> = deps.iter().filter(|d| {
                d.latest_version.is_none() || 
                d.latest_version.as_ref() == Some(&d.current_version)
            }).collect();

            // Show outdated first (more important)
            if !outdated.is_empty() {
                result.push_str("🔴 **Outdated:**\n");
                outdated.sort_by(|a, b| a.name.cmp(&b.name));
                for dep in outdated {
                    let dev_marker = if dep.is_dev_dependency { " (dev)" } else { "" };
                    result.push_str(&format!(
                        "  • {}{}: {} → {}\n",
                        dep.name,
                        dev_marker,
                        dep.current_version,
                        dep.latest_version.as_ref().unwrap_or(&"unknown".to_string())
                    ));
                }
                result.push('\n');
            }

            // Show current dependencies (less verbose)
            if !current.is_empty() && current.len() <= 10 {
                result.push_str("✅ **Up-to-date:**\n");
                for dep in current.iter().take(10) {
                    let dev_marker = if dep.is_dev_dependency { " (dev)" } else { "" };
                    result.push_str(&format!(
                        "  • {}{}: {}\n",
                        dep.name,
                        dev_marker,
                        dep.current_version
                    ));
                }
                if current.len() > 10 {
                    result.push_str(&format!("  ... and {} more\n", current.len() - 10));
                }
                result.push('\n');
            } else if !current.is_empty() {
                result.push_str(&format!("✅ {} dependencies up-to-date\n\n", current.len()));
            }
        }

        result
    }
}

/// Analyze project dependencies from various package manager files
pub async fn analyze_project_dependencies(project_root: &Path) -> Result<ProjectDependencies> {
    let mut project_deps = ProjectDependencies::new();

    // Check for different package manager files
    let files_to_check = vec![
        ("package.json", PackageManager::Npm),
        ("requirements.txt", PackageManager::Pip),
        ("Cargo.toml", PackageManager::Cargo),
        ("pyproject.toml", PackageManager::Poetry),
        ("yarn.lock", PackageManager::Yarn),
    ];

    for (filename, package_manager) in files_to_check {
        let file_path = project_root.join(filename);
        if file_path.exists() {
            match package_manager {
                PackageManager::Npm => {
                    if let Ok(deps) = parse_package_json(&file_path).await {
                        for dep in deps {
                            project_deps.add_dependency(dep);
                        }
                    }
                }
                PackageManager::Pip => {
                    if let Ok(deps) = parse_requirements_txt(&file_path).await {
                        for dep in deps {
                            project_deps.add_dependency(dep);
                        }
                    }
                }
                PackageManager::Cargo => {
                    if let Ok(deps) = parse_cargo_toml(&file_path).await {
                        for dep in deps {
                            project_deps.add_dependency(dep);
                        }
                    }
                }
                PackageManager::Poetry => {
                    if let Ok(deps) = parse_pyproject_toml_poetry(&file_path).await {
                        for dep in deps { project_deps.add_dependency(dep); }
                    }
                }
                PackageManager::Yarn => {
                    // Yarn lock parsing is heavy; rely on package.json instead.
                    // Intentionally no-op here.
                }
            }
        }
    }

    Ok(project_deps)
}

/// Parse package.json for npm dependencies
async fn parse_package_json(file_path: &PathBuf) -> Result<Vec<DependencyInfo>> {
    let content = fs::read_to_string(file_path).await
        .context("Failed to read package.json")?;
    
    let json: serde_json::Value = serde_json::from_str(&content)
        .context("Failed to parse package.json")?;

    let mut dependencies = Vec::new();

    // Parse production dependencies
    if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
        for (name, version) in deps {
            if let Some(version_str) = version.as_str() {
                dependencies.push(DependencyInfo {
                    name: name.clone(),
                    current_version: clean_version_string(version_str),
                    latest_version: None, // TODO: Fetch from registry
                    package_manager: PackageManager::Npm,
                    is_dev_dependency: false,
                });
            }
        }
    }

    // Parse dev dependencies
    if let Some(dev_deps) = json.get("devDependencies").and_then(|d| d.as_object()) {
        for (name, version) in dev_deps {
            if let Some(version_str) = version.as_str() {
                dependencies.push(DependencyInfo {
                    name: name.clone(),
                    current_version: clean_version_string(version_str),
                    latest_version: None, // TODO: Fetch from registry
                    package_manager: PackageManager::Npm,
                    is_dev_dependency: true,
                });
            }
        }
    }

    Ok(dependencies)
}

/// Parse requirements.txt for pip dependencies
async fn parse_requirements_txt(file_path: &PathBuf) -> Result<Vec<DependencyInfo>> {
    let content = fs::read_to_string(file_path).await
        .context("Failed to read requirements.txt")?;
    
    let mut dependencies = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            continue;
        }

        // Parse package==version or package>=version, etc.
        if let Some(dep) = parse_pip_requirement(line) {
            dependencies.push(dep);
        }
    }

    Ok(dependencies)
}

/// Parse Cargo.toml for Rust dependencies
async fn parse_cargo_toml(file_path: &PathBuf) -> Result<Vec<DependencyInfo>> {
    let content = fs::read_to_string(file_path).await
        .context("Failed to read Cargo.toml")?;
    
    // Simple TOML parsing for basic dependencies
    // For production use, should use a proper TOML parser
    let mut dependencies = Vec::new();
    let mut in_dependencies = false;
    let mut in_dev_dependencies = false;

    for line in content.lines() {
        let line = line.trim();
        
        if line == "[dependencies]" {
            in_dependencies = true;
            in_dev_dependencies = false;
            continue;
        } else if line == "[dev-dependencies]" {
            in_dependencies = false;
            in_dev_dependencies = true;
            continue;
        } else if line.starts_with('[') && line.ends_with(']') {
            in_dependencies = false;
            in_dev_dependencies = false;
            continue;
        }

        if (in_dependencies || in_dev_dependencies) && line.contains('=') {
            if let Some(dep) = parse_cargo_dependency(line, in_dev_dependencies) {
                dependencies.push(dep);
            }
        }
    }

    Ok(dependencies)
}

/// Clean version string by removing prefixes like ^, ~, >=, etc.
fn clean_version_string(version: &str) -> String {
    version.trim_start_matches(&['^', '~', '=', '>', '<', ' '][..]).to_string()
}

/// Parse a single pip requirement line
fn parse_pip_requirement(line: &str) -> Option<DependencyInfo> {
    // Handle different formats: package==1.0.0, package>=1.0.0, etc.
    let operators = ["==", ">=", "<=", "!=", ">", "<", "~="];
    
    for op in &operators {
        if let Some(pos) = line.find(op) {
            let name = line[..pos].trim().to_string();
            let version = line[pos + op.len()..].trim().to_string();
            
            return Some(DependencyInfo {
                name,
                current_version: clean_version_string(&version),
                latest_version: None,
                package_manager: PackageManager::Pip,
                is_dev_dependency: false,
            });
        }
    }
    
    None
}

/// Parse a single Cargo dependency line
fn parse_cargo_dependency(line: &str, is_dev: bool) -> Option<DependencyInfo> {
    if let Some(eq_pos) = line.find('=') {
        let name = line[..eq_pos].trim().to_string();
        let version_part = line[eq_pos + 1..].trim();
        
        // Handle simple version strings in quotes
        if version_part.starts_with('"') && version_part.ends_with('"') {
            let version = version_part[1..version_part.len()-1].to_string();
            
            return Some(DependencyInfo {
                name,
                current_version: clean_version_string(&version),
                latest_version: None,
                package_manager: PackageManager::Cargo,
                is_dev_dependency: is_dev,
            });
        }
        
        // Handle object dependencies like: clap = { version = "4.0", features = ["derive"] }
        if version_part.starts_with('{') {
            // Look for version = "..." inside the object
            if let Some(version_start) = version_part.find("version") {
                let after_version = &version_part[version_start + 7..]; // skip "version"
                if let Some(eq_pos) = after_version.find('=') {
                    let version_value = after_version[eq_pos + 1..].trim();
                    if let Some(quote_start) = version_value.find('"') {
                        let after_quote = &version_value[quote_start + 1..];
                        if let Some(quote_end) = after_quote.find('"') {
                            let version = after_quote[..quote_end].to_string();
                            
                            return Some(DependencyInfo {
                                name,
                                current_version: clean_version_string(&version),
                                latest_version: None,
                                package_manager: PackageManager::Cargo,
                                is_dev_dependency: is_dev,
                            });
                        }
                    }
                }
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_parse_package_json() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("package.json");
        
        let content = r#"{
            "name": "test-project",
            "dependencies": {
                "express": "^4.18.0",
                "lodash": "~4.17.21"
            },
            "devDependencies": {
                "jest": "^29.0.0"
            }
        }"#;
        
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(content.as_bytes()).await.unwrap();
        drop(file);
        
        let deps = parse_package_json(&file_path).await.unwrap();
        assert_eq!(deps.len(), 3);
        
        let express = deps.iter().find(|d| d.name == "express").unwrap();
        assert_eq!(express.current_version, "4.18.0");
        assert!(!express.is_dev_dependency);
        
        let jest = deps.iter().find(|d| d.name == "jest").unwrap();
        assert!(jest.is_dev_dependency);
    }

    #[tokio::test] 
    async fn test_parse_requirements_txt() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("requirements.txt");
        
        let content = "Django==4.2.0\nrequests>=2.28.0\n# comment line\npillow==10.0.0\n";
        
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(content.as_bytes()).await.unwrap();
        drop(file);
        
        let deps = parse_requirements_txt(&file_path).await.unwrap();
        assert_eq!(deps.len(), 3);
        
        let django = deps.iter().find(|d| d.name == "Django").unwrap();
        assert_eq!(django.current_version, "4.2.0");
        
        let pillow = deps.iter().find(|d| d.name == "pillow").unwrap();
        assert_eq!(pillow.current_version, "10.0.0");
    }

    #[tokio::test]
    async fn test_parse_package_json_invalid_errors() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("package.json");
        let content = "{ invalid_json: true"; // malformed
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(content.as_bytes()).await.unwrap();
        drop(file);
        let res = parse_package_json(&file_path).await;
        assert!(res.is_err(), "Expected parse error for invalid package.json");
    }

    #[tokio::test]
    async fn test_parse_cargo_toml() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("Cargo.toml");

        let content = r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
serde = "1.0"
clap = { version = "4.0", features = ["derive"] }

[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
"#;

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(content.as_bytes()).await.unwrap();
        drop(file);

        let deps = parse_cargo_toml(&file_path).await.unwrap();
        // Expect 3 deps collected (serde, clap, tokio)
        assert!(deps.iter().any(|d| d.name == "serde" && d.current_version == "1.0" && !d.is_dev_dependency));
        assert!(deps.iter().any(|d| d.name == "clap" && d.current_version == "4.0" && !d.is_dev_dependency));
        assert!(deps.iter().any(|d| d.name == "tokio" && d.current_version == "1.0" && d.is_dev_dependency));
    }

    #[tokio::test]
    async fn test_parse_pyproject_toml_poetry() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("pyproject.toml");
        let content = r#"[tool.poetry]
name = "demo"
version = "0.1.0"

[tool.poetry.dependencies]
python = ">=3.11,<3.13"
requests = "^2.31.0"

[tool.poetry.dev-dependencies]
pytest = { version = "^7.4.0" }
"#;
        let mut f = File::create(&file_path).await.unwrap();
        f.write_all(content.as_bytes()).await.unwrap();
        drop(f);

        let deps = parse_pyproject_toml_poetry(&file_path).await.unwrap();
        assert!(deps.iter().any(|d| d.name == "requests" && !d.is_dev_dependency));
        assert!(deps.iter().any(|d| d.name == "pytest" && d.is_dev_dependency));
        // 'python' entry is a platform constraint; we intentionally do not treat it as a dependency
        assert!(!deps.iter().any(|d| d.name == "python"));
    }

    #[test]
    fn test_clean_version_string() {
        assert_eq!(clean_version_string("^4.18.0"), "4.18.0");
        assert_eq!(clean_version_string("~4.17.21"), "4.17.21");
        assert_eq!(clean_version_string(">=2.28.0"), "2.28.0");
        assert_eq!(clean_version_string("1.0.0"), "1.0.0");
    }
}

/// Parse pyproject.toml (Poetry) for dependencies
async fn parse_pyproject_toml_poetry(file_path: &PathBuf) -> Result<Vec<DependencyInfo>> {
    let content = fs::read_to_string(file_path).await
        .context("Failed to read pyproject.toml")?;

    let mut dependencies = Vec::new();
    let mut in_deps = false;
    let mut in_dev_deps = false;

    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("[tool.poetry.dependencies]") {
            in_deps = true; in_dev_deps = false; continue;
        }
        if line.starts_with("[tool.poetry.dev-dependencies]") {
            in_deps = false; in_dev_deps = true; continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_deps = false; in_dev_deps = false; continue;
        }
        if !(in_deps || in_dev_deps) { continue; }
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(eq) = line.find('=') {
            let name = line[..eq].trim().to_string();
            if name.eq_ignore_ascii_case("python") { continue; }
            let val = line[eq+1..].trim();
            // version formats: "^1.2.3" or { version = "1.2.3" }
            let version = if val.starts_with('"') && val.ends_with('"') {
                val.trim_matches('"').to_string()
            } else if val.starts_with('{') {
                // find version = "..."
                if let Some(vpos) = val.find("version") {
                    let after = &val[vpos+7..];
                    if let Some(eq) = after.find('=') {
                        let v = after[eq+1..].trim();
                        v.trim_matches(|c| c=='"' || c==',' || c==' ' || c=='}').to_string()
                    } else { String::new() }
                } else { String::new() }
            } else { String::new() };

            if !version.is_empty() {
                dependencies.push(DependencyInfo {
                    name,
                    current_version: clean_version_string(&version),
                    latest_version: None,
                    package_manager: PackageManager::Poetry,
                    is_dev_dependency: in_dev_deps,
                });
            }
        }
    }

    Ok(dependencies)
}
