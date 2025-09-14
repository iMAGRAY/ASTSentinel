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
    child.stdin.as_mut().unwrap().write_all(input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn java_empty_catch_denied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("A.java");
    let old = "class A{ void f(){ try{ g(); } catch(Exception e){ e.printStackTrace(); } } }\n";
    let new = "class A{ void f(){ try{ g(); } catch(Exception e){ } } }\n";
    std::fs::write(&path, old).unwrap();
    let v = run_pretool(serde_json::json!({
        "tool_name":"Edit",
        "tool_input": {"file_path": path.to_string_lossy(), "old_string": old, "new_string": new},
        "hook_event_name":"PreToolUse"
    }));
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny");
}

#[test]
fn csharp_empty_catch_denied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("A.cs");
    let old = "class A{ void F(){ try{ G(); } catch(System.Exception ex){ System.Console.WriteLine(ex); } } }\n";
    let new = "class A{ void F(){ try{ G(); } catch(System.Exception ex){ } } }\n";
    std::fs::write(&path, old).unwrap();
    let v = run_pretool(serde_json::json!({
        "tool_name":"Edit",
        "tool_input": {"file_path": path.to_string_lossy(), "old_string": old, "new_string": new},
        "hook_event_name":"PreToolUse"
    }));
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny");
}

#[test]
fn go_discard_result_denied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.go");
    let old = "package main\nfunc g() error { return nil }\nfunc f(){ if err := g(); err != nil { panic(err) } }\n";
    let new = "package main\nfunc g() error { return nil }\nfunc f(){ _ = g() }\n";
    std::fs::write(&path, old).unwrap();
    let v = run_pretool(serde_json::json!({
        "tool_name":"Edit",
        "tool_input": {"file_path": path.to_string_lossy(), "old_string": old, "new_string": new},
        "hook_event_name":"PreToolUse"
    }));
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny");
}

