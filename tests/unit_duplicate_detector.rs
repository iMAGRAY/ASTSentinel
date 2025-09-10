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

    // If multiple ExactDuplicate groups exist, the largest (by total size) should appear first among that type
    // Create two more exact duplicate groups with different sizes
    std::fs::write(root.join("big1.bin"), vec![1u8; 2048]).unwrap();
    std::fs::write(root.join("big2.bin"), vec![1u8; 2048]).unwrap();
    std::fs::write(root.join("small1.bin"), vec![2u8; 16]).unwrap();
    std::fs::write(root.join("small2.bin"), vec![2u8; 16]).unwrap();

    let mut det2 = DuplicateDetector::new();
    det2.scan_directory(root).expect("scan2");
    let groups2 = det2.find_duplicates();
    // Collect exact groups and check that the first has the largest sum size
    let exact_groups: Vec<_> = groups2
        .iter()
        .filter(|g| g.conflict_type == ConflictType::ExactDuplicate)
        .collect();
    assert!(exact_groups.len() >= 2);
    let sum_sizes = |g: &&rust_validation_hooks::analysis::duplicate_detector::DuplicateGroup| -> u64 {
        g.files.iter().map(|f| f.size).sum()
    };
    let first_sum = sum_sizes(&exact_groups[0]);
    for g in &exact_groups[1..] {
        assert!(first_sum >= sum_sizes(g));
    }
}
