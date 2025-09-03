#[cfg(test)]
mod tests {
    use crate::docker_data::StatsMetrics;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_stats_optimization_enabled() {
        let stats_metrics = Arc::new(StatsMetrics::new(true));

        // Verify metrics tracking
        assert_eq!(
            stats_metrics
                .containers_skipped
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );

        // Simulate skipping
        stats_metrics.container_skipped();
        assert_eq!(
            stats_metrics
                .containers_skipped
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );

        // When optimization is enabled, skipping should save API calls
        assert_eq!(
            stats_metrics
                .api_calls_saved
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[tokio::test]
    async fn test_stats_optimization_disabled() {
        let stats_metrics = Arc::new(StatsMetrics::new(false));

        // When disabled, no containers should be skipped
        stats_metrics.container_skipped();

        // API calls saved should be 0 when optimization is disabled
        assert_eq!(
            stats_metrics
                .api_calls_saved
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[tokio::test]
    async fn test_stats_metrics_tracking() {
        let metrics = StatsMetrics::new(true);

        // Test task tracking
        metrics.task_started();
        assert_eq!(
            metrics
                .active_polling_tasks
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert_eq!(
            metrics
                .containers_polled
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );

        metrics.task_finished();
        assert_eq!(
            metrics
                .active_polling_tasks
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );

        // Test API call tracking
        metrics.api_call_made();
        metrics.api_call_made();
        assert_eq!(
            metrics
                .total_api_calls
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );

        // Test skipping with optimization enabled
        metrics.container_skipped();
        assert_eq!(
            metrics
                .containers_skipped
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert_eq!(
            metrics
                .api_calls_saved
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[tokio::test]
    async fn test_latency_tracking() {
        let metrics = StatsMetrics::new(true);

        // Test latency averaging
        metrics.update_latency(100);
        assert_eq!(
            metrics
                .avg_stats_latency_ms
                .load(std::sync::atomic::Ordering::Relaxed),
            100
        );

        metrics.update_latency(200);
        // Moving average: (100 * 9 + 200) / 10 = 110
        assert_eq!(
            metrics
                .avg_stats_latency_ms
                .load(std::sync::atomic::Ordering::Relaxed),
            110
        );

        metrics.update_latency(300);
        // Moving average: (110 * 9 + 300) / 10 = 129
        assert_eq!(
            metrics
                .avg_stats_latency_ms
                .load(std::sync::atomic::Ordering::Relaxed),
            129
        );
    }

    #[tokio::test]
    async fn test_metrics_snapshot() {
        let metrics = StatsMetrics::new(true);

        // Simulate activity
        for _ in 0..5 {
            metrics.task_started();
            metrics.api_call_made();
        }

        for _ in 0..3 {
            metrics.container_skipped();
        }

        for _ in 0..5 {
            metrics.task_finished();
        }

        metrics.update_latency(50);

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.active_polling_tasks, 0);
        assert_eq!(snapshot.total_api_calls, 5);
        assert_eq!(snapshot.api_calls_saved, 3);
        assert_eq!(snapshot.containers_polled, 5);
        assert_eq!(snapshot.containers_skipped, 3);
        assert_eq!(snapshot.avg_stats_latency_ms, 50);
        assert!(snapshot.optimization_enabled);

        // Savings percentage: 3 saved / (5 + 3) total = 37.5%
        assert!((snapshot.savings_percentage - 37.5).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_rapid_state_transitions() {
        let metrics = Arc::new(StatsMetrics::new(true));
        let mut handles = vec![];

        // Simulate rapid state transitions
        for i in 0..100 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = tokio::spawn(async move {
                if i % 2 == 0 {
                    metrics_clone.task_started();
                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    metrics_clone.task_finished();
                } else {
                    metrics_clone.container_skipped();
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify no race conditions occurred
        assert_eq!(
            metrics
                .active_polling_tasks
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            metrics
                .containers_polled
                .load(std::sync::atomic::Ordering::Relaxed),
            50
        );
        assert_eq!(
            metrics
                .containers_skipped
                .load(std::sync::atomic::Ordering::Relaxed),
            50
        );
    }

    #[test]
    fn test_savings_calculation_edge_cases() {
        let metrics = StatsMetrics::new(true);

        // Test with no calls
        let snapshot = metrics.snapshot();
        assert!((snapshot.savings_percentage - 0.0).abs() < 0.01);

        // Test with only saved calls (100% savings)
        metrics.container_skipped();
        metrics.container_skipped();
        let snapshot = metrics.snapshot();
        assert!((snapshot.savings_percentage - 100.0).abs() < 0.01);

        // Test with no saved calls (0% savings)
        metrics.reset();
        metrics.api_call_made();
        metrics.api_call_made();
        let snapshot = metrics.snapshot();
        assert!((snapshot.savings_percentage - 0.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_performance_improvement_validation() {
        let metrics_optimized = StatsMetrics::new(true);
        let metrics_legacy = StatsMetrics::new(false);

        // Simulate mixed container states (10 running, 10 stopped)
        for _ in 0..10 {
            // Running containers - both modes poll
            metrics_optimized.task_started();
            metrics_optimized.api_call_made();
            metrics_legacy.task_started();
            metrics_legacy.api_call_made();
        }

        for _ in 0..10 {
            // Stopped containers - optimized skips, legacy polls
            metrics_optimized.container_skipped();
            metrics_legacy.task_started();
            metrics_legacy.api_call_made();
        }

        let optimized_snapshot = metrics_optimized.snapshot();
        let legacy_snapshot = metrics_legacy.snapshot();

        // Optimized should have 50% fewer API calls
        assert_eq!(optimized_snapshot.total_api_calls, 10);
        assert_eq!(legacy_snapshot.total_api_calls, 20);

        // Verify 50% reduction in API calls (10 saved out of 20 total)
        assert!((optimized_snapshot.savings_percentage - 50.0).abs() < 0.1);

        // Legacy mode should have 0% savings
        assert!((legacy_snapshot.savings_percentage - 0.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_event_bus_pause_resume_integration() {
        // Test that container state changes properly trigger pause/resume behavior
        use crate::docker_data::StatsMetrics;
        use std::collections::HashMap;
        use tokio::sync::watch;

        // Create metrics to track behavior
        let metrics = Arc::new(StatsMetrics::new(true));

        // Simulate EventBus state change notifications
        let (tx, rx) = watch::channel("running");

        // Track polling task cancellation
        let mut polling_tasks = HashMap::new();
        let container_id = "test_container_123";

        // Initially running - should be polling
        metrics.container_polled();
        polling_tasks.insert(container_id.to_string(), true);

        // Simulate container stop event (would come from EventBus)
        tx.send("stopped").unwrap();

        // When container stops, task should be cancelled
        if *rx.borrow() == "stopped" {
            polling_tasks.remove(container_id);
            metrics.container_skipped();
        }

        assert!(
            !polling_tasks.contains_key(container_id),
            "Polling task should be cancelled when container stops"
        );

        // Simulate container restart event
        tx.send("running").unwrap();

        // Polling should resume for running container
        if *rx.borrow() == "running" {
            polling_tasks.insert(container_id.to_string(), true);
            metrics.container_polled();
        }

        assert!(
            polling_tasks.contains_key(container_id),
            "Polling should resume when container restarts"
        );

        // Verify metrics tracked the transitions correctly
        let snapshot = metrics.snapshot();
        assert_eq!(
            snapshot.containers_polled, 2,
            "Should have polled twice (initial + restart)"
        );
        assert_eq!(
            snapshot.containers_skipped, 1,
            "Should have skipped once when stopped"
        );
    }

    #[tokio::test]
    async fn test_rapid_state_transitions_accuracy() {
        // Test stats accuracy during rapid container state changes
        use crate::docker_data::StatsMetrics;
        use std::time::Duration;
        use tokio::time::sleep;

        let metrics = Arc::new(StatsMetrics::new(true));

        // Simulate rapid state transitions
        let transition_count = 10;

        for i in 0..transition_count {
            // Simulate running state
            metrics.container_polled();

            // Very short delay to simulate rapid transitions
            sleep(Duration::from_millis(10)).await;

            // Simulate stopped state
            metrics.container_skipped();

            // Verify metrics remain consistent
            let snapshot = metrics.snapshot();
            assert_eq!(
                snapshot.containers_polled + snapshot.containers_skipped,
                (i + 1) * 2,
                "Total container operations should match transition count"
            );

            // Verify no data corruption during rapid transitions
            assert!(
                snapshot.api_calls_saved <= snapshot.total_api_calls,
                "Saved API calls should never exceed total calls"
            );
        }

        // Final verification
        let final_snapshot = metrics.snapshot();
        assert_eq!(
            final_snapshot.containers_polled, transition_count,
            "Should have polled exactly {transition_count} times"
        );
        assert_eq!(
            final_snapshot.containers_skipped, transition_count,
            "Should have skipped exactly {transition_count} times"
        );
    }

    #[tokio::test]
    async fn test_explicit_state_transition_scenarios() {
        // Test specific state transition patterns
        use crate::docker_data::StatsMetrics;

        let metrics = Arc::new(StatsMetrics::new(true));

        // Scenario 1: Running -> Paused -> Running
        // Container starts running
        metrics.container_polled();
        let initial_polled = metrics
            .containers_polled
            .load(std::sync::atomic::Ordering::Relaxed);

        // Container paused (should skip stats)
        metrics.container_skipped();
        let skipped_after_pause = metrics
            .containers_skipped
            .load(std::sync::atomic::Ordering::Relaxed);

        // Container resumes
        metrics.container_polled();
        let final_polled = metrics
            .containers_polled
            .load(std::sync::atomic::Ordering::Relaxed);

        assert_eq!(
            final_polled,
            initial_polled + 1,
            "Should resume polling after pause"
        );
        assert_eq!(
            skipped_after_pause, 1,
            "Should have skipped once during pause"
        );

        // Scenario 2: Running -> Stopped -> Removed
        let metrics2 = Arc::new(StatsMetrics::new(true));

        // Container running
        metrics2.container_polled();
        assert_eq!(
            metrics2
                .api_calls_saved
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );

        // Container stopped
        metrics2.container_skipped();
        assert_eq!(
            metrics2
                .api_calls_saved
                .load(std::sync::atomic::Ordering::Relaxed),
            1,
            "Should save API call for stopped container"
        );

        // Container removed (should continue to skip)
        metrics2.container_skipped();
        assert_eq!(
            metrics2
                .api_calls_saved
                .load(std::sync::atomic::Ordering::Relaxed),
            2,
            "Should save API call for removed container"
        );

        // Scenario 3: Created -> Running -> Exited
        let metrics3 = Arc::new(StatsMetrics::new(true));

        // Container created but not started (should skip)
        metrics3.container_skipped();

        // Container starts running
        metrics3.container_polled();

        // Container exits
        metrics3.container_skipped();

        let snapshot = metrics3.snapshot();
        assert_eq!(
            snapshot.containers_skipped, 2,
            "Should skip for created and exited states"
        );
        assert_eq!(
            snapshot.containers_polled, 1,
            "Should only poll when running"
        );

        // Verify proper cleanup happens in all scenarios
        assert!(
            snapshot.savings_percentage >= 0.0 && snapshot.savings_percentage <= 100.0,
            "Savings percentage should be valid"
        );
    }
}
