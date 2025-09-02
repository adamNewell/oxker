//! Task registry for managing async task lifecycles

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Task metadata for tracking purposes
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    /// Unique task identifier
    pub id: Uuid,
    /// Task purpose/description
    pub purpose: String,
    /// When the task was spawned
    pub spawned_at: Instant,
    /// Expected lifetime (None means indefinite)
    pub expected_lifetime: Option<Duration>,
    /// Whether this is a critical task that should be restarted on failure
    pub critical: bool,
    /// Number of restart attempts
    pub restart_count: usize,
}

/// Task registry for managing spawned async tasks
pub struct TaskRegistry {
    /// Active tasks
    tasks: Arc<Mutex<HashMap<Uuid, TaskEntry>>>,
    /// Maximum restart attempts for critical tasks
    max_restarts: usize,
    /// Circuit breaker for failing operations
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

struct TaskEntry {
    metadata: TaskMetadata,
    handle: JoinHandle<()>,
}

/// Circuit breaker for handling repeated failures
#[derive(Debug)]
struct CircuitBreaker {
    /// Number of consecutive failures
    failure_count: usize,
    /// Maximum failures before opening circuit
    max_failures: usize,
    /// When circuit was opened
    opened_at: Option<Instant>,
    /// How long to keep circuit open
    cooldown: Duration,
}

impl CircuitBreaker {
    const fn new(max_failures: usize, cooldown: Duration) -> Self {
        Self {
            failure_count: 0,
            max_failures,
            opened_at: None,
            cooldown,
        }
    }

    fn is_open(&self) -> bool {
        if let Some(opened_at) = self.opened_at
            && opened_at.elapsed() < self.cooldown
        {
            return true;
        }
        // Circuit cooldown expired, can attempt to close
        false
    }

    #[allow(clippy::missing_const_for_fn)] // Mutates self
    fn record_success(&mut self) {
        self.failure_count = 0;
        self.opened_at = None;
    }

    fn record_failure(&mut self) {
        self.failure_count += 1;
        if self.failure_count >= self.max_failures {
            self.opened_at = Some(Instant::now());
            warn!(
                "Circuit breaker opened after {} failures",
                self.failure_count
            );
        }
    }

    #[allow(clippy::missing_const_for_fn)] // Mutates self
    fn reset(&mut self) {
        self.failure_count = 0;
        self.opened_at = None;
    }
}

impl TaskRegistry {
    /// Create a new task registry
    #[must_use]
    pub fn new(max_restarts: usize) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            max_restarts,
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(5, Duration::from_secs(30)))),
        }
    }

    /// Spawn a task and register it
    pub fn spawn<F>(
        &self,
        purpose: &str,
        critical: bool,
        expected_lifetime: Option<Duration>,
        future: F,
    ) -> Uuid
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let id = Uuid::new_v4();
        let metadata = TaskMetadata {
            id,
            purpose: purpose.to_string(),
            spawned_at: Instant::now(),
            expected_lifetime,
            critical,
            restart_count: 0,
        };

        let handle = tokio::spawn(future);

        let entry = TaskEntry { metadata, handle };

        self.tasks.lock().insert(id, entry);

        info!(
            "Spawned task {} (purpose: {}, critical: {})",
            id, purpose, critical
        );

        // Start health monitor for this task
        self.monitor_task(id);

        id
    }

    /// Spawn a critical background task with automatic restart
    pub fn spawn_critical<F, Fut>(
        &self,
        purpose: &str,
        expected_lifetime: Option<Duration>,
        task_factory: F,
    ) -> Uuid
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let id = Uuid::new_v4();
        let _tasks = Arc::clone(&self.tasks);
        let circuit_breaker = Arc::clone(&self.circuit_breaker);
        let max_restarts = self.max_restarts;
        let purpose_clone = purpose.to_string();

        let handle = tokio::spawn(async move {
            let mut restart_count = 0;
            let mut backoff = Duration::from_millis(100);

            loop {
                // Check circuit breaker
                if circuit_breaker.lock().is_open() {
                    warn!(
                        "Circuit breaker open, skipping task spawn for: {}",
                        purpose_clone
                    );
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }

                // Run the task
                let task_future = task_factory();
                task_future.await;
                // Task completed
                circuit_breaker.lock().record_success();

                // Check if we should restart
                if restart_count >= max_restarts {
                    error!(
                        "Critical task {} exceeded max restarts ({})",
                        purpose_clone, max_restarts
                    );
                    circuit_breaker.lock().record_failure();
                    break;
                }

                restart_count += 1;
                warn!(
                    "Restarting critical task {} (attempt {}/{})",
                    purpose_clone, restart_count, max_restarts
                );

                // Exponential backoff
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(60));
            }
        });

        let metadata = TaskMetadata {
            id,
            purpose: purpose.to_string(),
            spawned_at: Instant::now(),
            expected_lifetime,
            critical: true,
            restart_count: 0,
        };

        let entry = TaskEntry { metadata, handle };

        self.tasks.lock().insert(id, entry);
        info!("Spawned critical task {} (purpose: {})", id, purpose);

        id
    }

    /// Monitor a task for health
    fn monitor_task(&self, task_id: Uuid) {
        let tasks = Arc::clone(&self.tasks);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;

                let should_cleanup = {
                    let tasks_guard = tasks.lock();
                    tasks_guard.get(&task_id).is_none_or(|entry| {
                        // Check if task exceeded expected lifetime
                        if let Some(expected) = entry.metadata.expected_lifetime
                            && entry.metadata.spawned_at.elapsed() > expected * 2
                        {
                            warn!("Task {} exceeded expected lifetime by 2x", task_id);
                        }

                        // Check if handle is finished
                        entry.handle.is_finished()
                    })
                };

                if should_cleanup {
                    tasks.lock().remove(&task_id);
                    debug!("Cleaned up completed task {}", task_id);
                    break;
                }
            }
        });
    }

    /// Get active task count
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.tasks.lock().len()
    }

    /// Get task metadata
    #[must_use]
    pub fn get_task_info(&self, task_id: Uuid) -> Option<TaskMetadata> {
        self.tasks.lock().get(&task_id).map(|e| e.metadata.clone())
    }

    /// Cancel a task
    pub fn cancel_task(&self, task_id: Uuid) -> bool {
        let entry = self.tasks.lock().remove(&task_id);
        if let Some(entry) = entry {
            entry.handle.abort();
            info!(
                "Cancelled task {} (purpose: {})",
                task_id, entry.metadata.purpose
            );
            true
        } else {
            false
        }
    }

    /// Clean up completed tasks
    pub fn cleanup_completed(&self) -> usize {
        let initial_count;
        let removed;
        {
            let mut tasks = self.tasks.lock();
            initial_count = tasks.len();

            tasks.retain(|id, entry| {
                if entry.handle.is_finished() {
                    debug!(
                        "Removing completed task {} ({})",
                        id, entry.metadata.purpose
                    );
                    false
                } else {
                    true
                }
            });

            removed = initial_count - tasks.len();
        }
        if removed > 0 {
            info!("Cleaned up {} completed tasks", removed);
        }
        removed
    }

    /// Shutdown all tasks
    pub fn shutdown(&self) {
        let tasks: Vec<_> = {
            let mut tasks_guard = self.tasks.lock();
            tasks_guard.drain().map(|(_, entry)| entry).collect()
        };

        info!("Shutting down {} tasks", tasks.len());

        for entry in tasks {
            entry.handle.abort();
        }
    }

    /// Reset circuit breaker
    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker.lock().reset();
        info!("Circuit breaker reset");
    }

    /// Get statistics
    #[must_use]
    pub fn get_stats(&self) -> String {
        let (tasks_len, critical_count);
        {
            let tasks = self.tasks.lock();
            tasks_len = tasks.len();
            critical_count = tasks.values().filter(|e| e.metadata.critical).count();
        }

        let (failure_count, is_open);
        {
            let breaker = self.circuit_breaker.lock();
            failure_count = breaker.failure_count;
            is_open = breaker.is_open();
        }

        format!(
            "Tasks: {} active ({} critical), Circuit breaker: {} failures, {}",
            tasks_len,
            critical_count,
            failure_count,
            if is_open { "OPEN" } else { "CLOSED" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_registry_basic() {
        let registry = TaskRegistry::new(3);

        // Spawn a simple task
        let task_id = registry.spawn("Test task", false, Some(Duration::from_secs(1)), async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        assert_eq!(registry.active_count(), 1);
        assert!(registry.get_task_info(task_id).is_some());

        // Wait for task to complete
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Cleanup
        registry.cleanup_completed();
        assert_eq!(registry.active_count(), 0);
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let registry = TaskRegistry::new(3);

        let task_id = registry.spawn("Long task", false, None, async {
            tokio::time::sleep(Duration::from_secs(10)).await;
        });

        assert_eq!(registry.active_count(), 1);

        // Cancel the task
        assert!(registry.cancel_task(task_id));

        // Give time for cancellation
        tokio::time::sleep(Duration::from_millis(100)).await;
        registry.cleanup_completed();

        assert_eq!(registry.active_count(), 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let mut breaker = CircuitBreaker::new(3, Duration::from_millis(100));

        assert!(!breaker.is_open());

        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        assert!(!breaker.is_open()); // Not open yet

        breaker.record_failure();
        assert!(breaker.is_open()); // Now open

        // Wait for cooldown
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(!breaker.is_open()); // Should be closed after cooldown

        // Success resets
        breaker.record_success();
        assert_eq!(breaker.failure_count, 0);
    }
}
