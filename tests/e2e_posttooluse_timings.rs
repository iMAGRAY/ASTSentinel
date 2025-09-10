use std::io::Write;

#[test]
fn e2e_posttooluse_timings_summary_present_in_ast_only() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("timed.py");
    // Small but non-empty code to exercise parser and scorer
    let code = "def f(a,b,c,d):\n    x=a+b\n    return x\n";
    std::fs::write(&file_path, code).unwrap();

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
        .env("AST_TIMINGS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.contains("=== TIMINGS (ms) ==="), "timings summary missing: {}", ctx);
}

