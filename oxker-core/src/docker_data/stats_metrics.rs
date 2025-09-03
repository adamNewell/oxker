use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

/// Metrics for stats collection optimization
#[derive(Debug, Clone)]
pub struct StatsMetrics {
    /// Number of active stats polling tasks
    pub active_polling_tasks: Arc<AtomicUsize>,
    /// Number of API calls saved by skipping non-running containers
    pub api_calls_saved: Arc<AtomicU64>,
    /// Total API calls made for stats
    pub total_api_calls: Arc<AtomicU64>,
    /// Number of containers polled
    pub containers_polled: Arc<AtomicUsize>,
    /// Number of containers skipped
    pub containers_skipped: Arc<AtomicUsize>,
    /// Latency metrics
    pub avg_stats_latency_ms: Arc<AtomicU64>,
    /// Last metrics snapshot time
    pub last_snapshot_time: Arc<parking_lot::Mutex<Instant>>,
    /// Optimization enabled flag for tracking
    pub optimization_enabled: bool,
}

impl StatsMetrics {
    /// Create new stats metrics instance
    #[must_use]
    pub fn new(optimization_enabled: bool) -> Self {
        Self {
            active_polling_tasks: Arc::new(AtomicUsize::new(0)),
            api_calls_saved: Arc::new(AtomicU64::new(0)),
            total_api_calls: Arc::new(AtomicU64::new(0)),
            containers_polled: Arc::new(AtomicUsize::new(0)),
            containers_skipped: Arc::new(AtomicUsize::new(0)),
            avg_stats_latency_ms: Arc::new(AtomicU64::new(0)),
            last_snapshot_time: Arc::new(parking_lot::Mutex::new(Instant::now())),
            optimization_enabled,
        }
    }

    /// Record a stats polling task started
    pub fn task_started(&self) {
        self.active_polling_tasks.fetch_add(1, Ordering::Relaxed);
        self.containers_polled.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a container polled (for testing)
    pub fn container_polled(&self) {
        self.containers_polled.fetch_add(1, Ordering::Relaxed);
        self.total_api_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a stats polling task finished
    pub fn task_finished(&self) {
        self.active_polling_tasks.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a container was skipped for stats collection
    pub fn container_skipped(&self) {
        self.containers_skipped.fetch_add(1, Ordering::Relaxed);
        if self.optimization_enabled {
            self.api_calls_saved.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record an API call was made
    pub fn api_call_made(&self) {
        self.total_api_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Update average latency with a new measurement
    pub fn update_latency(&self, latency_ms: u64) {
        // Simple moving average for demonstration
        let current = self.avg_stats_latency_ms.load(Ordering::Relaxed);
        let new_avg = if current == 0 {
            latency_ms
        } else {
            (current * 9 + latency_ms) / 10 // 90% old, 10% new
        };
        self.avg_stats_latency_ms.store(new_avg, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics
    #[must_use]
    pub fn snapshot(&self) -> StatsMetricsSnapshot {
        let now = Instant::now();
        let elapsed = now.duration_since(*self.last_snapshot_time.lock());
        *self.last_snapshot_time.lock() = now;

        StatsMetricsSnapshot {
            active_polling_tasks: self.active_polling_tasks.load(Ordering::Relaxed),
            api_calls_saved: self.api_calls_saved.load(Ordering::Relaxed),
            total_api_calls: self.total_api_calls.load(Ordering::Relaxed),
            containers_polled: self.containers_polled.load(Ordering::Relaxed),
            containers_skipped: self.containers_skipped.load(Ordering::Relaxed),
            avg_stats_latency_ms: self.avg_stats_latency_ms.load(Ordering::Relaxed),
            elapsed_seconds: elapsed.as_secs(),
            optimization_enabled: self.optimization_enabled,
            savings_percentage: self.calculate_savings_percentage(),
        }
    }

    /// Calculate the percentage of API calls saved
    #[allow(clippy::cast_precision_loss)]
    fn calculate_savings_percentage(&self) -> f64 {
        let saved = self.api_calls_saved.load(Ordering::Relaxed) as f64;
        let total = self.total_api_calls.load(Ordering::Relaxed) as f64;

        if total > 0.0 {
            (saved / (total + saved)) * 100.0
        } else if saved > 0.0 {
            100.0
        } else {
            0.0
        }
    }

    /// Reset all metrics to zero
    pub fn reset(&self) {
        self.active_polling_tasks.store(0, Ordering::Relaxed);
        self.api_calls_saved.store(0, Ordering::Relaxed);
        self.total_api_calls.store(0, Ordering::Relaxed);
        self.containers_polled.store(0, Ordering::Relaxed);
        self.containers_skipped.store(0, Ordering::Relaxed);
        self.avg_stats_latency_ms.store(0, Ordering::Relaxed);
        *self.last_snapshot_time.lock() = Instant::now();
    }
}

/// Snapshot of stats metrics at a point in time
#[derive(Debug, Clone)]
pub struct StatsMetricsSnapshot {
    pub active_polling_tasks: usize,
    pub api_calls_saved: u64,
    pub total_api_calls: u64,
    pub containers_polled: usize,
    pub containers_skipped: usize,
    pub avg_stats_latency_ms: u64,
    pub elapsed_seconds: u64,
    pub optimization_enabled: bool,
    pub savings_percentage: f64,
}

impl StatsMetricsSnapshot {
    /// Format metrics as a string for logging
    #[must_use]
    pub fn format_for_logging(&self) -> String {
        format!(
            "Stats Metrics - Mode: {} | Active Tasks: {} | Calls Saved: {} ({:.1}%) | Containers: {} polled, {} skipped | Avg Latency: {}ms",
            if self.optimization_enabled {
                "Optimized"
            } else {
                "Legacy"
            },
            self.active_polling_tasks,
            self.api_calls_saved,
            self.savings_percentage,
            self.containers_polled,
            self.containers_skipped,
            self.avg_stats_latency_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_tracking() {
        let metrics = StatsMetrics::new(true);

        // Simulate some activity
        metrics.task_started();
        assert_eq!(metrics.active_polling_tasks.load(Ordering::Relaxed), 1);

        metrics.api_call_made();
        metrics.api_call_made();
        assert_eq!(metrics.total_api_calls.load(Ordering::Relaxed), 2);

        metrics.container_skipped();
        metrics.container_skipped();
        assert_eq!(metrics.containers_skipped.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.api_calls_saved.load(Ordering::Relaxed), 2);

        metrics.task_finished();
        assert_eq!(metrics.active_polling_tasks.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_savings_calculation() {
        let metrics = StatsMetrics::new(true);

        // 10 API calls made, 10 saved
        for _ in 0..10 {
            metrics.api_call_made();
            metrics.container_skipped();
        }

        let snapshot = metrics.snapshot();
        assert!((snapshot.savings_percentage - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_latency_averaging() {
        let metrics = StatsMetrics::new(false);

        metrics.update_latency(100);
        assert_eq!(metrics.avg_stats_latency_ms.load(Ordering::Relaxed), 100);

        metrics.update_latency(200);
        // Should be (100 * 9 + 200) / 10 = 110
        assert_eq!(metrics.avg_stats_latency_ms.load(Ordering::Relaxed), 110);
    }
}
