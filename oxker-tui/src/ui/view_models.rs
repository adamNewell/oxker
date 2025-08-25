use std::collections::{HashSet, VecDeque};
use std::time::Instant;
use oxker_core::{
    AppColors, AppError, ByteStats, Columns, ContainerId, ContainerPorts, CpuStats, CpuTuple, 
    DockerCommand, FilterBy, Header, MemTuple, SortedOrder, State, Stats,
};
use crate::handlers::UIContainerState;
use crate::ui::gui_state::{DeleteButton, GuiState, SelectablePanel, Status};

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
    pub position: usize,
    pub title: String,
}

/// Commands view data
#[derive(Debug, Clone)]
pub struct CommandsView {
    pub commands: Vec<DockerCommand>,
    pub selected: Option<usize>,
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
    pub selected_panel: SelectablePanel,
    pub scroll_title: Option<String>,
    pub sorted_by: Option<(Header, SortedOrder)>,
    pub status: HashSet<Status>,
}

impl FrameViewModel {
    /// Create a FrameViewModel from UIContainerState and GuiState
    pub fn from_state(
        ui_state: &UIContainerState,
        gui_state: &GuiState,
        colors: AppColors,
        screen_width: u16,
    ) -> Self {
        // Convert containers to view models
        let containers: Vec<ContainerView> = ui_state.get_container_items()
            .into_iter()
            .map(|c| ContainerView {
                id: c.id.clone(),
                name: c.name.to_string(),
                image: c.image.to_string(),
                state: c.state.clone(),
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
        let selected_container_data = selected_container
            .and_then(|idx| ui_state.containers.items.get(idx));

        // Prepare chart data if we have a selected container
        let chart_data = selected_container_data.map(|container| {
            let cpu_data: Vec<(f64, f64)> = container.cpu_stats.iter()
                .enumerate()
                .map(|(i, s)| (i as f64, s.get_value()))
                .collect();
            let mem_data: Vec<(f64, f64)> = container.mem_stats.iter()
                .enumerate()
                .map(|(i, s)| (i as f64, s.get_value() as f64))
                .collect();
            
            let max_cpu = container.cpu_stats.iter()
                .max()
                .copied()
                .unwrap_or_default();
            let max_mem = container.mem_stats.iter()
                .max()
                .copied()
                .unwrap_or_default();
            
            ChartData {
                cpu_data: (cpu_data, max_cpu, container.state.clone()),
                mem_data: (mem_data, max_mem, container.state.clone()),
            }
        });

        // Prepare port view
        let port_view = selected_container_data.map(|container| {
            let ports = container.ports.clone();
            let max_lens = calculate_port_max_lens(&ports);
            PortView {
                ports,
                state: container.state.clone(),
                max_lens,
            }
        });

        // Prepare log view
        let log_view = LogView {
            logs: ui_state.logs.iter().cloned().collect(),
            position: ui_state.log_position,
            title: selected_container_data
                .map(|c| format!("{} logs", c.name))
                .unwrap_or_else(|| "No container selected".to_string()),
        };

        // Prepare commands view
        let commands_view = CommandsView {
            commands: ui_state.docker_commands.items.clone(),
            selected: ui_state.docker_commands.state.selected(),
        };

        // Calculate columns based on container data
        let columns = calculate_columns(&containers, screen_width);

        // Build the complete view model
        Self {
            has_containers: !containers.is_empty(),
            containers,
            selected_container,
            chart_data,
            color_logs: true, // This should come from config
            columns,
            container_title: create_container_title(ui_state.get_container_count()),
            delete_confirm: gui_state.get_delete_container(),
            filter_by: FilterBy::Name, // Convert from ui_state.filter_by
            filter_term: if ui_state.filter_term.is_empty() { None } else { Some(ui_state.filter_term.clone()) },
            has_error: None, // Will need to get from somewhere
            info_text: gui_state.info_box_text.clone(),
            is_loading: gui_state.is_loading(),
            loading_icon: gui_state.get_loading().to_string(),
            log_view,
            log_height: gui_state.get_log_height(),
            show_logs: gui_state.get_show_logs(),
            port_view,
            commands_view,
            selected_panel: gui_state.get_selected_panel(),
            scroll_title: None, // Will need to calculate based on selected container
            sorted_by: Some((ui_state.sort_header.clone(), if ui_state.sort_ascending { SortedOrder::Asc } else { SortedOrder::Desc })),
            status: gui_state.get_status(),
        }
    }
}

/// Calculate maximum lengths for port display
fn calculate_port_max_lens(ports: &[ContainerPorts]) -> (usize, usize, usize) {
    let max_ip = ports.iter()
        .map(|p| p.ip.map(|ip| ip.to_string().len()).unwrap_or(0))
        .max()
        .unwrap_or(0);
    let max_private = ports.iter()
        .map(|p| p.private.to_string().len())
        .max()
        .unwrap_or(0);
    let max_public = ports.iter()
        .map(|p| p.public.as_ref().map(|p| p.to_string().len()).unwrap_or(0))
        .max()
        .unwrap_or(0);
    
    (max_ip, max_private, max_public)
}

/// Calculate column widths based on container data
fn calculate_columns(containers: &[ContainerView], screen_width: u16) -> Columns {
    // This is a simplified version - the real implementation would calculate
    // based on actual container data
    Columns {
        name: (Header::Name, 20),
        state: (Header::State, 10),
        status: (Header::Status, 20),
        cpu: (Header::Cpu, 6),
        mem: (Header::Memory, 10, 10),
        id: (Header::Id, 8),
        image: (Header::Image, 20),
        net_rx: (Header::Rx, 10),
        net_tx: (Header::Tx, 10),
    }
}

/// Create container title based on count
fn create_container_title(count: usize) -> String {
    if count == 0 {
        "Containers".to_string()
    } else {
        format!("Containers [{}]", count)
    }
}