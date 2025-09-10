use rust_validation_hooks::analysis::duplicate_detector::{ConflictType, DuplicateDetector};

#[test]
fn unit_duplicate_detector_finds_duplicates_and_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Exact duplicate pair
    std::fs::write(root.join("a.txt"), b"hello world\n").unwrap();
    std::fs::write(root.join("copy_of_a.txt"), b"hello world\n").unwrap();

    // Version conflict pair (different content, similar cleaned stem)
    std::fs::write(root.join("report.txt"), b"v2 content\n").unwrap();
    std::fs::write(root.join("report_old.txt"), b"old content\n").unwrap();

    // Run detector
    let mut det = DuplicateDetector::new();
    det.scan_directory(root).expect("scan");
    let groups = det.find_duplicates();
    assert!(!groups.is_empty());

    let has_exact = groups.iter().any(|g| g.conflict_type == ConflictType::ExactDuplicate);
    let has_version = groups.iter().any(|g| g.conflict_type == ConflictType::VersionConflict);
    assert!(has_exact, "expected an ExactDuplicate group: {:?}", groups);
    assert!(has_version, "expected a VersionConflict group: {:?}", groups);

    // Ordering: ExactDuplicate should come first by priority
    let first_ty = &groups[0].conflict_type;
    assert_eq!(first_ty, &ConflictType::ExactDuplicate, "first group is not ExactDuplicate: {:?}", groups[0]);

    // Format report sanity
    let report = det.format_report(&groups);
    assert!(report.contains("КРИТИЧНО"));
}
