use oxker_core::{DockerCommand, ContainerId};
use oxker_core::events::{CoreCommand, types::{SortField, SortOrder}};
use crate::ui::SelectablePanel;

/// Maps UI actions to CoreCommands (preparation for future CoreHandle integration)
pub struct CommandMapper;

impl CommandMapper {
    /// Convert a DockerCommand and ContainerId to the appropriate CoreCommand
    pub fn docker_command_to_core(command: DockerCommand, container_id: ContainerId) -> Option<CoreCommand> {
        match command {
            DockerCommand::Start => Some(CoreCommand::StartContainer(container_id.get().to_string())),
            DockerCommand::Stop => Some(CoreCommand::StopContainer(container_id.get().to_string())),
            DockerCommand::Pause => Some(CoreCommand::PauseContainer(container_id.get().to_string())),
            DockerCommand::Resume => Some(CoreCommand::UnpauseContainer(container_id.get().to_string())),
            DockerCommand::Restart => Some(CoreCommand::RestartContainer(container_id.get().to_string())),
            DockerCommand::Delete => Some(CoreCommand::RemoveContainer(container_id.get().to_string())),
        }
    }

    /// Convert header selection to sort command
    pub fn header_to_sort_command(header: oxker_core::Header) -> Option<CoreCommand> {
        let field = match header {
            oxker_core::Header::Name => SortField::Name,
            oxker_core::Header::State => SortField::State,
            oxker_core::Header::Status => SortField::Status,
            oxker_core::Header::Cpu => SortField::Cpu,
            oxker_core::Header::Memory => SortField::Memory,
            oxker_core::Header::Id => SortField::Id,
            oxker_core::Header::Image => SortField::Image,
            oxker_core::Header::Rx => SortField::NetworkRx,
            oxker_core::Header::Tx => SortField::NetworkTx,
        };
        
        // Default to ascending order - in a real implementation, 
        // this would check current sort state to toggle
        Some(CoreCommand::SortContainers(field, SortOrder::Ascending))
    }

    /// Get the appropriate refresh command for the current panel
    pub fn get_refresh_command(panel: SelectablePanel, container_id: Option<String>) -> Option<CoreCommand> {
        match panel {
            SelectablePanel::Containers => Some(CoreCommand::RefreshContainers),
            SelectablePanel::Logs => container_id.map(CoreCommand::RefreshLogs),
            SelectablePanel::Commands => Some(CoreCommand::RefreshContainers),
        }
    }

    /// Convert filter string to filter command
    pub fn create_filter_command(filter_term: String) -> CoreCommand {
        CoreCommand::FilterContainers(filter_term)
    }
}