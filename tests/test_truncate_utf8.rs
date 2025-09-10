use rust_validation_hooks::truncate_utf8_safe;

#[test]
fn truncate_utf8_safe_preserves_char_boundaries_and_adds_ellipsis() {
    // Mix of ASCII, multi-byte (emoji), and zero-width chars
    let s = "Hello\u{200B} 🌍🚀!"; // includes zero-width space
    // visible chars without zero-width: H e l l o   🌍 🚀 ! (9 visible)
    let t = truncate_utf8_safe(s, 5);
    // Expect 4 visible chars + ellipsis
    assert!(t.ends_with('…'));
    // Ensure no replacement characters present
    assert!(!t.contains('\u{FFFD}'));
}

#[test]
fn truncate_utf8_safe_no_truncate_when_short() {
    let s = "Привет";
    let t = truncate_utf8_safe(s, 10);
    assert_eq!(t, s);
}

