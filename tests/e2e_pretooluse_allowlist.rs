use std::io::Write;

#[test]
fn e2e_pretooluse_allow_default_salt_in_tests() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    // Path indicates tests/
    let test_dir = dir.join("tests");
    std::fs::create_dir_all(&test_dir).unwrap();
    let file_path = test_dir.join("allow.ts");
    let code = "const default_salt = 'abc'\n";
    std::fs::write(&file_path, code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": { "file_path": file_path.to_string_lossy(), "content": code },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .env("AST_ENV", "test")
        .env("AST_ALLOWLIST_VARS", "default_,test_,mock_")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(hook_input.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "allow");
}

#[test]
fn e2e_pretooluse_ignore_globs_allows_ignored_path() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let mock_dir = dir.join("mocks");
    std::fs::create_dir_all(&mock_dir).unwrap();
    let file_path = mock_dir.join("creds.py");
    let code = "password = 'secret'\n";
    std::fs::write(&file_path, code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": { "file_path": file_path.to_string_lossy(), "content": code },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .env("AST_IGNORE_GLOBS", "**/mocks/**")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(hook_input.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "allow");
}
