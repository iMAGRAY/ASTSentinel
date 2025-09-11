use std::io::Write;

#[test]
fn e2e_posttooluse_soft_budget_note_present() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("big.py");
    // Code exceeding tiny budget (set below)
    let code = "print('x')\n".repeat(50);
    std::fs::write(&file_path, &code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": code},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_AST_ONLY", "1")
        .env("AST_SOFT_BUDGET_BYTES", "10")
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
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.contains("Skipped AST analysis due to soft budget"));
}

#[test]
fn e2e_posttooluse_soft_budget_note_present_in_dry_run() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("big2.py");
    // Code exceeding tiny budget (set below)
    let code = "print('y')\n".repeat(40);
    std::fs::write(&file_path, &code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": code},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_DRY_RUN", "1")
        .env("AST_SOFT_BUDGET_BYTES", "10")
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
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.contains("Skipped AST analysis due to soft budget"));
}
