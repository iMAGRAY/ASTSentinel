/// Diff formatter for showing code changes in unified diff format
/// This helps AI understand exactly what changes are being made
use std::cmp::{max, min};

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

/// Find the starting position of a line sequence in content lines
fn find_line_sequence(content_lines: &[&str], target_lines: &[&str]) -> Option<usize> {
    if target_lines.is_empty() {
        return None;
    }

    for (i, window) in content_lines.windows(target_lines.len()).enumerate() {
        if window == target_lines {
            return Some(i);
        }
    }

    None
}

/// Maximum file size for full context (100KB)
const MAX_FILE_SIZE_FOR_FULL_CONTEXT: usize = 100_000;

/// Format simple unified diff for Edit operations  
pub fn format_edit_as_unified_diff(
    file_path: &str,
    file_content: Option<&str>,
    old_string: &str,
    new_string: &str,
) -> String {
    // Pre-allocate capacity for better performance
    let estimated_size = old_string.len() + new_string.len() + 200;
    let mut result = String::with_capacity(estimated_size);

    // Basic unified diff header
    result.push_str(&format!("--- a/{}\n", file_path));
    result.push_str(&format!("+++ b/{}\n", file_path));

    // Try to find context in the actual file content
    if let Some(content) = file_content {
        // Since posttooluse runs AFTER edit, new_string should be in the modified content
        // We need to find where the change occurred by looking for lines that match new_string

        let content_lines: Vec<&str> = content.lines().collect();
        let new_lines: Vec<&str> = new_string.lines().collect();
        let old_lines: Vec<&str> = old_string.lines().collect();

        // Find the position where new_lines start in content_lines
        if let Some(change_start_idx) = find_line_sequence(&content_lines, &new_lines) {
            // Calculate context
            let context_before = 3;
            let context_after = 3;
            let start_line = change_start_idx.saturating_sub(context_before);
            let end_line =
                (change_start_idx + new_lines.len() + context_after).min(content_lines.len());

            // Generate proper unified diff hunk header
            let old_start = start_line + 1;
            let old_count =
                (end_line - start_line).saturating_sub(new_lines.len()) + old_lines.len();
            let new_start = start_line + 1;
            let new_count = end_line - start_line;

            result.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                old_start, old_count, new_start, new_count
            ));

            // Show context before change
            for i in start_line..change_start_idx {
                if i < content_lines.len() {
                    result.push_str(&format!(" {}\n", content_lines[i]));
                }
            }

            // Show the actual change
            for line in old_lines.iter() {
                result.push_str(&format!("-{}\n", line));
            }
            for line in new_lines.iter() {
                result.push_str(&format!("+{}\n", line));
            }

            // Show context after change
            let after_start = change_start_idx + new_lines.len();
            for i in after_start..end_line {
                if i < content_lines.len() {
                    result.push_str(&format!(" {}\n", content_lines[i]));
                }
            }
        } else {
            // Fallback: couldn't find new_string, show simple diff
            result.push_str(&format!(
                "@@ -1,{} +1,{} @@\n",
                old_string.lines().count().max(1),
                new_string.lines().count().max(1)
            ));

            for line in old_string.lines() {
                result.push_str(&format!("-{}\n", line));
            }
            for line in new_string.lines() {
                result.push_str(&format!("+{}\n", line));
            }
        }
    } else {
        // No file content available, show simple diff
        result.push_str(&format!(
            "@@ -1,{} +1,{} @@\n",
            old_string.lines().count().max(1),
            new_string.lines().count().max(1)
        ));

        for line in old_string.lines() {
            result.push_str(&format!("-{}\n", line));
        }
        for line in new_string.lines() {
            result.push_str(&format!("+{}\n", line));
        }
    }

    result
}

/// Generate clean unified diff between two contents
pub fn format_simple_unified_diff(file_path: &str, old_content: &str, new_content: &str) -> String {
    use std::cmp::min;

    let mut result = String::new();
    result.push_str(&format!("--- a/{}\n", file_path));
    result.push_str(&format!("+++ b/{}\n", file_path));

    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    // Find changes
    let mut hunks = Vec::new();
    let mut i = 0;
    let max_len = std::cmp::max(old_lines.len(), new_lines.len());

    while i < max_len {
        let old_line = old_lines.get(i);
        let new_line = new_lines.get(i);

        if old_line != new_line {
            // Found a difference - create hunk
            let hunk_start = i;
            let mut hunk_end = i + 1;

            // Extend hunk to include consecutive changes
            while hunk_end < max_len {
                let old_next = old_lines.get(hunk_end);
                let new_next = new_lines.get(hunk_end);
                if old_next == new_next {
                    break;
                }
                hunk_end += 1;
            }

            hunks.push((hunk_start, hunk_end));
            i = hunk_end;
        } else {
            i += 1;
        }
    }

    // Generate hunks
    for (hunk_start, hunk_end) in &hunks {
        let context_before = 3;
        let context_after = 3;

        let start_line = hunk_start.saturating_sub(context_before);
        let end_line = min(max_len, hunk_end.saturating_add(context_after));

        let old_count = min(old_lines.len(), end_line) - start_line;
        let new_count = min(new_lines.len(), end_line) - start_line;

        result.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            start_line + 1,
            old_count,
            start_line + 1,
            new_count
        ));

        for line_idx in start_line..end_line {
            let old_line = old_lines.get(line_idx);
            let new_line = new_lines.get(line_idx);

            match (old_line, new_line) {
                (Some(old), Some(new)) if old == new => {
                    result.push_str(&format!(" {}\n", old));
                }
                (Some(old), Some(new)) => {
                    result.push_str(&format!("-{}\n", old));
                    result.push_str(&format!("+{}\n", new));
                }
                (Some(old), None) => {
                    result.push_str(&format!("-{}\n", old));
                }
                (None, Some(new)) => {
                    result.push_str(&format!("+{}\n", new));
                }
                (None, None) => {} // Shouldn't happen in this range
            }
        }
    }

    if hunks.is_empty() {
        result.push_str("@@ No changes @@\n");
    }

    result
}

/// Format MultiEdit changes with full file context
pub fn format_multi_edit_full_context(
    file_path: &str,
    file_content: Option<&str>,
    edits: &[(String, String)], // Vec of (old_string, new_string) pairs
) -> String {
    let mut result = String::with_capacity(1024);

    result.push_str(&format!("=== MultiEdit on file: {} ===\n", file_path));

    if let Some(content) = file_content {
        // Apply all edits to get the final content
        let mut modified_content = content.to_string();
        let mut successful_edits = 0;
        let mut failed_edits = Vec::new();

        for (i, (old_str, new_str)) in edits.iter().enumerate() {
            if modified_content.contains(old_str) {
                modified_content = modified_content.replace(old_str, new_str);
                successful_edits += 1;
            } else {
                failed_edits.push(i + 1);
            }
        }

        // Report any failed edits
        if !failed_edits.is_empty() {
            result.push_str(&format!(
                "⚠️ {} edit(s) could not be applied (string not found): {:?}\n\n",
                failed_edits.len(),
                failed_edits
            ));
        }

        // Show full file with all changes
        result.push_str(&format!(
            "Applied {} of {} edits:\n\n",
            successful_edits,
            edits.len()
        ));
        result.push_str(&format_full_file_with_changes(
            file_path,
            Some(content),
            Some(&modified_content),
        ));
    } else {
        // No file content available, list the edits
        result.push_str("File content not available. Edits to apply:\n");
        for (i, (old_str, new_str)) in edits.iter().enumerate() {
            result.push_str(&format!("\nEdit #{}:\n", i + 1));
            result.push_str(&format!(
                "  - Replace: '{}'\n",
                truncate_for_display(old_str, 100)
            ));
            result.push_str(&format!(
                "  + With:    '{}'\n",
                truncate_for_display(new_str, 100)
            ));
        }
    }

    result.push_str(&format!("\n=== End of {} ===\n", file_path));
    result
}

/// Truncate string for display purposes (UTF-8 safe)
pub fn truncate_for_display(s: &str, max_len: usize) -> String {
    const ELLIPSIS: &str = "...";
    const ELLIPSIS_LEN: usize = 3;

    // Handle edge cases
    if max_len == 0 || s.is_empty() {
        return String::new();
    }

    if s.len() <= max_len {
        return s.to_string();
    }

    // Not enough space for ellipsis, just truncate
    if max_len <= ELLIPSIS_LEN {
        let mut char_count = 0;
        let mut byte_count = 0;

        for ch in s.chars() {
            let ch_len = ch.len_utf8();
            if byte_count + ch_len > max_len {
                break;
            }
            byte_count += ch_len;
            char_count += 1;
        }

        return s.chars().take(char_count).collect();
    }

    // Normal case: truncate and add ellipsis
    let content_max_len = max_len.saturating_sub(ELLIPSIS_LEN);

    // Count chars that fit within the byte limit
    let mut byte_count = 0;
    let mut char_boundary = 0;

    for (i, ch) in s.char_indices() {
        let ch_len = ch.len_utf8();
        if byte_count + ch_len > content_max_len {
            char_boundary = i;
            break;
        }
        byte_count += ch_len;
        char_boundary = i + ch_len;
    }

    // Handle special case: if we have room for at least content_max_len bytes
    // Ensure we use that space
    if char_boundary > 0 {
        let mut result = String::from(&s[..char_boundary]);
        result.push_str(ELLIPSIS);
        result
    } else {
        // Edge case: even first character doesn't fit
        ELLIPSIS.to_string()
    }
}

/// Format a single line with line number and change marker
#[inline]
fn format_line(line_num: usize, marker: &str, content: &str) -> String {
    // Ensure consistent spacing: "+" or "-" or "  " for unchanged
    let padded_marker = if marker == " " {
        "  ".to_string()
    } else {
        format!("{} ", marker)
    };
    format!("{:4} {}{}\n", line_num, padded_marker, content)
}

/// Safely truncate content at line boundary to avoid splitting UTF-8 chars
fn truncate_at_line_boundary(content: &str, max_size: usize) -> &str {
    if content.len() <= max_size {
        return content;
    }

    // Find the last newline before max_size
    let truncate_pos = content[..max_size].rfind('\n').unwrap_or_else(|| {
        // If no newline found, truncate at last valid UTF-8 boundary
        let mut pos = max_size;
        while pos > 0 && !content.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    });

    &content[..truncate_pos]
}

/// Format the entire file content with diff markers showing changes
/// This provides full context for AI analysis with performance optimizations
pub fn format_full_file_with_changes(
    file_path: &str,
    original_content: Option<&str>,
    modified_content: Option<&str>,
) -> String {
    // Safely truncate large files at line boundaries
    let (original, modified, was_truncated) = match (original_content, modified_content) {
        (Some(o), Some(m))
            if o.len() > MAX_FILE_SIZE_FOR_FULL_CONTEXT
                || m.len() > MAX_FILE_SIZE_FOR_FULL_CONTEXT =>
        {
            let truncated_o = truncate_at_line_boundary(o, MAX_FILE_SIZE_FOR_FULL_CONTEXT);
            let truncated_m = truncate_at_line_boundary(m, MAX_FILE_SIZE_FOR_FULL_CONTEXT);
            (Some(truncated_o), Some(truncated_m), true)
        }
        (o, m) => (o, m, false),
    };

    // Pre-allocate capacity for better performance
    let estimated_size =
        original.map(|s| s.len()).unwrap_or(0) + modified.map(|s| s.len()).unwrap_or(0) + 200; // Extra space for headers
    let mut result = String::with_capacity(estimated_size);

    // Add file header
    result.push_str(&format!("=== Full file: {} ===\n", file_path));

    // Add warning for large files
    if was_truncated {
        result.push_str("⚠️ File truncated for display (exceeds 100KB)\n\n");
    }

    match (original, modified) {
        (None, Some(new)) if new.is_empty() => {
            result.push_str("(New empty file)\n");
        }
        (None, Some(new)) => {
            // New file - show all lines as additions
            result.push_str("(New file)\n\n");
            for (i, line) in new.lines().enumerate() {
                result.push_str(&format_line(i + 1, "+", line));
            }
        }
        (Some(old), None) if old.is_empty() => {
            result.push_str("(Empty file deleted)\n");
        }
        (Some(old), None) => {
            // File deletion - show all lines as deletions
            result.push_str("(File deleted)\n\n");
            for (i, line) in old.lines().enumerate() {
                result.push_str(&format_line(i + 1, "-", line));
            }
        }
        (Some(old), Some(new)) => {
            // File modification - show full file with changes marked
            let old_lines: Vec<&str> = old.lines().collect();
            let new_lines: Vec<&str> = new.lines().collect();

            // Simple line-by-line diff for full file view
            let max_lines = std::cmp::max(old_lines.len(), new_lines.len());
            let mut line_num = 1;

            for i in 0..max_lines {
                match (old_lines.get(i), new_lines.get(i)) {
                    (Some(old_line), Some(new_line)) if old_line == new_line => {
                        // Unchanged line
                        result.push_str(&format_line(line_num, " ", old_line));
                        line_num += 1;
                    }
                    (Some(old_line), Some(new_line)) => {
                        // Changed line - show both old and new
                        result.push_str(&format_line(line_num, "-", old_line));
                        result.push_str(&format_line(line_num, "+", new_line));
                        line_num += 1;
                    }
                    (Some(old_line), None) => {
                        // Line deleted
                        result.push_str(&format_line(line_num, "-", old_line));
                    }
                    (None, Some(new_line)) => {
                        // Line added
                        result.push_str(&format_line(line_num, "+", new_line));
                        line_num += 1;
                    }
                    _ => {}
                }
            }
        }
        (None, None) => {
            result.push_str("(No content)\n");
        }
    }

    result.push_str(&format!("\n=== End of {} ===\n", file_path));
    result
}

/// Format Edit operation showing full file with changes inline
pub fn format_edit_full_context(
    file_path: &str,
    file_content: Option<&str>,
    old_string: &str,
    new_string: &str,
) -> String {
    let mut result = String::new();

    result.push_str(&format!(
        "=== Full file with Edit changes: {} ===\n",
        file_path
    ));

    if let Some(content) = file_content {
        // Apply the edit to get the new content
        if let Some(pos) = content.find(old_string) {
            let mut new_content = String::new();
            new_content.push_str(&content[..pos]);
            new_content.push_str(new_string);
            new_content.push_str(&content[pos + old_string.len()..]);

            // Now show the full file with changes
            for (i, (old_line, new_line)) in content.lines().zip(new_content.lines()).enumerate() {
                if old_line == new_line {
                    result.push_str(&format!("{:4}   {}\n", i + 1, old_line));
                } else {
                    result.push_str(&format!("{:4} - {}\n", i + 1, old_line));
                    result.push_str(&format!("{:4} + {}\n", i + 1, new_line));
                }
            }

            // Handle any extra lines in the new content
            let old_line_count = content.lines().count();
            let new_line_count = new_content.lines().count();

            if new_line_count > old_line_count {
                for (i, line) in new_content.lines().skip(old_line_count).enumerate() {
                    result.push_str(&format!("{:4} + {}\n", old_line_count + i + 1, line));
                }
            }
        } else {
            // Old string not found, show original file
            result.push_str("(Edit target not found, showing original file)\n\n");
            for (i, line) in content.lines().enumerate() {
                result.push_str(&format!("{:4}   {}\n", i + 1, line));
            }
        }
    } else {
        result.push_str("(File content not available)\n");
    }

    result.push_str(&format!("\n=== End of {} ===\n", file_path));
    result
}

/// Format MultiEdit operations as a unified diff
pub fn format_multi_edit_diff(
    file_path: &str,
    file_content: Option<&str>,
    edits: &[(String, String)], // Vec of (old_string, new_string)
) -> String {
    let mut result = String::new();

    // Check if file content is available
    if file_content.is_none() {
        result.push_str("File content not available\n");
        return result;
    }

    // Apply edits sequentially to show cumulative changes
    let mut current_content = file_content.unwrap_or("").to_string();

    result.push_str(&format!("--- {}\n", file_path));
    result.push_str(&format!("+++ {} (modified)\n", file_path));
    result.push_str(&format!("@@ {} edit operations @@\n", edits.len()));

    for (i, (old_str, new_str)) in edits.iter().enumerate() {
        result.push_str(&format!("\n== Edit #{} ==\n", i + 1));

        if old_str.is_empty() {
            // Special case for empty old_string
            result.push_str(&format!(
                "  ! Edit #{} failed: empty search string\n",
                i + 1
            ));
        } else if let Some(pos) = current_content.find(old_str) {
            // Show the specific change
            let line_num = current_content[..pos].lines().count() + 1;

            result.push_str(&format!("@@ Line {} @@\n", line_num));
            for line in old_str.lines() {
                result.push_str(&format!("  - {}\n", line));
            }
            for line in new_str.lines() {
                result.push_str(&format!("  + {}\n", line));
            }
            result.push_str("  Applied successfully\n");

            // Apply the edit to current content for next iteration
            current_content.replace_range(pos..pos + old_str.len(), new_str);
        } else {
            result.push_str(&format!(
                "  ! Edit #{} failed: String not found: \"{}\"\n",
                i + 1,
                truncate_for_display(old_str, 50)
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

        let diff = format_edit_diff("test.txt", Some(file_content), old_string, new_string, 2);

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

    #[test]
    fn test_truncate_for_display_short_string() {
        let short = "Hello";
        let result = truncate_for_display(short, 10);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_truncate_for_display_exact_length() {
        let exact = "Hello World";
        let result = truncate_for_display(exact, 11);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_truncate_for_display_long_string() {
        let long = "This is a very long string that should be truncated";
        let result = truncate_for_display(long, 20);
        // max_len=20, ellipsis takes 3, so 17 chars + "..."
        assert_eq!(result, "This is a very lo...");
    }

    #[test]
    fn test_truncate_for_display_utf8() {
        let russian = "Привет мир это тестовая строка с русскими буквами";
        let result = truncate_for_display(russian, 20);
        // max_len=20, ellipsis takes 3, so 17 chars + "..."
        // Each Cyrillic char is 2 bytes but we count by chars
        assert_eq!(result, "Привет ми...");
    }

    #[test]
    fn test_truncate_for_display_emoji() {
        let emoji = "Hello 👋 World 🌍 Test";
        let result = truncate_for_display(emoji, 15);
        // max_len=15, ellipsis takes 3, so 12 chars + "..."
        // Emoji counts as 1 char even though it's 4 bytes
        assert_eq!(result, "Hello 👋 W...");
    }

    #[test]
    fn test_truncate_for_display_mixed_utf8() {
        let mixed = "Test тест 测试 テスト";
        let result = truncate_for_display(mixed, 15);
        // 15 chars limit means we can fit "Test тес" (8 chars) + "..." (3 chars)
        assert_eq!(result, "Test тес...");
    }

    #[test]
    fn test_truncate_for_display_zero_length() {
        let text = "Hello World";
        let result = truncate_for_display(text, 0);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_for_display_boundary_cases() {
        // Test truncation at multi-byte character boundary
        let text = "абвгдеёжзийклмнопрстуфхцчшщъыьэюя";
        let result = truncate_for_display(text, 10);
        // max_len=10, ellipsis takes 3, so 7 chars + "..."
        assert_eq!(result, "абв...");
    }
}
