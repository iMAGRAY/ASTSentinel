use rust_validation_hooks::validation::diff_formatter::format_edit_as_unified_diff;

#[test]
fn format_edit_as_unified_diff_multiline_and_not_found_fallback() {
    // Multiline replace in content
    let file = "a\nX\nY\nb\n";
    let old = "X\nY";
    let new = "X1\nY1";
    let diff = format_edit_as_unified_diff("f.txt", Some(file), old, new);
    assert!(diff.contains("-X"));
    assert!(diff.contains("-Y"));
    assert!(diff.contains("+X1"));
    assert!(diff.contains("+Y1"));

    // Not found: show fallback hunk with old/new
    let file2 = "no match here\n";
    let diff2 = format_edit_as_unified_diff("f.txt", Some(file2), "OLD", "NEW");
    assert!(diff2.contains("-OLD"));
    assert!(diff2.contains("+NEW"));
}

#[test]
fn format_edit_as_unified_diff_handles_crlf() {
    use rust_validation_hooks::validation::diff_formatter::format_edit_as_unified_diff;
    let file = "line1\r\nline2\r\n";
    let old = "line2\r\n";
    let new = "line2_changed\r\n";
    let diff = format_edit_as_unified_diff("f.txt", Some(file), old, new);
    assert!(diff.contains("-line2"));
    assert!(diff.contains("+line2_changed"));
}
