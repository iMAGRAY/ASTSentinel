use rust_validation_hooks::config::{self, Environment, Sensitivity};

fn with_env<K: AsRef<str>, V: AsRef<str>, F: FnOnce()>(pairs: &[(K, V)], f: F) {
    let saved: Vec<(String, Option<String>)> = pairs
        .iter()
        .map(|(k, _)| (k.as_ref().to_string(), std::env::var(k.as_ref()).ok()))
        .collect();
    for (k, v) in pairs.iter() {
        std::env::set_var(k.as_ref(), v.as_ref());
    }
    f();
    for (k, v) in saved {
        match v {
            Some(val) => std::env::set_var(k, val),
            None => std::env::remove_var(k),
        }
    }
}

#[test]
fn config_loads_from_env_and_json() {
    // Prepare JSON config in temp file
    let td = tempfile::tempdir().unwrap();
    let cfg_file = td.path().join(".hooks-config.json");
    let cfg_text = r#"{
      "sensitivity": "high",
      "environment": "test",
      "allowlist_vars": ["demo_", "fixture_"],
      "ignore_globs": ["**/snapshots/**", "**/*.snap"]
    }"#;
    std::fs::write(&cfg_file, cfg_text).unwrap();

    with_env(
        &[
            ("SENSITIVITY", "low"),
            ("AST_ENV", "production"),
            ("AST_ALLOWLIST_VARS", "example_,sample_"),
            ("AST_IGNORE_GLOBS", "**/mocks/**,**/fixtures/**"),
            ("HOOKS_CONFIG_FILE", cfg_file.to_string_lossy().as_ref()),
        ],
        || {
            let cfg = config::load_config();
            // JSON should override env for sensitivity/environment if present
            assert_eq!(cfg.sensitivity, Sensitivity::High);
            assert_eq!(cfg.env, Environment::Test);
            // Allowlist_vars should be from JSON when present (demo_, fixture_)
            let low = cfg.allowlist_vars.join(",");
            assert!(low.contains("demo_") && low.contains("fixture_"));
            // Ignore globs should include patterns
            assert!(config::should_ignore_path(&cfg, "foo/snapshots/a.txt"));
            assert!(config::should_ignore_path(&cfg, "x/file.snap"));
        },
    );
    // Cleanup temp file auto by tempdir
}

#[test]
fn is_test_context_matches_paths() {
    let td = tempfile::tempdir().unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(td.path()).unwrap();
    let cfg = config::Config {
        env: config::Environment::Production,
        ..Default::default()
    };
    assert!(config::is_test_context(&cfg, "src/tests/foo.rs"));
    assert!(config::is_test_context(&cfg, "src/__tests__/bar.ts"));
    assert!(config::is_test_context(&cfg, "lib/sum_test.rs"));
    assert!(config::is_test_context(&cfg, "spec/calc.spec.ts"));
    assert!(!config::is_test_context(&cfg, "src/main.rs"));
    std::env::set_current_dir(prev).unwrap();
}
