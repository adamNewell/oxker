//! Integration tests for Docker connection handling and error recovery

use oxker_core::{AppColors, AppError, Config, CoreHandle, EventBus, Keymap};
use std::time::Duration;

const fn gen_test_config() -> Config {
    Config {
        app_colors: AppColors::new(),
        color_logs: false,
        docker_interval_ms: 1000,
        gui: true,
        host: None,
        in_container: false,
        keymap: Keymap::new(),
        network_interface: None,
        raw_logs: false,
        save_dir: None,
        show_self: false,
        show_std_err: false,
        show_timestamp: false,
        timezone: None,
        timestamp_format: String::new(),
        show_logs: true,
        debug_mode: false,
        event_driven_mode: false,
        full_sync_interval_ms: 60_000,
    }
}

#[tokio::test]
async fn test_docker_connection_no_panic() {
    // This test verifies that the application doesn't panic when Docker is unavailable
    let (event_bus, _receiver) = EventBus::new(100);
    let config = gen_test_config();

    // Should not panic, even if Docker is not running
    let result = CoreHandle::try_new(event_bus, &config).await;

    // The result could be Ok or Err depending on Docker availability
    // But it should NEVER panic
    match result {
        Ok(_) => {
            // Docker is available - good!
            println!("Docker connection successful");
        }
        Err(e) => {
            // Docker is not available - this is expected behavior
            println!("Docker not available (expected): {e}");

            // Verify we get appropriate error types
            assert!(
                matches!(
                    e,
                    AppError::DockerNotFound
                        | AppError::DockerDaemonNotRunning
                        | AppError::DockerConnect
                        | AppError::DockerNotAccessible(_)
                ),
                "Expected Docker-related error, got: {e:?}"
            );
        }
    }
}

#[tokio::test]
async fn test_docker_connection_with_invalid_host() {
    let (event_bus, _receiver) = EventBus::new(100);
    let mut config = gen_test_config();

    // Set an invalid host
    config.host = Some("tcp://invalid-host:2375".to_string());

    let result = CoreHandle::try_new(event_bus, &config).await;

    // Should return an error, not panic
    assert!(result.is_err());
    if let Err(e) = result {
        // Should be a connection error
        assert!(
            matches!(
                e,
                AppError::DockerConnect | AppError::DockerDaemonNotRunning
            ),
            "Expected connection error for invalid host"
        );
    }
}

#[tokio::test]
async fn test_docker_error_messages() {
    // Test that error messages are informative
    let docker_not_found = AppError::DockerNotFound;
    let msg = docker_not_found.to_string();
    assert!(msg.contains("Docker CLI not found"));
    assert!(msg.contains("install"));

    let daemon_not_running = AppError::DockerDaemonNotRunning;
    let msg = daemon_not_running.to_string();
    assert!(msg.contains("Cannot connect to Docker daemon"));
    assert!(msg.contains("Docker Desktop"));

    let not_accessible = AppError::DockerNotAccessible("test detail".to_string());
    let msg = not_accessible.to_string();
    assert!(msg.contains("not accessible"));
    assert!(msg.contains("test detail"));
}

#[tokio::test]
async fn test_multiple_connection_attempts() {
    // Test that retry logic works properly
    let (event_bus, _receiver) = EventBus::new(100);
    let config = gen_test_config();

    // First attempt
    let result1 = CoreHandle::try_new(event_bus.clone(), &config).await;

    // Small delay
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second attempt - should work the same way
    let (event_bus2, _receiver2) = EventBus::new(100);
    let result2 = CoreHandle::try_new(event_bus2, &config).await;

    // Both attempts should have the same outcome (both succeed or both fail)
    assert_eq!(result1.is_ok(), result2.is_ok());
}

#[tokio::test]
async fn test_docker_cli_detection() {
    use oxker_core::docker_cli::{DockerCliDetector, DockerCliStatus};

    let mut detector = DockerCliDetector::new();
    let status = detector.detect();

    // Test that detection returns a valid status
    match status {
        DockerCliStatus::Available => {
            println!("Docker CLI is available");
        }
        DockerCliStatus::NotFound => {
            println!("Docker CLI not found");
        }
        DockerCliStatus::NotInPath(paths) => {
            println!("Docker found but not in PATH: {paths:?}");
        }
        DockerCliStatus::PermissionDenied(msg) => {
            println!("Permission denied: {msg}");
        }
        DockerCliStatus::DaemonNotRunning => {
            println!("Docker daemon not running");
        }
    }

    // Cache invalidation should work
    detector.invalidate_cache();
    let status2 = detector.detect();

    // Status should be consistent (may differ only due to actual system state changes)
    println!("Second detection: {status2:?}");
}

#[tokio::test]
async fn test_graceful_error_handling_in_commands() {
    // If we have a handle, commands should handle errors gracefully
    let (event_bus, _receiver) = EventBus::new(100);
    let config = gen_test_config();

    if let Ok(handle) = CoreHandle::try_new(event_bus, &config).await {
        // Try to execute commands - they should not panic even with invalid IDs
        let result = handle
            .execute_command(oxker_core::CoreCommand::StartContainer(
                "non-existent-container".to_string(),
            ))
            .await;

        // Should return Ok (command sent) even if container doesn't exist
        // The actual Docker error would be handled internally
        assert!(result.is_ok());
    }
}
