use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Deterministic ignore patterns loaded from project root (.gitignore + built-ins).
#[derive(Debug, Clone)]
pub struct Patterns {
    pub root: PathBuf,
    pub entries: HashSet<String>,
}

impl Patterns {
    pub fn load(root: &Path) -> Result<Self> {
        let mut entries = HashSet::new();

        // Built-in ignore patterns (directories and files commonly not part of source)
        const BUILTIN: &[&str] = &[
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
            // VCS
            ".git/",
            ".svn/",
            ".hg/",
            ".bzr/",
            // IDE/editor artefacts
            ".vscode/",
            ".idea/",
            "*.swp",
            "*.swo",
            "*~",
            ".DS_Store",
            "Thumbs.db",
            // Temp
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
            // Language specific
            "__pycache__/",
            "*.pyc",
            "*.pyo",
            ".pytest_cache/",
            "*.class",
            ".gradle/",
            ".maven/",
            ".nuget/",
            "packages/",
            // Compilation artefacts
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
            // OS specific
            ".Trash/",
            "$RECYCLE.BIN/",
        ];
        for p in BUILTIN {
            entries.insert((*p).to_string());
        }

        // Only the project's own .gitignore (deterministic) — no global or parent .gitignore
        let gitignore_path = root.join(".gitignore");
        if gitignore_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&gitignore_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        entries.insert(line.to_string());
                    }
                }
            } else {
                // Do not fail hard on unreadable .gitignore; deterministic fallback
                tracing::warn!(path=%gitignore_path.display(), "Cannot read .gitignore");
            }
        }

        Ok(Self { root: root.to_path_buf(), entries })
    }

    pub fn overlay(mut self, extra: &[&str]) -> Self {
        for e in extra {
            self.entries.insert((*e).to_string());
        }
        self
    }
}

/// Combined ignore: .gitignore patterns + optional globset from config.
#[derive(Debug, Clone)]
pub struct CombinedIgnore {
    root: PathBuf,
    patterns: Patterns,
    globset: Option<GlobSet>,
}

impl CombinedIgnore {
    pub fn new(root: &Path, cfg_globs: Option<&GlobSet>) -> Result<Self> {
        let patterns = Patterns::load(root)?;
        Ok(Self { root: root.to_path_buf(), patterns, globset: cfg_globs.cloned() })
    }

    pub fn with_overlay(mut self, extra: &[&str]) -> Self {
        self.patterns = self.patterns.overlay(extra);
        self
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        // Globset (config) first
        if let Some(gs) = &self.globset {
            if gs.is_match(path) {
                return true;
            }
        }
        // Then project patterns
        matches(&self.patterns, &self.root, path)
    }
}

impl CombinedIgnore {
    /// Construct an ignore filter that matches nothing (safe fallback)
    pub fn empty_for(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            patterns: Patterns { root: root.to_path_buf(), entries: HashSet::new() },
            globset: None,
        }
    }
}

/// Public helpers (wrappers) used by older modules
pub fn matches(patterns: &Patterns, root: &Path, path: &Path) -> bool {
    let relative_path = match path.strip_prefix(root) {
        Ok(rel) => rel,
        Err(_) => return true,
    };
    let path_str = relative_path.to_string_lossy().replace('\\', "/");
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    for pattern in &patterns.entries {
        if pattern.ends_with('/') {
            let dir = &pattern[..pattern.len() - 1];
            if path_str.starts_with(dir) || path_str.contains(&format!("/{dir}")) {
                return true;
            }
        } else if pattern.contains('*') {
            if glob_match(&file_name, pattern) || glob_match(&path_str, pattern) {
                return true;
            }
        } else if path_str == *pattern || file_name == *pattern || path_str.ends_with(&format!("/{pattern}")) {
            return true;
        }
    }
    false
}

pub fn glob_match(text: &str, pattern: &str) -> bool {
    if !pattern.contains('*') { return text == pattern; }
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.is_empty() { return true; }
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() { continue; }
        if i == 0 {
            if !text.starts_with(part) { return false; }
            pos = part.len();
        } else if i == parts.len() - 1 {
            if !text[pos..].ends_with(part) { return false; }
        } else if let Some(found) = text[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
}

/// Helper to build a GlobSet from string patterns (optional)
pub fn build_globset(patterns: &[&str]) -> Option<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    let mut any = false;
    for p in patterns {
        if let Ok(g) = Glob::new(p) {
            builder.add(g);
            any = true;
        }
    }
    if any { builder.build().ok() } else { None }
}
