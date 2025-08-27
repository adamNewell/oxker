use oxker_core::events::{
    CoreCommand,
    types::{SortField, SortOrder},
};
use oxker_core::{ContainerId, DockerCommand};

/// Maps UI actions to CoreCommands (preparation for future CoreHandle integration)
pub struct CommandMapper;

impl CommandMapper {
    /// Convert a DockerCommand and ContainerId to the appropriate CoreCommand
    pub fn docker_command_to_core(
        command: DockerCommand,
        container_id: &ContainerId,
    ) -> CoreCommand {
        match command {
            DockerCommand::Start => CoreCommand::StartContainer(container_id.get().to_string()),
            DockerCommand::Stop => CoreCommand::StopContainer(container_id.get().to_string()),
            DockerCommand::Pause => CoreCommand::PauseContainer(container_id.get().to_string()),
            DockerCommand::Resume => CoreCommand::UnpauseContainer(container_id.get().to_string()),
            DockerCommand::Restart => CoreCommand::RestartContainer(container_id.get().to_string()),
            DockerCommand::Delete => CoreCommand::RemoveContainer(container_id.get().to_string()),
        }
    }

    /// Convert header selection to sort command
    pub const fn header_to_sort_command(
        header: oxker_core::Header,
        ascending: bool,
    ) -> CoreCommand {
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

        let order = if ascending {
            SortOrder::Ascending
        } else {
            SortOrder::Descending
        };

        CoreCommand::SortContainers(field, order)
    }
}
