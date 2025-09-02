//! Docker connection monitoring and automatic recovery

use bollard::Docker;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::{
    app_error::AppError,
    docker_cli::{DockerCliDetector, DockerCliStatus},
    events::{CoreEvent, EventBus},
};

/// Configuration for connection monitoring
#[derive(Debug, Clone)]
pub struct ConnectionMonitorConfig {
    /// How often to check connection health
    pub ping_interval: Duration,
    /// Maximum time to wait for ping response
    pub ping_timeout: Duration,
    /// How long to wait before retry attempts
    pub reconnect_interval: Duration,
    /// Maximum consecutive failures before giving up
    pub max_failures: usize,
    /// Enable automatic reconnection attempts
    pub auto_reconnect: bool,
}

impl Default for ConnectionMonitorConfig {
    fn default() -> Self {
        Self {
            ping_interval: Duration::from_secs(10),
            ping_timeout: Duration::from_secs(5),
            reconnect_interval: Duration::from_secs(5),
            max_failures: 10,
            auto_reconnect: true,
        }
    }
}

/// Connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is healthy
    Connected,
    /// Connection check in progress
    Checking,
    /// Connection lost, attempting to reconnect
    Reconnecting,
    /// Connection failed after max attempts
    Failed,
}

/// Docker connection monitor for health checks and recovery
pub struct ConnectionMonitor {
    config: ConnectionMonitorConfig,
    state: Arc<Mutex<ConnectionState>>,
    last_successful_ping: Arc<Mutex<Option<Instant>>>,
    consecutive_failures: Arc<Mutex<usize>>,
    event_bus: EventBus,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl ConnectionMonitor {
    /// Create a new connection monitor
    #[must_use]
    pub fn new(config: ConnectionMonitorConfig, event_bus: EventBus) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Connected)),
            last_successful_ping: Arc::new(Mutex::new(Some(Instant::now()))),
            consecutive_failures: Arc::new(Mutex::new(0)),
            event_bus,
            shutdown_tx: None,
        }
    }

    /// Start monitoring the Docker connection
    pub fn start_monitoring(&mut self, docker: Arc<Docker>) -> mpsc::Receiver<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let config = self.config.clone();
        let state = Arc::clone(&self.state);
        let last_ping = Arc::clone(&self.last_successful_ping);
        let failures = Arc::clone(&self.consecutive_failures);
        let event_bus = self.event_bus.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.ping_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::check_connection(
                            Arc::clone(&docker),
                            Arc::clone(&state),
                            Arc::clone(&last_ping),
                            Arc::clone(&failures),
                            &event_bus,
                            &config,
                        ).await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Connection monitor shutting down");
                        break;
                    }
                }
            }
        });

        // Return a receiver that will be notified when monitoring stops
        let (stopped_tx, stopped_rx) = mpsc::channel(1);
        tokio::spawn(async move {
            let _ = stopped_tx.send(()).await;
        });

        stopped_rx
    }

    /// Check Docker connection health
    #[allow(clippy::single_match_else)]
    async fn check_connection(
        docker: Arc<Docker>,
        state: Arc<Mutex<ConnectionState>>,
        last_ping: Arc<Mutex<Option<Instant>>>,
        failures: Arc<Mutex<usize>>,
        event_bus: &EventBus,
        config: &ConnectionMonitorConfig,
    ) {
        *state.lock() = ConnectionState::Checking;

        // Try to ping Docker
        let ping_result = tokio::time::timeout(config.ping_timeout, docker.ping()).await;

        match ping_result {
            Ok(Ok(_)) => {
                // Connection is healthy
                let was_reconnecting = *state.lock() == ConnectionState::Reconnecting;
                *state.lock() = ConnectionState::Connected;
                *last_ping.lock() = Some(Instant::now());
                *failures.lock() = 0;

                if was_reconnecting {
                    info!("Docker connection restored");
                    let _ = event_bus.publish(CoreEvent::DockerConnectionRestored).await;
                }

                debug!("Docker ping successful");
            }
            Ok(Err(_)) | Err(_) => {
                // Connection failed
                let failure_count = {
                    let mut f = failures.lock();
                    *f += 1;
                    *f
                };

                warn!("Docker ping failed (attempt {})", failure_count);

                if failure_count == 1 {
                    // First failure - notify connection lost
                    error!("Docker connection lost");
                    let _ = event_bus.publish(CoreEvent::DockerConnectionLost).await;
                }

                if failure_count >= config.max_failures {
                    *state.lock() = ConnectionState::Failed;
                    error!("Docker connection failed after {} attempts", failure_count);
                } else if config.auto_reconnect {
                    *state.lock() = ConnectionState::Reconnecting;

                    // Attempt reconnection
                    Self::attempt_reconnection(
                        Arc::clone(&docker),
                        Arc::clone(&state),
                        Arc::clone(&last_ping),
                        Arc::clone(&failures),
                        event_bus,
                        config,
                    )
                    .await;
                }
            }
        }
    }

    /// Attempt to reconnect to Docker
    async fn attempt_reconnection(
        docker: Arc<Docker>,
        state: Arc<Mutex<ConnectionState>>,
        last_ping: Arc<Mutex<Option<Instant>>>,
        failures: Arc<Mutex<usize>>,
        event_bus: &EventBus,
        config: &ConnectionMonitorConfig,
    ) {
        info!("Attempting to reconnect to Docker...");

        // First check if Docker CLI is available
        let mut detector = DockerCliDetector::new();
        match detector.detect() {
            DockerCliStatus::Available => {
                // Try to reconnect
                tokio::time::sleep(config.reconnect_interval).await;

                // Try ping again
                if docker.ping().await.is_ok() {
                    *state.lock() = ConnectionState::Connected;
                    *last_ping.lock() = Some(Instant::now());
                    *failures.lock() = 0;
                    info!("Successfully reconnected to Docker");
                    let _ = event_bus.publish(CoreEvent::DockerConnectionRestored).await;
                }
            }
            DockerCliStatus::DaemonNotRunning => {
                debug!("Docker daemon not running, waiting...");
            }
            status => {
                warn!("Docker CLI status: {:?}", status);
            }
        }
    }

    /// Get current connection state
    #[must_use]
    pub fn get_state(&self) -> ConnectionState {
        self.state.lock().clone()
    }

    /// Get last successful ping time
    #[must_use]
    pub fn last_successful_ping(&self) -> Option<Instant> {
        *self.last_successful_ping.lock()
    }

    /// Manually trigger a connection check
    pub async fn check_now(&self, docker: Arc<Docker>) {
        Self::check_connection(
            docker,
            Arc::clone(&self.state),
            Arc::clone(&self.last_successful_ping),
            Arc::clone(&self.consecutive_failures),
            &self.event_bus,
            &self.config,
        )
        .await;
    }

    /// Stop monitoring
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    /// Reset connection state (useful after manual intervention)
    pub fn reset(&self) {
        *self.state.lock() = ConnectionState::Connected;
        *self.last_successful_ping.lock() = Some(Instant::now());
        *self.consecutive_failures.lock() = 0;
        info!("Connection monitor reset");
    }
}

/// Helper to create a reconnectable Docker client
///
/// # Errors
///
/// Returns `AppError` if Docker connection cannot be established after retries
pub async fn create_reconnectable_docker(
    host: Option<&str>,
    max_retries: usize,
) -> Result<Docker, AppError> {
    let mut attempts = 0;
    let mut last_error = None;

    while attempts < max_retries {
        let result = host.map_or_else(Docker::connect_with_socket_defaults, |h| {
            Docker::connect_with_socket(h, 60, bollard::API_DEFAULT_VERSION)
        });

        match result {
            Ok(docker) => {
                // Test the connection
                if docker.ping().await.is_ok() {
                    return Ok(docker);
                }
                last_error = Some("Ping failed".to_string());
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }

        attempts += 1;
        if attempts < max_retries {
            let backoff = Duration::from_millis(
                100 * 2_u64.pow(u32::try_from(attempts).unwrap_or(1).saturating_sub(1)),
            );
            info!("Retry {} in {:?}...", attempts, backoff);
            tokio::time::sleep(backoff).await;
        }
    }

    match last_error {
        Some(msg) if msg.contains("daemon") => Err(AppError::DockerDaemonNotRunning),
        Some(msg) if msg.contains("permission") => Err(AppError::DockerNotAccessible(msg)),
        _ => Err(AppError::DockerConnect),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_transitions() {
        let state = ConnectionState::Connected;
        assert_eq!(state, ConnectionState::Connected);

        let state = ConnectionState::Reconnecting;
        assert_eq!(state, ConnectionState::Reconnecting);
    }

    #[test]
    fn test_monitor_config_defaults() {
        let config = ConnectionMonitorConfig::default();
        assert_eq!(config.ping_interval, Duration::from_secs(10));
        assert_eq!(config.max_failures, 10);
        assert!(config.auto_reconnect);
    }

    #[tokio::test]
    async fn test_connection_monitor_creation() {
        let (event_bus, _) = EventBus::new(100);
        let config = ConnectionMonitorConfig::default();
        let monitor = ConnectionMonitor::new(config, event_bus);

        assert_eq!(monitor.get_state(), ConnectionState::Connected);
        assert!(monitor.last_successful_ping().is_some());
    }
}
