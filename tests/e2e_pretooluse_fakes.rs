use std::io::Write;

fn run_pretool(input: serde_json::Value) -> serde_json::Value {
    let bin = env!("CARGO_BIN_EXE_pretooluse");
    let mut child = std::process::Command::new(bin)
        .env("PRETOOL_AST_ONLY", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn pretool");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn js_empty_catch_denied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.js");
    let old = "function f(){ try { x(); } catch(e){ console.log(e); } }\n";
    let new = "function f(){ try { x(); } catch(e){} }\n";
    std::fs::write(&path, old).unwrap();
    let input = serde_json::json!({
        "tool_name":"Edit",
        "tool_input": {"file_path": path.to_string_lossy(), "old_string": old, "new_string": new},
        "hook_event_name":"PreToolUse"
    });
    let v = run_pretool(input);
    let decision = v["hookSpecificOutput"]["permissionDecision"].as_str().unwrap();
    assert_eq!(decision, "deny");
}

#[test]
fn js_promise_catch_swallow_denied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.js");
    let old = "doWork().catch(err=>{ console.error(err); });\n";
    let new = "doWork().catch(()=>{});\n";
    std::fs::write(&path, old).unwrap();
    let input = serde_json::json!({
        "tool_name":"Edit",
        "tool_input": {"file_path": path.to_string_lossy(), "old_string": old, "new_string": new},
        "hook_event_name":"PreToolUse"
    });
    let v = run_pretool(input);
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny");
}

#[test]
fn py_except_pass_denied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.py");
    let old = "def f():\n  try:\n    x()\n  except Exception as e:\n    print(e)\n";
    let new = "def f():\n  try:\n    x()\n  except:\n    pass\n";
    std::fs::write(&path, old).unwrap();
    let input = serde_json::json!({
        "tool_name":"Edit",
        "tool_input": {"file_path": path.to_string_lossy(), "old_string": old, "new_string": new},
        "hook_event_name":"PreToolUse"
    });
    let v = run_pretool(input);
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny");
}

#[test]
fn rs_discard_result_denied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib.rs");
    let old = "fn g()->Result<(),()> { Ok(()) } fn f(){ match g(){ Ok(_) => (), Err(_)=> panic!(\"err\") }\n";
    let new = "fn g()->Result<(),()> { Ok(()) } fn f(){ let _ = g(); }\n";
    std::fs::write(&path, old).unwrap();
    let input = serde_json::json!({
        "tool_name":"Edit",
        "tool_input": {"file_path": path.to_string_lossy(), "old_string": old, "new_string": new},
        "hook_event_name":"PreToolUse"
    });
    let v = run_pretool(input);
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny");
}

#[test]
fn allow_non_fake_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.ts");
    let old = "export function sum(a:number,b:number){ return a+b }\n";
    let new = "export function sum(a:number,b:number){ const c=a+b; return c }\n";
    std::fs::write(&path, old).unwrap();
    let input = serde_json::json!({
        "tool_name":"Edit",
        "tool_input": {"file_path": path.to_string_lossy(), "old_string": old, "new_string": new},
        "hook_event_name":"PreToolUse"
    });
    let v = run_pretool(input);
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "allow");
}

