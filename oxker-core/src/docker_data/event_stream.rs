//! Docker event stream handler for monitoring container lifecycle events
//!
//! This module provides a real-time event stream listener that monitors Docker container
//! lifecycle events (create, start, stop, destroy) and publishes them through the EventBus.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bollard::Docker;
use bollard::models::EventMessage;
use bollard::query_parameters::EventsOptions;
use futures_util::StreamExt;
use parking_lot::Mutex;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::events::{bus::EventBus, types::CoreEvent};

/// Metrics for tracking event processing performance
#[derive(Debug, Default)]
pub struct EventMetrics {
    /// Total number of events processed
    pub total_events: AtomicU64,
    /// Number of create events
    pub create_events: AtomicU64,
    /// Number of start events
    pub start_events: AtomicU64,
    /// Number of stop events
    pub stop_events: AtomicU64,
    /// Number of destroy events
    pub destroy_events: AtomicU64,
    /// Number of reconnection attempts
    pub reconnect_attempts: AtomicU64,
    /// Number of failed reconnections
    pub reconnect_failures: AtomicU64,
}

impl EventMetrics {
    /// Creates new metrics instance
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a snapshot of current metrics
    pub fn snapshot(&self) -> EventMetricsSnapshot {
        EventMetricsSnapshot {
            total_events: self.total_events.load(Ordering::Relaxed),
            create_events: self.create_events.load(Ordering::Relaxed),
            start_events: self.start_events.load(Ordering::Relaxed),
            stop_events: self.stop_events.load(Ordering::Relaxed),
            destroy_events: self.destroy_events.load(Ordering::Relaxed),
            reconnect_attempts: self.reconnect_attempts.load(Ordering::Relaxed),
            reconnect_failures: self.reconnect_failures.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of event metrics at a point in time
#[derive(Debug, Clone)]
pub struct EventMetricsSnapshot {
    pub total_events: u64,
    pub create_events: u64,
    pub start_events: u64,
    pub stop_events: u64,
    pub destroy_events: u64,
    pub reconnect_attempts: u64,
    pub reconnect_failures: u64,
}

/// Tracks recent events for deduplication
#[derive(Debug, Clone)]
struct EventRecord {
    /// Timestamp when event was processed
    timestamp: Instant,
    /// Event sequence number from Docker (if available)
    sequence: Option<u64>,
}

/// Handles Docker event streaming with automatic reconnection
pub struct DockerEventHandler {
    /// Docker client for API communication
    docker: Docker,
    /// Event bus for publishing container events
    event_bus: EventBus,
    /// Delay between reconnection attempts
    reconnect_delay: Duration,
    /// Metrics for tracking event processing
    metrics: Arc<EventMetrics>,
    /// Event deduplication cache - maps container_id+action to last event record
    event_cache: Arc<Mutex<HashMap<String, EventRecord>>>,
    /// Deduplication window duration (default: 5 seconds)
    dedup_window: Duration,
}

impl DockerEventHandler {
    /// Creates a new Docker event handler
    ///
    /// # Arguments
    /// * `docker` - Docker client instance
    /// * `event_bus` - EventBus for publishing events
    /// * `reconnect_delay` - Initial delay for reconnection attempts
    #[must_use]
    pub fn new(docker: Docker, event_bus: EventBus, reconnect_delay: Duration) -> Self {
        Self {
            docker,
            event_bus,
            reconnect_delay,
            metrics: Arc::new(EventMetrics::new()),
            event_cache: Arc::new(Mutex::new(HashMap::new())),
            dedup_window: Duration::from_secs(5),
        }
    }

    /// Creates a new Docker event handler with custom deduplication window
    ///
    /// # Arguments
    /// * `docker` - Docker client instance
    /// * `event_bus` - EventBus for publishing events
    /// * `reconnect_delay` - Initial delay for reconnection attempts
    /// * `dedup_window` - Duration for event deduplication window
    #[must_use]
    pub fn new_with_dedup_window(
        docker: Docker,
        event_bus: EventBus,
        reconnect_delay: Duration,
        dedup_window: Duration,
    ) -> Self {
        Self {
            docker,
            event_bus,
            reconnect_delay,
            metrics: Arc::new(EventMetrics::new()),
            event_cache: Arc::new(Mutex::new(HashMap::new())),
            dedup_window,
        }
    }

    /// Gets the current metrics
    #[must_use]
    pub fn metrics(&self) -> Arc<EventMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Checks if an event is a duplicate within the deduplication window
    ///
    /// # Arguments
    /// * `container_id` - Container ID
    /// * `action` - Event action (create, start, stop, destroy)
    /// * `sequence` - Optional event sequence number
    ///
    /// # Returns
    /// Returns true if this is a duplicate event that should be skipped
    fn is_duplicate_event(&self, container_id: &str, action: &str, sequence: Option<u64>) -> bool {
        let cache_key = format!("{container_id}:{action}");
        let mut cache = self.event_cache.lock();
        let now = Instant::now();

        // Clean up old entries outside the dedup window
        cache.retain(|_, record| now.duration_since(record.timestamp) < self.dedup_window);

        // Check if we've seen this event recently
        if let Some(existing) = cache.get(&cache_key) {
            // If we have sequence numbers, use them for exact matching
            if let (Some(existing_seq), Some(new_seq)) = (existing.sequence, sequence) {
                if existing_seq >= new_seq {
                    debug!(
                        "Duplicate event detected (by sequence): container={}, action={}, seq={}",
                        container_id, action, new_seq
                    );
                    return true;
                }
            } else {
                // Without sequence numbers, use time window
                if now.duration_since(existing.timestamp) < self.dedup_window {
                    debug!(
                        "Duplicate event detected (by time): container={}, action={}, window={:?}",
                        container_id, action, self.dedup_window
                    );
                    return true;
                }
            }
        }

        // Record this event
        cache.insert(
            cache_key,
            EventRecord {
                timestamp: now,
                sequence,
            },
        );

        false
    }

    /// Starts the event stream with automatic reconnection on failure
    ///
    /// This method runs indefinitely, processing Docker events and automatically
    /// reconnecting if the connection is lost.
    pub async fn start_event_stream(&self) {
        let mut current_delay = self.reconnect_delay;
        let max_delay = Duration::from_secs(60); // Max 1 minute between attempts

        loop {
            info!("Starting Docker event stream");
            self.metrics
                .reconnect_attempts
                .fetch_add(1, Ordering::Relaxed);

            match self.process_events().await {
                Ok(()) => {
                    // Connection ended normally, reset delay
                    current_delay = self.reconnect_delay;
                    debug!("Event stream ended normally");
                }
                Err(e) => {
                    error!("Event stream error: {:#}", e);
                    self.metrics
                        .reconnect_failures
                        .fetch_add(1, Ordering::Relaxed);
                    warn!("Reconnecting in {:?}", current_delay);

                    sleep(current_delay).await;

                    // Exponential backoff with max delay
                    current_delay = std::cmp::min(current_delay * 2, max_delay);
                }
            }
        }
    }

    /// Processes events from the Docker event stream
    ///
    /// # Returns
    /// Returns Ok(()) when the stream ends normally, or an error if processing fails
    async fn process_events(&self) -> Result<()> {
        let options = EventsOptions::default();

        let mut stream = self.docker.events(Some(options));

        while let Some(event_result) = stream.next().await {
            let start_time = Instant::now();
            let event = event_result.context("Failed to read Docker event")?;

            // Only process container events with actions we care about
            if let Some(typ) = &event.typ {
                // Check if this is a container event
                if matches!(typ, bollard::models::EventMessageTypeEnum::CONTAINER)
                    && let Some(action) = event.action.as_deref()
                {
                    self.metrics.total_events.fetch_add(1, Ordering::Relaxed);

                    match action {
                        "create" => self.handle_container_create(&event).await,
                        "start" => self.handle_container_start(&event).await,
                        "stop" => self.handle_container_stop(&event).await,
                        "destroy" => self.handle_container_destroy(&event).await,
                        _ => {
                            // Ignore other container events
                            debug!("Ignoring container event: {}", action);
                        }
                    }

                    // Track processing latency
                    #[allow(clippy::cast_possible_truncation)]
                    let latency_ms = start_time.elapsed().as_millis() as u64;
                    if latency_ms > 100 {
                        warn!(
                            "Event processing took {}ms for action: {}",
                            latency_ms, action
                        );
                    }

                    // Publish latency metrics
                    let _ = self
                        .event_bus
                        .publish(CoreEvent::DebugLatency {
                            operation: format!("docker_event_{action}"),
                            latency_ms,
                            context: None,
                        })
                        .await;
                }
            }
        }

        Ok(())
    }

    /// Handles container create events
    async fn handle_container_create(&self, event: &EventMessage) {
        if let Some(actor) = &event.actor
            && let Some(id) = &actor.id
        {
            // Check for duplicate events
            let sequence = event.time_nano.and_then(|t| u64::try_from(t).ok());
            if self.is_duplicate_event(id, "create", sequence) {
                return;
            }

            self.metrics.create_events.fetch_add(1, Ordering::Relaxed);
            info!("Container created: {}", id);
            // Emit event that container list needs updating
            let _ = self
                .event_bus
                .publish(CoreEvent::ContainerListUpdated)
                .await;
            debug!("Container create event processed for: {}", id);
        }
    }

    /// Handles container start events
    async fn handle_container_start(&self, event: &EventMessage) {
        if let Some(actor) = &event.actor
            && let Some(id) = &actor.id
        {
            // Check for duplicate events
            let sequence = event.time_nano.and_then(|t| u64::try_from(t).ok());
            if self.is_duplicate_event(id, "start", sequence) {
                return;
            }

            self.metrics.start_events.fetch_add(1, Ordering::Relaxed);
            info!("Container started: {}", id);
            // Emit event that container list needs updating (state changed)
            let _ = self
                .event_bus
                .publish(CoreEvent::ContainerListUpdated)
                .await;
            debug!("Container start event processed for: {}", id);
        }
    }

    /// Handles container stop events
    async fn handle_container_stop(&self, event: &EventMessage) {
        if let Some(actor) = &event.actor
            && let Some(id) = &actor.id
        {
            // Check for duplicate events
            let sequence = event.time_nano.and_then(|t| u64::try_from(t).ok());
            if self.is_duplicate_event(id, "stop", sequence) {
                return;
            }

            self.metrics.stop_events.fetch_add(1, Ordering::Relaxed);
            info!("Container stopped: {}", id);
            // Emit event that container list needs updating (state changed)
            let _ = self
                .event_bus
                .publish(CoreEvent::ContainerListUpdated)
                .await;
            debug!("Container stop event processed for: {}", id);
        }
    }

    /// Handles container destroy events
    async fn handle_container_destroy(&self, event: &EventMessage) {
        if let Some(actor) = &event.actor
            && let Some(id) = &actor.id
        {
            // Check for duplicate events
            let sequence = event.time_nano.and_then(|t| u64::try_from(t).ok());
            if self.is_duplicate_event(id, "destroy", sequence) {
                return;
            }

            self.metrics.destroy_events.fetch_add(1, Ordering::Relaxed);
            info!("Container destroyed: {}", id);
            // Container was removed, emit the appropriate event
            let _ = self
                .event_bus
                .publish(CoreEvent::ContainerRemoved(id.clone()))
                .await;
            debug!("Container destroy event processed for: {}", id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a test Docker event handler with mock dependencies
    fn create_test_handler() -> DockerEventHandler {
        let docker =
            Docker::connect_with_local_defaults().expect("Docker connection required for tests");
        let (event_bus, _rx) = EventBus::new(100);
        let reconnect_delay = Duration::from_secs(5);

        DockerEventHandler::new(docker, event_bus, reconnect_delay)
    }

    /// Creates a mock container event
    fn create_mock_event(action: &str, container_id: &str) -> EventMessage {
        use bollard::models::EventActor;
        use std::collections::HashMap;

        EventMessage {
            typ: Some(bollard::models::EventMessageTypeEnum::CONTAINER),
            action: Some(action.to_string()),
            actor: Some(EventActor {
                id: Some(container_id.to_string()),
                attributes: Some(HashMap::new()),
            }),
            time: Some(1_234_567_890),
            time_nano: Some(1_234_567_890_000_000_000),
            scope: None,
        }
    }

    #[tokio::test]
    async fn test_docker_event_handler_creation() {
        let handler = create_test_handler();
        assert_eq!(handler.reconnect_delay, Duration::from_secs(5));

        // Verify metrics are initialized at zero
        let metrics = handler.metrics();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_events, 0);
        assert_eq!(snapshot.create_events, 0);
        assert_eq!(snapshot.start_events, 0);
        assert_eq!(snapshot.stop_events, 0);
        assert_eq!(snapshot.destroy_events, 0);
        assert_eq!(snapshot.reconnect_attempts, 0);
        assert_eq!(snapshot.reconnect_failures, 0);
    }

    #[tokio::test]
    async fn test_handle_container_create() {
        let handler = create_test_handler();
        let event = create_mock_event("create", "test-container-123");

        handler.handle_container_create(&event).await;

        // Verify create event counter incremented
        let metrics = handler.metrics();
        assert_eq!(metrics.create_events.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_handle_container_start() {
        let handler = create_test_handler();
        let event = create_mock_event("start", "test-container-456");

        handler.handle_container_start(&event).await;

        // Verify start event counter incremented
        let metrics = handler.metrics();
        assert_eq!(metrics.start_events.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_handle_container_stop() {
        let handler = create_test_handler();
        let event = create_mock_event("stop", "test-container-789");

        handler.handle_container_stop(&event).await;

        // Verify stop event counter incremented
        let metrics = handler.metrics();
        assert_eq!(metrics.stop_events.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_handle_container_destroy() {
        let handler = create_test_handler();
        let event = create_mock_event("destroy", "test-container-999");

        handler.handle_container_destroy(&event).await;

        // Verify destroy event counter incremented
        let metrics = handler.metrics();
        assert_eq!(metrics.destroy_events.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let handler = create_test_handler();

        // Process multiple events
        let events = vec![
            create_mock_event("create", "container-1"),
            create_mock_event("start", "container-1"),
            create_mock_event("stop", "container-1"),
            create_mock_event("destroy", "container-1"),
            create_mock_event("create", "container-2"),
            create_mock_event("start", "container-2"),
        ];

        for event in events {
            match event.action.as_deref() {
                Some("create") => handler.handle_container_create(&event).await,
                Some("start") => handler.handle_container_start(&event).await,
                Some("stop") => handler.handle_container_stop(&event).await,
                Some("destroy") => handler.handle_container_destroy(&event).await,
                _ => {}
            }
        }

        // Verify cumulative metrics
        let snapshot = handler.metrics().snapshot();
        assert_eq!(snapshot.create_events, 2);
        assert_eq!(snapshot.start_events, 2);
        assert_eq!(snapshot.stop_events, 1);
        assert_eq!(snapshot.destroy_events, 1);
    }

    #[test]
    fn test_event_metrics_default() {
        let metrics = EventMetrics::new();
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.total_events, 0);
        assert_eq!(snapshot.create_events, 0);
        assert_eq!(snapshot.start_events, 0);
        assert_eq!(snapshot.stop_events, 0);
        assert_eq!(snapshot.destroy_events, 0);
        assert_eq!(snapshot.reconnect_attempts, 0);
        assert_eq!(snapshot.reconnect_failures, 0);
    }

    #[test]
    fn test_event_metrics_increments() {
        let metrics = EventMetrics::new();

        metrics.total_events.fetch_add(5, Ordering::Relaxed);
        metrics.create_events.fetch_add(2, Ordering::Relaxed);
        metrics.reconnect_attempts.fetch_add(3, Ordering::Relaxed);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_events, 5);
        assert_eq!(snapshot.create_events, 2);
        assert_eq!(snapshot.reconnect_attempts, 3);
    }

    #[tokio::test]
    async fn test_eventbus_publish_on_create() {
        let docker = Docker::connect_with_local_defaults().expect("Docker connection required");
        let (event_bus, mut rx) = EventBus::new(100);
        let handler = DockerEventHandler::new(docker, event_bus, Duration::from_secs(5));

        let event = create_mock_event("create", "test-container");
        handler.handle_container_create(&event).await;

        // Verify EventBus received ContainerListUpdated event
        tokio::select! {
            Some(core_event) = rx.recv() => {
                matches!(core_event, CoreEvent::ContainerListUpdated);
            }
            () = tokio::time::sleep(Duration::from_millis(100)) => {
                panic!("EventBus did not receive expected event");
            }
        }
    }

    #[tokio::test]
    async fn test_eventbus_publish_on_start() {
        let docker = Docker::connect_with_local_defaults().expect("Docker connection required");
        let (event_bus, mut rx) = EventBus::new(100);
        let handler = DockerEventHandler::new(docker, event_bus, Duration::from_secs(5));

        let event = create_mock_event("start", "test-container");
        handler.handle_container_start(&event).await;

        // Verify EventBus received ContainerListUpdated event
        tokio::select! {
            Some(core_event) = rx.recv() => {
                matches!(core_event, CoreEvent::ContainerListUpdated);
            }
            () = tokio::time::sleep(Duration::from_millis(100)) => {
                panic!("EventBus did not receive expected event");
            }
        }
    }

    #[tokio::test]
    async fn test_eventbus_publish_on_stop() {
        let docker = Docker::connect_with_local_defaults().expect("Docker connection required");
        let (event_bus, mut rx) = EventBus::new(100);
        let handler = DockerEventHandler::new(docker, event_bus, Duration::from_secs(5));

        let event = create_mock_event("stop", "test-container");
        handler.handle_container_stop(&event).await;

        // Verify EventBus received ContainerListUpdated event
        tokio::select! {
            Some(core_event) = rx.recv() => {
                matches!(core_event, CoreEvent::ContainerListUpdated);
            }
            () = tokio::time::sleep(Duration::from_millis(100)) => {
                panic!("EventBus did not receive expected event");
            }
        }
    }

    #[tokio::test]
    async fn test_eventbus_publish_on_destroy() {
        let docker = Docker::connect_with_local_defaults().expect("Docker connection required");
        let (event_bus, mut rx) = EventBus::new(100);
        let handler = DockerEventHandler::new(docker, event_bus, Duration::from_secs(5));

        let event = create_mock_event("destroy", "test-container-id");
        handler.handle_container_destroy(&event).await;

        // Verify EventBus received ContainerRemoved event with correct ID
        tokio::select! {
            Some(core_event) = rx.recv() => {
                match core_event {
                    CoreEvent::ContainerRemoved(id) => {
                        assert_eq!(id, "test-container-id");
                    }
                    _ => panic!("Expected ContainerRemoved event"),
                }
            }
            () = tokio::time::sleep(Duration::from_millis(100)) => {
                panic!("EventBus did not receive expected event");
            }
        }
    }

    #[tokio::test]
    async fn test_event_deduplication_by_time() {
        let handler = create_test_handler();
        let event = create_mock_event("create", "test-container-dedup");

        // Process the same event twice quickly
        handler.handle_container_create(&event).await;
        handler.handle_container_create(&event).await;

        // Only one event should be counted
        let metrics = handler.metrics();
        assert_eq!(metrics.create_events.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_event_deduplication_with_sequence() {
        let docker = Docker::connect_with_local_defaults().expect("Docker connection required");
        let (event_bus, _rx) = EventBus::new(100);
        let handler = DockerEventHandler::new(docker, event_bus, Duration::from_secs(5));

        // Create events with sequence numbers
        let mut event1 = create_mock_event("start", "container-seq");
        event1.time_nano = Some(1000);

        let mut event2 = create_mock_event("start", "container-seq");
        event2.time_nano = Some(1000); // Same sequence

        let mut event3 = create_mock_event("start", "container-seq");
        event3.time_nano = Some(2000); // New sequence

        handler.handle_container_start(&event1).await;
        handler.handle_container_start(&event2).await; // Should be deduplicated
        handler.handle_container_start(&event3).await; // Should be processed

        let metrics = handler.metrics();
        assert_eq!(metrics.start_events.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn test_event_deduplication_window_expiry() {
        let docker = Docker::connect_with_local_defaults().expect("Docker connection required");
        let (event_bus, _rx) = EventBus::new(100);
        // Use a very short dedup window for testing
        let handler = DockerEventHandler::new_with_dedup_window(
            docker,
            event_bus,
            Duration::from_secs(5),
            Duration::from_millis(100),
        );

        let event = create_mock_event("stop", "container-expiry");

        // Process first event
        handler.handle_container_stop(&event).await;

        // Wait for dedup window to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Process same event again - should not be deduplicated
        handler.handle_container_stop(&event).await;

        let metrics = handler.metrics();
        assert_eq!(metrics.stop_events.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn test_event_deduplication_different_actions() {
        let handler = create_test_handler();
        let container_id = "multi-action-container";

        // Different actions for the same container should not be deduplicated
        let create_event = create_mock_event("create", container_id);
        let start_event = create_mock_event("start", container_id);
        let stop_event = create_mock_event("stop", container_id);

        handler.handle_container_create(&create_event).await;
        handler.handle_container_start(&start_event).await;
        handler.handle_container_stop(&stop_event).await;

        let metrics = handler.metrics();
        assert_eq!(metrics.create_events.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.start_events.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.stop_events.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_event_deduplication_different_containers() {
        let handler = create_test_handler();

        // Same action for different containers should not be deduplicated
        let event1 = create_mock_event("create", "container-1");
        let event2 = create_mock_event("create", "container-2");
        let event3 = create_mock_event("create", "container-3");

        handler.handle_container_create(&event1).await;
        handler.handle_container_create(&event2).await;
        handler.handle_container_create(&event3).await;

        let metrics = handler.metrics();
        assert_eq!(metrics.create_events.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_rapid_duplicate_events() {
        let handler = create_test_handler();
        let event = create_mock_event("start", "rapid-container");

        // Simulate rapid duplicate events
        for _ in 0..10 {
            handler.handle_container_start(&event).await;
        }

        // Only first event should be processed
        let metrics = handler.metrics();
        assert_eq!(metrics.start_events.load(Ordering::Relaxed), 1);
    }
}
