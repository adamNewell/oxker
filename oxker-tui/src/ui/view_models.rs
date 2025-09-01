use crate::handlers::{DebugEvent, UIContainerState};
use crate::ui::gui_state::{GuiState, Status};
use oxker_core::{
    AppColors, AppError, ByteStats, Columns, ContainerId, ContainerPorts, CpuStats, DockerCommand,
    FilterBy, Header, SortedOrder, State, Stats,
};
use std::collections::HashSet;
use std::time::Instant;

/// Container view data for rendering
#[derive(Debug, Clone)]
pub struct ContainerView {
    pub id: ContainerId,
    pub name: String,
    pub image: String,
    pub state: State,
    pub status: String,
    pub cpu_stats: CpuStats,
    pub mem_stats: ByteStats,
    pub mem_limit: ByteStats,
    pub rx: ByteStats,
    pub tx: ByteStats,
}

/// Chart data for rendering CPU/Memory graphs
#[derive(Debug, Clone)]
pub struct ChartData {
    pub cpu_data: (Vec<(f64, f64)>, CpuStats, State),
    pub mem_data: (Vec<(f64, f64)>, ByteStats, State),
    pub current_cpu: CpuStats,
    pub current_mem: ByteStats,
}

/// Port information for the selected container
#[derive(Debug, Clone)]
pub struct PortView {
    pub ports: Vec<ContainerPorts>,
    pub state: State,
    pub max_lens: (usize, usize, usize),
}

/// Log view data
#[derive(Debug, Clone)]
pub struct LogView {
    pub logs: Vec<String>,
    pub title: String,
    pub is_loading: bool,
}

/// Commands view data
#[derive(Debug, Clone)]
pub struct CommandsView {
    pub commands: Vec<DockerCommand>,
}

/// All data needed for rendering a frame
#[derive(Debug, Clone)]
pub struct FrameViewModel {
    pub containers: Vec<ContainerView>,
    pub selected_container: Option<usize>,
    pub chart_data: Option<ChartData>,
    pub color_logs: bool,
    pub columns: Columns,
    pub container_title: String,
    pub delete_confirm: Option<ContainerId>,
    pub filter_by: FilterBy,
    pub filter_term: Option<String>,
    pub has_containers: bool,
    pub has_error: Option<AppError>,
    pub info_text: Option<(String, Instant)>,
    pub is_loading: bool,
    pub loading_icon: String,
    pub log_view: LogView,
    pub log_height: u16,
    pub show_logs: bool,
    pub port_view: Option<PortView>,
    pub commands_view: CommandsView,
    pub scroll_title: Option<String>,
    pub sorted_by: Option<(Header, SortedOrder)>,
    pub status: HashSet<Status>,
    pub debug_events: Vec<DebugEvent>,
}

impl FrameViewModel {
    /// Create a FrameViewModel from UIContainerState and GuiState
    #[allow(clippy::cast_precision_loss)]
    pub fn from_state(
        ui_state: &UIContainerState,
        gui_state: &GuiState,
        _colors: AppColors,
        screen_width: u16,
    ) -> Self {
        use crate::debug::{EventCategory, writer};
        writer::write_debug(
            EventCategory::State,
            "FrameViewModel",
            "from_state",
            "Creating new view model from state",
        );
        // Convert containers to view models
        let containers: Vec<ContainerView> = ui_state
            .get_container_items()
            .into_iter()
            .map(|c| ContainerView {
                id: c.id.clone(),
                name: c.name.to_string(),
                image: c.image.to_string(),
                state: c.state,
                status: c.status.get().to_string(),
                cpu_stats: c.cpu_stats.back().copied().unwrap_or_default(),
                mem_stats: c.mem_stats.back().copied().unwrap_or_default(),
                mem_limit: c.mem_limit,
                rx: c.rx,
                tx: c.tx,
            })
            .collect();

        // Get selected container for chart data and ports
        let selected_container = ui_state.containers.state.selected();
        let selected_container_data =
            selected_container.and_then(|idx| ui_state.containers.items.get(idx));

        // Prepare chart data if we have a selected container
        let chart_data = selected_container_data.map(|container| {
            let cpu_data: Vec<(f64, f64)> = container
                .cpu_stats
                .iter()
                .enumerate()
                .map(|(i, s)| (i as f64, s.get_value()))
                .collect();
            let mem_data: Vec<(f64, f64)> = container
                .mem_stats
                .iter()
                .enumerate()
                .map(|(i, s)| (i as f64, s.get_value()))
                .collect();

            let max_cpu = container
                .cpu_stats
                .iter()
                .max()
                .copied()
                .unwrap_or_default();
            let max_mem = container
                .mem_stats
                .iter()
                .max()
                .copied()
                .unwrap_or_default();

            // Get the current stats from the container
            let current_cpu = container.cpu_stats.back().copied().unwrap_or_default();
            let current_mem = container.mem_stats.back().copied().unwrap_or_default();

            ChartData {
                cpu_data: (cpu_data, max_cpu, container.state),
                mem_data: (mem_data, max_mem, container.state),
                current_cpu,
                current_mem,
            }
        });

        // Prepare port view
        let port_view = selected_container_data.map(|container| {
            let mut ports = container.ports.clone();
            // Sort ports by public port number (ascending), with None values at the end
            ports.sort_by(|a, b| match (a.public, b.public) {
                (Some(pa), Some(pb)) => pa.cmp(&pb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.private.cmp(&b.private), // Sort by private port if both public are None
            });
            let max_lens = calculate_port_max_lens(&ports);
            PortView {
                ports,
                state: container.state,
                max_lens,
            }
        });

        // Prepare log view
        let logs_collected: Vec<String> = ui_state.logs.iter().cloned().collect();
        writer::write_debug(
            EventCategory::Logs,
            "FrameViewModel",
            "from_state",
            &format!("Collecting logs: count={}", logs_collected.len()),
        );
        let log_view = LogView {
            logs: logs_collected,
            title: selected_container_data.map_or_else(String::new, |c| {
                format!("Logs - {} - {}", c.name.get(), c.image.get())
            }),
            is_loading: ui_state.logs_loading,
        };

        // Prepare commands view
        let commands_view = CommandsView {
            commands: ui_state.docker_commands.items.clone(),
        };

        // Calculate columns based on container data
        let columns = calculate_columns(&containers, screen_width);

        // Calculate scroll title for logs (shows column position)
        let scroll_title = if ui_state.logs.is_empty() {
            None
        } else {
            // TODO: Proper column tracking
            Some(" 1/80 → ".to_string())
        };

        // Build the complete view model
        Self {
            has_containers: !containers.is_empty(),
            containers,
            selected_container,
            chart_data,
            color_logs: true, // This should come from config
            columns,
            container_title: create_container_title(
                ui_state.get_container_count(),
                ui_state.containers.state.selected(),
            ),
            delete_confirm: gui_state.get_delete_container(),
            filter_by: match ui_state.filter_by {
                Header::Name => FilterBy::Name,
                Header::Image => FilterBy::Image,
                Header::Status | Header::State => FilterBy::Status, // Map State to Status for filtering
                _ => FilterBy::All, // Default to searching all fields for other headers (Id, CPU, Memory, etc.)
            },
            filter_term: if ui_state.filter_term.is_empty() {
                None
            } else {
                Some(ui_state.filter_term.clone())
            },
            has_error: None, // Will need to get from somewhere
            info_text: gui_state.info_box_text.clone(),
            is_loading: gui_state.is_loading(),
            loading_icon: gui_state.get_loading().to_string(),
            log_view,
            log_height: gui_state.get_log_height(),
            show_logs: gui_state.get_show_logs(),
            port_view,
            commands_view,
            scroll_title,
            sorted_by: ui_state.sort_header.as_ref().map(|header| {
                (
                    *header,
                    if ui_state.sort_ascending {
                        SortedOrder::Asc
                    } else {
                        SortedOrder::Desc
                    },
                )
            }),
            status: gui_state.get_status(),
            debug_events: ui_state.debug_events.iter().cloned().collect(),
        }
    }
}

impl Default for FrameViewModel {
    fn default() -> Self {
        Self {
            containers: Vec::new(),
            selected_container: None,
            chart_data: None,
            color_logs: false,
            columns: Columns::new(),
            container_title: String::new(),
            delete_confirm: None,
            filter_by: FilterBy::Name,
            filter_term: None,
            has_containers: false,
            has_error: None,
            info_text: None,
            is_loading: false,
            loading_icon: String::new(),
            log_view: LogView {
                logs: Vec::new(),
                title: String::new(),
                is_loading: false,
            },
            log_height: 4,
            show_logs: true,
            port_view: None,
            commands_view: CommandsView {
                commands: Vec::new(),
            },
            scroll_title: None,
            sorted_by: None,
            status: HashSet::new(),
            debug_events: Vec::new(),
        }
    }
}

/// Calculate maximum lengths for port display
fn calculate_port_max_lens(ports: &[ContainerPorts]) -> (usize, usize, usize) {
    let max_ip = ports
        .iter()
        .map(|p| p.ip.map_or(0, |ip| ip.to_string().len()))
        .max()
        .unwrap_or(0)
        .max(7);
    let max_private = ports
        .iter()
        .map(|p| p.private.to_string().len())
        .max()
        .unwrap_or(0);
    let max_public = ports
        .iter()
        .map(|p| p.public.as_ref().map_or(0, |p| p.to_string().len()))
        .max()
        .unwrap_or(0)
        .max(6);

    (max_ip, max_private, max_public)
}

/// Calculate column widths based on container data
fn calculate_columns(containers: &[ContainerView], _screen_width: u16) -> Columns {
    // Calculate max widths for each column based on data
    let mut name_width = 4; // min "Name"
    let mut state_width = 5; // min "State"
    let mut status_width = 6; // min "Status"
    let mut image_width = 5; // min "Image"

    for container in containers {
        name_width = name_width.max(container.name.len());
        state_width = state_width.max(container.state.to_string().len());
        status_width = status_width.max(container.status.len());
        image_width = image_width.max(container.image.len());
    }

    // Add some padding
    name_width = (name_width + 2).min(30);
    state_width = (state_width + 2).min(12);
    status_width = (status_width + 2).min(30);
    image_width = (image_width + 2).min(40);

    // Fixed widths for numeric columns
    let cpu_width = 8; // "100.00%"
    let mem_current_width = 10; // "999.99 MB"
    let mem_limit_width = 10; // "999.99 GB"
    let id_width = 8; // 8 chars of ID
    let net_width = 10; // "999.99 MB"

    Columns {
        name: (Header::Name, u8::try_from(name_width).unwrap_or(30)),
        state: (Header::State, u8::try_from(state_width).unwrap_or(12)),
        status: (Header::Status, u8::try_from(status_width).unwrap_or(30)),
        cpu: (Header::Cpu, cpu_width),
        mem: (Header::Memory, mem_current_width, mem_limit_width),
        id: (Header::Id, id_width),
        image: (Header::Image, u8::try_from(image_width).unwrap_or(40)),
        net_rx: (Header::Rx, net_width),
        net_tx: (Header::Tx, net_width),
    }
}

/// Create container title based on count and selection
fn create_container_title(count: usize, selected: Option<usize>) -> String {
    if count == 0 {
        "Containers".to_string()
    } else if let Some(idx) = selected {
        format!("Containers {}/{}", idx + 1, count)
    } else {
        format!("Containers [{count}]")
    }
}

#[cfg(test)]
mod tests {
    use oxker_core::ContainerPorts;
    use std::net::IpAddr;

    #[test]
    fn test_port_sorting_ascending_by_public_port() {
        // Arrange: Create unsorted ports
        let mut ports = [
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 80,
                public: Some(8080),
            },
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 443,
                public: Some(443),
            },
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 22,
                public: Some(2222),
            },
        ];

        // Act: Apply the same sorting logic
        ports.sort_by(|a, b| match (a.public, b.public) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.private.cmp(&b.private),
        });

        // Assert: Verify ports are sorted by public port ascending
        assert_eq!(ports[0].public, Some(443));
        assert_eq!(ports[1].public, Some(2222));
        assert_eq!(ports[2].public, Some(8080));
    }

    #[test]
    fn test_port_sorting_none_public_ports_at_end() {
        // Arrange: Mix of ports with and without public mappings
        let mut ports = [
            ContainerPorts {
                ip: None,
                private: 3000,
                public: None,
            },
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 80,
                public: Some(8080),
            },
            ContainerPorts {
                ip: None,
                private: 5000,
                public: None,
            },
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 443,
                public: Some(443),
            },
        ];

        // Act: Apply sorting
        ports.sort_by(|a, b| match (a.public, b.public) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.private.cmp(&b.private),
        });

        // Assert: Ports with public mappings come first, None values at end
        assert_eq!(ports[0].public, Some(443));
        assert_eq!(ports[1].public, Some(8080));
        assert_eq!(ports[2].public, None);
        assert_eq!(ports[2].private, 3000); // Sorted by private when both public are None
        assert_eq!(ports[3].public, None);
        assert_eq!(ports[3].private, 5000);
    }

    #[test]
    fn test_port_sorting_stability_same_public_port() {
        // Arrange: Ports with duplicate public port numbers (different protocols implied)
        let mut ports = [
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 80,
                public: Some(8080),
            },
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 8080,
                public: Some(8080), // Same public port
            },
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 443,
                public: Some(443),
            },
        ];

        // Act: Apply sorting
        ports.sort_by(|a, b| match (a.public, b.public) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.private.cmp(&b.private),
        });

        // Assert: Sort is stable - duplicates maintain relative order
        assert_eq!(ports[0].public, Some(443));
        assert_eq!(ports[1].public, Some(8080));
        assert_eq!(ports[1].private, 80); // First 8080 port
        assert_eq!(ports[2].public, Some(8080));
        assert_eq!(ports[2].private, 8080); // Second 8080 port
    }

    #[test]
    fn test_port_sorting_large_port_numbers() {
        // Arrange: Test with very large port numbers
        let mut ports = [
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 65535,
                public: Some(65535),
            },
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 1,
                public: Some(1),
            },
            ContainerPorts {
                ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                private: 32768,
                public: Some(32768),
            },
        ];

        // Act: Apply sorting
        ports.sort_by(|a, b| match (a.public, b.public) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.private.cmp(&b.private),
        });

        // Assert: Correct ascending order
        assert_eq!(ports[0].public, Some(1));
        assert_eq!(ports[1].public, Some(32768));
        assert_eq!(ports[2].public, Some(65535));
    }

    #[test]
    fn test_port_sorting_empty_list() {
        // Arrange: Empty port list
        let mut ports: Vec<ContainerPorts> = vec![];

        // Act: Apply sorting (should not panic)
        ports.sort_by(|a, b| match (a.public, b.public) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.private.cmp(&b.private),
        });

        // Assert: List remains empty
        assert!(ports.is_empty());
    }

    #[test]
    fn test_port_sorting_only_private_ports() {
        // Arrange: Only private ports (no public mappings)
        let mut ports = [
            ContainerPorts {
                ip: None,
                private: 5000,
                public: None,
            },
            ContainerPorts {
                ip: None,
                private: 3000,
                public: None,
            },
            ContainerPorts {
                ip: None,
                private: 4000,
                public: None,
            },
        ];

        // Act: Apply sorting
        ports.sort_by(|a, b| match (a.public, b.public) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.private.cmp(&b.private),
        });

        // Assert: Sorted by private port when all public are None
        assert_eq!(ports[0].private, 3000);
        assert_eq!(ports[1].private, 4000);
        assert_eq!(ports[2].private, 5000);
    }
}
