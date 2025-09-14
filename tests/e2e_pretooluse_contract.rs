use std::io::Write;

// e2e: ослабление контракта (Python) должно приводить к deny
#[test]
fn e2e_contract_weaken_python_deny() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("module.py");

    let old_code = r#"def add(a, b):
    return a + b

result = add(1, 2)
"#;
    let new_code = r#"def add(a):
    return a

result = add(1, 2)
"#;
    // Создадим файл (для контекста диффа; не обязательно)
    std::fs::write(&file_path, old_code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": {
            "file_path": file_path.to_string_lossy(),
            "old_string": old_code,
            "new_string": new_code
        },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });

    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1") // оффлайн режим
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn pretooluse");

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
    let reason = v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap();
    assert_eq!(decision, "deny");
    assert!(
        reason.to_ascii_lowercase().contains("contract") || reason.to_ascii_lowercase().contains("контракт")
    );
}

// e2e: безопасный рефакторинг (JS) без изменения сигнатуры должен allow при
// низкой чувствительности
#[test]
fn e2e_contract_preserved_js_allow() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let file_path = dir.join("calc.js");

    let old_code = r#"function sum(a, b) {
  return a + b;
}

const r = sum(1, 2);
"#;
    let new_code = r#"function sum(a, b) {
  const c = a + b;
  return c;
}

const r = sum(1, 2);
"#;
    std::fs::write(&file_path, old_code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": {
            "file_path": file_path.to_string_lossy(),
            "old_string": old_code,
            "new_string": new_code
        },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PreToolUse"
    });

    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("PRETOOL_AST_ONLY", "1")
        .env("SENSITIVITY", "low") // минимальная чувствительность: не должно быть deny
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn pretooluse");

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
