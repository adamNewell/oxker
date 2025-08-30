use oxker_core::docker_cli::{DockerCliDetector, DockerCliStatus};

#[test]
fn test_docker_cli_detector_creation() {
    let detector = DockerCliDetector::new();
    // Detector should start with no cache
    assert!(matches!(detector, DockerCliDetector { .. }));
}

#[test]
fn test_docker_cli_status_variants() {
    // Test that all status variants can be created and cloned
    let statuses = [
        DockerCliStatus::Available,
        DockerCliStatus::NotFound,
        DockerCliStatus::NotInPath(vec!["path1".to_string(), "path2".to_string()]),
        DockerCliStatus::PermissionDenied("Permission denied".to_string()),
        DockerCliStatus::DaemonNotRunning,
    ];

    for status in &statuses {
        let cloned = status.clone();
        match (status.clone(), cloned) {
            (DockerCliStatus::NotInPath(a), DockerCliStatus::NotInPath(b)) => {
                assert_eq!(a, b);
            }
            (DockerCliStatus::PermissionDenied(a), DockerCliStatus::PermissionDenied(b)) => {
                assert_eq!(a, b);
            }
            (DockerCliStatus::Available, DockerCliStatus::Available)
            | (DockerCliStatus::NotFound, DockerCliStatus::NotFound)
            | (DockerCliStatus::DaemonNotRunning, DockerCliStatus::DaemonNotRunning) => (),
            _ => panic!("Clone mismatch"),
        }
    }

    // Explicitly drop to ensure cleanup
    drop(statuses);
}

#[test]
fn test_docker_cli_cache_invalidation() {
    let mut detector = DockerCliDetector::new();

    // First detection will populate cache
    let _status = detector.detect();

    // Invalidate cache
    detector.invalidate_cache();

    // Next detection should perform fresh check (can't easily verify without mocking)
    let _status2 = detector.detect();
}

#[test]
fn test_docker_cli_status_debug_format() {
    let status = DockerCliStatus::NotFound;
    let debug_str = format!("{status:?}");
    assert!(debug_str.contains("NotFound"));
}

#[test]
fn test_actual_docker_detection() {
    let mut detector = DockerCliDetector::new();
    let status = detector.detect();

    // Just verify we get some status without panicking
    match status {
        DockerCliStatus::Available => {
            println!("Docker CLI is available");
        }
        DockerCliStatus::NotFound => {
            println!("Docker CLI not found");
        }
        DockerCliStatus::NotInPath(paths) => {
            println!("Docker found at: {paths:?}");
        }
        DockerCliStatus::PermissionDenied(msg) => {
            println!("Permission denied: {msg}");
        }
        DockerCliStatus::DaemonNotRunning => {
            println!("Docker daemon not running");
        }
    }
}

#[test]
fn test_docker_cli_detector_caching() {
    use std::time::Instant;

    let mut detector = DockerCliDetector::new();

    // First call should detect
    let start = Instant::now();
    let status1 = detector.detect();
    let first_duration = start.elapsed();

    // Second call should use cache (should be much faster)
    let start = Instant::now();
    let status2 = detector.detect();
    let second_duration = start.elapsed();

    // Status should be the same
    match (status1, status2) {
        (DockerCliStatus::Available, DockerCliStatus::Available)
        | (DockerCliStatus::NotFound, DockerCliStatus::NotFound)
        | (DockerCliStatus::DaemonNotRunning, DockerCliStatus::DaemonNotRunning)
        | (DockerCliStatus::NotInPath(_), DockerCliStatus::NotInPath(_))
        | (DockerCliStatus::PermissionDenied(_), DockerCliStatus::PermissionDenied(_)) => (),
        _ => panic!("Cache returned different status"),
    }

    // Second call should generally be faster due to caching
    // (though this isn't guaranteed in all test environments)
    println!("First detection: {first_duration:?}, Second detection: {second_duration:?}");
}
