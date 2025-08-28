//! Ports panel component for displaying container port mappings

use crate::ui::{FrameViewModel, color_conversion::IntoRatatuiColor, components::Component};
use oxker_core::{AppColors, State, config::Color as CoreColor};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

/// Ports panel component for displaying container port mappings
pub struct PortsPanel {
    // No internal state needed
}

pub struct PortsPanelProps<'a> {
    pub view_model: &'a FrameViewModel,
    pub theme: &'a AppColors,
}

impl PortsPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Get the port title color based on container state
    const fn get_port_title_color(colors: &AppColors, state: State) -> CoreColor {
        if state.is_alive() {
            colors.chart_ports.title
        } else {
            state.get_color(*colors)
        }
    }
}

impl<'p> Component<'p> for PortsPanel {
    type Props = PortsPanelProps<'p>;
    type Event = ();

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        if let Some(port_view) = props.view_model.port_view.as_ref() {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::new().fg(props.theme.chart_ports.border.into_ratatui_color()))
                .title_alignment(Alignment::Center)
                .title(Span::styled(
                    " ports ",
                    Style::default()
                        .fg(Self::get_port_title_color(props.theme, port_view.state)
                            .into_ratatui_color())
                        .bg(props.theme.chart_ports.background.into_ratatui_color())
                        .add_modifier(Modifier::BOLD),
                ));

            // Column specifications
            let (ip_len, _, _) = port_view.max_lens;
            let ip_width = ip_len.clamp(2, 16); // Min 2 (for "ip" header), max 16 characters
            let private_width = 7; // Always 7 text columns for private
            let public_width = 7; // Always 7 text columns for public

            if port_view.ports.is_empty() {
                // Show empty state based on container state
                let text = match port_view.state {
                    State::Running(_) | State::Paused | State::Restarting => "no ports",
                    _ => "",
                };

                let paragraph = Paragraph::new(Span::from(text).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center)
                    .block(block)
                    .bg(props.theme.chart_ports.background.into_ratatui_color());
                frame.render_widget(paragraph, area);
            } else {
                // Build the port list
                let mut output = vec![];

                // Header line
                let header_line = format!(
                    "{:>ip_width$}   {:>private_width$}  {:>public_width$}",
                    "ip", "private", "public"
                );
                output.push(Line::from(
                    Span::from(header_line).fg(props
                        .theme
                        .chart_ports
                        .headings
                        .into_ratatui_color()),
                ));

                // Port entries
                for item in &port_view.ports {
                    let (ip, private, public) = item.get_all();
                    let data_line = format!(
                        "{ip:>ip_width$}   {private:>private_width$}  {public:>public_width$}"
                    );
                    output.push(Line::from(
                        Span::from(data_line).fg(props.theme.chart_ports.text.into_ratatui_color()),
                    ));
                }

                let paragraph = Paragraph::new(output).block(block).bg(props
                    .theme
                    .chart_ports
                    .background
                    .into_ratatui_color());
                frame.render_widget(paragraph, area);
            }
        }
    }
}

impl Default for PortsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::PortView;
    use oxker_core::ContainerPorts;
    use oxker_core::State;
    use ratatui::{Terminal, backend::TestBackend};
    use std::net::IpAddr;

    #[test]
    fn test_ports_panel_no_ports() {
        let panel = PortsPanel::new();
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Set up port view with no ports
        fd.port_view = Some(PortView {
            ports: vec![],
            max_lens: (2, 7, 7), // Default minimum lengths
            state: State::Running(oxker_core::RunningState::Healthy),
        });

        let props = PortsPanelProps {
            view_model: &fd,
            theme: &theme,
        };

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify renders without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 40);
        assert_eq!(buffer.area.height, 10);
    }

    #[test]
    fn test_ports_panel_with_ports() {
        let panel = PortsPanel::new();
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Set up port view with some ports
        fd.port_view = Some(PortView {
            ports: vec![
                ContainerPorts {
                    ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                    private: 80,
                    public: Some(8080),
                },
                ContainerPorts {
                    ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
                    private: 443,
                    public: Some(8443),
                },
            ],
            max_lens: (7, 7, 7), // Lengths for formatting
            state: State::Running(oxker_core::RunningState::Healthy),
        });

        let props = PortsPanelProps {
            view_model: &fd,
            theme: &theme,
        };

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify renders without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 40);
        assert_eq!(buffer.area.height, 10);
    }

    #[test]
    fn test_ports_panel_dead_container() {
        let panel = PortsPanel::new();
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Set up port view with dead state
        fd.port_view = Some(PortView {
            ports: vec![],
            max_lens: (2, 7, 7),
            state: State::Dead,
        });

        let props = PortsPanelProps {
            view_model: &fd,
            theme: &theme,
        };

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify renders without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 40);
        assert_eq!(buffer.area.height, 10);
    }
}
