use oxker_core::{AppError, ContainerId, exec_docker_cli};

#[test]
#[allow(clippy::match_same_arms)]
fn test_exec_docker_cli_with_invalid_container() {
    // This test will fail with an error when trying to exec into a non-existent container
    let fake_id = ContainerId::from("nonexistent_container_12345");
    let result = exec_docker_cli(&fake_id);

    // The result will vary based on whether Docker is installed
    match result {
        Ok(()) => {
            // This should not happen with a fake container ID
            panic!("Expected error for non-existent container");
        }
        Err(e) => {
            // We should get some kind of error
            match e {
                AppError::DockerNotFound => {}
                AppError::DockerNotAccessible(_) => {}
                AppError::DockerDaemonNotRunning => {}
                AppError::DockerExec => {}
                AppError::Terminal => {}
                _ => {}
            }
        }
    }
}

#[test]
fn test_container_id_handling() {
    let test_ids = vec![
        "simple_id",
        "container-with-dashes",
        "container_with_underscores",
        "container123",
        "a1b2c3d4e5f6",                                         // Short hash format
        "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2w3x4y5z6", // Long hash format
    ];

    for id_str in test_ids {
        let container_id = ContainerId::from(id_str);
        assert_eq!(container_id.get(), id_str);
    }
}

#[test]
fn test_app_error_display() {
    let errors = vec![
        AppError::DockerNotFound,
        AppError::DockerNotAccessible("PATH issue".to_string()),
        AppError::ContainerNotFound("test_container".to_string()),
        AppError::ContainerNotRunning("nginx".to_string()),
        AppError::DockerDaemonNotRunning,
    ];

    for error in errors {
        let display = format!("{error}");
        assert!(!display.is_empty());

        // Check for key phrases in error messages
        match error {
            AppError::DockerNotFound => {
                assert!(display.contains("Docker CLI not found"));
                assert!(display.contains("install"));
            }
            AppError::DockerNotAccessible(_) => {
                assert!(display.contains("not accessible"));
                assert!(display.contains("PATH"));
            }
            AppError::ContainerNotFound(_) => {
                assert!(display.contains("not found"));
                assert!(display.contains("container"));
            }
            AppError::ContainerNotRunning(_) => {
                assert!(display.contains("not running"));
                assert!(display.contains("Start"));
            }
            AppError::DockerDaemonNotRunning => {
                assert!(display.contains("daemon"));
                assert!(display.contains("Docker Desktop"));
            }
            _ => {}
        }
    }
}

#[test]
fn test_app_error_clone() {
    let errors = vec![
        AppError::DockerNotFound,
        AppError::DockerNotAccessible("test".to_string()),
        AppError::ContainerNotFound("container".to_string()),
        AppError::ContainerNotRunning("container".to_string()),
        AppError::DockerDaemonNotRunning,
        AppError::DockerExec,
        AppError::Terminal,
    ];

    for error in errors {
        let cloned = error.clone();
        let original_str = format!("{error}");
        let cloned_str = format!("{cloned}");
        assert_eq!(original_str, cloned_str);
    }
}

#[test]
fn test_error_message_formatting() {
    let error = AppError::DockerNotFound;
    let display = format!("{error}");

    // Should have multiple lines
    assert!(display.lines().count() > 1);

    // Should contain helpful information
    assert!(display.contains("docker.com") || display.contains("docs.docker.com")); // Contains links
    assert!(display.contains("Docker")); // Mentions Docker

    // Test error with container name
    let error2 = AppError::ContainerNotRunning("my_container".to_string());
    let display2 = format!("{error2}");
    assert!(display2.contains("my_container"));
    assert!(display2.contains("running"));
}

#[test]
fn test_tty_readable_function() {
    use oxker_core::tty_readable;

    let result = tty_readable();

    #[cfg(unix)]
    {
        // On Unix systems with a TTY, this might return true
        // In CI or containers, it might return false
        // Just verify it doesn't panic
        let _ = result;
    }

    #[cfg(windows)]
    {
        // On Windows, /dev/tty doesn't exist
        assert!(!result);
    }
}
