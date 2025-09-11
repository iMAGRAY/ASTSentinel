use globset::{Glob, GlobSet, GlobSetBuilder};

#[derive(Debug, Clone)]
pub struct Config {
    pub sensitivity: Sensitivity,
    pub ignore_globs: Option<GlobSet>,
    pub env: Environment,
    pub allowlist_vars: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sensitivity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Production,
    Test,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sensitivity: Sensitivity::Medium,
            // None means: do not ignore anything
            ignore_globs: None,
            env: Environment::Production,
            allowlist_vars: vec![
                "default_".to_string(),
                "example_".to_string(),
                "sample_".to_string(),
                "mock_".to_string(),
                "test_".to_string(),
                "dummy_".to_string(),
            ],
        }
    }
}

pub fn load_config() -> Config {
    let mut cfg = Config::default();

    // Sensitivity
    if let Ok(val) = std::env::var("SENSITIVITY") {
        cfg.sensitivity = match val.to_ascii_lowercase().as_str() {
            "low" => Sensitivity::Low,
            "high" => Sensitivity::High,
            _ => Sensitivity::Medium,
        };
    }

    // Environment
    if let Ok(val) = std::env::var("AST_ENV") {
        cfg.env = match val.to_ascii_lowercase().as_str() {
            "test" => Environment::Test,
            _ => Environment::Production,
        };
    }

    // Allowlist vars
    if let Ok(val) = std::env::var("AST_ALLOWLIST_VARS") {
        let list = val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if !list.is_empty() {
            cfg.allowlist_vars = list;
        }
    }

    // Ignore globs
    if let Ok(val) = std::env::var("AST_IGNORE_GLOBS") {
        let mut builder = GlobSetBuilder::new();
        for pat in val.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            if let Ok(glob) = Glob::new(pat) {
                builder.add(glob);
            }
        }
        if let Ok(set) = builder.build() {
            cfg.ignore_globs = Some(set);
        }
    }

    // Optional JSON config file: path from HOOKS_CONFIG_FILE or .hooks-config.json in CWD
    let cfg_path = std::env::var("HOOKS_CONFIG_FILE")
        .ok()
        .unwrap_or_else(|| ".hooks-config.json".to_string());
    if let Ok(text) = std::fs::read_to_string(&cfg_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(sens) = json.get("sensitivity").and_then(|v| v.as_str()) {
                cfg.sensitivity = match sens.to_ascii_lowercase().as_str() {
                    "low" => Sensitivity::Low,
                    "high" => Sensitivity::High,
                    _ => Sensitivity::Medium,
                };
            }
            if let Some(env) = json.get("environment").and_then(|v| v.as_str()) {
                cfg.env = match env.to_ascii_lowercase().as_str() {
                    "test" => Environment::Test,
                    _ => Environment::Production,
                };
            }
            if let Some(list) = json.get("allowlist_vars").and_then(|v| v.as_array()) {
                let mut vars = Vec::new();
                for it in list {
                    if let Some(s) = it.as_str() {
                        vars.push(s.to_string());
                    }
                }
                if !vars.is_empty() {
                    cfg.allowlist_vars = vars;
                }
            }
            if let Some(globs) = json.get("ignore_globs").and_then(|v| v.as_array()) {
                let mut builder = GlobSetBuilder::new();
                for it in globs {
                    if let Some(pat) = it.as_str() {
                        if let Ok(glob) = Glob::new(pat) {
                            builder.add(glob);
                        }
                    }
                }
                if let Ok(set) = builder.build() {
                    cfg.ignore_globs = Some(set);
                }
            }
        }
    }

    cfg
}

pub fn should_ignore_path(cfg: &Config, path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let p = std::path::Path::new(path);
    cfg.ignore_globs
        .as_ref()
        .map(|set| set.is_match(p))
        .unwrap_or(false)
}

pub fn is_test_context(cfg: &Config, path: &str) -> bool {
    if matches!(cfg.env, Environment::Test) {
        return true;
    }
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("__tests__")
        || lower.contains("/spec/")
        || lower.ends_with("_test.rs")
        || lower.ends_with(".spec.ts")
}

pub fn code_contains_allowlisted_vars(cfg: &Config, code: &str) -> bool {
    let low = code.to_ascii_lowercase();
    cfg.allowlist_vars.iter().any(|k| low.contains(k))
}
