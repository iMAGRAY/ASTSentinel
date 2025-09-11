use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn pretooluse_ast_only_denies_structural_breakage() {
    // Invalid Python code: missing colon/closing paren
    let invalid = "def f(x)\n    return x\n";
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("broken.py");
    std::fs::write(&file_path, invalid).expect("write code");

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": invalid},
        "hook_event_name": "PreToolUse"
    });
    let input_str = hook_input.to_string();
    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = Command::new(bin)
        .current_dir(dir.path())
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn pretooluse");
    child.stdin.as_mut().unwrap().write_all(input_str.as_bytes()).unwrap();
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "deny", "broken syntax should be denied");
    let reason = v["hookSpecificOutput"]["permissionDecisionReason"].as_str().unwrap();
    assert!(reason.to_lowercase().contains("structural"));
}

#[test]
fn posttooluse_offline_fallback_without_keys() {
    // Ensure PostToolUse gracefully renders offline when no keys and not DRY_RUN
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("a.py");
    let before = "print('a')\n";
    let after = "password='x'\nprint('a')\n";
    std::fs::write(&file_path, after).unwrap();
    let hook_input = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": {"file_path": file_path.to_string_lossy(), "old_string": before, "new_string": after},
        "cwd": dir.path().to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = Command::new(bin)
        .current_dir(dir.path())
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("GOOGLE_API_KEY")
        .env_remove("XAI_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn posttooluse");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.contains("=== RISK REPORT ===") || ctx.contains("=== CODE HEALTH ===") || ctx.contains("=== CHANGE SUMMARY ===") || ctx.contains("=== NEXT STEPS ==="));
}

