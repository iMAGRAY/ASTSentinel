use std::io::Write;

#[test]
fn e2e_userpromptsubmit_produces_project_context() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    // Create a small project
    std::fs::write(dir.join("main.py"), "print('ok')\n").unwrap();
    std::fs::write(dir.join("lib.ts"), "export const x = 1;\n").unwrap();
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
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(hook_input.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let txt = String::from_utf8_lossy(&out.stdout);
    assert!(txt.contains("# COMPREHENSIVE PROJECT CONTEXT") || txt.contains("# PROJECT ANALYSIS"));
}
