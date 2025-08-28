//! Comprehensive snapshot tests for all UI components
//! This ensures complete UI regression test coverage

use insta::assert_snapshot;
use oxker_core::{
    AppColors, ByteStats, ContainerName, ContainerPorts, CpuStats, DockerCommand, FilterBy, Keymap,
    RunningState, State, StatefulList, Stats,
};
use oxker_core::{ContainerId, ContainerItem, ContainerItemInit, ContainerStatus};
use std::collections::VecDeque;

// Simple test container builder for this test file
struct TestContainerBuilder {
    id: ContainerId,
    name: String,
    image: String,
    state: State,
    status: ContainerStatus,
    cpu_stats: VecDeque<CpuStats>,
    mem_stats: VecDeque<ByteStats>,
    mem_limit: ByteStats,
    ports: Vec<ContainerPorts>,
}

impl TestContainerBuilder {
    fn new(id: &str) -> Self {
        Self {
            id: ContainerId::from(id),
            name: format!("container_{}", id),
            image: "test_image:latest".to_string(),
            state: State::Running(RunningState::Healthy),
            status: ContainerStatus::from("Up 1 hour".to_string()),
            cpu_stats: VecDeque::new(),
            mem_stats: VecDeque::new(),
            mem_limit: ByteStats::new(1024 * 1024 * 1024), // 1GB
            ports: vec![],
        }
    }

    fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    fn with_image(mut self, image: &str) -> Self {
        self.image = image.to_string();
        self
    }

    fn with_state(mut self, state: State) -> Self {
        self.state = state;
        self
    }

    fn with_status(mut self, status: &str) -> Self {
        self.status = ContainerStatus::from(status.to_string());
        self
    }

    fn with_cpu_stats(mut self, stats: Vec<f64>) -> Self {
        self.cpu_stats = stats.into_iter().map(CpuStats::new).collect();
        self
    }

    fn with_mem_stats(mut self, stats: Vec<u64>) -> Self {
        self.mem_stats = stats.into_iter().map(ByteStats::new).collect();
        self
    }

    fn with_mem_limit(mut self, limit: u64) -> Self {
        self.mem_limit = ByteStats::new(limit);
        self
    }

    fn build(self) -> ContainerItem {
        let mut container = ContainerItem::new(ContainerItemInit {
            created: 0,
            id: self.id,
            image: self.image,
            name: self.name,
            state: self.state,
            status: self.status,
            is_oxker: false,
            ports: vec![],
        });
        container.cpu_stats = self.cpu_stats;
        container.mem_stats = self.mem_stats;
        container.mem_limit = self.mem_limit;
        if !self.ports.is_empty() {
            container.ports = self.ports;
        }
        container.rx = ByteStats::new(0);
        container.tx = ByteStats::new(0);
        container
    }
}
use oxker_tui::handlers::UIContainerState;
use oxker_tui::ui::components::Component;
use oxker_tui::ui::components::panels::{
    charts::{ChartsPanel, ChartsPanelProps},
    commands::{CommandsPanel, CommandsPanelProps},
    containers::{ContainersPanel, ContainersPanelProps},
    delete_confirm::{DeleteConfirmPanel, DeleteConfirmPanelProps},
    filter::{FilterPanel, FilterPanelProps},
    headers::{HeadersPanel, HeadersPanelProps},
    logs::{LogsPanel, LogsPanelProps},
    ports::{PortsPanel, PortsPanelProps},
};
use oxker_tui::ui::{ContainerView, FrameViewModel, GuiState, LogView, PortView, Rerender};
use parking_lot::Mutex;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::sync::Arc;

/// Helper to create a test FrameViewModel
fn create_test_view_model() -> FrameViewModel {
    FrameViewModel::default()
}

/// Helper to create test GUI state
fn create_test_gui_state() -> Arc<Mutex<GuiState>> {
    let rerender = Arc::new(Rerender::new());
    Arc::new(Mutex::new(GuiState::new(&rerender, true)))
}

/// Helper to create test container state
fn create_test_container_state() -> Arc<Mutex<UIContainerState>> {
    Arc::new(Mutex::new(UIContainerState::default()))
}

/// Helper to render a component and capture its output
fn render_component<'a, C: Component<'a>>(
    component: &C,
    props: &'a C::Props,
    width: u16,
    height: u16,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            component.render(props, f.area(), f);
        })
        .unwrap();

    terminal_to_string(&terminal, width, height)
}

fn terminal_to_string(terminal: &Terminal<TestBackend>, width: u16, height: u16) -> String {
    let mut output = String::new();
    let buffer = terminal.backend().buffer();

    for y in 0..height {
        for x in 0..width {
            if let Some(cell) = buffer.cell((x, y)) {
                output.push_str(cell.symbol());
            }
        }
        if y < height - 1 {
            output.push('\n');
        }
    }

    output
}

#[test]
fn test_charts_panel_snapshots() {
    let charts_panel = ChartsPanel::new();
    let theme = AppColors::new();
    let mut view_model = create_test_view_model();

    // Test empty data
    view_model.show_logs = true;
    view_model.chart_data = None;

    let props = ChartsPanelProps {
        view_model: &view_model,
        theme: &theme,
    };

    assert_snapshot!(
        "charts_empty",
        render_component(&charts_panel, &props, 80, 10)
    );

    // Test with data
    let cpu_data = vec![25.5, 30.0, 28.3, 32.1, 29.7];
    let mem_data = vec![
        ByteStats::new(1024 * 1024 * 100), // 100MB
        ByteStats::new(1024 * 1024 * 120), // 120MB
        ByteStats::new(1024 * 1024 * 115), // 115MB
        ByteStats::new(1024 * 1024 * 125), // 125MB
        ByteStats::new(1024 * 1024 * 118), // 118MB
    ];

    let cpu_chart_data: Vec<(f64, f64)> = cpu_data
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect();
    let mem_chart_data: Vec<(f64, f64)> = mem_data
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, v.get_value()))
        .collect();

    view_model.chart_data = Some(oxker_tui::ui::ChartData {
        cpu_data: (
            cpu_chart_data,
            CpuStats::new(30.0),
            State::Running(RunningState::Healthy),
        ),
        mem_data: (
            mem_chart_data,
            ByteStats::new(125 * 1024 * 1024),
            State::Running(RunningState::Healthy),
        ),
        current_cpu: CpuStats::new(26.0),
        current_mem: ByteStats::new(118 * 1024 * 1024),
    });

    let props = ChartsPanelProps {
        view_model: &view_model,
        theme: &theme,
    };

    assert_snapshot!(
        "charts_with_data",
        render_component(&charts_panel, &props, 80, 15)
    );

    // Test with graphs hidden
    view_model.show_logs = false;

    let props = ChartsPanelProps {
        view_model: &view_model,
        theme: &theme,
    };

    assert_snapshot!(
        "charts_hidden",
        render_component(&charts_panel, &props, 80, 10)
    );
}

#[test]
fn test_commands_panel_snapshots() {
    let commands_panel = CommandsPanel::new();
    let theme = AppColors::new();
    let view_model = create_test_view_model();
    let gui_state = create_test_gui_state();
    let container_state = create_test_container_state();

    // Test with no commands
    let props = CommandsPanelProps {
        view_model: &view_model,
        theme: &theme,
        gui_state: &gui_state,
        container_state: &container_state,
    };

    assert_snapshot!(
        "commands_empty",
        render_component(&commands_panel, &props, 60, 8)
    );

    // Test with commands for running container
    container_state.lock().docker_commands = StatefulList::new(vec![
        DockerCommand::Stop,
        DockerCommand::Restart,
        DockerCommand::Pause,
        DockerCommand::Delete,
    ]);
    container_state.lock().docker_commands.state.select(Some(0));

    let props = CommandsPanelProps {
        view_model: &view_model,
        theme: &theme,
        gui_state: &gui_state,
        container_state: &container_state,
    };

    assert_snapshot!(
        "commands_running",
        render_component(&commands_panel, &props, 60, 10)
    );

    // Test with commands for stopped container
    container_state.lock().docker_commands =
        StatefulList::new(vec![DockerCommand::Start, DockerCommand::Delete]);
    container_state.lock().docker_commands.state.select(Some(1));

    let props = CommandsPanelProps {
        view_model: &view_model,
        theme: &theme,
        gui_state: &gui_state,
        container_state: &container_state,
    };

    assert_snapshot!(
        "commands_stopped",
        render_component(&commands_panel, &props, 60, 8)
    );
}

#[test]
fn test_delete_confirm_panel_snapshots() {
    let delete_panel = DeleteConfirmPanel::new();
    let theme = AppColors::new();
    let keymap = Keymap::default();
    let gui_state = create_test_gui_state();

    // Test container deletion
    let container_name = ContainerName::from("nginx-web".to_string());
    let props = DeleteConfirmPanelProps {
        container_name: &container_name,
        theme: &theme,
        keymap: &keymap,
        gui_state: &gui_state,
    };

    assert_snapshot!(
        "delete_confirm_container",
        render_component(&delete_panel, &props, 80, 24)
    );

    // Test with long names
    let container_name =
        ContainerName::from("very-long-container-name-that-might-wrap".to_string());
    let props = DeleteConfirmPanelProps {
        container_name: &container_name,
        theme: &theme,
        keymap: &keymap,
        gui_state: &gui_state,
    };

    assert_snapshot!(
        "delete_confirm_long_names",
        render_component(&delete_panel, &props, 100, 30)
    );
}

// Info panel is just a placeholder - actual implementation is InfoBox widget
// Skipping info panel tests

#[test]
fn test_logs_panel_snapshots() {
    let logs_panel = LogsPanel::new();
    let theme = AppColors::new();
    let mut view_model = create_test_view_model();
    let gui_state = create_test_gui_state();
    let container_state = create_test_container_state();

    // Test empty logs
    view_model.log_view = LogView {
        logs: vec![],
        title: String::new(),
    };

    let props = LogsPanelProps {
        view_model: &view_model,
        theme: &theme,
        gui_state: &gui_state,
        container_state: &container_state,
    };

    assert_snapshot!("logs_empty", render_component(&logs_panel, &props, 80, 15));

    // Test with logs
    let logs = vec![
        "[2024-01-01 10:00:00] Starting nginx".to_string(),
        "[2024-01-01 10:00:01] nginx: [notice] 1#1: using the \"epoll\" event method".to_string(),
        "[2024-01-01 10:00:01] nginx: [notice] 1#1: nginx/1.25.3".to_string(),
        "[2024-01-01 10:00:01] nginx: [notice] 1#1: start worker processes".to_string(),
        "[2024-01-01 10:00:02] Server started on port 80".to_string(),
    ];

    view_model.log_view = LogView {
        logs,
        title: "Container Logs".to_string(),
    };

    let props = LogsPanelProps {
        view_model: &view_model,
        theme: &theme,
        gui_state: &gui_state,
        container_state: &container_state,
    };

    assert_snapshot!(
        "logs_with_entries",
        render_component(&logs_panel, &props, 80, 15)
    );

    // Note: Search functionality has been removed from the current implementation
    // TODO: Validate that search functionality is present

    // Test stopped container logs
    view_model.log_view = LogView {
        logs: vec!["Container stopped.".to_string()],
        title: "Container Logs".to_string(),
    };

    let props = LogsPanelProps {
        view_model: &view_model,
        theme: &theme,
        gui_state: &gui_state,
        container_state: &container_state,
    };

    assert_snapshot!(
        "logs_stopped",
        render_component(&logs_panel, &props, 80, 10)
    );
}

#[test]
fn test_ports_panel_snapshots() {
    let ports_panel = PortsPanel::new();
    let theme = AppColors::new();
    let mut view_model = create_test_view_model();

    // Test no ports
    view_model.port_view = None;

    let props = PortsPanelProps {
        view_model: &view_model,
        theme: &theme,
    };

    assert_snapshot!("ports_empty", render_component(&ports_panel, &props, 60, 8));

    // Test single port
    let ports = vec![ContainerPorts {
        ip: Some("0.0.0.0".parse().unwrap()),
        private: 80,
        public: Some(8080),
    }];

    view_model.port_view = Some(PortView {
        ports,
        state: State::Running(RunningState::Healthy),
        max_lens: (10, 5, 5), // Example max lengths
    });

    let props = PortsPanelProps {
        view_model: &view_model,
        theme: &theme,
    };

    assert_snapshot!(
        "ports_single",
        render_component(&ports_panel, &props, 60, 8)
    );

    // Test multiple ports
    let ports = vec![
        ContainerPorts {
            ip: Some("0.0.0.0".parse().unwrap()),
            private: 80,
            public: Some(8080),
        },
        ContainerPorts {
            ip: Some("0.0.0.0".parse().unwrap()),
            private: 443,
            public: Some(8443),
        },
        ContainerPorts {
            ip: None,
            private: 3306,
            public: None,
        },
        ContainerPorts {
            ip: Some("127.0.0.1".parse().unwrap()),
            private: 5432,
            public: Some(5432),
        },
    ];

    view_model.port_view = Some(PortView {
        ports,
        state: State::Running(RunningState::Healthy),
        max_lens: (10, 5, 5), // Example max lengths
    });

    let props = PortsPanelProps {
        view_model: &view_model,
        theme: &theme,
    };

    assert_snapshot!(
        "ports_multiple",
        render_component(&ports_panel, &props, 70, 10)
    );
}

#[test]
fn test_headers_panel_snapshots() {
    let headers_panel = HeadersPanel::new();
    let theme = AppColors::new();
    let keymap = Keymap::default();
    let gui_state = create_test_gui_state();
    let mut view_model = create_test_view_model();

    // Test default headers
    view_model.sorted_by = Some((oxker_core::Header::Name, oxker_core::SortedOrder::Asc));

    let props = HeadersPanelProps {
        view_model: &view_model,
        theme: &theme,
        keymap: &keymap,
        gui_state: &gui_state,
    };

    assert_snapshot!(
        "headers_default",
        render_component(&headers_panel, &props, 120, 2)
    );

    // Test sorted by CPU
    view_model.sorted_by = Some((oxker_core::Header::Cpu, oxker_core::SortedOrder::Asc));

    let props = HeadersPanelProps {
        view_model: &view_model,
        theme: &theme,
        keymap: &keymap,
        gui_state: &gui_state,
    };

    assert_snapshot!(
        "headers_sorted_cpu",
        render_component(&headers_panel, &props, 120, 2)
    );

    // Test sorted by name reversed
    view_model.sorted_by = Some((oxker_core::Header::Name, oxker_core::SortedOrder::Desc));

    let props = HeadersPanelProps {
        view_model: &view_model,
        theme: &theme,
        keymap: &keymap,
        gui_state: &gui_state,
    };

    assert_snapshot!(
        "headers_sorted_name_desc",
        render_component(&headers_panel, &props, 120, 2)
    );
}

#[test]
fn test_containers_panel_snapshots() {
    let containers_panel = ContainersPanel::new();
    let theme = AppColors::new();
    let gui_state = create_test_gui_state();
    let container_state = create_test_container_state();
    let mut view_model = create_test_view_model();

    // Test empty state
    view_model.containers = vec![];

    let props = ContainersPanelProps {
        view_model: &view_model,
        theme: &theme,
        gui_state: &gui_state,
        container_state: &container_state,
    };

    assert_snapshot!(
        "containers_empty",
        render_component(&containers_panel, &props, 120, 10)
    );

    // Test with single container
    let container = TestContainerBuilder::new("abc123")
        .with_name("nginx-web")
        .with_image("nginx:latest")
        .with_state(State::Running(RunningState::Healthy))
        .with_status("Up 5 minutes")
        .with_cpu_stats(vec![25.5, 30.0, 28.0])
        .with_mem_stats(vec![100 * 1024 * 1024, 105 * 1024 * 1024])
        .with_mem_limit(1024 * 1024 * 1024)
        .build();

    view_model.containers = vec![ContainerView {
        id: container.id.clone(),
        name: container.name.to_string(),
        image: container.image.to_string(),
        state: container.state,
        status: container.status.get().to_string(),
        cpu_stats: container.cpu_stats.back().copied().unwrap_or_default(),
        mem_stats: container.mem_stats.back().copied().unwrap_or_default(),
        mem_limit: container.mem_limit,
        rx: container.rx,
        tx: container.tx,
    }];
    container_state.lock().containers.state.select(Some(0));

    let props = ContainersPanelProps {
        view_model: &view_model,
        theme: &theme,
        gui_state: &gui_state,
        container_state: &container_state,
    };

    assert_snapshot!(
        "containers_single_selected",
        render_component(&containers_panel, &props, 120, 10)
    );

    // Test with multiple containers in different states
    let containers = vec![
        TestContainerBuilder::new("abc123")
            .with_name("nginx-web")
            .with_image("nginx:latest")
            .with_state(State::Running(RunningState::Healthy))
            .with_status("Up 5 minutes")
            .with_cpu_stats(vec![25.5])
            .with_mem_stats(vec![100 * 1024 * 1024])
            .with_mem_limit(1024 * 1024 * 1024)
            .build(),
        TestContainerBuilder::new("def456")
            .with_name("postgres-db")
            .with_image("postgres:15")
            .with_state(State::Exited)
            .with_status("Exited (0) 2 hours ago")
            .with_cpu_stats(vec![0.0])
            .with_mem_stats(vec![0])
            .with_mem_limit(2 * 1024 * 1024 * 1024)
            .build(),
        TestContainerBuilder::new("ghi789")
            .with_name("redis-cache")
            .with_image("redis:7-alpine")
            .with_state(State::Paused)
            .with_status("Up 1 hour (Paused)")
            .with_cpu_stats(vec![5.2])
            .with_mem_stats(vec![50 * 1024 * 1024])
            .with_mem_limit(512 * 1024 * 1024)
            .build(),
    ];

    view_model.containers = containers
        .into_iter()
        .map(|container| ContainerView {
            id: container.id.clone(),
            name: container.name.to_string(),
            image: container.image.to_string(),
            state: container.state,
            status: container.status.get().to_string(),
            cpu_stats: container.cpu_stats.back().copied().unwrap_or_default(),
            mem_stats: container.mem_stats.back().copied().unwrap_or_default(),
            mem_limit: container.mem_limit,
            rx: container.rx,
            tx: container.tx,
        })
        .collect();
    container_state.lock().containers.state.select(Some(1)); // Select middle container

    let props = ContainersPanelProps {
        view_model: &view_model,
        theme: &theme,
        gui_state: &gui_state,
        container_state: &container_state,
    };

    assert_snapshot!(
        "containers_multiple_states",
        render_component(&containers_panel, &props, 120, 12)
    );
}

#[test]
fn test_combined_ui_layouts() {
    // Test a typical layout combining multiple panels
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),  // Filter
                    Constraint::Length(2),  // Headers
                    Constraint::Length(10), // Containers
                    Constraint::Length(10), // Charts
                    Constraint::Min(5),     // Logs
                ])
                .split(f.area());

            let theme = AppColors::new();
            let keymap = Keymap::default();
            let gui_state = create_test_gui_state();
            let container_state = create_test_container_state();
            let mut view_model = create_test_view_model();

            // Render filter
            let filter_panel = FilterPanel::new();
            filter_panel.render(
                &FilterPanelProps {
                    filter_by: FilterBy::Name,
                    filter_term: Some("web".to_string()),
                    theme: theme.clone(),
                },
                chunks[0],
                f,
            );

            // Render headers
            let headers_panel = HeadersPanel::new();
            view_model.sorted_by = Some((oxker_core::Header::Name, oxker_core::SortedOrder::Asc));

            headers_panel.render(
                &HeadersPanelProps {
                    view_model: &view_model,
                    theme: &theme,
                    keymap: &keymap,
                    gui_state: &gui_state,
                },
                chunks[1],
                f,
            );

            // Render containers
            let containers_panel = ContainersPanel::new();
            let container = TestContainerBuilder::new("abc123")
                .with_name("nginx-web")
                .with_state(State::Running(RunningState::Healthy))
                .with_cpu_stats(vec![25.5])
                .build();

            view_model.containers = vec![ContainerView {
                id: container.id.clone(),
                name: container.name.to_string(),
                image: container.image.to_string(),
                state: container.state,
                status: container.status.get().to_string(),
                cpu_stats: container.cpu_stats.back().copied().unwrap_or_default(),
                mem_stats: container.mem_stats.back().copied().unwrap_or_default(),
                mem_limit: container.mem_limit,
                rx: container.rx,
                tx: container.tx,
            }];
            container_state.lock().containers.state.select(Some(0));

            containers_panel.render(
                &ContainersPanelProps {
                    view_model: &view_model,
                    theme: &theme,
                    gui_state: &gui_state,
                    container_state: &container_state,
                },
                chunks[2],
                f,
            );

            // Render charts
            let charts_panel = ChartsPanel::new();
            let cpu_data = vec![20.0, 25.0, 30.0, 28.0, 26.0];
            let mem_data = vec![
                ByteStats::new(100 * 1024 * 1024),
                ByteStats::new(120 * 1024 * 1024),
                ByteStats::new(115 * 1024 * 1024),
                ByteStats::new(125 * 1024 * 1024),
                ByteStats::new(118 * 1024 * 1024),
            ];

            let cpu_chart_data: Vec<(f64, f64)> = cpu_data
                .iter()
                .enumerate()
                .map(|(i, &v)| (i as f64, v))
                .collect();
            let mem_chart_data: Vec<(f64, f64)> = mem_data
                .iter()
                .enumerate()
                .map(|(i, v)| (i as f64, v.get_value()))
                .collect();

            view_model.chart_data = Some(oxker_tui::ui::ChartData {
                cpu_data: (
                    cpu_chart_data,
                    CpuStats::new(30.0),
                    State::Running(RunningState::Healthy),
                ),
                mem_data: (
                    mem_chart_data,
                    ByteStats::new(125 * 1024 * 1024),
                    State::Running(RunningState::Healthy),
                ),
                current_cpu: CpuStats::new(26.0),
                current_mem: ByteStats::new(118 * 1024 * 1024),
            });

            charts_panel.render(
                &ChartsPanelProps {
                    view_model: &view_model,
                    theme: &theme,
                },
                chunks[3],
                f,
            );

            // Render logs
            let logs_panel = LogsPanel::new();
            view_model.log_view = LogView {
                logs: vec![
                    "[10:00:00] Starting application...".to_string(),
                    "[10:00:01] Server listening on port 80".to_string(),
                    "[10:00:02] Ready to accept connections".to_string(),
                ],
                title: "Container Logs".to_string(),
            };

            logs_panel.render(
                &LogsPanelProps {
                    view_model: &view_model,
                    theme: &theme,
                    gui_state: &gui_state,
                    container_state: &container_state,
                },
                chunks[4],
                f,
            );
        })
        .unwrap();

    let output = terminal_to_string(&terminal, 120, 40);
    assert_snapshot!("combined_layout_main", output);
}
