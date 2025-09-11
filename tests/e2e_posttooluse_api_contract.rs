use std::io::Write;

#[test]
fn e2e_posttooluse_api_contract_in_ast_only() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("api.py");

    // Write new content (after)
    let new_code = "def f(a):\n    return a\n";
    std::fs::write(&file_path, new_code).unwrap();

    // Build MultiEdit hook with old/new strings to trigger contract diff
    let hook_input = serde_json::json!({
        "tool_name": "MultiEdit",
        "tool_input": {"file_path": file_path.to_string_lossy(), "edits": [{"old_string": "def f(a,b):\n    return a+b\n", "new_string": new_code }]},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });

    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_AST_ONLY", "1")
        .env("API_CONTRACT", "1")
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
    assert!(ctx.contains("=== API CONTRACT ==="), "ctx: {}", ctx);
    assert!(
        ctx.contains("parameter count reduced") || ctx.contains("parameter"),
        "ctx: {}",
        ctx
    );
}
