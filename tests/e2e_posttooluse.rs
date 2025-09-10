use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

// E2E: run compiled posttooluse binary, feed HookInput JSON via stdin, assert additionalContext contains AST insights
#[test]
fn e2e_posttooluse_ast_only_mode() {
    // Create temp project dir and a Python file with a critical issue (hardcoded credential)
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    let file_path = dir.join("bad.py");
    let code = "password = 'supersecret'\nprint('ok')\n";
    std::fs::write(&file_path, code).expect("write code");

    // Build HookInput JSON for Write tool
    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {
            "file_path": file_path.to_string_lossy(),
            "content": code,
        },
        "session_id": "e2e",
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let input_str = hook_input.to_string();

    // Path to compiled binary provided by Cargo
    let bin_path = env!("CARGO_BIN_EXE_posttooluse");

    // Run binary with AST-only mode to avoid network, capture stdout
    let mut child = Command::new(bin_path)
        .current_dir(dir)
        .env("POSTTOOL_AST_ONLY", "1")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("GEMINI_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn posttooluse");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(input_str.as_bytes())
            .expect("write stdin");
    }

    let output = child.wait_with_output().expect("wait output");
    if !output.status.success() {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        panic!(
            "posttooluse exited with error: status={:?}\nSTDOUT:\n{}\nSTDERR:\n{}",
            output.status.code(), stdout_str, stderr_str
        );
    }

    // Parse JSON output
    let stdout_str = String::from_utf8(output.stdout).expect("utf8 stdout");
    let v: serde_json::Value = serde_json::from_str(&stdout_str).expect("parse json");
    let ctx = &v["hookSpecificOutput"]["additionalContext"];
    assert!(ctx.is_string(), "additionalContext should be string");
    let ctx_str = ctx.as_str().unwrap();

    // Should contain deterministic AST score header or issue listing
    assert!(
        ctx_str.contains("Deterministic Score") || ctx_str.contains("Concrete Issues Found"),
        "additionalContext must include AST insights"
    );

    // Hardcoded creds message should be present for our code
    assert!(
        ctx_str.to_lowercase().contains("credential") || ctx_str.to_lowercase().contains("hardcoded"),
        "expected hardcoded credential message in AST output"
    );
}

#[test]
fn e2e_posttooluse_dry_run_edit_with_prompt_and_diff() {
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();

    // Prepare prompts directory required by prompt loader
    let prompts_dir = dir.join("prompts");
    std::fs::create_dir_all(&prompts_dir).expect("mkdir prompts");
    std::fs::write(prompts_dir.join("post_edit_validation.txt"), "Validate changes.").expect("write prompt");
    std::fs::write(prompts_dir.join("output_template.txt"), "TEMPLATE: RESULT\n").expect("write template");

    // Create file with edited content already applied (post-tool behaviour)
    let file_path = dir.join("app.py");
    let original = "print('ok')\n";
    let edited = "password = 'x'\nprint('changed')\n"; // triggers AST security rule
    std::fs::write(&file_path, edited).expect("write code");

    // HookInput for Edit
    let hook_input = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": {
            "file_path": file_path.to_string_lossy(),
            "old_string": original,
            "new_string": edited,
        },
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let input_str = hook_input.to_string();

    let bin_path = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = Command::new(bin_path)
        .current_dir(dir)
        .env("POSTTOOL_DRY_RUN", "1")
        .env("DEBUG_HOOKS", "true")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn posttooluse");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(input_str.as_bytes()).expect("stdin write");
    }

    let output = child.wait_with_output().expect("wait output");
    assert!(output.status.success(), "posttooluse exited with error in dry run");

    // additionalContext should include AST insights
    let stdout_str = String::from_utf8(output.stdout).expect("utf8 stdout");
    let v: serde_json::Value = serde_json::from_str(&stdout_str).expect("parse json");
    let ctx_str = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx_str.contains("Deterministic Score") || ctx_str.contains("Concrete Issues Found"));

    // Prompt was written to post-context.txt due to DEBUG_HOOKS
    let prompt_path = dir.join("post-context.txt");
    let prompt_text = std::fs::read_to_string(&prompt_path).expect("read prompt");
    assert!(prompt_text.contains("PROJECT_STRUCTURE:"), "project context in prompt");
    assert!(prompt_text.contains("CODE CHANGES (diff format):"), "diff context in prompt");
}

#[test]
fn e2e_posttooluse_dry_run_multiedit_prompt_diff() {
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    // Prepare prompts required by prompt loader
    let prompts = dir.join("prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(prompts.join("post_edit_validation.txt"), "Validate changes.").unwrap();
    std::fs::write(prompts.join("output_template.txt"), "TEMPLATE").unwrap();
    let file_path = dir.join("multi.py");
    let base = "print('hello')\nvalue = 1\nprint('bye')\n";
    std::fs::write(&file_path, base).expect("write base");
    // MultiEdit: change two lines
    let edits = serde_json::json!([
        {"old_string": "print('hello')", "new_string": "print('HELLO')"},
        {"old_string": "value = 1", "new_string": "value = 2"}
    ]);
    let hook_input = serde_json::json!({
        "tool_name": "MultiEdit",
        "tool_input": {"file_path": file_path.to_string_lossy(), "edits": edits},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_DRY_RUN", "1")
        .env("DEBUG_HOOKS", "true")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success());
    // Prompt should contain MultiEdit summary
    let prompt_text = std::fs::read_to_string(dir.join("post-context.txt")).expect("read prompt");
    assert!(prompt_text.contains("MultiEdit on file:"));
    assert!(prompt_text.contains("Applied") || prompt_text.contains("Edit #1"));
}

#[cfg(windows)]
#[test]
fn e2e_posttooluse_windows_transcript_backslash_path() {
    use std::io::Write;
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    // Prepare prompts
    let prompts = dir.join("prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(prompts.join("post_edit_validation.txt"), "Validate changes.").unwrap();
    std::fs::write(prompts.join("output_template.txt"), "TEMPLATE").unwrap();
    // Transcript JSONL
    let transcript = dir.join("transcript.jsonl");
    std::fs::write(&transcript, "{\"role\":\"user\",\"content\":\"Do it\"}\n").unwrap();
    let win_transcript = transcript.to_string_lossy().replace('/', "\\");
    // Code file
    let file_path = dir.join("file.py");
    std::fs::write(&file_path, "print('x')\n").unwrap();
    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": "print('y')\n"},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse",
        "transcript_path": win_transcript
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_DRY_RUN", "1")
        .env("DEBUG_HOOKS", "true")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success());
}

#[test]
fn e2e_posttooluse_write_aliases() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();

    // Use WriteFile alias
    let file_path = dir.join("alias.ts");
    let code = "function f(){ return 1 }\n";
    let hook_input = serde_json::json!({
        "tool_name": "WriteFile",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": code},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    // additionalContext must be a string; may be empty for clean code
    assert!(ctx.is_empty() || ctx.contains("Deterministic") || ctx.contains("Concrete Issues"));
}

#[test]
fn e2e_posttooluse_append_alias() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    let file_path = dir.join("append.py");
    std::fs::write(&file_path, "print('a')\n").unwrap();
    let hook_input = serde_json::json!({
        "tool_name": "AppendToFile",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": "print('b')\n"},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    // Context may be empty if no issues; ensure it is a string
    assert!(ctx.is_empty() || ctx.contains("Deterministic") || ctx.contains("Concrete Issues"));
}

#[cfg(windows)]
#[test]
fn e2e_posttooluse_windows_writefile_backslash_path() {
    use std::io::Write;
    // Windows-specific: verify backslash paths are accepted
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    let file_path = dir.join("win_alias.ts");

    // Create content to write
    let code = "function f(){ return 1 }\n";

    // Build backslash file path string
    let win_path = file_path.to_string_lossy().replace('/', "\\");

    let hook_input = serde_json::json!({
        "tool_name": "WriteFile",
        "tool_input": {"file_path": win_path, "content": code},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "posttooluse should succeed with backslash path");
}

#[cfg(windows)]
#[test]
fn e2e_posttooluse_windows_append_backslash_path() {
    use std::io::Write;
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    let file_path = dir.join("append_win.py");
    std::fs::write(&file_path, "print('a')\n").unwrap();
    let win_path = file_path.to_string_lossy().replace('/', "\\");
    let hook_input = serde_json::json!({
        "tool_name": "AppendToFile",
        "tool_input": {"file_path": win_path, "content": "print('b')\n"},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
}

#[test]
fn e2e_posttooluse_dry_run_with_transcript_and_limit() {
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    // Prepare prompts to allow prompt building
    let prompts = dir.join("prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(prompts.join("post_edit_validation.txt"), "Validate changes.").unwrap();
    std::fs::write(prompts.join("output_template.txt"), "TEMPLATE").unwrap();
    // Code with many long lines to inflate context
    let file_path = dir.join("long.py");
    let long_line = "x".repeat(200);
    let code = format!("{}\n{}\n{}\n", long_line, long_line, long_line);
    std::fs::write(&file_path, &code).unwrap();
    // Transcript JSONL
    let transcript = dir.join("transcript.jsonl");
    std::fs::write(&transcript, "{\"role\":\"user\",\"content\":\"Do it\"}\n{\"role\":\"assistant\",\"content\":\"Ok\"}\n").unwrap();
    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": code},
        "cwd": dir.to_string_lossy(),
        "transcript_path": transcript.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_DRY_RUN", "1")
        .env("DEBUG_HOOKS", "true")
        .env("ADDITIONAL_CONTEXT_LIMIT_CHARS", "10000")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success());
    // Verify inclusion of transcript in prompt
    let prompt_text = std::fs::read_to_string(dir.join("post-context.txt")).expect("read prompt");
    assert!(prompt_text.contains("CONVERSATION CONTEXT:"));
    assert!(prompt_text.contains("Current user task:"));
    // Verify additionalContext bounded
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.len() <= 10_000, "context length {} exceeds limit", ctx.len());
}

#[test]
fn e2e_posttooluse_dry_run_additional_context_limit_enforced() {
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    let prompts = dir.join("prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(prompts.join("post_edit_validation.txt"), "Validate changes.").unwrap();
    std::fs::write(prompts.join("output_template.txt"), "TEMPLATE").unwrap();
    // Create file with many long lines to blow up context
    let file_path = dir.join("big.ts");
    let long_line = "x".repeat(500);
    let mut code = String::new();
    for _ in 0..200 { code.push_str(&long_line); code.push('\n'); }
    std::fs::write(&file_path, &code).unwrap();
    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": code},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_DRY_RUN", "1")
        .env("ADDITIONAL_CONTEXT_LIMIT_CHARS", "10000")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.len() <= 10100, "context must be limited by ADDITIONAL_CONTEXT_LIMIT_CHARS lower bound");
}

#[test]
fn e2e_posttooluse_pass_through_non_code_file() {
    // .md should bypass analysis and return empty additionalContext
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    let file_path = dir.join("README.md");
    std::fs::write(&file_path, "# Title\n").unwrap();

    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": file_path.to_string_lossy(), "content": "# Title"},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });

    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.is_empty(), "non-code file must pass-through with empty context");
}

#[test]
fn e2e_posttooluse_path_validation_fallback_to_tool_input() {
    // Invalid path (traversal) should fail file read, then fallback to tool_input content
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    // Construct a traversal-like path (works as a string trigger for validator)
    let bad_path = "../evil.py"; // validator forbids ".."
    let insecure = "password = 'bad'\nprint('x')\n"; // triggers AST credential rule
    let hook_input = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": {"file_path": bad_path, "old_string": "print('x')", "new_string": insecure},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_AST_ONLY", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.contains("Deterministic Score") || ctx.contains("Concrete Issues"));
}

#[test]
fn e2e_posttooluse_dry_run_edit_diff_contains_markers() {
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    let prompts = dir.join("prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(prompts.join("post_edit_validation.txt"), "Validate changes.").unwrap();
    std::fs::write(prompts.join("output_template.txt"), "TEMPLATE").unwrap();
    let file_path = dir.join("edit.py");
    let before = "print('A')\nprint('B')\n";
    let after = "print('A!')\nprint('B')\n";
    std::fs::write(&file_path, after).unwrap(); // post-tool: file already modified
    let hook_input = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": {"file_path": file_path.to_string_lossy(), "old_string": before, "new_string": after},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_DRY_RUN", "1")
        .env("DEBUG_HOOKS", "true")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let prompt = std::fs::read_to_string(dir.join("post-context.txt")).expect("read prompt");
    // Expect unified diff markers of old/new content in the prompt
    assert!(prompt.contains("CODE CHANGES (diff format):"));
    assert!(prompt.contains("-print('A')") || prompt.contains("- print('A')"));
    assert!(prompt.contains("+print('A!')") || prompt.contains("+ print('A!')"));
}

#[test]
fn e2e_posttooluse_ast_only_max_issues_cap() {
    // Ensure AST_MAX_ISSUES caps issues and prints truncation note
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    let file_path = dir.join("many.py");
    // Produce multiple long-line issues (>120 chars)
    let long = "y".repeat(130);
    let mut code = String::new();
    for _ in 0..20 { code.push_str(&format!("{}\n", long)); }
    std::fs::write(&file_path, &code).unwrap();
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
        .env("AST_MAX_ISSUES", "5") // will clamp to minimum 10
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    let bullets = ctx.matches("• Line ").count();
    assert_eq!(bullets, 10, "should be capped to 10 issues (min clamp)");
    assert!(ctx.contains("truncated: showing 10 of 20"));
}

#[test]
fn e2e_posttooluse_dry_run_project_ast_skip_large() {
    // Create a large file (>500k) to trigger skip in project-wide AST analysis
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    // Prompts
    let prompts = dir.join("prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(prompts.join("post_edit_validation.txt"), "Validate").unwrap();
    std::fs::write(prompts.join("output_template.txt"), "T").unwrap();
    // Big file
    let big_path = dir.join("big.py");
    let chunk = "a".repeat(1024);
    let mut f = std::fs::File::create(&big_path).unwrap();
    for _ in 0..600 { use std::io::Write; f.write_all(chunk.as_bytes()).unwrap(); }
    // A normal code file to trigger hook
    let code_path = dir.join("ok.py");
    std::fs::write(&code_path, "print('ok')\n").unwrap();
    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": code_path.to_string_lossy(), "content": "print('ok')\n"},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_DRY_RUN", "1")
        .env("DEBUG_HOOKS", "true")
        .env("AST_TIMINGS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let prompt = std::fs::read_to_string(dir.join("post-context.txt")).expect("read prompt");
    assert!(prompt.contains("PROJECT-WIDE AST ANALYSIS"));
    assert!(prompt.contains("Skipped (too large): 1"), "should report 1 large file skipped");
}

#[test]
fn e2e_posttooluse_pass_through_non_modifying_tool() {
    // Tools other than Write/Edit/MultiEdit must pass through
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    let file_path = dir.join("noop.py");
    std::fs::write(&file_path, "print('noop')\n").unwrap();
    let hook_input = serde_json::json!({
        "tool_name": "ReadFile", // non-modifying
        "tool_input": {"file_path": file_path.to_string_lossy()},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.is_empty());
}

#[test]
fn e2e_posttooluse_ast_only_percent_encoded_path_fallback() {
    // Percent-encoded path should be rejected by validator; fallback to tool_input
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    let bad_path = "file%2ename.py";
    let content = "print('ok')\n";
    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": bad_path, "content": content},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_AST_ONLY", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.contains("Deterministic Score") || ctx.contains("Concrete Issues"));
}

#[test]
fn e2e_posttooluse_pass_through_json_toml() {
    // JSON/TOML should be treated as non-code for posttooluse and pass-through
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    let json_path = dir.join("config.json");
    let toml_path = dir.join("pyproject.toml");
    std::fs::write(&json_path, "{\"a\":1}").unwrap();
    std::fs::write(&toml_path, "[tool]\nname='x'\n").unwrap();

    for (tool_name, file_path, content) in [
        ("Write", json_path.clone(), "{\"a\":1}".to_string()),
        ("Write", toml_path.clone(), "[tool]\nname='x'\n".to_string()),
    ] {
        let hook_input = serde_json::json!({
            "tool_name": tool_name,
            "tool_input": {"file_path": file_path.to_string_lossy(), "content": content},
            "cwd": dir.to_string_lossy(),
            "hook_event_name": "PostToolUse"
        });
        let bin = env!("CARGO_BIN_EXE_posttooluse");
        let mut child = std::process::Command::new(bin)
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn().expect("spawn");
        child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
        assert!(ctx.is_empty());
    }
}

#[test]
fn e2e_posttooluse_ast_only_windows_backslashes_path_fallback() {
    // Backslash-heavy path should be accepted, but if invalid (UNC style in non-Windows) fallback to tool_input works
    let temp = tempdir().expect("tempdir");
    let dir = temp.path();
    // Simulate UNC path
    let unc = r"\\server\share\file.py";
    let code = "print('ok')\n";
    let hook_input = serde_json::json!({
        "tool_name": "Write",
        "tool_input": {"file_path": unc, "content": code},
        "cwd": dir.to_string_lossy(),
        "hook_event_name": "PostToolUse"
    });
    let bin = env!("CARGO_BIN_EXE_posttooluse");
    let mut child = std::process::Command::new(bin)
        .current_dir(dir)
        .env("POSTTOOL_AST_ONLY", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn");
    child.stdin.as_mut().unwrap().write_all(hook_input.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert!(ctx.contains("Deterministic Score") || ctx.contains("Concrete Issues"));
}
