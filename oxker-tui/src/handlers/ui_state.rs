use oxker_core::events::types::{
    ContainerItem as EventContainerItem, LogLine, Stats as EventStats,
};
use oxker_core::{
    ContainerId, ContainerItem, DockerCommand, Header, State as ContainerState, StatefulList,
};
use std::collections::VecDeque;

/// UI-specific state for managing containers
/// This maintains the state needed for rendering and user interaction
#[derive(Debug)]
pub struct UIContainerState {
    /// List of containers with selection state
    pub containers: StatefulList<ContainerItem>,
    /// Currently selected container for operations
    selected_container_id: Option<ContainerId>,
    /// Docker commands available for the selected container
    pub docker_commands: StatefulList<DockerCommand>,
    /// Logs for the selected container
    pub logs: VecDeque<String>,
    /// Filter term for container list
    pub filter_term: String,
    /// Field to filter by
    pub filter_by: Header,
    /// Current sort header (None means unsorted)
    pub sort_header: Option<Header>,
    /// Sort order (true = ascending, false = descending)
    pub sort_ascending: bool,
    /// Version counter for change detection
    version: u64,
    /// Last significant change version
    last_significant_change: u64,
}

impl UIContainerState {
    pub fn new() -> Self {
        Self {
            containers: StatefulList::new(vec![]),
            selected_container_id: None,
            // Default to showing all possible commands
            docker_commands: StatefulList::new(vec![
                DockerCommand::Start,
                DockerCommand::Stop,
                DockerCommand::Restart,
                DockerCommand::Pause,
                DockerCommand::Resume,
                DockerCommand::Delete,
            ]),
            logs: VecDeque::new(),
            filter_term: String::new(),
            filter_by: Header::Name,
            sort_header: None, // Start unsorted
            sort_ascending: true,
            version: 0,
            last_significant_change: 0,
        }
    }

    /// Check if the state has changed significantly since last view model creation
    pub fn has_significant_changes(&self) -> bool {
        self.version != self.last_significant_change
    }

    /// Mark that changes have been rendered
    pub fn mark_changes_rendered(&mut self) {
        self.last_significant_change = self.version;
    }

    /// Increment version for change tracking
    fn increment_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    /// Update the container list from events
    /// Returns the selected container ID if logs should be refreshed
    pub fn update_containers(
        &mut self,
        event_containers: Vec<EventContainerItem>,
    ) -> Option<String> {
        // Preserve selection BEFORE draining items
        let selected_id = self
            .containers
            .state
            .selected()
            .and_then(|idx| self.containers.items.get(idx))
            .map(|c| c.id.clone());

        // Build a map of existing containers to preserve stats
        let mut existing_containers: std::collections::HashMap<String, ContainerItem> = self
            .containers
            .items
            .drain(..)
            .map(|c| (c.id.get().to_string(), c))
            .collect();

        // Convert event containers to UI containers, preserving stats
        let containers: Vec<ContainerItem> = event_containers
            .into_iter()
            .enumerate()
            .map(|(index, ec)| {
                // Check if we have existing container data
                if let Some(mut existing) = existing_containers.remove(&ec.id) {
                    // Update only the fields that change
                    existing.name = ec.name.into();
                    existing.image = ec.image.into();
                    existing.status = oxker_core::ContainerStatus::from(ec.status.clone());
                    existing.ports = ec
                        .ports
                        .iter()
                        .map(|p| oxker_core::ContainerPorts {
                            ip: p.ip.as_ref().and_then(|ip| ip.parse().ok()),
                            private: p.private,
                            public: p.public,
                        })
                        .collect();
                    existing.state = match ec.state.as_str() {
                        "dead" => ContainerState::Dead,
                        "exited" => ContainerState::Exited,
                        "paused" => ContainerState::Paused,
                        "removing" => ContainerState::Removing,
                        "restarting" => ContainerState::Restarting,
                        "running" => {
                            // Check if status contains health info
                            if ec.status.contains("(unhealthy)") {
                                ContainerState::Running(oxker_core::RunningState::Unhealthy)
                            } else {
                                ContainerState::Running(oxker_core::RunningState::Healthy)
                            }
                        }
                        _ => ContainerState::Unknown,
                    };
                    existing
                } else {
                    // Create new container
                    ContainerItem::new(
                        index as u64,
                        ContainerId::from(ec.id.as_str()),
                        ec.image,
                        false, // is_oxker - we'll need to determine this
                        ec.name,
                        ec.ports
                            .iter()
                            .map(|p| oxker_core::ContainerPorts {
                                ip: p.ip.as_ref().and_then(|ip| ip.parse().ok()),
                                private: p.private,
                                public: p.public,
                            })
                            .collect(),
                        // Parse state and health from status
                        match ec.state.as_str() {
                            "dead" => ContainerState::Dead,
                            "exited" => ContainerState::Exited,
                            "paused" => ContainerState::Paused,
                            "removing" => ContainerState::Removing,
                            "restarting" => ContainerState::Restarting,
                            "running" => {
                                // Check if status contains health info
                                if ec.status.contains("(unhealthy)") {
                                    ContainerState::Running(oxker_core::RunningState::Unhealthy)
                                } else {
                                    ContainerState::Running(oxker_core::RunningState::Healthy)
                                }
                            }
                            _ => ContainerState::Unknown,
                        },
                        oxker_core::ContainerStatus::from(ec.status),
                    )
                }
            })
            .collect();

        // Update items without resetting state
        self.containers.items = containers;

        // Restore selection by ID since order might have changed
        if let Some(id) = selected_id {
            if let Some(index) = self.containers.items.iter().position(|c| c.id == id) {
                self.containers.state.select(Some(index));
            } else if !self.containers.items.is_empty() {
                // If selected container was removed, select first
                self.containers.state.select(Some(0));
            }
        } else if !self.containers.items.is_empty() && selected_id.is_none() {
            // Select first container if none selected
            self.containers.state.select(Some(0));
        }

        // Update docker commands for selected container
        self.update_docker_commands();

        self.increment_version();

        // Return selected container ID for log refresh
        self.containers
            .state
            .selected()
            .and_then(|idx| self.containers.items.get(idx))
            .map(|c| c.id.get().to_string())
    }

    /// Update stats for a specific container
    pub fn update_container_stats(&mut self, container_id: String, stats: EventStats) {
        if let Some(container) = self
            .containers
            .items
            .iter_mut()
            .find(|c| c.id.get() == container_id)
        {
            // Update CPU stats - push to back to add newest data on the right
            container
                .cpu_stats
                .push_back(oxker_core::CpuStats::new(stats.cpu_usage));
            if container.cpu_stats.len() > 60 {
                container.cpu_stats.pop_front();
            }

            // Update memory stats - push to back to add newest data on the right
            container
                .mem_stats
                .push_back(oxker_core::ByteStats::new(stats.memory_usage));
            if container.mem_stats.len() > 60 {
                container.mem_stats.pop_front();
            }
            container.mem_limit = oxker_core::ByteStats::new(stats.memory_limit);

            // Update network stats
            container.rx = oxker_core::ByteStats::new(stats.network_rx);
            container.tx = oxker_core::ByteStats::new(stats.network_tx);

            // Only increment version for selected container stats updates
            if self
                .containers
                .state
                .selected()
                .and_then(|idx| self.containers.items.get(idx))
                .map(|c| c.id.get() == container_id)
                .unwrap_or(false)
            {
                self.increment_version();
            }
        }
    }

    /// Add logs for a container
    pub fn add_logs(&mut self, container_id: String, logs: Vec<LogLine>) {
        // Only add logs if this is the selected container
        if let Some(selected) = self
            .containers
            .state
            .selected()
            .and_then(|idx| self.containers.items.get(idx))
        {
            if selected.id.get() == container_id {
                // If logs are empty, it means the container has no logs to show
                if logs.is_empty() {
                    self.logs.clear();
                } else {
                    for log in logs {
                        self.logs.push_back(log.message);
                        // Keep log buffer reasonable size
                        if self.logs.len() > 10000 {
                            self.logs.pop_front();
                        }
                    }
                }
                self.increment_version();
            }
        }
    }

    /// Remove a container
    pub fn remove_container(&mut self, container_id: String) {
        self.containers.items.retain(|c| c.id.get() != container_id);
        // Reset selection if needed
        if self.containers.state.selected().is_some() {
            let len = self.containers.items.len();
            if len == 0 {
                self.containers.state.select(None);
            } else if let Some(selected) = self.containers.state.selected() {
                if selected >= len {
                    self.containers.state.select(Some(len - 1));
                }
            }
        }
        self.increment_version();
    }

    /// Get the currently selected container ID
    pub fn get_selected_container_id(&self) -> Option<ContainerId> {
        self.containers
            .state
            .selected()
            .and_then(|idx| self.containers.items.get(idx))
            .map(|c| c.id.clone())
    }

    /// Navigate to next container
    pub fn next_container(&mut self) {
        self.containers.next();
        self.clear_logs();
        self.update_docker_commands();
        self.increment_version();
    }

    /// Update docker commands based on selected container state
    fn update_docker_commands(&mut self) {
        if let Some(container) = self
            .containers
            .state
            .selected()
            .and_then(|idx| self.containers.items.get(idx))
        {
            let commands = DockerCommand::gen_vec(container.state.clone());

            // Preserve the current selection if possible
            let current_selection = self.docker_commands.state.selected();
            let selected_command = current_selection
                .and_then(|idx| self.docker_commands.items.get(idx))
                .copied();

            self.docker_commands = StatefulList::new(commands);

            // Restore selection if the same command is still available
            if let Some(cmd) = selected_command {
                if let Some(new_index) = self.docker_commands.items.iter().position(|&c| c == cmd) {
                    self.docker_commands.state.select(Some(new_index));
                } else if !self.docker_commands.items.is_empty() {
                    // If the previously selected command is no longer available, select first
                    self.docker_commands.state.select(Some(0));
                }
            } else if !self.docker_commands.items.is_empty() {
                // If no previous selection, select first command
                self.docker_commands.state.select(Some(0));
            }
        }
    }

    /// Navigate to previous container
    pub fn previous_container(&mut self) {
        self.containers.previous();
        self.clear_logs();
        self.update_docker_commands();
        self.increment_version();
    }

    /// Navigate to first container
    pub fn first_container(&mut self) {
        self.containers.start();
        self.clear_logs();
        self.update_docker_commands();
        self.increment_version();
    }

    /// Navigate to last container
    pub fn last_container(&mut self) {
        self.containers.end();
        self.clear_logs();
        self.update_docker_commands();
        self.increment_version();
    }

    /// Get container items for rendering
    pub fn get_container_items(&self) -> Vec<ContainerItem> {
        self.containers.items.clone()
    }

    /// Get the total number of containers
    pub fn get_container_count(&self) -> usize {
        self.containers.items.len()
    }

    /// Clear logs
    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }

    /// Get logs for display
    pub fn get_logs(&self) -> &VecDeque<String> {
        &self.logs
    }

    /// Navigate docker commands
    pub fn next_docker_command(&mut self) {
        self.docker_commands.next();
    }

    pub fn previous_docker_command(&mut self) {
        self.docker_commands.previous();
    }

    pub fn first_docker_command(&mut self) {
        self.docker_commands.start();
    }

    pub fn last_docker_command(&mut self) {
        self.docker_commands.end();
    }

    pub fn get_selected_docker_command(&self) -> Option<&DockerCommand> {
        self.docker_commands
            .state
            .selected()
            .and_then(|idx| self.docker_commands.items.get(idx))
    }

    /// Update filter term
    pub fn set_filter_term(&mut self, term: String) {
        self.filter_term = term;
    }

    pub fn push_filter_char(&mut self, c: char) {
        self.filter_term.push(c);
    }

    pub fn pop_filter_char(&mut self) {
        self.filter_term.pop();
    }

    pub fn clear_filter(&mut self) {
        self.filter_term.clear();
    }

    pub fn next_filter_field(&mut self) {
        self.filter_by = match self.filter_by {
            Header::Name => Header::Image,
            Header::Image => Header::State,
            Header::State => Header::Status,
            Header::Status => Header::Id,
            _ => Header::Name,
        };
    }

    pub fn prev_filter_field(&mut self) {
        self.filter_by = match self.filter_by {
            Header::Name => Header::Id,
            Header::Image => Header::Name,
            Header::State => Header::Image,
            Header::Status => Header::State,
            Header::Id => Header::Status,
            _ => Header::Name,
        };
    }
}

impl Default for UIContainerState {
    fn default() -> Self {
        Self::new()
    }
}
