use bollard::{
    Docker,
    query_parameters::{
        ListContainersOptions, LogsOptions, RemoveContainerOptions, RestartContainerOptions,
        StartContainerOptions, StatsOptions, StopContainerOptions,
    },
    secret::ContainerStatsResponse,
    service::ContainerSummary,
};
use futures_util::StreamExt;
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicUsize},
};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use crate::{
    ENTRY_POINT,
    app_data::{AppData, ContainerId, DockerCommand, State},
    app_error::AppError,
    config::Config,
    events::{
        EventBus,
        types::{CoreEvent, LogLine, Stats},
    },
};
mod event_stream;
mod message;
mod network;
mod stats_metrics;
#[cfg(test)]
mod stats_optimization_tests;
pub use event_stream::{DockerEventHandler, EventMetrics, EventMetricsSnapshot};
pub use message::DockerMessage;
pub use stats_metrics::{StatsMetrics, StatsMetricsSnapshot};

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
enum SpawnId {
    Stats((ContainerId, Binate)),
    Log(ContainerId),
}

impl SpawnId {
    /// Extract the &ContainerId out of self
    const fn get_id(&self) -> &ContainerId {
        match self {
            Self::Log(id) | Self::Stats((id, _)) => id,
        }
    }
}

/// Cpu & Mem stats take twice as long as the update interval to get a value, so will have two being executed at the same time
/// SpawnId::Stats takes container_id and binate value to enable both cycles of the same container_id to be inserted into the hashmap
/// Binate value is toggled when all handles have been spawned off
/// Also effectively means that the minimum docker_update interval will be 1000ms
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
enum Binate {
    One,
    Two,
}

impl Binate {
    const fn toggle(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::One,
        }
    }
}

pub struct DockerData {
    app_data: Arc<Mutex<AppData>>,
    binate: Binate,
    config: Config,
    docker: Arc<Docker>,
    event_bus: Arc<EventBus>,
    network_detector: Arc<Mutex<network::NetworkInterfaceDetector>>,
    receiver: Receiver<DockerMessage>,
    spawns: Arc<Mutex<HashMap<SpawnId, JoinHandle<()>>>>,
    #[allow(dead_code)]
    event_handler: Option<Arc<DockerEventHandler>>,
    last_full_sync: Arc<Mutex<std::time::Instant>>,
    container_tracker: Arc<Mutex<std::collections::HashSet<String>>>,
    stats_metrics: Arc<StatsMetrics>,
}

impl DockerData {
    /// Use docker stats to calculate current cpu usage
    #[allow(clippy::cast_precision_loss)]
    fn calculate_usage(stats: &ContainerStatsResponse) -> f64 {
        let mut cpu_percentage = 0.0;

        let total_usage = stats.precpu_stats.as_ref().map_or(0, |i| {
            i.cpu_usage
                .as_ref()
                .map_or(0, |i| i.total_usage.unwrap_or_default())
        });

        let cpu_delta = stats.cpu_stats.as_ref().map_or(0, |i| {
            i.cpu_usage.as_ref().map_or(0, |i| {
                i.total_usage
                    .unwrap_or_default()
                    .saturating_sub(total_usage)
            })
        }) as f64;

        if let (Some(Some(cpu_stats_usage)), Some(Some(precpu_stats_usage))) = (
            stats.cpu_stats.as_ref().map(|i| i.system_cpu_usage),
            stats.precpu_stats.as_ref().map(|i| i.system_cpu_usage),
        ) {
            let system_delta = cpu_stats_usage.saturating_sub(precpu_stats_usage) as f64;
            let online_cpus = f64::from(stats.cpu_stats.as_ref().map_or(0, |i| {
                i.online_cpus.unwrap_or_else(|| {
                    u32::try_from(
                        stats
                            .cpu_stats
                            .clone()
                            .unwrap_or_default()
                            .cpu_usage
                            .unwrap_or_default()
                            .percpu_usage
                            .as_ref()
                            .map_or(0, std::vec::Vec::len),
                    )
                    .unwrap_or_default()
                })
            }));
            if system_delta > 0.0 && cpu_delta > 0.0 {
                cpu_percentage = (cpu_delta / system_delta) * online_cpus * 100.0;
            }
        }
        cpu_percentage
    }

    /// Get a single docker stat in order to update mem and cpu usage
    /// don't take &self, so that can tokio::spawn into it's own thread
    /// remove if from spawns hashmap when complete
    #[allow(clippy::too_many_arguments)]
    async fn update_container_stat(
        app_data: Arc<Mutex<AppData>>,
        docker: Arc<Docker>,
        state: State,
        spawn_id: SpawnId,
        spawns: Arc<Mutex<HashMap<SpawnId, JoinHandle<()>>>>,
        event_bus: Arc<EventBus>,
        network_detector: Arc<Mutex<network::NetworkInterfaceDetector>>,
        network_interface_config: Option<String>,
        stats_metrics: Arc<StatsMetrics>,
    ) {
        let start_time = std::time::Instant::now();
        let id = spawn_id.get_id();

        // Track task started
        stats_metrics.task_started();

        let mut stream = docker
            .stats(
                id.get(),
                Some(StatsOptions {
                    stream: false,
                    one_shot: false,
                }),
            )
            .take(1);

        // Track API call
        stats_metrics.api_call_made();

        while let Some(Ok(stats)) = stream.next().await {
            // Memory stats are only collected if the container is alive - is this the behaviour we want?

            let mem_limit = stats
                .memory_stats
                .as_ref()
                .and_then(|m| m.limit)
                .unwrap_or_default();

            let (mem_stat, cpu_stats) = if state.is_alive() {
                let mem_cache = stats.memory_stats.as_ref().map_or(&0, |i| {
                    i.stats
                        .as_ref()
                        .map_or(&0, |i| i.get("inactive_file").unwrap_or(&0))
                });
                (
                    Some(
                        stats
                            .memory_stats
                            .as_ref()
                            .map_or(0, |i| i.usage.unwrap_or_default())
                            .saturating_sub(*mem_cache),
                    ),
                    Some(Self::calculate_usage(&stats)),
                )
            } else {
                (None, None)
            };

            // Use configured or auto-detected network interface
            let (rx, tx) = stats.networks.as_ref().map_or((0, 0), |networks| {
                let interface = network_detector.lock().detect_interface(
                    id.get(),
                    networks,
                    network_interface_config.as_deref().or(Some("auto")),
                );

                interface
                    .and_then(|iface| networks.get(&iface))
                    .map_or((0, 0), |x| {
                        (
                            x.rx_bytes.unwrap_or_default(),
                            x.tx_bytes.unwrap_or_default(),
                        )
                    })
            });

            app_data
                .lock()
                .update_stats_by_id(id, cpu_stats, mem_stat, mem_limit, rx, tx);

            // Publish stats update event
            if let (Some(cpu), Some(mem)) = (cpu_stats, mem_stat) {
                let _ = event_bus
                    .publish(CoreEvent::ContainerStatsUpdate {
                        container_id: id.get().to_string(),
                        stats: Stats {
                            container_id: id.get().to_string(),
                            cpu_usage: cpu,
                            memory_usage: mem,
                            memory_limit: mem_limit,
                            network_rx: rx,
                            network_tx: tx,
                        },
                    })
                    .await;
            }
        }

        // Track latency and task completion
        let latency_ms = u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX);
        stats_metrics.update_latency(latency_ms);
        stats_metrics.task_finished();

        spawns.lock().remove(&spawn_id);
    }

    /// Update all stats, spawn each container into own tokio::spawn thread
    fn update_all_container_stats(&mut self) {
        let all_ids = self.app_data.lock().get_all_id_state();

        // Log the optimization mode
        if self.config.stats_optimization_enabled {
            trace!("Stats optimization enabled - polling only running containers");
        }

        for (state, id) in all_ids {
            // Skip non-running containers if optimization is enabled
            if self.config.stats_optimization_enabled && !state.is_alive() {
                // Track skipped container
                self.stats_metrics.container_skipped();

                // Clear stats data for stopped containers
                self.app_data.lock().update_stats_by_id(
                    &id, None, // Clear CPU stats
                    None, // Clear memory stats
                    0,    // Clear memory limit
                    0,    // Clear RX
                    0,    // Clear TX
                );

                // Remove any existing stats task for non-running containers
                let spawn_id = SpawnId::Stats((id.clone(), self.binate));
                let handle = self.spawns.lock().remove(&spawn_id);
                if let Some(handle) = handle {
                    handle.abort();
                    trace!(
                        "Cancelled stats task for non-running container: {}",
                        id.get()
                    );
                }

                // Also remove the opposite binate task
                let opposite_spawn_id = SpawnId::Stats((id, self.binate.toggle()));
                let handle = self.spawns.lock().remove(&opposite_spawn_id);
                if let Some(handle) = handle {
                    handle.abort();
                    trace!("Cancelled opposite binate stats task for non-running container");
                }
                continue;
            }

            let spawn_id = SpawnId::Stats((id, self.binate));

            if let std::collections::hash_map::Entry::Vacant(spawns) =
                self.spawns.lock().entry(spawn_id.clone())
            {
                spawns.insert(tokio::spawn(Self::update_container_stat(
                    Arc::clone(&self.app_data),
                    Arc::clone(&self.docker),
                    state,
                    spawn_id,
                    Arc::clone(&self.spawns),
                    Arc::clone(&self.event_bus),
                    Arc::clone(&self.network_detector),
                    self.config.network_interface.clone(),
                    Arc::clone(&self.stats_metrics),
                )));
            }
        }
        self.binate = self.binate.toggle();
    }

    /// Get all current containers, handle into ContainerItem in the app_data struct rather than here
    /// Just make sure that items sent are guaranteed to have an id
    /// If in a containerised runtime, will ignore any container that uses the `/app/oxker` as an entry point, unless the `-s` flag is set
    async fn update_all_containers(&self) {
        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                ..Default::default()
            }))
            .await
            .unwrap_or_default();

        // Track container IDs for validation
        self.container_tracker.lock().clear();

        let output = containers
            .into_iter()
            .filter_map(|f| match f.id {
                Some(ref id) => {
                    // Add to tracker
                    self.container_tracker.lock().insert(id.clone());

                    if self.config.in_container
                        && f.command
                            .as_ref()
                            .is_some_and(|c| c.starts_with(ENTRY_POINT))
                        && self.config.show_self
                    {
                        None
                    } else {
                        Some(f)
                    }
                }
                None => None,
            })
            .collect::<Vec<ContainerSummary>>();
        self.app_data.lock().update_containers(output);
    }

    /// Update single container logs
    /// remove it from spawns hashmap when complete
    async fn update_log(
        app_data: Arc<Mutex<AppData>>,
        docker: Arc<Docker>,
        id: ContainerId,
        since: u64,
        spawns: Arc<Mutex<HashMap<SpawnId, JoinHandle<()>>>>,
        stderr: bool,
        event_bus: Arc<EventBus>,
    ) {
        let options = Some(LogsOptions {
            stdout: true,
            stderr,
            timestamps: true,
            since: i32::try_from(since).unwrap_or_default(),
            ..Default::default()
        });

        let mut logs = docker.logs(id.get(), options);
        let mut output = vec![];

        while let Some(Ok(value)) = logs.next().await {
            let data = value.to_string();
            if !data.trim().is_empty() {
                output.push(data);
            }
        }

        // Update internal state
        app_data.lock().update_log_by_id(output.clone(), &id);

        // Always publish logs event, even if empty
        let logs: Vec<LogLine> = output
            .into_iter()
            .map(|msg| LogLine {
                container_id: id.get().to_string(),
                timestamp: String::new(), // Timestamp is embedded in message
                message: msg,
            })
            .collect();

        let _ = event_bus
            .publish(CoreEvent::ContainerLogsUpdate {
                container_id: id.get().to_string(),
                logs,
            })
            .await;

        spawns.lock().remove(&SpawnId::Log(id));
    }

    /// Update all logs, spawn each container into own tokio::spawn thread
    fn init_all_logs(&self, all_ids: Vec<(State, ContainerId)>) -> Arc<AtomicUsize> {
        let init = Arc::new(AtomicUsize::new(0));
        for (_, id) in all_ids {
            let app_data: Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, AppData>> =
                Arc::clone(&self.app_data);
            let docker = Arc::clone(&self.docker);
            let spawns = Arc::clone(&self.spawns);
            let std_err = self.config.show_std_err;
            let init = Arc::clone(&init);
            let event_bus = Arc::clone(&self.event_bus);
            self.spawns.lock().insert(
                SpawnId::Log(id.clone()),
                tokio::spawn(async move {
                    Self::update_log(app_data, docker, id, 0, spawns, std_err, event_bus).await;
                    init.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }),
            );
        }
        init
    }

    /// Initialize docker container data, before any messages are received
    async fn initialise_container_data(&mut self) {
        let loading_uuid = Uuid::new_v4();
        drop(
            self.event_bus
                .publish(CoreEvent::LoadingStarted(loading_uuid.to_string())),
        );

        self.update_all_containers().await;
        let all_ids = self.app_data.lock().get_all_id_state();
        let all_ids_len = all_ids.len();
        let init = self.init_all_logs(all_ids);
        self.update_all_container_stats();

        while init.load(std::sync::atomic::Ordering::SeqCst) != all_ids_len {
            // Don't sort containers automatically - only sort when user requests it
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        drop(
            self.event_bus
                .publish(CoreEvent::LoadingFinished(loading_uuid.to_string())),
        );
    }

    /// Update all cpu_mem, and selected container log (if a log update join_handle isn't currently being executed)
    async fn update_everything(&mut self) {
        self.update_all_containers().await;
        if let Some(container) = self.app_data.lock().get_selected_container() {
            let last_updated = container.last_updated;
            let spawn_id = SpawnId::Log(container.id.clone());
            // Only spawn if not already spawned with a given id/binate pair
            if let std::collections::hash_map::Entry::Vacant(spawns) =
                self.spawns.lock().entry(spawn_id)
            {
                spawns.insert(tokio::spawn(Self::update_log(
                    Arc::clone(&self.app_data),
                    Arc::clone(&self.docker),
                    container.id.clone(),
                    last_updated,
                    Arc::clone(&self.spawns),
                    self.config.show_std_err,
                    Arc::clone(&self.event_bus),
                )));
            }
        }
        self.update_all_container_stats();
        // Don't sort containers automatically - only sort when user requests it
        // self.app_data.lock().sort_containers();
    }

    /// Set the global error as the docker error
    fn set_error(app_data: &Arc<Mutex<AppData>>, error: DockerCommand, event_bus: &Arc<EventBus>) {
        let error = AppError::DockerCommand(error);
        drop(event_bus.publish(CoreEvent::Error(error.to_string())));
        app_data.lock().set_error(error);
    }

    /// Execute docker commands (start, stop etc) on it's own tokio thread
    async fn execute_command(&mut self, control: DockerCommand, id: ContainerId) {
        let (app_data, docker, event_bus) = (
            Arc::clone(&self.app_data),
            Arc::clone(&self.docker),
            Arc::clone(&self.event_bus),
        );
        tokio::spawn(async move {
            let uuid = Uuid::new_v4();
            drop(event_bus.publish(CoreEvent::LoadingStarted(uuid.to_string())));

            if match control {
                DockerCommand::Delete => {
                    drop(
                        event_bus
                            .publish(CoreEvent::ContainerDeletionStarted(id.get().to_string())),
                    );
                    docker
                        .remove_container(
                            id.get(),
                            Some(RemoveContainerOptions {
                                v: false,
                                force: true,
                                link: false,
                            }),
                        )
                        .await
                }
                DockerCommand::Pause => docker.pause_container(id.get()).await,
                DockerCommand::Restart => {
                    docker
                        .restart_container(id.get(), None::<RestartContainerOptions>)
                        .await
                }
                DockerCommand::Resume => docker.unpause_container(id.get()).await,
                DockerCommand::Start => {
                    docker
                        .start_container(id.get(), None::<StartContainerOptions>)
                        .await
                }
                DockerCommand::Stop => {
                    docker
                        .stop_container(id.get(), None::<StopContainerOptions>)
                        .await
                }
            }
            .is_err()
            {
                Self::set_error(&app_data, control, &event_bus);
            }

            drop(event_bus.publish(CoreEvent::LoadingFinished(uuid.to_string())));
        });

        self.update_everything().await;
    }

    /// Periodic full sync task for event-driven mode
    fn start_periodic_sync(&self, docker_tx: Sender<DockerMessage>) -> Option<JoinHandle<()>> {
        if !self.config.event_driven_mode {
            return None;
        }

        let sync_interval = std::time::Duration::from_millis(u64::from(
            self.config.full_sync_interval_ms.clamp(30_000, 300_000),
        ));

        info!(
            "Starting periodic full sync with interval: {:?}",
            sync_interval
        );

        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(sync_interval);
            loop {
                interval.tick().await;
                if docker_tx.send(DockerMessage::FullSync).await.is_err() {
                    break;
                }
            }
        }))
    }

    /// Perform a full container list sync
    async fn perform_full_sync(&self) {
        info!("Performing full container list sync");
        let start = std::time::Instant::now();

        // Get containers before sync for comparison
        let before_count = self.container_tracker.lock().len();

        self.update_all_containers().await;
        *self.last_full_sync.lock() = std::time::Instant::now();

        // Validate container tracking
        let after_count = self.container_tracker.lock().len();
        let sync_duration = start.elapsed();

        // Log metrics
        let _ = self
            .event_bus
            .publish(CoreEvent::DebugInfo {
                category: "Sync".to_string(),
                message: {
                    let delta = if after_count >= before_count {
                        format!("+{}", after_count - before_count)
                    } else {
                        format!("-{}", before_count - after_count)
                    };
                    format!(
                        "Full sync completed: {after_count} containers (Δ{delta}), took {sync_duration:?}"
                    )
                },
                metadata: Some(format!("before={before_count}, after={after_count}")),
            })
            .await;

        // Check for consistency
        self.validate_container_consistency().await;
    }

    /// Validate that tracked containers match actual containers
    async fn validate_container_consistency(&self) {
        let tracked = self.container_tracker.lock().clone();
        let actual_containers = self
            .docker
            .list_containers(Some(bollard::query_parameters::ListContainersOptions {
                all: true,
                ..Default::default()
            }))
            .await
            .unwrap_or_default();

        let actual_ids: std::collections::HashSet<String> = actual_containers
            .iter()
            .filter_map(|c| c.id.clone())
            .collect();

        // Check for missed containers
        let missed: Vec<_> = actual_ids.difference(&tracked).cloned().collect();
        let phantom: Vec<_> = tracked.difference(&actual_ids).cloned().collect();

        if !missed.is_empty() {
            warn!(
                "Container tracking missed {} containers: {:?}",
                missed.len(),
                missed
            );
            let _ = self
                .event_bus
                .publish(CoreEvent::DebugInfo {
                    category: "Validation".to_string(),
                    message: format!("Missed {} containers", missed.len()),
                    metadata: Some(format!("{missed:?}")),
                })
                .await;
        }

        if !phantom.is_empty() {
            warn!(
                "Container tracking has {} phantom containers: {:?}",
                phantom.len(),
                phantom
            );
            let _ = self
                .event_bus
                .publish(CoreEvent::DebugInfo {
                    category: "Validation".to_string(),
                    message: format!("Phantom {} containers", phantom.len()),
                    metadata: Some(format!("{phantom:?}")),
                })
                .await;
        }

        if missed.is_empty() && phantom.is_empty() {
            debug!(
                "Container tracking validation passed: {} containers",
                tracked.len()
            );
        }
    }

    /// Handle incoming messages, container controls & all container information update
    /// Spawn Docker commands off into own thread
    async fn message_handler(&mut self) {
        while let Some(message) = self.receiver.recv().await {
            match message {
                DockerMessage::ConfirmDelete(id) => {
                    drop(
                        self.event_bus
                            .publish(CoreEvent::ContainerDeletionStarted(id.get().to_string())),
                    );
                }
                DockerMessage::Control((command, id)) => self.execute_command(command, id).await,
                DockerMessage::Exec(docker_tx) => {
                    docker_tx.send(Arc::clone(&self.docker)).ok();
                }
                DockerMessage::Update => {
                    if self.config.event_driven_mode {
                        // In event-driven mode, only update stats and logs
                        if let Some(container) = self.app_data.lock().get_selected_container() {
                            let last_updated = container.last_updated;
                            let spawn_id = SpawnId::Log(container.id.clone());
                            // Only spawn if not already spawned with a given id/binate pair
                            if let std::collections::hash_map::Entry::Vacant(spawns) =
                                self.spawns.lock().entry(spawn_id)
                            {
                                spawns.insert(tokio::spawn(Self::update_log(
                                    Arc::clone(&self.app_data),
                                    Arc::clone(&self.docker),
                                    container.id.clone(),
                                    last_updated,
                                    Arc::clone(&self.spawns),
                                    self.config.show_std_err,
                                    Arc::clone(&self.event_bus),
                                )));
                            }
                        }
                        self.update_all_container_stats();
                    } else {
                        // In polling mode, update everything
                        self.update_everything().await;
                    }
                }
                DockerMessage::EventUpdate => {
                    // Handle event-driven container list update
                    if self.config.event_driven_mode {
                        info!("Processing event-driven container list update");
                        self.update_all_containers().await;
                    }
                }
                DockerMessage::FullSync => {
                    // Perform full sync (event-driven mode)
                    if self.config.event_driven_mode {
                        self.perform_full_sync().await;
                    }
                }
                DockerMessage::RefreshLogs(container_id) => {
                    // Fetch logs for the specified container without changing selection
                    let container_id_obj = ContainerId::from(container_id.as_str());

                    // Get the container to check if it exists
                    // For RefreshLogs, always get all logs from the beginning (timestamp 0)
                    let container_info = {
                        let app_data = self.app_data.lock();
                        app_data
                            .get_container_items()
                            .iter()
                            .find(|c| c.id == container_id_obj)
                            .map(|c| (c.id.clone(), 0u64)) // Always get all logs
                    };

                    if let Some((container_id, last_updated)) = container_info {
                        let spawn_id = SpawnId::Log(container_id.clone());

                        // Only spawn if not already spawned with a given id
                        if let std::collections::hash_map::Entry::Vacant(spawns) =
                            self.spawns.lock().entry(spawn_id)
                        {
                            spawns.insert(tokio::spawn(Self::update_log(
                                Arc::clone(&self.app_data),
                                Arc::clone(&self.docker),
                                container_id,
                                last_updated,
                                Arc::clone(&self.spawns),
                                self.config.show_std_err,
                                Arc::clone(&self.event_bus),
                            )));
                        }
                    }
                }
                DockerMessage::Shutdown => {
                    info!("DockerData received shutdown signal");
                    // Abort all spawned tasks
                    for (_id, handle) in self.spawns.lock().drain() {
                        handle.abort();
                    }
                    break; // Exit the loop to shutdown
                }
            }
        }
        info!("DockerData shutting down");
    }

    /// Send an update message every x ms, where x is the args.docker_interval
    /// In event-driven mode, this is kept commented for rollback capability
    fn heartbeat(config: &Config, docker_tx: Sender<DockerMessage>) -> Option<JoinHandle<()>> {
        // If event-driven mode is enabled, don't start the polling heartbeat
        if config.event_driven_mode {
            info!("Event-driven mode enabled - polling heartbeat disabled");
            return None;
        }

        // Original polling implementation - kept for rollback capability
        let update_duration =
            std::time::Duration::from_millis(u64::from(config.docker_interval_ms));
        let mut now = std::time::Instant::now();
        Some(tokio::spawn(async move {
            loop {
                if docker_tx.send(DockerMessage::Update).await.is_err() {
                    // Channel closed, exit heartbeat
                    break;
                }
                if let Some(to_sleep) = update_duration.checked_sub(now.elapsed()) {
                    tokio::time::sleep(to_sleep).await;
                }
                now = std::time::Instant::now();
            }
        }))
    }

    /// Initialise self, and start the message receiving loop
    pub async fn start(
        app_data: Arc<Mutex<AppData>>,
        docker: Docker,
        docker_rx: Receiver<DockerMessage>,
        docker_tx: Sender<DockerMessage>,
        event_bus: Arc<EventBus>,
    ) {
        let args = app_data.lock().config.clone();
        if app_data.lock().get_error().is_none() {
            // Create event handler if in event-driven mode
            let event_handler = if args.event_driven_mode {
                info!("Initializing event-driven mode");
                // EventBus needs to be dereferenced from Arc
                let (event_bus_for_handler, _) = EventBus::new(100);
                let handler = Arc::new(DockerEventHandler::new(
                    docker.clone(),
                    event_bus_for_handler,
                    std::time::Duration::from_secs(5),
                ));

                // Start event stream in background
                let docker_tx_clone = docker_tx.clone();
                let docker_clone = docker.clone();
                tokio::spawn(async move {
                    // Connect event stream to message handler
                    let (local_bus, mut rx) = EventBus::new(100);
                    let stream_handler = DockerEventHandler::new(
                        docker_clone,
                        local_bus,
                        std::time::Duration::from_secs(5),
                    );

                    // Start event stream
                    tokio::spawn(async move {
                        stream_handler.start_event_stream().await;
                    });

                    // Forward events to message handler
                    while let Some(event) = rx.recv().await {
                        match event {
                            CoreEvent::ContainerListUpdated | CoreEvent::ContainerRemoved(_) => {
                                let _ = docker_tx_clone.send(DockerMessage::EventUpdate).await;
                            }
                            _ => {}
                        }
                    }
                });

                Some(handler)
            } else {
                None
            };

            let stats_metrics = Arc::new(StatsMetrics::new(args.stats_optimization_enabled));

            // Log the optimization mode at startup
            if args.stats_optimization_enabled {
                info!("Stats optimization enabled - will poll only running containers");
            } else {
                info!("Stats optimization disabled - using legacy polling mode");
            }

            let mut inner = Self {
                app_data,
                config: args.clone(),
                binate: Binate::One,
                docker: Arc::new(docker),
                event_bus,
                network_detector: Arc::new(Mutex::new(network::NetworkInterfaceDetector::new())),
                receiver: docker_rx,
                spawns: Arc::new(Mutex::new(HashMap::new())),
                event_handler,
                last_full_sync: Arc::new(Mutex::new(std::time::Instant::now())),
                container_tracker: Arc::new(Mutex::new(std::collections::HashSet::new())),
                stats_metrics,
            };

            inner.initialise_container_data().await;

            // Start heartbeat for polling mode or periodic sync for event-driven mode
            let heartbeat_handle = Self::heartbeat(&inner.config, docker_tx.clone());
            let sync_handle = inner.start_periodic_sync(docker_tx);

            inner.message_handler().await;

            // Cleanup: abort handles when message handler exits
            if let Some(handle) = heartbeat_handle {
                handle.abort();
            }
            if let Some(handle) = sync_handle {
                handle.abort();
            }
        }
    }
}

// tests, use redis-test container, check logs exists, and selector of logs, and that it increases, and matches end, when you run restart on the docker containers
#[cfg(test)]
mod tests {

    use bollard::secret::{ContainerCpuStats, ContainerCpuUsage};

    use super::*;

    fn gen_stats() -> ContainerStatsResponse {
        ContainerStatsResponse {
            read: None,
            preread: None,
            num_procs: Some(1),
            pids_stats: None,
            networks: None,
            memory_stats: None,
            blkio_stats: None,
            cpu_stats: Some(ContainerCpuStats {
                cpu_usage: Some(ContainerCpuUsage {
                    percpu_usage: Some(vec![50]),
                    usage_in_usermode: Some(10),
                    total_usage: Some(100),
                    usage_in_kernelmode: Some(20),
                }),
                system_cpu_usage: Some(400),
                online_cpus: Some(1),
                throttling_data: None,
            }),
            precpu_stats: Some(ContainerCpuStats {
                cpu_usage: Some(ContainerCpuUsage {
                    percpu_usage: Some(vec![50]),
                    usage_in_usermode: Some(10),
                    total_usage: Some(100),
                    usage_in_kernelmode: Some(20),
                }),
                system_cpu_usage: Some(400),
                online_cpus: Some(1),
                throttling_data: None,
            }),
            storage_stats: None,
            name: None,
            id: None,
        }
    }

    #[test]
    fn test_calculate_usage_50() {
        let mut stats = gen_stats();
        stats.precpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![50]),
                usage_in_usermode: Some(10),
                total_usage: Some(100),
                usage_in_kernelmode: Some(20),
            }),
            system_cpu_usage: Some(400),
            online_cpus: Some(1),
            throttling_data: None,
        });
        stats.cpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![150]),
                usage_in_usermode: Some(20),
                total_usage: Some(150),
                usage_in_kernelmode: Some(30),
            }),
            system_cpu_usage: Some(500),
            online_cpus: Some(1),
            throttling_data: None,
        });
        let cpu_percentage = DockerData::calculate_usage(&stats);
        assert!((cpu_percentage - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_usage_25() {
        let mut stats = gen_stats();
        stats.precpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![50]),
                usage_in_usermode: Some(10),
                total_usage: Some(100),
                usage_in_kernelmode: Some(20),
            }),
            system_cpu_usage: Some(400),
            online_cpus: Some(1),
            throttling_data: None,
        });
        stats.cpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![75]),
                usage_in_usermode: Some(20),
                total_usage: Some(125),
                usage_in_kernelmode: Some(30),
            }),
            system_cpu_usage: Some(500),
            online_cpus: Some(1),
            throttling_data: None,
        });
        let cpu_percentage = DockerData::calculate_usage(&stats);
        assert!((cpu_percentage - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_usage_75() {
        let mut stats = gen_stats();
        stats.precpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![50]),
                usage_in_usermode: Some(10),
                total_usage: Some(100),
                usage_in_kernelmode: Some(20),
            }),
            system_cpu_usage: Some(400),
            online_cpus: Some(1),
            throttling_data: None,
        });
        stats.cpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![175]),
                usage_in_usermode: Some(20),
                total_usage: Some(175),
                usage_in_kernelmode: Some(30),
            }),
            system_cpu_usage: Some(500),
            online_cpus: Some(1),
            throttling_data: None,
        });
        let cpu_percentage = DockerData::calculate_usage(&stats);
        assert!((cpu_percentage - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_usage_100() {
        let mut stats = gen_stats();
        stats.precpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![50]),
                usage_in_usermode: Some(10),
                total_usage: Some(100),
                usage_in_kernelmode: Some(20),
            }),
            system_cpu_usage: Some(400),
            online_cpus: Some(1),
            throttling_data: None,
        });
        stats.cpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![200]),
                usage_in_usermode: Some(20),
                total_usage: Some(200),
                usage_in_kernelmode: Some(30),
            }),
            system_cpu_usage: Some(500),
            online_cpus: Some(1),
            throttling_data: None,
        });
        let cpu_percentage = DockerData::calculate_usage(&stats);
        assert!((cpu_percentage - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_usage_175() {
        let mut stats = gen_stats();
        stats.precpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![50]),
                usage_in_usermode: Some(10),
                total_usage: Some(100),
                usage_in_kernelmode: Some(20),
            }),
            system_cpu_usage: Some(400),
            online_cpus: Some(1),
            throttling_data: None,
        });
        stats.cpu_stats = Some(ContainerCpuStats {
            cpu_usage: Some(ContainerCpuUsage {
                percpu_usage: Some(vec![275]),
                usage_in_usermode: Some(20),
                total_usage: Some(275),
                usage_in_kernelmode: Some(30),
            }),
            system_cpu_usage: Some(500),
            online_cpus: Some(1),
            throttling_data: None,
        });
        let cpu_percentage = DockerData::calculate_usage(&stats);
        assert!((cpu_percentage - 175.0).abs() < f64::EPSILON);
    }
}
