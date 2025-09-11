#[test]
fn unit_timings_summary_produces_output() {
    std::env::set_var("AST_TIMINGS", "1");
    // Record several samples under a label
    rust_validation_hooks::analysis::timings::record("score/python", 10);
    rust_validation_hooks::analysis::timings::record("score/python", 12);
    rust_validation_hooks::analysis::timings::record("score/python", 8);
    let s = rust_validation_hooks::analysis::timings::summary();
    assert!(s.contains("=== TIMINGS (ms) ==="));
    assert!(s.contains("score/python"));
}
