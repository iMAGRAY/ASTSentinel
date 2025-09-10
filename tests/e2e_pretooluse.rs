use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

#[test]
fn e2e_pretooluse_ast_only_allow_write() {
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("ok.py");
    let code = "print('ok')\n";
    std::fs::write(&file_path, code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": { "file_path": file_path.to_string_lossy(), "content": code },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let input = hook_input.to_string();

    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn pretooluse");
    child.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "allow");
}

#[test]
fn e2e_pretooluse_ast_only_deny_write_hardcoded_creds() {
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("bad.py");
    let code = "password = 'secret'\n";
    std::fs::write(&file_path, code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": { "file_path": file_path.to_string_lossy(), "content": code },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let input = hook_input.to_string();

    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn pretooluse");
    child.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "deny");
    let reason = v["hookSpecificOutput"]["permissionDecisionReason"].as_str().unwrap();
    let low = reason.to_lowercase();
    assert!(low.contains("hardcoded") || low.contains("секрет") || low.contains("credential") || low.contains("sec001"));
}

#[test]
fn e2e_pretooluse_ast_only_deny_ts_hardcoded_creds() {
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("bad.ts");
    let code = "const password = 'secret'\n";
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
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "deny");
}

#[test]
fn e2e_pretooluse_ast_only_deny_js_sql_literal() {
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("bad.js");
    let code = "const q = \"SELECT * FROM t WHERE id=1\"\n";
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
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "deny");
}

#[test]
fn e2e_pretooluse_ast_only_multiedit_sql_injection_denied() {
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("sql.py");
    let base = "print('start')\n";
    std::fs::write(&file_path, base).unwrap();

    // MultiEdit combining harmless and dangerous changes
    let edits = serde_json::json!([
        {"old_string": "print('start')", "new_string": "print('ok')"},
        {"old_string": "", "new_string": "query = f\"SELECT * FROM t WHERE id = {user_id}\"\n"}
    ]);

    let hook_input = serde_json::json!({
        "tool_name": "MultiEdit",
        "tool_input": {"file_path": file_path.to_string_lossy(), "edits": edits},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "deny");
}

#[test]
fn e2e_pretooluse_ast_only_unknown_extension_allows() {
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("data.unknown");
    std::fs::write(&file_path, "whatever").unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": "whatever"},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "allow");
}

#[test]
fn e2e_pretooluse_ast_only_deny_yaml_hardcoded_creds() {
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("config.yaml");
    let content = "password: secret123\n";
    std::fs::write(&file_path, content).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": { "file_path": file_path.to_string_lossy(), "content": content },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let input = hook_input.to_string();

    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn pretooluse");
    child.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "deny");
}

#[cfg(windows)]
#[test]
fn e2e_pretooluse_windows_allow_writefile_backslash() {
    use std::io::Write;
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("ok_win.ts");
    let code = "export const x = 1;\n";
    // Use backslash path
    let win_path = file_path.to_string_lossy().replace('/', "\\");
    let hook_input = serde_json::json!({
        "tool_name": "WriteFile",
        "tool_input": { "file_path": win_path, "content": code },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "allow");
}

#[cfg(windows)]
#[test]
fn e2e_pretooluse_windows_allow_append_backslash() {
    use std::io::Write;
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("append_win.py");
    std::fs::write(&file_path, "print('a')\n").unwrap();
    let win_path = file_path.to_string_lossy().replace('/', "\\");
    let hook_input = serde_json::json!({
        "tool_name": "AppendToFile",
        "tool_input": { "file_path": win_path, "content": "print('b')\n" },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "allow");
}

#[cfg(windows)]
#[test]
fn e2e_pretooluse_windows_deny_sql_injection_backslash_path() {
    use std::io::Write;
    let temp = tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("sql_win.py");
    std::fs::write(&file_path, "print('start')\n").unwrap();
    let win_path = file_path.to_string_lossy().replace('/', "\\");
    let edits = serde_json::json!([
        {"old_string": "print('start')", "new_string": "print('ok')"},
        {"old_string": "", "new_string": "q = f\"SELECT * FROM t WHERE id={uid}\"\n"}
    ]);
    let hook_input = serde_json::json!({
        "tool_name": "MultiEdit",
        "tool_input": {"file_path": win_path, "edits": edits},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "deny");
}
