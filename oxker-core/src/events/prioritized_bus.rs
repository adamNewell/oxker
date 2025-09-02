//! Prioritized event bus with separate channels for critical and normal events

use super::CoreEvent;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::info;
use tracing::{debug, error, warn};

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    /// Critical events that must be delivered
    Critical,
    /// High priority events
    High,
    /// Normal priority events
    Normal,
    /// Low priority events that can be dropped under load
    Low,
}

/// Event with priority metadata
#[derive(Debug, Clone)]
pub struct PrioritizedEvent {
    pub event: CoreEvent,
    pub priority: EventPriority,
    pub timestamp: Instant,
}

/// Statistics for event bus monitoring
#[derive(Debug, Default)]
pub struct EventBusStats {
    pub total_published: usize,
    pub total_delivered: usize,
    pub total_dropped: usize,
    pub critical_delivered: usize,
    pub high_delivered: usize,
    pub normal_delivered: usize,
    pub low_dropped: usize,
    pub max_latency_ms: u64,
    pub avg_latency_ms: u64,
}

/// Configuration for the prioritized event bus
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Buffer size for critical events channel
    pub critical_buffer: usize,
    /// Buffer size for normal events channel  
    pub normal_buffer: usize,
    /// Maximum age for low priority events before dropping
    pub low_priority_max_age: Duration,
    /// Enable dropping of low priority events under load
    pub drop_low_priority: bool,
    /// Channel utilization threshold for dropping (0.0 to 1.0)
    pub drop_threshold: f32,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            critical_buffer: 100,
            normal_buffer: 200,
            low_priority_max_age: Duration::from_secs(5),
            drop_low_priority: true,
            drop_threshold: 0.8,
        }
    }
}

/// Prioritized event bus with resilience features
#[derive(Clone)]
pub struct PrioritizedEventBus {
    critical_sender: Sender<PrioritizedEvent>,
    normal_sender: Sender<PrioritizedEvent>,
    stats: Arc<Mutex<EventBusStats>>,
    config: EventBusConfig,
    overflow_queue: Arc<Mutex<VecDeque<PrioritizedEvent>>>,
}

/// Combined receiver for prioritized events
pub struct PrioritizedReceiver {
    critical_rx: Receiver<PrioritizedEvent>,
    normal_rx: Receiver<PrioritizedEvent>,
}

impl PrioritizedReceiver {
    /// Receive the next event, prioritizing critical events
    pub async fn recv(&mut self) -> Option<CoreEvent> {
        tokio::select! {
            // Always check critical channel first
            biased;

            Some(event) = self.critical_rx.recv() => {
                Some(event.event)
            }
            Some(event) = self.normal_rx.recv() => {
                Some(event.event)
            }
            else => None
        }
    }
}

impl PrioritizedEventBus {
    /// Create a new prioritized event bus
    #[must_use]
    pub fn new(config: EventBusConfig) -> (Self, PrioritizedReceiver) {
        let (critical_tx, critical_rx) = mpsc::channel(config.critical_buffer);
        let (normal_tx, normal_rx) = mpsc::channel(config.normal_buffer);

        let bus = Self {
            critical_sender: critical_tx,
            normal_sender: normal_tx,
            stats: Arc::new(Mutex::new(EventBusStats::default())),
            config,
            overflow_queue: Arc::new(Mutex::new(VecDeque::new())),
        };

        let receiver = PrioritizedReceiver {
            critical_rx,
            normal_rx,
        };

        (bus, receiver)
    }

    /// Determine event priority based on event type
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Can't be const due to match on enum with data
    pub fn classify_event(&self, event: &CoreEvent) -> EventPriority {
        match event {
            // Critical events - must be delivered
            CoreEvent::Error(_)
            | CoreEvent::ContainerRemoved(_)
            | CoreEvent::DockerConnectionLost
            | CoreEvent::DockerConnectionRestored => EventPriority::Critical,

            // High priority - container state changes
            CoreEvent::ContainerListUpdate(_) => EventPriority::High,

            // Normal priority - regular updates
            CoreEvent::ContainerStatsUpdate { .. } | CoreEvent::ContainerLogsUpdate { .. } => {
                EventPriority::Normal
            }

            // Low priority - debug and info
            CoreEvent::DebugLatency { .. } | CoreEvent::DebugInfo { .. } => EventPriority::Low,

            // All other events default to Normal priority
            _ => EventPriority::Normal,
        }
    }

    /// Publish an event with automatic priority classification
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be sent to the appropriate channel
    pub async fn publish(&self, event: CoreEvent) -> Result<(), String> {
        let priority = self.classify_event(&event);
        self.publish_with_priority(event, priority).await
    }

    /// Publish an event with explicit priority
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be sent to the appropriate channel
    pub async fn publish_with_priority(
        &self,
        event: CoreEvent,
        priority: EventPriority,
    ) -> Result<(), String> {
        let prioritized = PrioritizedEvent {
            event: event.clone(),
            priority,
            timestamp: Instant::now(),
        };

        // Update stats
        self.stats.lock().total_published += 1;

        // Check if we should drop low priority events
        if priority == EventPriority::Low && self.should_drop_low_priority() {
            self.stats.lock().total_dropped += 1;
            self.stats.lock().low_dropped += 1;
            debug!("Dropped low priority event due to load");
            return Ok(());
        }

        // Route to appropriate channel
        let result = match priority {
            EventPriority::Critical => {
                self.critical_sender.send(prioritized).await.map_err(|e| {
                    error!("Failed to send critical event: {}", e);
                    // Critical events go to overflow queue if channel fails
                    self.overflow_queue.lock().push_back(PrioritizedEvent {
                        event,
                        priority,
                        timestamp: Instant::now(),
                    });
                    format!("Critical channel full, queued event: {e}")
                })
            }
            EventPriority::High | EventPriority::Normal => {
                self.normal_sender.send(prioritized).await.map_err(|e| {
                    warn!("Failed to send normal priority event: {}", e);
                    format!("Normal channel full: {e}")
                })
            }
            EventPriority::Low => {
                // Try to send, but don't worry if it fails
                if self.normal_sender.try_send(prioritized).is_err() {
                    self.stats.lock().total_dropped += 1;
                    self.stats.lock().low_dropped += 1;
                }
                Ok(())
            }
        };

        if result.is_ok() {
            let mut stats = self.stats.lock();
            stats.total_delivered += 1;
            match priority {
                EventPriority::Critical => stats.critical_delivered += 1,
                EventPriority::High => stats.high_delivered += 1,
                EventPriority::Normal => stats.normal_delivered += 1,
                EventPriority::Low => {} // Already counted if delivered
            }
        }

        result
    }

    /// Check if low priority events should be dropped
    #[allow(clippy::cast_precision_loss)]
    fn should_drop_low_priority(&self) -> bool {
        if !self.config.drop_low_priority {
            return false;
        }

        // Check channel utilization
        let critical_cap = self.critical_sender.capacity();
        let normal_cap = self.normal_sender.capacity();

        let critical_util = if critical_cap > 0 {
            1.0 - (self.critical_sender.capacity() as f32 / self.config.critical_buffer as f32)
        } else {
            1.0
        };

        let normal_util = if normal_cap > 0 {
            1.0 - (normal_cap as f32 / self.config.normal_buffer as f32)
        } else {
            1.0
        };

        critical_util > self.config.drop_threshold || normal_util > self.config.drop_threshold
    }

    /// Process overflow queue
    #[allow(clippy::significant_drop_tightening)]
    pub fn process_overflow(&self) -> usize {
        let mut processed = 0;
        let mut queue = self.overflow_queue.lock();

        while let Some(event) = queue.pop_front() {
            // Try to send to critical channel
            match self.critical_sender.try_send(event) {
                Ok(()) => {
                    processed += 1;
                }
                Err(mpsc::error::TrySendError::Full(returned_event)) => {
                    // Put it back and stop trying
                    queue.push_front(returned_event);
                    break;
                }
                Err(mpsc::error::TrySendError::Closed(returned_event)) => {
                    // Channel closed, put it back
                    queue.push_front(returned_event);
                    break;
                }
            }
        }

        if processed > 0 {
            info!("Processed {} events from overflow queue", processed);
        }

        processed
    }

    /// Get current statistics
    #[must_use]
    pub fn get_stats(&self) -> EventBusStats {
        // Clone the stats - EventBusStats must implement Clone
        let stats = self.stats.lock();
        EventBusStats {
            total_published: stats.total_published,
            total_delivered: stats.total_delivered,
            total_dropped: stats.total_dropped,
            critical_delivered: stats.critical_delivered,
            high_delivered: stats.high_delivered,
            normal_delivered: stats.normal_delivered,
            low_dropped: stats.low_dropped,
            max_latency_ms: stats.max_latency_ms,
            avg_latency_ms: stats.avg_latency_ms,
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = EventBusStats::default();
    }

    /// Get channel utilization
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn get_utilization(&self) -> (f32, f32) {
        let critical_util =
            1.0 - (self.critical_sender.capacity() as f32 / self.config.critical_buffer as f32);
        let normal_util =
            1.0 - (self.normal_sender.capacity() as f32 / self.config.normal_buffer as f32);
        (critical_util, normal_util)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prioritized_event_bus() {
        let config = EventBusConfig::default();
        let (bus, mut receiver) = PrioritizedEventBus::new(config);

        // Publish events of different priorities
        bus.publish(CoreEvent::Error("Critical error".to_string()))
            .await
            .unwrap();
        bus.publish(CoreEvent::ContainerListUpdate(vec![]))
            .await
            .unwrap();
        bus.publish(CoreEvent::DebugInfo {
            category: "test".to_string(),
            message: "Low priority".to_string(),
            metadata: None,
        })
        .await
        .unwrap();

        // Critical should be received first
        let event = receiver.recv().await.unwrap();
        assert!(matches!(event, CoreEvent::Error(_)));

        let stats = bus.get_stats();
        assert_eq!(stats.total_published, 3);
        assert_eq!(stats.critical_delivered, 1);
    }

    #[tokio::test]
    async fn test_event_classification() {
        let config = EventBusConfig::default();
        let (bus, _) = PrioritizedEventBus::new(config);

        // Test classification
        assert_eq!(
            bus.classify_event(&CoreEvent::Error("test".to_string())),
            EventPriority::Critical
        );

        assert_eq!(
            bus.classify_event(&CoreEvent::ContainerListUpdate(vec![])),
            EventPriority::High
        );

        assert_eq!(
            bus.classify_event(&CoreEvent::ContainerStatsUpdate {
                container_id: "test".to_string(),
                stats: crate::events::types::Stats {
                    container_id: "test".to_string(),
                    cpu_usage: 0.0,
                    memory_usage: 0,
                    memory_limit: 0,
                    network_rx: 0,
                    network_tx: 0,
                },
            }),
            EventPriority::Normal
        );
    }

    #[tokio::test]
    async fn test_overflow_queue() {
        let config = EventBusConfig {
            critical_buffer: 2, // Small buffer to test overflow
            ..EventBusConfig::default()
        };

        let (bus, mut receiver) = PrioritizedEventBus::new(config);

        // Fill the critical channel quickly using try_send to avoid blocking
        for i in 0..5 {
            let event = PrioritizedEvent {
                event: CoreEvent::Error(format!("Error {i}")),
                priority: EventPriority::Critical,
                timestamp: Instant::now(),
            };

            // Try to send directly, if it fails it should go to overflow
            if bus.critical_sender.try_send(event.clone()).is_err() {
                // Manually add to overflow queue when channel is full
                bus.overflow_queue.lock().push_back(event);
            }
        }

        // Some events should be in overflow (3 out of 5, since buffer is 2)
        assert!(!bus.overflow_queue.lock().is_empty());
        assert_eq!(bus.overflow_queue.lock().len(), 3);

        // Drain some events from receiver to make room
        let _ = receiver.recv().await;

        // Process overflow - should move one event from overflow to channel
        let processed = bus.process_overflow();
        assert_eq!(processed, 1);

        // Verify overflow queue has been reduced
        assert_eq!(bus.overflow_queue.lock().len(), 2);
    }
}
