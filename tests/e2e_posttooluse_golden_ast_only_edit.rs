use std::io::Write;

#[test]
fn e2e_posttooluse_golden_sections_order_ast_only_edit() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("golden_ast_edit.py");

    let old_code = "def f(a,b):\n    return a+b\n";
    let new_code = "def f(a):\n    return a\n";
    std::fs::write(&file_path, new_code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": {"file_path": file_path.to_string_lossy(), "old_string": old_code, "new_string": new_code},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_AST_ONLY", "1")
        .env("QUICK_TIPS", "0")
        .env("API_CONTRACT", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();

    let idx_change = ctx.find("=== CHANGE SUMMARY ===").unwrap();
    let idx_risk = ctx.find("=== RISK REPORT ===").unwrap();
    let idx_health = ctx.find("=== CODE HEALTH ===").unwrap();
    let idx_contract = ctx.find("=== API CONTRACT ===").unwrap();
    let idx_next = ctx.find("=== NEXT STEPS ===").unwrap();
    assert!(idx_change < idx_risk && idx_risk < idx_health && idx_health < idx_contract && idx_contract < idx_next, "order mismatch: {}", ctx);
}

