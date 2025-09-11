use std::io::Write;

#[test]
fn unit_userpromptsubmit_golden_minimal_project() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();

    // Minimal Python file
    std::fs::write(dir.join("a.py"), "def f(x):\n    return x\n").unwrap();

    // Build HookInput JSON
    let hook_input = serde_json::json!({
        "tool_name": "UserPromptSubmit",
        "tool_input": {},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "UserPromptSubmit"
    });

    let bin = env!("CARGO_BIN_EXE_userpromptsubmit");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("USERPROMPT_CONTEXT_LIMIT", "8000")
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

    // Golden shape checks (order + key lines)
    let idx_title = txt.find("# COMPREHENSIVE PROJECT CONTEXT").unwrap();
    let idx_summary = txt.find("=== PROJECT SUMMARY ===").unwrap();
    let idx_risk = txt.find("=== RISK/HEALTH SNAPSHOT ===").unwrap();
    assert!(idx_title < idx_summary && idx_summary < idx_risk);

    assert!(txt.contains("Files: 1"), "should report single file: {}", txt);
    assert!(
        txt.contains("Dependencies: total 0"),
        "deps count should be zero: {}",
        txt
    );
}
