use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn e2e_posttooluse_entity_snippets_python() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    let file_path = dir.join("calc.py");

    let old_code = "def add(a,b):\n    return a+b\n";
    // Изменение в теле функции с добавлением хардкода секрета — попадёт в AST issues
    let new_code = "def add(a,b):\n    password = 'secret'\n    return a + b\n";
    // PostToolUse выполняется после применения изменения — записываем новую версию
    std::fs::write(&file_path, new_code).expect("write new code");

    let hook_input = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": { "file_path": file_path.to_string_lossy(), "old_string": old_code, "new_string": new_code },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_AST_ONLY", "1")
        .env("AST_DIFF_ONLY", "1")
        .env("AST_DIFF_CONTEXT", "1")
        .env("AST_ENTITY_SNIPPETS", "1")
        .env("AST_MAX_SNIPPETS", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn posttooluse");
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
    assert!(ctx.contains("=== CHANGE CONTEXT ==="));
    // Ожидаем, что ровно один сниппет и он помечает изменённую строку '>'
    assert!(
        ctx.lines().filter(|l| l.starts_with("- [")).count() >= 1,
        "expected at least one snippet header"
    );
    assert!(ctx.contains("\n>"), "expected a marked line with '>'");
}
