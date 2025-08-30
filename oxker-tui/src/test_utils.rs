pub mod mock_core_handle;

#[cfg(test)]
mod mock_core_handle_test;

#[cfg(test)]
#[allow(clippy::module_inception)]
pub mod test_utils {
    use crate::handlers::UIContainerState;
    use crate::ui::{FrameViewModel, GuiState, Rerender};
    use oxker_core::{
        AppColors, ByteStats, Config, ContainerId, ContainerItem, ContainerItemInit,
        ContainerPorts, ContainerStatus, CoreHandle, CpuStats, EventBus, Keymap, RunningState,
        State,
    };
    use parking_lot::Mutex;
    use std::collections::VecDeque;
    use std::sync::Arc;

    pub struct TestConfigBuilder {
        config: Config,
    }

    impl Default for TestConfigBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TestConfigBuilder {
        #[must_use]
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
                },
            }
        }

        #[must_use]
        pub fn with_show_logs(mut self, show: bool) -> Self {
            self.config.show_logs = show;
            self
        }

        #[must_use]
        pub fn with_colors(mut self, colors: AppColors) -> Self {
            self.config.app_colors = colors;
            self
        }

        #[must_use]
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
        #[must_use]
        pub fn new(id: &str) -> Self {
            Self {
                id: ContainerId::from(id),
                name: format!("container_{id}"),
                image: format!("image_{id}"),
                state: State::Running(RunningState::Healthy),
                status: ContainerStatus::from("Up 1 hour".to_string()),
                ports: vec![],
                cpu_stats: VecDeque::new(),
                mem_stats: VecDeque::new(),
                mem_limit: ByteStats::new(1_073_741_824), // 1GB
                rx: ByteStats::new(0),
                tx: ByteStats::new(0),
            }
        }

        #[must_use]
        pub fn with_name(mut self, name: &str) -> Self {
            self.name = name.to_string();
            self
        }

        #[must_use]
        pub fn with_image(mut self, image: &str) -> Self {
            self.image = image.to_string();
            self
        }

        #[must_use]
        pub fn with_state(mut self, state: State) -> Self {
            self.state = state;
            self
        }

        #[must_use]
        pub fn with_port(mut self, private: u16, public: Option<u16>) -> Self {
            self.ports.push(ContainerPorts {
                ip: None,
                private,
                public,
            });
            self
        }

        #[must_use]
        pub fn with_cpu_stats(mut self, stats: Vec<f64>) -> Self {
            self.cpu_stats = stats.into_iter().map(CpuStats::new).collect();
            self
        }

        #[must_use]
        pub fn with_memory_stats(mut self, stats: Vec<u64>) -> Self {
            self.mem_stats = stats.into_iter().map(ByteStats::new).collect();
            self
        }

        #[must_use]
        pub fn build(self) -> ContainerItem {
            let mut container = ContainerItem::new(ContainerItemInit {
                created: 0, // index
                id: self.id,
                image: self.image,
                is_oxker: false, // is_oxker
                name: self.name,
                ports: self.ports,
                state: self.state,
                status: self.status,
            });

            // Set stats
            container.cpu_stats = self.cpu_stats;
            container.mem_stats = self.mem_stats;
            container.mem_limit = self.mem_limit;
            container.rx = self.rx;
            container.tx = self.tx;

            container
        }
    }

    pub struct TestSetup {
        pub core_handle: CoreHandle,
        pub gui_state: Arc<Mutex<GuiState>>,
        pub container_state: Arc<Mutex<UIContainerState>>,
        pub rerender: Arc<Rerender>,
        pub terminal: TestTerminal,
    }

    impl TestSetup {
        #[must_use]
        pub fn new(width: u16, height: u16, show_logs: bool) -> Self {
            let config = TestConfigBuilder::new().with_show_logs(show_logs).build();

            let (event_bus, _receiver) = EventBus::new(10);
            let core_handle = CoreHandle::new(event_bus, &config);

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
        #[must_use]
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

        #[must_use]
        pub fn with_logs(self, logs: Vec<&str>) -> Self {
            use oxker_core::events::types::LogLine;

            let container_id = self.container_state.lock().get_selected_container_id();
            if let Some(container_id) = container_id {
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
                    .add_logs(container_id.get(), log_lines);
            }

            self
        }

        #[must_use]
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
                    .update_container_stats(container_id, &stats);
            }

            self
        }
    }

    pub struct TestTerminal {
        terminal: ratatui::Terminal<ratatui::backend::TestBackend>,
    }

    impl TestTerminal {
        /// Creates a new test terminal with the specified dimensions.
        ///
        /// # Panics
        ///
        /// Panics if the terminal cannot be created with the test backend.
        #[must_use]
        pub fn new(width: u16, height: u16) -> Self {
            let backend = ratatui::backend::TestBackend::new(width, height);
            let terminal = ratatui::Terminal::new(backend).unwrap();
            Self { terminal }
        }

        #[must_use]
        pub fn buffer(&self) -> &ratatui::buffer::Buffer {
            self.terminal.backend().buffer()
        }

        /// Draws content to the test terminal.
        ///
        /// # Errors
        ///
        /// Returns an error if the terminal draw operation fails.
        pub fn draw<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>>
        where
            F: FnOnce(&mut ratatui::Frame),
        {
            self.terminal.draw(f)?;
            Ok(())
        }
    }

    #[must_use]
    pub fn gen_containers() -> (Vec<ContainerId>, Vec<ContainerItem>) {
        gen_containers_n(3)
    }

    #[must_use]
    pub fn gen_containers_n(n: usize) -> (Vec<ContainerId>, Vec<ContainerItem>) {
        let mut ids = Vec::new();
        let items = (1..=n)
            .map(|index| {
                let id = ContainerId::from(format!("{index}").as_str());
                ids.push(id.clone());
                gen_item(&id, index)
            })
            .collect::<Vec<_>>();
        (ids, items)
    }

    fn gen_item(id: &ContainerId, index: usize) -> ContainerItem {
        ContainerItem::new(ContainerItemInit {
            created: u64::try_from(index).unwrap(),
            id: id.clone(),
            image: format!("image_{index}"),
            is_oxker: false,
            name: format!("container_{index}"),
            ports: vec![ContainerPorts {
                ip: None,
                private: u16::try_from(index).unwrap_or(1) + 8000,
                public: Some(u16::try_from(index).unwrap_or(1) + 9000),
            }],
            state: State::Running(RunningState::Healthy),
            status: ContainerStatus::from("Up 1 hour".to_owned()),
        })
    }

    #[must_use]
    pub fn gen_appdata(_containers: &[ContainerItem]) -> oxker_core::AppData {
        let (event_bus, _receiver) = EventBus::new(100);
        let event_bus = Arc::new(event_bus);

        // Note: This is a workaround for tests - containers field is private
        // In real code, we'd use the public API methods
        oxker_core::AppData::new(gen_config(), event_bus)
    }

    /// Default test config
    #[must_use]
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
            show_logs: true,
            timezone: None,
        }
    }

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
