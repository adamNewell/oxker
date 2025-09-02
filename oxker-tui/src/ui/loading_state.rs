//! Unified loading state management to prevent memory leaks and orphaned UUIDs

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, warn};
use uuid::Uuid;

/// Configuration for loading state management
#[derive(Debug, Clone)]
pub struct LoadingConfig {
    /// Maximum time a loading operation can be tracked before automatic cleanup
    pub max_duration: Duration,
    /// Interval for periodic health checks
    pub health_check_interval: Duration,
    /// Maximum number of concurrent loading operations before warning
    pub max_concurrent_warning: usize,
}

impl Default for LoadingConfig {
    fn default() -> Self {
        Self {
            max_duration: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(30),
            max_concurrent_warning: 10,
        }
    }
}

/// Metadata for a loading operation
#[derive(Debug, Clone)]
pub struct LoadingOperation {
    /// Unique identifier for this operation
    pub uuid: Uuid,
    /// When this operation started
    pub started_at: Instant,
    /// Optional description of what's loading
    pub description: Option<String>,
    /// Whether this is part of an animation
    pub animated: bool,
}

/// Unified loading state manager
#[derive(Debug)]
pub struct LoadingStateManager {
    /// All active loading operations
    operations: HashMap<Uuid, LoadingOperation>,
    /// Configuration
    config: LoadingConfig,
    /// Last health check time
    last_health_check: Instant,
    /// Statistics for debugging
    stats: LoadingStats,
}

#[derive(Debug, Default)]
struct LoadingStats {
    total_started: usize,
    total_completed: usize,
    total_expired: usize,
    peak_concurrent: usize,
}

impl LoadingStateManager {
    /// Create a new loading state manager
    #[must_use]
    pub fn new(config: LoadingConfig) -> Self {
        Self {
            operations: HashMap::new(),
            config,
            last_health_check: Instant::now(),
            stats: LoadingStats::default(),
        }
    }

    /// Start tracking a loading operation
    pub fn start_loading(&mut self, uuid: Uuid, description: Option<&str>, animated: bool) {
        let operation = LoadingOperation {
            uuid,
            started_at: Instant::now(),
            description: description.map(String::from),
            animated,
        };

        if self.operations.insert(uuid, operation).is_some() {
            warn!(
                "Loading operation {} was already tracked (possible duplicate)",
                uuid
            );
        }

        self.stats.total_started += 1;
        self.stats.peak_concurrent = self.stats.peak_concurrent.max(self.operations.len());

        if self.operations.len() > self.config.max_concurrent_warning {
            warn!(
                "High number of concurrent loading operations: {} (UUID: {}, desc: {:?})",
                self.operations.len(),
                uuid,
                description
            );
        }

        debug!(
            "Started loading operation {} (total active: {})",
            uuid,
            self.operations.len()
        );

        // Perform health check if needed
        self.maybe_health_check();
    }

    /// Stop tracking a loading operation
    pub fn stop_loading(&mut self, uuid: Uuid) -> bool {
        if let Some(operation) = self.operations.remove(&uuid) {
            let duration = operation.started_at.elapsed();
            self.stats.total_completed += 1;

            debug!(
                "Completed loading operation {} after {:?} (remaining: {})",
                uuid,
                duration,
                self.operations.len()
            );

            if duration > self.config.max_duration {
                warn!(
                    "Loading operation {} took longer than expected: {:?}",
                    uuid, duration
                );
            }

            true
        } else {
            debug!("Attempted to stop non-existent loading operation: {}", uuid);
            false
        }
    }

    /// Check if any loading operations are active
    #[must_use]
    pub fn is_loading(&self) -> bool {
        !self.operations.is_empty()
    }

    /// Get count of active loading operations
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.operations.len()
    }

    /// Get all active loading UUIDs
    #[must_use]
    pub fn active_uuids(&self) -> Vec<Uuid> {
        self.operations.keys().copied().collect()
    }

    /// Check if a specific UUID is loading
    #[must_use]
    pub fn is_uuid_loading(&self, uuid: &Uuid) -> bool {
        self.operations.contains_key(uuid)
    }

    /// Get animated loading operations
    #[must_use]
    pub fn animated_operations(&self) -> Vec<&LoadingOperation> {
        self.operations.values().filter(|op| op.animated).collect()
    }

    /// Perform health check and cleanup if needed
    fn maybe_health_check(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_health_check) >= self.config.health_check_interval {
            self.health_check();
            self.last_health_check = now;
        }
    }

    /// Perform health check and cleanup expired operations
    pub fn health_check(&mut self) {
        let now = Instant::now();
        let mut expired = Vec::new();

        for (uuid, operation) in &self.operations {
            let age = now.duration_since(operation.started_at);
            if age > self.config.max_duration {
                expired.push(*uuid);
                warn!(
                    "Loading operation {} expired after {:?} (desc: {:?})",
                    uuid, age, operation.description
                );
            }
        }

        for uuid in expired {
            self.operations.remove(&uuid);
            self.stats.total_expired += 1;
        }

        if !self.operations.is_empty() {
            debug!(
                "Health check: {} active operations, {} expired this session",
                self.operations.len(),
                self.stats.total_expired
            );
        }
    }

    /// Clear all loading operations (use with caution)
    pub fn clear_all(&mut self) {
        let count = self.operations.len();
        if count > 0 {
            warn!("Clearing {} active loading operations", count);
            self.operations.clear();
        }
    }

    /// Get statistics for debugging
    #[must_use]
    pub fn get_stats(&self) -> String {
        format!(
            "Loading stats: {} started, {} completed, {} expired, {} active, {} peak",
            self.stats.total_started,
            self.stats.total_completed,
            self.stats.total_expired,
            self.operations.len(),
            self.stats.peak_concurrent
        )
    }

    /// Check invariants for debugging
    ///
    /// # Errors
    ///
    /// Returns an error if any operation is older than twice the max duration or
    /// if a zombie operation (operation in memory but already timed out) is detected.
    pub fn check_invariants(&self) -> Result<(), String> {
        // Check for very old operations
        let now = Instant::now();
        for (uuid, op) in &self.operations {
            let age = now.duration_since(op.started_at);
            if age > self.config.max_duration * 2 {
                return Err(format!("Operation {uuid} is extremely old: {age:?}"));
            }
        }

        // Check for reasonable number of operations
        if self.operations.len() > self.config.max_concurrent_warning * 2 {
            return Err(format!(
                "Excessive number of operations: {}",
                self.operations.len()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_state_manager_basic() {
        let mut manager = LoadingStateManager::new(LoadingConfig::default());
        let uuid = Uuid::new_v4();

        assert!(!manager.is_loading());

        manager.start_loading(uuid, Some("Test operation"), false);
        assert!(manager.is_loading());
        assert_eq!(manager.active_count(), 1);
        assert!(manager.is_uuid_loading(&uuid));

        assert!(manager.stop_loading(uuid));
        assert!(!manager.is_loading());
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_loading_state_manager_expiration() {
        let config = LoadingConfig {
            max_duration: Duration::from_millis(100),
            health_check_interval: Duration::from_millis(50),
            max_concurrent_warning: 5,
        };
        let mut manager = LoadingStateManager::new(config);
        let uuid = Uuid::new_v4();

        manager.start_loading(uuid, None, false);
        assert_eq!(manager.active_count(), 1);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        manager.health_check();
        assert_eq!(manager.active_count(), 0);
        assert_eq!(manager.stats.total_expired, 1);
    }

    #[test]
    fn test_loading_state_manager_concurrent() {
        let mut manager = LoadingStateManager::new(LoadingConfig::default());
        let mut uuids = Vec::new();

        for i in 0..5 {
            let uuid = Uuid::new_v4();
            uuids.push(uuid);
            manager.start_loading(uuid, Some(&format!("Operation {i}")), i % 2 == 0);
        }

        assert_eq!(manager.active_count(), 5);
        assert_eq!(manager.animated_operations().len(), 3); // Operations 0, 2, 4

        // Remove some operations
        manager.stop_loading(uuids[1]);
        manager.stop_loading(uuids[3]);
        assert_eq!(manager.active_count(), 3);

        // Clear all
        manager.clear_all();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_loading_state_manager_invariants() {
        let mut manager = LoadingStateManager::new(LoadingConfig::default());

        // Should pass with no operations
        assert!(manager.check_invariants().is_ok());

        // Add normal operations
        for _ in 0..5 {
            manager.start_loading(Uuid::new_v4(), None, false);
        }
        assert!(manager.check_invariants().is_ok());
    }
}
