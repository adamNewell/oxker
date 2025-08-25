#[cfg(test)]
pub mod test_utils {
    use crate::handlers::UIContainerState;
    use crate::ui::{FrameViewModel, GuiState, Rerender, SelectablePanel, Status};
    use oxker_core::{
        AppColors, ByteStats, Config, ContainerId, ContainerItem, ContainerPorts, ContainerStatus,
        CoreHandle, CpuStats, EventBus, Keymap, RunningState, State,
    };
    use oxker_core::{FilterBy, Header, SortedOrder};
    use parking_lot::Mutex;
    use std::collections::{HashSet, VecDeque};
    use std::sync::Arc;
    use std::time::Instant;

    /// Test configuration builder
    pub struct TestConfigBuilder {
        config: Config,
    }

    impl TestConfigBuilder {
        pub fn new() -> Self {
            Self {
                config: Config {
                    app_colors: AppColors::new(),
                    color_logs: false,
                    docker_interval_ms: 1000,
                    keymap: Keymap::new(),
                    show_logs: true,
                    show_timestamp: false,
                    timestamp_format: String::new(),
                    timezone: None,
                    save_dir: None,
                    gui: true,
                    show_self: false,
                    in_container: false,
                    raw_logs: false,
                    show_std_err: false,
                    host: None,
                    use_cli: false,
                },
            }
        }

        pub fn with_show_logs(mut self, show: bool) -> Self {
            self.config.show_logs = show;
            self
        }

        pub fn with_colors(mut self, colors: AppColors) -> Self {
            self.config.app_colors = colors;
            self
        }

        pub fn build(self) -> Config {
            self.config
        }
    }

    /// Test container builder
    pub struct TestContainerBuilder {
        id: ContainerId,
        name: String,
        image: String,
        state: State,
        status: ContainerStatus,
        ports: Vec<ContainerPorts>,
        cpu_stats: VecDeque<CpuStats>,
        mem_stats: VecDeque<ByteStats>,
        mem_limit: ByteStats,
        rx: ByteStats,
        tx: ByteStats,
    }

    impl TestContainerBuilder {
        pub fn new(id: &str) -> Self {
            Self {
                id: ContainerId::from(id),
                name: format!("container_{}", id),
                image: format!("image_{}", id),
                state: State::Running(RunningState::Healthy),
                status: ContainerStatus::from("Up 1 hour".to_string()),
                ports: vec![],
                cpu_stats: VecDeque::new(),
                mem_stats: VecDeque::new(),
                mem_limit: ByteStats::new(1073741824), // 1GB
                rx: ByteStats::new(0),
                tx: ByteStats::new(0),
            }
        }

        pub fn with_name(mut self, name: &str) -> Self {
            self.name = name.to_string();
            self
        }

        pub fn with_image(mut self, image: &str) -> Self {
            self.image = image.to_string();
            self
        }

        pub fn with_state(mut self, state: State) -> Self {
            self.state = state;
            self
        }

        pub fn with_port(mut self, private: u16, public: Option<u16>) -> Self {
            self.ports.push(ContainerPorts {
                ip: None,
                private,
                public,
            });
            self
        }

        pub fn with_cpu_stats(mut self, stats: Vec<f64>) -> Self {
            self.cpu_stats = stats.into_iter().map(CpuStats::new).collect();
            self
        }

        pub fn with_memory_stats(mut self, stats: Vec<u64>) -> Self {
            self.mem_stats = stats.into_iter().map(ByteStats::new).collect();
            self
        }

        pub fn build(self) -> ContainerItem {
            let mut container = ContainerItem::new(
                0, // index
                self.id,
                self.image,
                false, // is_oxker
                self.name,
                self.ports,
                self.state,
                self.status,
            );

            // Set stats
            container.cpu_stats = self.cpu_stats;
            container.mem_stats = self.mem_stats;
            container.mem_limit = self.mem_limit;
            container.rx = self.rx;
            container.tx = self.tx;

            container
        }
    }

    /// Test setup structure that provides all necessary components
    pub struct TestSetup {
        pub core_handle: CoreHandle,
        pub gui_state: Arc<Mutex<GuiState>>,
        pub container_state: Arc<Mutex<UIContainerState>>,
        pub rerender: Arc<Rerender>,
        pub terminal: TestTerminal,
    }

    impl TestSetup {
        pub fn new(width: u16, height: u16, show_logs: bool) -> Self {
            let config = TestConfigBuilder::new().with_show_logs(show_logs).build();

            let (event_bus, _receiver) = EventBus::new(10);
            let core_handle = CoreHandle::new(event_bus, config);

            let rerender = Arc::new(Rerender::new());
            let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, show_logs)));
            let container_state = Arc::new(Mutex::new(UIContainerState::new()));

            let terminal = TestTerminal::new(width, height);

            Self {
                core_handle,
                gui_state,
                container_state,
                rerender,
                terminal,
            }
        }

        /// Add containers to the UI state
        pub fn with_containers(self, containers: Vec<ContainerItem>) -> Self {
            use oxker_core::events::types::ContainerItem as EventContainerItem;

            let event_containers: Vec<EventContainerItem> = containers
                .into_iter()
                .map(|c| EventContainerItem {
                    id: c.id.get().to_string(),
                    name: c.name.to_string(),
                    image: c.image.to_string(),
                    state: c.state.to_string(),
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

            self.container_state
                .lock()
                .update_containers(event_containers);
            self
        }

        /// Add logs to the selected container
        pub fn with_logs(self, logs: Vec<&str>) -> Self {
            use oxker_core::events::types::LogLine;

            if let Some(container_id) = self.container_state.lock().get_selected_container_id() {
                let log_lines: Vec<LogLine> = logs
                    .into_iter()
                    .map(|msg| LogLine {
                        container_id: container_id.get().to_string(),
                        message: msg.to_string(),
                        timestamp: String::new(),
                    })
                    .collect();

                self.container_state
                    .lock()
                    .add_logs(container_id.get().to_string(), log_lines);
            }

            self
        }

        /// Add CPU stats to a container
        pub fn with_cpu_stats(self, container_id: &str, stats: Vec<f64>) -> Self {
            use oxker_core::events::types::Stats;

            for (i, cpu_usage) in stats.into_iter().enumerate() {
                let stats = Stats {
                    container_id: container_id.to_string(),
                    cpu_usage,
                    memory_usage: 100 * 1024 * 1024,  // 100MB default
                    memory_limit: 1024 * 1024 * 1024, // 1GB default
                    network_rx: (i as u64) * 1000,
                    network_tx: (i as u64) * 500,
                };

                self.container_state
                    .lock()
                    .update_container_stats(container_id.to_string(), stats);
            }

            self
        }
    }

    /// Test terminal for capturing rendered output
    pub struct TestTerminal {
        terminal: ratatui::Terminal<ratatui::backend::TestBackend>,
    }

    impl TestTerminal {
        pub fn new(width: u16, height: u16) -> Self {
            let backend = ratatui::backend::TestBackend::new(width, height);
            let terminal = ratatui::Terminal::new(backend).unwrap();
            Self { terminal }
        }

        pub fn buffer(&self) -> &ratatui::buffer::Buffer {
            self.terminal.backend().buffer()
        }

        pub fn draw<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>>
        where
            F: FnOnce(&mut ratatui::Frame),
        {
            self.terminal.draw(f)?;
            Ok(())
        }
    }

    /// Generate test containers
    pub fn gen_containers() -> (Vec<ContainerId>, Vec<ContainerItem>) {
        gen_containers_n(3)
    }

    pub fn gen_containers_n(n: usize) -> (Vec<ContainerId>, Vec<ContainerItem>) {
        let mut ids = Vec::new();
        let items = (1..=n)
            .map(|index| {
                let id = ContainerId::from(format!("{}", index).as_str());
                ids.push(id.clone());
                gen_item(&id, index)
            })
            .collect::<Vec<_>>();
        (ids, items)
    }

    fn gen_item(id: &ContainerId, index: usize) -> ContainerItem {
        ContainerItem::new(
            u64::try_from(index).unwrap(),
            id.clone(),
            format!("image_{}", index),
            false,
            format!("container_{}", index),
            vec![ContainerPorts {
                ip: None,
                private: u16::try_from(index).unwrap_or(1) + 8000,
                public: Some(u16::try_from(index).unwrap_or(1) + 9000),
            }],
            State::Running(RunningState::Healthy),
            ContainerStatus::from("Up 1 hour".to_owned()),
        )
    }

    /// Generate test AppData
    pub fn gen_appdata(containers: &[ContainerItem]) -> oxker_core::AppData {
        use oxker_core::StatefulList;
        let (event_bus, _receiver) = EventBus::new(100);
        let event_bus = Arc::new(event_bus);
        let mut app_data = oxker_core::AppData::new(gen_config(), event_bus);
        // Note: This is a workaround for tests - containers field is private
        // In real code, we'd use the public API methods
        app_data
    }

    /// Default test config
    pub fn gen_config() -> Config {
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
            timestamp_format: "HH:MM:SS.NNNNN dd-mm-yyyy".to_owned(),
            show_timestamp: false,
            use_cli: false,
            show_logs: true,
            timezone: None,
        }
    }

    /// Helper to create FrameViewModel for tests
    pub fn create_test_frame_view_model(
        gui_state: &Arc<Mutex<GuiState>>,
        container_state: &Arc<Mutex<UIContainerState>>,
        config: &Config,
    ) -> FrameViewModel {
        let gui_data = gui_state.lock();
        let container_data = container_state.lock();
        FrameViewModel::from_state(
            &container_data,
            &gui_data,
            config.app_colors,
            gui_data.get_screen_width(),
        )
    }
}
