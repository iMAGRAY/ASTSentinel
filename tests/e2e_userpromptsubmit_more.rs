use std::io::Write;

#[test]
fn e2e_userpromptsubmit_invalid_json_fallbacks_to_default() {
    // Create a minimal temp project dir
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    // Add one code file
    std::fs::write(dir.join("hello.py"), "print('hi')\n").unwrap();

    // Spawn without valid JSON to trigger default HookInput
    let bin = env!("CARGO_BIN_EXE_userpromptsubmit");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn userpromptsubmit");

    // Write invalid JSON
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"this is not json")
        .unwrap();

    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let txt = String::from_utf8_lossy(&out.stdout);
    assert!(
        txt.contains("=== PROJECT SUMMARY ==="),
        "must render summary on fallback"
    );
    assert!(!txt.contains("Project analysis unavailable"));
}

#[test]
fn e2e_userpromptsubmit_nonexistent_cwd_prints_unavailable() {
    // Provide explicit JSON with invalid cwd
    let hook_input = serde_json::json!({
        "tool_name": "UserPromptSubmit",
        "tool_input": {},
        "cwd": "_definitely_nonexistent_dir_12345_",
        "hook_event_name": "UserPromptSubmit"
    });

    let bin = env!("CARGO_BIN_EXE_userpromptsubmit");
    let mut child = std::process::Command::new(bin)
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
    assert!(txt.contains("Project analysis unavailable"));
}
