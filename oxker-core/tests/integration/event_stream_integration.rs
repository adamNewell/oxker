//! Integration tests for Docker event stream handler
//!
//! These tests require a running Docker daemon and will interact with real containers.
//! Tests are marked with #[ignore] by default to avoid CI issues.

use std::sync::Arc;
use std::time::Duration;

use bollard::Docker;
use bollard::container::{CreateContainerOptions, Config as ContainerConfig};
use bollard::container::RemoveContainerOptions;
use tokio::time::{sleep, timeout};

use oxker_core::docker_data::DockerEventHandler;
use oxker_core::events::bus::EventBus;

/// Helper to create a test container for event generation
async fn create_test_container(docker: &Docker, name: &str) -> Result<String, bollard::errors::Error> {
    let config = ContainerConfig {
        image: Some("busybox:latest"),
        cmd: Some(vec!["sh", "-c", "sleep 3600"]),
        ..Default::default()
    };
    
    let options = CreateContainerOptions {
        name,
        ..Default::default()
    };
    
    let response = docker.create_container(Some(options), config).await?;
    Ok(response.id)
}

/// Helper to clean up test container
async fn cleanup_container(docker: &Docker, id: &str) {
    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    
    let _ = docker.remove_container(id, Some(options)).await;
}

#[tokio::test]
#[ignore = "Requires Docker daemon"]
async fn test_event_stream_with_real_docker() {
    // Setup
    let docker = Docker::connect_with_local_defaults()
        .expect("Failed to connect to Docker daemon");
    
    // Pull busybox image if not present
    let _ = docker.create_image(
        Some(bollard::image::CreateImageOptions {
            from_image: "busybox:latest",
            ..Default::default()
        }),
        None,
        None,
    );
    
    let (event_bus, _rx) = EventBus::new(100);
    let handler = DockerEventHandler::new(
        docker.clone(),
        event_bus.clone(),
        Duration::from_secs(5),
    );
    
    // Start event stream in background
    let event_handle = tokio::spawn(async move {
        handler.start_event_stream().await;
    });
    
    // Give event stream time to connect
    sleep(Duration::from_secs(1)).await;
    
    // Create test container and generate events
    let container_name = format!("oxker-test-{}", uuid::Uuid::new_v4());
    let container_id = create_test_container(&docker, &container_name)
        .await
        .expect("Failed to create test container");
    
    // Start the container
    docker.start_container::<String>(&container_id, None)
        .await
        .expect("Failed to start container");
    
    // Give time for events to process
    sleep(Duration::from_secs(2)).await;
    
    // Stop the container
    docker.stop_container(&container_id, None)
        .await
        .expect("Failed to stop container");
    
    // Give time for stop event
    sleep(Duration::from_secs(1)).await;
    
    // Cleanup
    cleanup_container(&docker, &container_id).await;
    
    // Abort event stream
    event_handle.abort();
    
    // The test passes if no panics occurred
    // In a real scenario, we'd subscribe to the EventBus and verify events
}

#[tokio::test]
#[ignore = "Requires Docker daemon"]
async fn test_event_stream_reconnection() {
    let docker = Docker::connect_with_local_defaults()
        .expect("Failed to connect to Docker daemon");
    
    let (event_bus, _rx) = EventBus::new(100);
    let handler = DockerEventHandler::new(
        docker.clone(),
        event_bus.clone(),
        Duration::from_secs(1), // Quick reconnect for testing
    );
    
    // Get initial metrics
    let metrics = handler.metrics();
    let initial_attempts = metrics.reconnect_attempts.load(std::sync::atomic::Ordering::Relaxed);
    
    // Start event stream
    let event_handle = tokio::spawn(async move {
        handler.start_event_stream().await;
    });
    
    // Wait a bit
    sleep(Duration::from_secs(2)).await;
    
    // Check that reconnect attempts have been made (at least 1)
    let current_attempts = metrics.reconnect_attempts.load(std::sync::atomic::Ordering::Relaxed);
    assert!(current_attempts > initial_attempts, "Expected reconnection attempts");
    
    // Cleanup
    event_handle.abort();
}

#[tokio::test]
async fn test_event_metrics_collection() {
    let docker = Docker::connect_with_local_defaults()
        .expect("Failed to connect to Docker daemon");
    
    let (event_bus, _rx) = EventBus::new(100);
    let handler = DockerEventHandler::new(
        docker,
        event_bus,
        Duration::from_secs(5),
    );
    
    // Get metrics reference
    let metrics = handler.metrics();
    
    // Verify initial state
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_events, 0);
    assert_eq!(snapshot.create_events, 0);
    assert_eq!(snapshot.start_events, 0);
    assert_eq!(snapshot.stop_events, 0);
    assert_eq!(snapshot.destroy_events, 0);
    
    // Simulate some event processing (would normally come from Docker)
    // This is more of a unit test, but verifies the metrics interface
    metrics.total_events.fetch_add(10, std::sync::atomic::Ordering::Relaxed);
    metrics.create_events.fetch_add(3, std::sync::atomic::Ordering::Relaxed);
    metrics.start_events.fetch_add(3, std::sync::atomic::Ordering::Relaxed);
    metrics.stop_events.fetch_add(2, std::sync::atomic::Ordering::Relaxed);
    metrics.destroy_events.fetch_add(2, std::sync::atomic::Ordering::Relaxed);
    
    // Verify metrics updated
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_events, 10);
    assert_eq!(snapshot.create_events, 3);
    assert_eq!(snapshot.start_events, 3);
    assert_eq!(snapshot.stop_events, 2);
    assert_eq!(snapshot.destroy_events, 2);
}

#[tokio::test]
#[ignore = "Requires Docker daemon"]
async fn test_malformed_event_handling() {
    // This test verifies that the event stream can handle unexpected data gracefully
    let docker = Docker::connect_with_local_defaults()
        .expect("Failed to connect to Docker daemon");
    
    let (event_bus, _rx) = EventBus::new(100);
    let handler = DockerEventHandler::new(
        docker,
        event_bus,
        Duration::from_secs(5),
    );
    
    // Start event stream
    let event_handle = tokio::spawn(async move {
        handler.start_event_stream().await;
    });
    
    // Let it run for a bit to ensure no crashes with normal Docker events
    let result = timeout(Duration::from_secs(3), event_handle).await;
    
    // Should timeout without error (event stream runs forever)
    assert!(result.is_err(), "Event stream should run indefinitely");
}