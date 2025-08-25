use std::collections::VecDeque;
use oxker_core::{ContainerId, ContainerItem, DockerCommand, Header, StatefulList, State as ContainerState};
use oxker_core::events::types::{ContainerItem as EventContainerItem, Stats as EventStats, LogLine};

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
    /// Current log position
    pub log_position: usize,
    /// Filter term for container list
    pub filter_term: String,
    /// Field to filter by
    pub filter_by: Header,
    /// Current sort header
    pub sort_header: Header,
    /// Sort order (ascending/descending)
    pub sort_ascending: bool,
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
            log_position: 0,
            filter_term: String::new(),
            filter_by: Header::Name,
            sort_header: Header::Name,
            sort_ascending: true,
        }
    }

    /// Update the container list from events
    pub fn update_containers(&mut self, event_containers: Vec<EventContainerItem>) {
        // Convert event containers to UI containers
        let containers: Vec<ContainerItem> = event_containers
            .into_iter()
            .enumerate()
            .map(|(index, ec)| {
                // Create a minimal ContainerItem for UI purposes
                // This is a temporary solution until we have a proper UI-only container type
                ContainerItem::new(
                    index as u64,
                    ContainerId::from(ec.id.as_str()),
                    ec.image,
                    false, // is_oxker - we'll need to determine this
                    ec.name,
                    vec![], // ports - not provided in event
                    // Simple state parsing without status info
                    match ec.state.as_str() {
                        "dead" => ContainerState::Dead,
                        "exited" => ContainerState::Exited,
                        "paused" => ContainerState::Paused,
                        "removing" => ContainerState::Removing,
                        "restarting" => ContainerState::Restarting,
                        "running" => ContainerState::Running(oxker_core::RunningState::Healthy),
                        _ => ContainerState::Unknown,
                    },
                    oxker_core::ContainerStatus::from(ec.status),
                )
            })
            .collect();

        // Preserve selection if possible
        let selected_id = self.containers.state.selected()
            .and_then(|idx| self.containers.items.get(idx))
            .map(|c| c.id.clone());
        self.containers = StatefulList::new(containers);
        
        // Restore selection
        if let Some(id) = selected_id {
            if let Some(index) = self.containers.items.iter().position(|c| c.id == id) {
                self.containers.state.select(Some(index));
            }
        }
    }

    /// Update stats for a specific container
    pub fn update_container_stats(&mut self, container_id: String, stats: EventStats) {
        if let Some(container) = self.containers.items.iter_mut().find(|c| c.id.get() == container_id) {
            // Update CPU stats
            container.cpu_stats.push_front(oxker_core::CpuStats::new(stats.cpu_usage));
            if container.cpu_stats.len() > 60 {
                container.cpu_stats.pop_back();
            }

            // Update memory stats
            container.mem_stats.push_front(oxker_core::ByteStats::new(stats.memory_usage));
            if container.mem_stats.len() > 60 {
                container.mem_stats.pop_back();
            }
            container.mem_limit = oxker_core::ByteStats::new(stats.memory_limit);

            // Update network stats
            container.rx = oxker_core::ByteStats::new(stats.network_rx);
            container.tx = oxker_core::ByteStats::new(stats.network_tx);
        }
    }

    /// Add logs for a container
    pub fn add_logs(&mut self, container_id: String, logs: Vec<LogLine>) {
        // Only add logs if this is the selected container
        if let Some(selected) = self.containers.state.selected()
            .and_then(|idx| self.containers.items.get(idx)) {
            if selected.id.get() == container_id {
                for log in logs {
                    self.logs.push_back(log.message);
                    // Keep log buffer reasonable size
                    if self.logs.len() > 10000 {
                        self.logs.pop_front();
                    }
                }
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
    }

    /// Get the currently selected container ID
    pub fn get_selected_container_id(&self) -> Option<ContainerId> {
        self.containers.state.selected()
            .and_then(|idx| self.containers.items.get(idx))
            .map(|c| c.id.clone())
    }

    /// Navigate to next container
    pub fn next_container(&mut self) {
        self.containers.next();
        self.update_docker_commands();
    }
    
    /// Update docker commands based on selected container state
    fn update_docker_commands(&mut self) {
        if let Some(container) = self.containers.state.selected()
            .and_then(|idx| self.containers.items.get(idx)) {
            let commands = DockerCommand::gen_vec(container.state.clone());
            self.docker_commands = StatefulList::new(commands);
        }
    }

    /// Navigate to previous container
    pub fn previous_container(&mut self) {
        self.containers.previous();
        self.update_docker_commands();
    }

    /// Navigate to first container
    pub fn first_container(&mut self) {
        self.containers.start();
        self.update_docker_commands();
    }

    /// Navigate to last container
    pub fn last_container(&mut self) {
        self.containers.end();
        self.update_docker_commands();
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
        self.log_position = 0;
    }

    /// Navigate logs forward
    pub fn log_forward(&mut self, _width: u16) {
        // Simple implementation - can be enhanced
        self.log_position = self.log_position.saturating_add(1);
    }

    /// Navigate logs backward
    pub fn log_back(&mut self) {
        self.log_position = self.log_position.saturating_sub(1);
    }

    /// Go to start of logs
    pub fn log_start(&mut self) {
        self.log_position = 0;
    }

    /// Go to end of logs
    pub fn log_end(&mut self) {
        self.log_position = self.logs.len().saturating_sub(1);
    }

    /// Navigate to next log line
    pub fn log_next(&mut self) {
        if self.log_position < self.logs.len().saturating_sub(1) {
            self.log_position += 1;
        }
    }

    /// Navigate to previous log line
    pub fn log_previous(&mut self) {
        if self.log_position > 0 {
            self.log_position -= 1;
        }
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
        self.docker_commands.state.selected()
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