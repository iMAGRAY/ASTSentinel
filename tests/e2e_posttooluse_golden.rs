use std::io::Write;

#[test]
fn e2e_posttooluse_golden_sections_order() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("golden.py");

    // Code with a simple issue (too many params / unreachable, etc.)
    let code = "def f(a,b,c,d,e):\n    print(a)\n    return 1\n    print('x')\n";
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
        .env("QUICK_TIPS", "0") // keep output stable
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();

    // Sections appear and in the intended order
    let idx_change = ctx.find("=== CHANGE SUMMARY ===").unwrap();
    let idx_risk = ctx.find("=== RISK REPORT ===").unwrap();
    let idx_health = ctx.find("=== CODE HEALTH ===").unwrap();
    assert!(idx_change < idx_risk && idx_risk < idx_health, "order mismatch: {}", ctx);
}

