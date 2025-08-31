//! Mock implementation of CoreHandle for testing

use oxker_core::{
    AppColors, Config, ContainerId, ContainerItem, ContainerItemInit, ContainerPorts,
    ContainerStatus, CoreCommand, CoreEvent, CoreHandle, CoreStateView, EventBus, Keymap,
    RunningState, State,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Mock implementation of CoreHandle for testing UI components without Docker
pub struct MockCoreHandle {
    pub commands: Arc<Mutex<Vec<CoreCommand>>>,
    pub containers: Arc<Mutex<Vec<ContainerItem>>>,
    pub logs: Arc<Mutex<HashMap<String, Vec<String>>>>,
    pub exec_outputs: Arc<Mutex<HashMap<String, String>>>,
    event_bus: EventBus,
    handle: CoreHandle,
}

impl MockCoreHandle {
    #[must_use]
    pub fn new(event_bus: EventBus) -> Self {
        let config = Self::gen_config();
        let handle = CoreHandle::new(event_bus.clone(), &config);

        Self {
            commands: Arc::new(Mutex::new(Vec::new())),
            containers: Arc::new(Mutex::new(Self::default_containers())),
            logs: Arc::new(Mutex::new(HashMap::new())),
            exec_outputs: Arc::new(Mutex::new(HashMap::new())),
            event_bus,
            handle,
        }
    }

    /// Get default test containers
    fn default_containers() -> Vec<ContainerItem> {
        vec![
            Self::gen_item(&ContainerId::from("test1"), 1),
            Self::gen_item(&ContainerId::from("test2"), 2),
            Self::gen_item(&ContainerId::from("test3"), 3),
        ]
    }

    /// Generate test config with defaults
    fn gen_config() -> Config {
        Config {
            color_logs: false,
            docker_interval_ms: 1000,
            gui: true,
            host: None,
            show_std_err: false,
            in_container: false,
            save_dir: None,
            raw_logs: false,
            show_self: false,
            app_colors: AppColors::new(),
            keymap: Keymap::new(),
            network_interface: None,
            timestamp_format: "HH:MM:SS.NNNNN dd-mm-yyyy".to_owned(),
            show_timestamp: false,
            show_logs: true,
            timezone: None,
            debug_mode: false,
        }
    }

    /// Generate a test container item
    fn gen_item(id: &ContainerId, index: usize) -> ContainerItem {
        ContainerItem::new(ContainerItemInit {
            created: u64::try_from(index).unwrap_or(0),
            id: id.clone(),
            image: format!("image_{index}"),
            is_oxker: false,
            name: format!("container_{index}"),
            ports: vec![ContainerPorts {
                ip: None,
                private: u16::try_from(index).unwrap_or(0) + 8000,
                public: Some(u16::try_from(index).unwrap_or(0) + 9000),
            }],
            state: State::Running(RunningState::Healthy),
            status: ContainerStatus::from("Up 1 hour".to_owned()),
        })
    }

    /// Add a test container
    pub fn add_container(&self, container: ContainerItem) {
        self.containers.lock().push(container);
    }

    /// Set containers for testing
    pub fn set_containers(&self, containers: Vec<ContainerItem>) {
        *self.containers.lock() = containers;
    }

    /// Add log entries for a container
    pub fn add_logs(&self, container_id: &str, logs: Vec<String>) {
        self.logs.lock().insert(container_id.to_string(), logs);
    }

    /// Set exec output for a container
    pub fn set_exec_output(&self, container_id: &str, output: &str) {
        self.exec_outputs
            .lock()
            .insert(container_id.to_string(), output.to_string());
    }

    /// Get recorded commands
    #[must_use]
    pub fn get_commands(&self) -> Vec<CoreCommand> {
        self.commands.lock().clone()
    }

    /// Clear recorded commands
    pub fn clear_commands(&self) {
        self.commands.lock().clear();
    }

    /// Send mock events
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be published to the event bus
    pub async fn send_event(&self, event: CoreEvent) -> Result<(), String> {
        self.event_bus.publish(event).await
    }

    /// Execute a command and record it
    ///
    /// # Errors
    ///
    /// Returns an error if command execution fails or events cannot be sent
    pub async fn execute_command(&self, command: CoreCommand) -> Result<(), String> {
        self.commands.lock().push(command.clone());

        // Simulate responses based on command type
        match command {
            CoreCommand::RefreshContainers => {
                let containers = self.containers.lock().clone();
                let event_containers = containers
                    .iter()
                    .map(|c| oxker_core::events::types::ContainerItem {
                        id: c.id.get().to_string(),
                        name: c.name.to_string(),
                        image: c.image.to_string(),
                        state: c.state.as_str().to_string(),
                        status: c.status.to_string(),
                        ports: c
                            .ports
                            .iter()
                            .map(|p| oxker_core::events::types::ContainerPort {
                                ip: p.ip.map(|ip| ip.to_string()),
                                private: p.private,
                                public: p.public,
                            })
                            .collect(),
                    })
                    .collect();
                self.send_event(CoreEvent::ContainerListUpdate(event_containers))
                    .await?;
            }
            CoreCommand::RefreshLogs(container_id) => {
                let logs = {
                    let log_guard = self.logs.lock();
                    log_guard
                        .get(&container_id)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|msg| oxker_core::events::types::LogLine {
                            container_id: container_id.clone(),
                            timestamp: String::new(),
                            message: msg,
                        })
                        .collect()
                };
                self.send_event(CoreEvent::ContainerLogsUpdate { container_id, logs })
                    .await?;
            }
            CoreCommand::StartContainer(id) => {
                // Update container state
                {
                    let mut containers = self.containers.lock();
                    if let Some(container) = containers.iter_mut().find(|c| c.id.get() == id) {
                        container.state = State::Running(RunningState::Healthy);
                        container.status = ContainerStatus::from("Up 1 second".to_string());
                    }
                }

                // Send notification event
                self.handle
                    .execute_command(CoreCommand::RefreshContainers)
                    .await?;
            }
            CoreCommand::StopContainer(id) => {
                // Update container state
                {
                    let mut containers = self.containers.lock();
                    if let Some(container) = containers.iter_mut().find(|c| c.id.get() == id) {
                        container.state = State::Exited;
                        container.status =
                            ContainerStatus::from("Exited (0) 1 second ago".to_string());
                    }
                }

                // Send notification event
                self.handle
                    .execute_command(CoreCommand::RefreshContainers)
                    .await?;
            }
            CoreCommand::RestartContainer(id) => {
                // Simulate restart by updating state twice
                {
                    let mut containers = self.containers.lock();
                    if let Some(container) = containers.iter_mut().find(|c| c.id.get() == id) {
                        // First set to exited
                        container.state = State::Exited;
                        container.status =
                            ContainerStatus::from("Exited (0) 1 second ago".to_string());
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                {
                    let mut containers = self.containers.lock();
                    if let Some(container) = containers.iter_mut().find(|c| c.id.get() == id) {
                        // Then set to running
                        container.state = State::Running(RunningState::Healthy);
                        container.status = ContainerStatus::from("Up 1 second".to_string());
                    }
                }

                self.handle
                    .execute_command(CoreCommand::RefreshContainers)
                    .await?;
            }
            CoreCommand::PauseContainer(id) => {
                {
                    let mut containers = self.containers.lock();
                    if let Some(container) = containers.iter_mut().find(|c| c.id.get() == id) {
                        container.state = State::Paused;
                        container.status =
                            ContainerStatus::from("Up 1 minute (Paused)".to_string());
                    }
                }

                self.handle
                    .execute_command(CoreCommand::RefreshContainers)
                    .await?;
            }
            CoreCommand::UnpauseContainer(id) => {
                {
                    let mut containers = self.containers.lock();
                    if let Some(container) = containers.iter_mut().find(|c| c.id.get() == id) {
                        container.state = State::Running(RunningState::Healthy);
                        container.status = ContainerStatus::from("Up 1 minute".to_string());
                    }
                }

                self.handle
                    .execute_command(CoreCommand::RefreshContainers)
                    .await?;
            }
            CoreCommand::RemoveContainer(id) => {
                {
                    let mut containers = self.containers.lock();
                    containers.retain(|c| c.id.get() != id);
                }

                self.handle
                    .execute_command(CoreCommand::RefreshContainers)
                    .await?;
            }
            CoreCommand::ExecuteCommand {
                container_id: id,
                command: _cmd,
            } => {
                let output = self
                    .exec_outputs
                    .lock()
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "Mock exec output".to_string());

                // For now, just log it
                tracing::debug!("Mock exec output for {}: {}", id, output);
            }
            _ => {
                // For other commands, delegate to real handle
                self.handle.execute_command(command).await?;
            }
        }

        Ok(())
    }

    /// Get the state view
    #[must_use]
    pub fn state_view(&self) -> CoreStateView {
        self.handle.state_view()
    }

    /// Get the keymap
    #[must_use]
    pub fn get_keymap(&self) -> Keymap {
        self.handle.get_keymap()
    }

    /// Check if selected container is oxker
    #[must_use]
    pub fn is_oxker(&self) -> bool {
        self.handle.is_oxker()
    }
}
