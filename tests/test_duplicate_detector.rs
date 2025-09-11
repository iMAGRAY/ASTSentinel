use rust_validation_hooks::analysis::duplicate_detector::{ConflictType, DuplicateDetector};

#[test]
fn duplicate_detector_finds_exact_and_version_conflicts() {
    use std::fs;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // Create structure with duplicates and version-like names
    let a = root.join("a");
    let b = root.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    let file1 = a.join("config.json");
    let file2 = b.join("config_copy.json"); // same content
    let file3 = b.join("config_old.json"); // slightly different -> version conflict

    fs::write(&file1, "{\"k\":1}\n").unwrap();
    fs::write(&file2, "{\"k\":1}\n").unwrap();
    fs::write(&file3, "{\"k\":2}\n").unwrap();

    let mut det = DuplicateDetector::new();
    det.scan_directory(root).unwrap();
    let groups = det.find_duplicates();

    // Expect at least one exact duplicate group and one version/similar group
    assert!(groups
        .iter()
        .any(|g| g.conflict_type == ConflictType::ExactDuplicate));
    assert!(groups.iter().any(|g| matches!(
        g.conflict_type,
        ConflictType::VersionConflict | ConflictType::SimilarName
    )));

    // Format report should include our file names
    let report = det.format_report(&groups);
    assert!(report.contains("config.json"));
}
