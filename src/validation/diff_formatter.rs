/// Diff formatter for showing code changes in unified diff format
/// This helps AI understand exactly what changes are being made

use std::cmp::{min, max};

/// Format code changes as unified diff for AI context
pub fn format_code_diff(
    file_path: &str,
    old_content: Option<&str>,
    new_content: Option<&str>,
    context_lines: usize,
) -> String {
    let mut result = String::new();
    
    // Add file header
    result.push_str(&format!("--- {}\n", file_path));
    result.push_str(&format!("+++ {} (modified)\n", file_path));
    
    match (old_content, new_content) {
        (None, Some(new)) => {
            // New file creation
            result.push_str("@@ -0,0 +1,");
            let lines: Vec<&str> = new.lines().collect();
            result.push_str(&format!("{} @@\n", lines.len()));
            
            for (i, line) in lines.iter().enumerate() {
                result.push_str(&format!("{} + {}\n", i + 1, line));
            }
        }
        (Some(old), None) => {
            // File deletion
            let lines: Vec<&str> = old.lines().collect();
            result.push_str(&format!("@@ -1,{} +0,0 @@\n", lines.len()));
            
            for (i, line) in lines.iter().enumerate() {
                result.push_str(&format!("{} - {}\n", i + 1, line));
            }
        }
        (Some(old), Some(new)) => {
            // File modification - compute diff
            let diff = compute_line_diff(old, new, context_lines);
            result.push_str(&diff);
        }
        (None, None) => {
            result.push_str("@@ No changes @@\n");
        }
    }
    
    result
}

/// Format Edit operation (old_string -> new_string) as diff
pub fn format_edit_diff(
    file_path: &str,
    file_content: Option<&str>,
    old_string: &str,
    new_string: &str,
    context_lines: usize,
) -> String {
    let mut result = String::new();
    
    // Add file header
    result.push_str(&format!("--- {}\n", file_path));
    result.push_str(&format!("+++ {} (modified)\n", file_path));
    
    if let Some(content) = file_content {
        // Find the location of old_string in the file
        if let Some(pos) = content.find(old_string) {
            let before = &content[..pos];
            let _after = &content[pos + old_string.len()..];
            
            // Count line numbers
            let line_num = before.lines().count() + 1;
            let old_lines: Vec<&str> = old_string.lines().collect();
            let new_lines: Vec<&str> = new_string.lines().collect();
            
            // Get context lines before and after
            let all_lines: Vec<&str> = content.lines().collect();
            let start_line = max(1, line_num as i32 - context_lines as i32) as usize;
            let end_line = min(all_lines.len(), line_num + old_lines.len() + context_lines);
            
            result.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                start_line,
                end_line - start_line + 1,
                start_line,
                end_line - start_line + new_lines.len() - old_lines.len() + 1
            ));
            
            // Add context before
            for i in start_line..line_num {
                if i <= all_lines.len() {
                    result.push_str(&format!("{:3}   {}\n", i, all_lines[i - 1]));
                }
            }
            
            // Add removed lines
            for (i, line) in old_lines.iter().enumerate() {
                result.push_str(&format!("{:3} - {}\n", line_num + i, line));
            }
            
            // Add added lines
            for (i, line) in new_lines.iter().enumerate() {
                result.push_str(&format!("{:3} + {}\n", line_num + i, line));
            }
            
            // Add context after
            let after_start = line_num + old_lines.len();
            for i in after_start..min(after_start + context_lines, all_lines.len() + 1) {
                if i <= all_lines.len() {
                    result.push_str(&format!("{:3}   {}\n", i, all_lines[i - 1]));
                }
            }
        } else {
            // Old string not found, show what we're trying to add
            result.push_str("@@ String not found, showing new content @@\n");
            for (i, line) in new_string.lines().enumerate() {
                result.push_str(&format!("{:3} + {}\n", i + 1, line));
            }
        }
    } else {
        // No file content available, show the change as-is
        result.push_str("@@ File content not available @@\n");
        result.push_str(&format!("- {}\n", old_string.replace('\n', "\n- ")));
        result.push_str(&format!("+ {}\n", new_string.replace('\n', "\n+ ")));
    }
    
    result
}

/// Compute line-by-line diff between two strings
fn compute_line_diff(old: &str, new: &str, context_lines: usize) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    
    // Simple line-by-line comparison (could use more sophisticated diff algorithm)
    let mut result = String::new();
    let mut changes = Vec::new();
    
    // Find changed regions
    let max_len = max(old_lines.len(), new_lines.len());
    let mut in_change = false;
    let mut change_start = 0;
    
    for i in 0..max_len {
        let old_line = old_lines.get(i).copied();
        let new_line = new_lines.get(i).copied();
        
        if old_line != new_line {
            if !in_change {
                change_start = max(0, i as i32 - context_lines as i32) as usize;
                in_change = true;
            }
        } else if in_change {
            let change_end = min(max_len, i + context_lines);
            changes.push((change_start, change_end));
            in_change = false;
        }
    }
    
    if in_change {
        let change_end = min(max_len, max_len + context_lines);
        changes.push((change_start, change_end));
    }
    
    // Format changes
    let has_changes = !changes.is_empty();
    
    for (start, end) in changes {
        result.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            start + 1,
            min(end - start, old_lines.len() - start),
            start + 1,
            min(end - start, new_lines.len() - start)
        ));
        
        for i in start..end {
            if i < old_lines.len() && i < new_lines.len() {
                if old_lines[i] == new_lines[i] {
                    // Context line
                    result.push_str(&format!("{:3}   {}\n", i + 1, old_lines[i]));
                } else {
                    // Changed line
                    result.push_str(&format!("{:3} - {}\n", i + 1, old_lines[i]));
                    result.push_str(&format!("{:3} + {}\n", i + 1, new_lines[i]));
                }
            } else if i < old_lines.len() {
                // Deleted line
                result.push_str(&format!("{:3} - {}\n", i + 1, old_lines[i]));
            } else if i < new_lines.len() {
                // Added line
                result.push_str(&format!("{:3} + {}\n", i + 1, new_lines[i]));
            }
        }
    }
    
    if !has_changes {
        result.push_str("@@ No changes detected @@\n");
    }
    
    result
}

/// Format MultiEdit operations as a unified diff
pub fn format_multi_edit_diff(
    file_path: &str,
    file_content: Option<&str>,
    edits: &[(String, String)], // Vec of (old_string, new_string)
    _context_lines: usize,
) -> String {
    let mut result = String::new();
    
    // Apply edits sequentially to show cumulative changes
    let mut current_content = file_content.unwrap_or("").to_string();
    
    result.push_str(&format!("--- {}\n", file_path));
    result.push_str(&format!("+++ {} (modified)\n", file_path));
    result.push_str(&format!("@@ {} edit operations @@\n", edits.len()));
    
    for (i, (old_str, new_str)) in edits.iter().enumerate() {
        result.push_str(&format!("\n== Edit {} ==\n", i + 1));
        
        if let Some(pos) = current_content.find(old_str) {
            // Show the specific change
            let line_num = current_content[..pos].lines().count() + 1;
            
            result.push_str(&format!("@@ Line {} @@\n", line_num));
            for line in old_str.lines() {
                result.push_str(&format!("  - {}\n", line));
            }
            for line in new_str.lines() {
                result.push_str(&format!("  + {}\n", line));
            }
            
            // Apply the edit to current content for next iteration
            current_content.replace_range(pos..pos + old_str.len(), new_str);
        } else {
            result.push_str(&format!("  ! String not found: \"{}\"\n", 
                if old_str.len() > 50 { 
                    format!("{}...", &old_str[..50]) 
                } else { 
                    old_str.clone() 
                }
            ));
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_edit_diff() {
        let file_content = "line 1\nline 2\nline 3\nline 4\nline 5";
        let old_string = "line 3";
        let new_string = "modified line 3";
        
        let diff = format_edit_diff(
            "test.txt",
            Some(file_content),
            old_string,
            new_string,
            2
        );
        
        assert!(diff.contains("--- test.txt"));
        assert!(diff.contains("+++ test.txt (modified)"));
        assert!(diff.contains("- line 3"));
        assert!(diff.contains("+ modified line 3"));
    }
    
    #[test]
    fn test_format_code_diff() {
        let old = "line 1\nline 2\nline 3";
        let new = "line 1\nmodified line 2\nline 3\nline 4";
        
        let diff = format_code_diff("test.txt", Some(old), Some(new), 1);
        
        assert!(diff.contains("--- test.txt"));
        assert!(diff.contains("- line 2"));
        assert!(diff.contains("+ modified line 2"));
        assert!(diff.contains("+ line 4"));
    }
}