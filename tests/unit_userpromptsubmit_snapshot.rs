use std::io::Write;

#[test]
fn unit_userpromptsubmit_compact_headers_and_limit() {
    // Prepare minimal project
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    std::fs::write(dir.join("a.py"), "print('ok')\n").unwrap();

    // Build HookInput
    let hook_input = serde_json::json!({
        "tool_name": "UserPromptSubmit",
        "tool_input": {},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "UserPromptSubmit"
    });

    let bin = env!("CARGO_BIN_EXE_userpromptsubmit");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("USERPROMPT_CONTEXT_LIMIT", "1200")
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
    assert!(txt.contains("=== PROJECT SUMMARY ==="));
    assert!(txt.contains("=== RISK/HEALTH SNAPSHOT ==="));
    assert!(txt.len() <= 1205, "length should be capped to ~limit");
}
