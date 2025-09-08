use rust_validation_hooks::validation::diff_formatter::{
    format_edit_full_context, format_full_file_with_changes, format_multi_edit_full_context,
};
use rust_validation_hooks::{HookInput, PreToolUseHookOutput, PreToolUseOutput};
use std::io::{self, Read};

fn main() {
    // Read input from stdin
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("Failed to read input: {}", e);
        std::process::exit(1);
    }

    // Parse input
    let hook_input: HookInput = match serde_json::from_str(&input) {
        Ok(input) => input,
        Err(e) => {
            eprintln!("Failed to parse input: {}", e);
            std::process::exit(1);
        }
    };

    // Generate diff based on tool type
    let diff_output = match hook_input.tool_name.as_str() {
        "Edit" => {
            let file_path = hook_input
                .tool_input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let old_string = hook_input
                .tool_input
                .get("old_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let new_string = hook_input
                .tool_input
                .get("new_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Read the actual file if it exists
            let file_content = std::fs::read_to_string(file_path).ok();

            format_edit_full_context(file_path, file_content.as_deref(), old_string, new_string)
        }
        "Write" => {
            let file_path = hook_input
                .tool_input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let content = hook_input
                .tool_input
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Read existing file if it exists
            let existing = std::fs::read_to_string(file_path).ok();

            format_full_file_with_changes(file_path, existing.as_deref(), Some(content))
        }
        "MultiEdit" => {
            let file_path = hook_input
                .tool_input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // Parse edits array
            let edits = hook_input
                .tool_input
                .get("edits")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|edit| {
                            let old = edit.get("old_string")?.as_str()?;
                            let new = edit.get("new_string")?.as_str()?;
                            Some((old.to_string(), new.to_string()))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(Vec::new);

            // Read the actual file if it exists
            let file_content = std::fs::read_to_string(file_path).ok();

            // Use the new format_multi_edit_full_context function
            format_multi_edit_full_context(file_path, file_content.as_deref(), &edits)
        }
        _ => format!("Tool {} not supported for diff", hook_input.tool_name),
    };

    // Create output showing the diff
    let output = PreToolUseOutput {
        hook_specific_output: PreToolUseHookOutput {
            hook_event_name: "PreToolUse".to_string(),
            permission_decision: "allow".to_string(),
            permission_decision_reason: Some(format!("Diff output test:\n\n{}", diff_output)),
        },
    };

    // Output the result
    println!(
        "{}",
        serde_json::to_string(&output).unwrap_or_else(|_| "Error".to_string())
    );
}
