use std::io::Write;

#[test]
fn e2e_posttooluse_quick_tips_present() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    // Code with a few issues to trigger multiple tips
    let file_path = dir.join("bad.ts");
    let code = "function f(this: any, a:number,b:number,c:number,d:number,e:number,f:number){ console.log(a); return 1; console.log('x'); }\n";
    std::fs::write(&file_path, code).unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": code},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_AST_ONLY", "1")
        .env("QUICK_TIPS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.contains("=== QUICK TIPS ==="));
    // ensure lines are short
    for line in ctx.lines() {
        if line.starts_with("- ") { assert!(line.chars().count() <= 150, "tip too long: {}", line); }
    }
}

