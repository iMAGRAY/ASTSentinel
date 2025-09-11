use std::io::Write;

#[test]
fn e2e_userpromptsubmit_pyproject_counts() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();

    let pyproject = r#"[tool.poetry]
name = "demo"
version = "0.1.0"

[tool.poetry.dependencies]
python = ">=3.11,<3.13"
requests = "^2.31.0"

[tool.poetry.dev-dependencies]
pytest = { version = "^7.4.0" }
"#;
    std::fs::write(dir.join("pyproject.toml"), pyproject).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "UserPromptSubmit",
        "tool_input": {},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "UserPromptSubmit"
    });

    let bin = env!("CARGO_BIN_EXE_userpromptsubmit");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("USERPROMPT_CONTEXT_LIMIT", "4000")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn userpromptsubmit");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(hook_input.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let txt = String::from_utf8_lossy(&out.stdout);
    assert!(
        txt.contains("Dependencies: total 2"),
        "expected 2 deps (prod+dev); output: {}",
        txt
    );
}
