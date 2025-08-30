//! Main view that orchestrates the entire UI layout

use crate::ui::{
    FrameViewModel, GuiState, Status,
    components::{
        Component,
        panels::{
            ChartsPanel, CommandsPanel, ConfirmationModal, ContainersPanel, DeleteConfirmPanel, ErrorPanel,
            FilterPanel, HeadersPanel, HelpPanel, LogsPanel, PortsPanel,
        },
        widgets::InfoBox,
    },
};
use oxker_core::{Config, Keymap};
use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::sync::Arc;

pub struct MainView<'a> {
    // Component instances
    headers: HeadersPanel,
    containers: ContainersPanel,
    logs: LogsPanel,
    commands: CommandsPanel,
    charts: ChartsPanel,
    ports: PortsPanel,
    filter: FilterPanel,
    delete_confirm: DeleteConfirmPanel,
    confirmation_modal: ConfirmationModal,
    error: ErrorPanel,
    help: HelpPanel,
    info_box: InfoBox,

    // References passed in from parent
    config: &'a Config,
    keymap: &'a Keymap,
    gui_state: &'a Arc<Mutex<GuiState>>,
    container_state: &'a Arc<Mutex<crate::handlers::UIContainerState>>,
}

impl<'a> MainView<'a> {
    pub fn new(
        config: &'a Config,
        keymap: &'a Keymap,
        gui_state: &'a Arc<Mutex<GuiState>>,
        container_state: &'a Arc<Mutex<crate::handlers::UIContainerState>>,
    ) -> Self {
        Self {
            headers: HeadersPanel::new(),
            containers: ContainersPanel::new(),
            logs: LogsPanel::new(),
            commands: CommandsPanel::new(),
            charts: ChartsPanel::new(),
            ports: PortsPanel::new(),
            filter: FilterPanel::new(),
            delete_confirm: DeleteConfirmPanel::new(),
            confirmation_modal: ConfirmationModal::new(),
            error: ErrorPanel::new(),
            help: HelpPanel::new(),
            info_box: InfoBox::new(),
            config,
            keymap,
            gui_state,
            container_state,
        }
    }

    /// Calculate the main layout constraints
    fn calculate_main_layout(model: &FrameViewModel, area: Rect) -> Vec<Rect> {
        let constraints = if model.status.contains(&Status::Filter) {
            vec![Constraint::Max(1), Constraint::Min(1), Constraint::Max(1)]
        } else {
            vec![Constraint::Max(1), Constraint::Min(1)]
        };

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }

    /// Calculate the upper/lower split
    fn calculate_vertical_split(model: &FrameViewModel, area: Rect) -> Vec<Rect> {
        let constraints = if model.has_containers {
            vec![Constraint::Percentage(75), Constraint::Percentage(25)]
        } else {
            vec![Constraint::Percentage(100), Constraint::Percentage(0)]
        };

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }

    /// Calculate containers/logs split
    fn calculate_containers_logs_split(model: &FrameViewModel, area: Rect) -> Vec<Rect> {
        let constraints = if model.show_logs {
            vec![Constraint::Min(6), Constraint::Percentage(model.log_height)]
        } else {
            vec![Constraint::Percentage(100)]
        };

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }

    /// Calculate containers/commands split
    fn calculate_containers_commands_split(model: &FrameViewModel, area: Rect) -> Vec<Rect> {
        let constraints = if model.has_containers {
            vec![Constraint::Percentage(90), Constraint::Percentage(10)]
        } else {
            vec![Constraint::Percentage(100)]
        };

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }

    /// Calculate charts/ports split
    fn calculate_charts_ports_split(model: &FrameViewModel, area: Rect) -> Vec<Rect> {
        // Dynamic ports panel width based on IP length
        let ports_width = model.port_view.as_ref().map_or(27, |port_view| {
            let ip_width = port_view.max_lens.0.clamp(7, 15);
            u16::try_from(ip_width + 21).unwrap_or(28)
        });
        let ports_width = ports_width.clamp(27, 35);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(ports_width)])
            .split(area)
            .to_vec()
    }
}

impl super::View for MainView<'_> {
    fn render(&self, model: &FrameViewModel, frame: &mut Frame) {
        let theme = &self.config.app_colors;

        // Main UI rendering
        {
            // Calculate main layout
            let whole_layout = Self::calculate_main_layout(model, frame.area());

            // Render headers
            let headers_props = crate::ui::components::panels::headers::HeadersPanelProps {
                view_model: model,
                theme,
                keymap: self.keymap,
                gui_state: self.gui_state,
            };
            self.headers.render(&headers_props, whole_layout[0], frame);

            // Render filter if active
            if let Some(filter_rect) = whole_layout.get(2) {
                let filter_props = crate::ui::components::panels::filter::FilterPanelProps {
                    filter_by: model.filter_by,
                    filter_term: model.filter_term.clone(),
                    theme: *theme,
                };
                self.filter.render(&filter_props, *filter_rect, frame);
            }

            // Calculate vertical split (containers/logs vs charts/ports)
            let upper_main = Self::calculate_vertical_split(model, whole_layout[1]);

            // Calculate containers/logs area
            let containers_logs_section =
                Self::calculate_containers_logs_split(model, upper_main[0]);

            // Calculate containers/commands split
            let containers_commands =
                Self::calculate_containers_commands_split(model, containers_logs_section[0]);

            // Render containers
            let containers_props =
                crate::ui::components::panels::containers::ContainersPanelProps {
                    view_model: model,
                    theme,
                    gui_state: self.gui_state,
                    container_state: self.container_state,
                };
            self.containers
                .render(&containers_props, containers_commands[0], frame);

            // Render logs if visible
            if model.show_logs && containers_logs_section.len() > 1 {
                let logs_props = crate::ui::components::panels::logs::LogsPanelProps {
                    view_model: model,
                    theme,
                    gui_state: self.gui_state,
                    container_state: self.container_state,
                };
                self.logs
                    .render(&logs_props, containers_logs_section[1], frame);
            }

            // Render commands and lower section if there are containers
            if model.has_containers {
                // Render commands panel
                if let Some(commands_rect) = containers_commands.get(1) {
                    let commands_props =
                        crate::ui::components::panels::commands::CommandsPanelProps {
                            view_model: model,
                            theme,
                            gui_state: self.gui_state,
                            container_state: self.container_state,
                        };
                    self.commands.render(&commands_props, *commands_rect, frame);
                }

                // Only render charts/ports if we have vertical space
                if upper_main.len() > 1 && upper_main[1].height > 4 {
                    // Calculate and render charts/ports
                    let lower = Self::calculate_charts_ports_split(model, upper_main[1]);

                    // Render charts only if area is valid
                    if lower.len() >= 2 && lower[0].width > 10 && lower[0].height > 5 {
                        let charts_props =
                            crate::ui::components::panels::charts::ChartsPanelProps {
                                view_model: model,
                                theme,
                            };
                        self.charts.render(&charts_props, lower[0], frame);
                    }

                    // Render ports
                    if lower.len() >= 2 && lower[1].width > 2 && lower[1].height > 2 {
                        let ports_props = crate::ui::components::panels::ports::PortsPanelProps {
                            view_model: model,
                            theme,
                        };
                        self.ports.render(&ports_props, lower[1], frame);
                    }
                }
            }
        } // End of main UI rendering

        // Render overlays - these use Clear widget to properly overlay

        // Delete confirmation dialog
        if let Some(container_id) = model.delete_confirm.as_ref() {
            // Find container name from UIContainerState
            let container_name = self
                .container_state
                .lock()
                .get_container_items()
                .iter()
                .find(|c| &c.id == container_id)
                .map(|c| c.name.clone());

            if let Some(name) = container_name {
                let delete_props =
                    crate::ui::components::panels::delete_confirm::DeleteConfirmPanelProps {
                        container_name: &name,
                        theme,
                        keymap: self.keymap,
                        gui_state: self.gui_state,
                    };
                self.delete_confirm
                    .render(&delete_props, frame.area(), frame);
            } else {
                // Container was deleted externally, clear the dialog
                self.gui_state.lock().set_delete_container(None);
            }
        }

        // Command confirmation dialog
        if model.status.contains(&Status::CommandConfirm) {
            if let Some((command, container_id)) = self.gui_state.lock().get_command_confirm() {
                // Find container name from UIContainerState
                let container_name = self
                    .container_state
                    .lock()
                    .get_container_items()
                    .iter()
                    .find(|c| c.id == container_id)
                    .map(|c| c.name.clone());

                if let Some(name) = container_name {
                    let confirm_props = crate::ui::components::panels::confirmation_modal::ConfirmationModalProps {
                        command,
                        container_id: container_id.get(),
                        container_name: name.get(),
                        theme,
                        keymap: self.keymap,
                        gui_state: self.gui_state,
                    };
                    self.confirmation_modal.render(&confirm_props, frame.area(), frame);
                } else {
                    // Container was deleted externally, clear the dialog
                    self.gui_state.lock().set_command_confirm(None);
                }
            }
        }

        // Info box
        if let Some((text, instant)) = model.info_text.as_ref() {
            let info_props = crate::ui::components::widgets::info_box::InfoBoxProps {
                message: text,
                start_time: *instant,
                theme,
            };
            self.info_box.render(&info_props, frame.area(), frame);
        }

        // Help panel
        if model.status.contains(&Status::Help) {
            let help_props = crate::ui::components::panels::help::HelpPanelProps {
                theme: *theme,
                keymap: self.keymap.clone(),
                show_timestamp: self.config.show_timestamp,
                timezone: self.config.timezone.clone(),
            };
            self.help.render(&help_props, frame.area(), frame);
        }

        // Error dialog
        if let Some(error) = model.has_error.as_ref() {
            let error_props = crate::ui::components::panels::error::ErrorPanelProps {
                error,
                theme,
                keymap: self.keymap,
                auto_close_seconds: None,
            };
            self.error.render(&error_props, frame.area(), frame);
        }
    }
}
