use std::io::Write;

#[test]
fn e2e_userpromptsubmit_reports_dependencies_counts() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();

    // package.json with prod+dev
    let pkg = r#"{
  "name": "demo",
  "dependencies": {"left-pad": "^1.3.0"},
  "devDependencies": {"eslint": "^8.0.0"}
}"#;
    std::fs::write(dir.join("package.json"), pkg).unwrap();

    // requirements.txt with two deps
    std::fs::write(dir.join("requirements.txt"), "Django==4.2.0\nrequests>=2.28.0\n").unwrap();

    // Cargo.toml with prod+dev
    let cargo = r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
serde = "1.0"

[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
"#;
    std::fs::write(dir.join("Cargo.toml"), cargo).unwrap();

    // Build HookInput
    let hook_input = serde_json::json!({
        "tool_name": "UserPromptSubmit",
        "tool_input": {},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "UserPromptSubmit"
    });

    let bin = env!("CARGO_BIN_EXE_userpromptsubmit");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("USERPROMPT_CONTEXT_LIMIT", "5000")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn userpromptsubmit");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(hook_input.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let txt = String::from_utf8_lossy(&out.stdout);
    assert!(txt.contains("=== PROJECT SUMMARY ==="));
    // We expect total 6 (1+1 npm + 2 pip + 2 cargo)
    assert!(txt.contains("Dependencies: total 6"), "Output: {}", txt);
}
