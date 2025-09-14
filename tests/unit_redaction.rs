use rust_validation_hooks::security::redaction::redact_with_report;

#[test]
fn redact_detects_and_masks_common_secrets() {
    let src = "password = 'hunter2'\nAPI_KEY: abcdefghijklmnop1234567890abcdef\nBearer abc.def.ghi";
    let (out, n) = redact_with_report(src);
    assert!(n >= 2, "should redact at least 2 items, got {}", n);
    assert!(!out.contains("hunter2"));
    assert!(!out.contains("abcdefghijklmnop"));
    assert!(out.contains("<REDACTED"));
}
