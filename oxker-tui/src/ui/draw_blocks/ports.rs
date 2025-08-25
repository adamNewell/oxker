use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use oxker_core::{State, AppColors};
use crate::ui::FrameViewModel;

/// Get the port title color, at the moment the color is only customizable if the container is alive
const fn get_port_title_color(colors: AppColors, state: State) -> Color {
    if state.is_alive() {
        colors.chart_ports.title
    } else {
        state.get_color(colors)
    }
}

/// Display the ports in a formatted list
pub fn draw(area: Rect, colors: AppColors, f: &mut Frame, fd: &FrameViewModel) {
    if let Some(port_view) = fd.port_view.as_ref() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::new().fg(colors.chart_ports.border))
            .title_alignment(Alignment::Center)
            .title(Span::styled(
                " ports ",
                Style::default()
                    .fg(get_port_title_color(colors, port_view.state.clone()))
                    .bg(colors.chart_ports.background)
                    .add_modifier(Modifier::BOLD),
            ));

        // Column specifications
        let (ip_len, _, _) = port_view.max_lens;
        let ip_width = ip_len.max(2).min(16); // Min 2 (for "ip" header), max 16 characters
        let private_width = 7; // Always 7 text columns for private
        let public_width = 7;  // Always 7 text columns for public

        if port_view.ports.is_empty() {
            let text = match port_view.state {
                State::Running(_) | State::Paused | State::Restarting => "no ports",
                _ => "",
            };
            let paragraph = Paragraph::new(Span::from(text).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(block)
                .bg(colors.chart_ports.background);
            f.render_widget(paragraph, area);
        } else {
            let mut output = vec![];
            
            // Header line: IP + 3 spaces + Private(7) + 3 spaces + Public(7)
            let header_line = format!(
                "{:>ip_width$}   {:>private_width$}  {:>public_width$}",
                "ip", "private", "public"
            );
            output.push(Line::from(Span::from(header_line).fg(colors.chart_ports.headings)));
            
            for item in &port_view.ports {
                let strings = item.get_all();
                let data_line = format!(
                    "{:>ip_width$}   {:>private_width$}  {:>public_width$}",
                    strings.0, strings.1, strings.2
                );
                output.push(Line::from(Span::from(data_line).fg(colors.chart_ports.text)));
            }
            let paragraph = Paragraph::new(output)
                .block(block)
                .bg(colors.chart_ports.background);
            f.render_widget(paragraph, area);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use insta::assert_snapshot;
    use ratatui::style::{Color, Modifier};

    use oxker_core::{ContainerPorts, RunningState, State, AppColors};
    use crate::{
        ui::{
            FrameViewModel,
            draw_blocks::tests::{COLOR_ORANGE, COLOR_RX, COLOR_TX, get_result, test_setup, test_setup_no_ports, test_setup_custom, test_setup_multiple_ports},
        },
    };

    #[test]
    /// Port section when container has no ports
    fn test_draw_blocks_ports_no_ports() {
        let mut setup = test_setup_no_ports(30, 8);

        let fd = setup.fd.clone();
        setup
            .terminal
            .draw(|f| {
                super::draw(setup.area, setup.config.app_colors, f, &fd);
            })
            .unwrap();
        assert_snapshot!(setup.terminal.backend());

        for (row_index, result_row) in get_result(&setup) {
            for (result_cell_index, result_cell) in result_row.iter().enumerate() {
                match (row_index, result_cell_index) {
                    (0, 11..=17) => {
                        assert_eq!(result_cell.bg, Color::Reset);
                        assert_eq!(result_cell.fg, Color::Green);
                        assert_eq!(result_cell.modifier, Modifier::BOLD);
                    }
                    (1, 11..=18) => {
                        assert_eq!(result_cell.bg, Color::Reset);
                        assert_eq!(result_cell.fg, Color::White);
                        assert_eq!(result_cell.modifier, Modifier::BOLD);
                    }
                    _ => {
                        assert_eq!(result_cell.bg, Color::Reset);
                        assert_eq!(result_cell.fg, Color::White);
                        assert!(result_cell.modifier.is_empty());
                    }
                }
            }
        }
    }

    #[test]
    /// Port section when container has no ports
    // When state is "State::Running | State::Paused | State::Restarting, won't show "no ports"
    fn test_draw_blocks_ports_no_ports_dead() {
        // First test with running state and no ports
        let mut setup = test_setup_no_ports(30, 8);

        let fd = setup.fd.clone();
        setup
            .terminal
            .draw(|f| {
                super::draw(setup.area, setup.config.app_colors, f, &fd);
            })
            .unwrap();
        // split

        // Now test with Dead state and no ports
        let mut setup = test_setup_custom(30, 8, |containers| {
            if !containers.is_empty() {
                containers[0].ports = vec![];
                containers[0].state = State::Dead;
            }
        });

        let fd = setup.fd.clone();
        setup
            .terminal
            .draw(|f| {
                super::draw(setup.area, setup.config.app_colors, f, &fd);
            })
            .unwrap();

        assert_snapshot!(setup.terminal.backend());

        for (row_index, result_row) in get_result(&setup) {
            for (result_cell_index, result_cell) in result_row.iter().enumerate() {
                assert_eq!(result_cell.bg, Color::Reset);
                if let (0, 11..=17) = (row_index, result_cell_index) {
                    assert_eq!(result_cell.fg, Color::Red);
                    assert_eq!(result_cell.modifier, Modifier::BOLD);
                } else {
                    assert_eq!(result_cell.fg, Color::White);
                    assert!(result_cell.modifier.is_empty());
                }
            }
        }
    }

    #[test]
    /// Port section when container has multiple ports
    fn test_draw_blocks_ports_multiple_ports() {
        let mut setup = test_setup_multiple_ports(32, 8);

        let fd = setup.fd.clone();
        setup
            .terminal
            .draw(|f| {
                super::draw(setup.area, setup.config.app_colors, f, &fd);
            })
            .unwrap();
        assert_snapshot!(setup.terminal.backend());

        for (row_index, result_row) in get_result(&setup) {
            for (result_cell_index, result_cell) in result_row.iter().enumerate() {
                assert_eq!(result_cell.bg, Color::Reset);

                match (row_index, result_cell_index) {
                    (0, 12..=18) => {
                        assert_eq!(result_cell.fg, Color::Green);
                        assert_eq!(result_cell.modifier, Modifier::BOLD);
                    }
                    (1, 1..=28) => {
                        assert_eq!(result_cell.fg, Color::Yellow);
                        assert!(result_cell.modifier.is_empty());
                    }
                    (2..=4, 1..=28) | (0 | 2..=9, 0..=31) | (1, 0 | 29..=31) => {
                        assert_eq!(result_cell.fg, Color::White);
                        assert!(result_cell.modifier.is_empty());
                    }
                    _ => {
                        assert_eq!(result_cell.fg, Color::Reset);
                        assert!(result_cell.modifier.is_empty());
                    }
                }
            }
        }
    }

    #[test]
    /// Port section title color correct dependant on state
    fn test_draw_blocks_ports_container_state() {
        let mut setup = test_setup(32, 8, true, true);

        let fd = setup.fd.clone();
        setup
            .terminal
            .draw(|f| {
                super::draw(setup.area, setup.config.app_colors, f, &fd);
            })
            .unwrap();

        assert_snapshot!(setup.terminal.backend());

        for (row_index, result_row) in get_result(&setup) {
            for (result_cell_index, result_cell) in result_row.iter().enumerate() {
                assert_eq!(result_cell.bg, Color::Reset);
                if let (0, 12..=18) = (row_index, result_cell_index) {
                    assert_eq!(result_cell.fg, Color::Green);
                    assert_eq!(result_cell.modifier, Modifier::BOLD);
                }
            }
        }

        // Note: State changes would be done through event system or test setup methods
        let fd = setup.fd.clone();
        setup
            .terminal
            .draw(|f| {
                super::draw(setup.area, setup.config.app_colors, f, &fd);
            })
            .unwrap();

        for (row_index, result_row) in get_result(&setup) {
            for (result_cell_index, result_cell) in result_row.iter().enumerate() {
                assert_eq!(result_cell.bg, Color::Reset);
                if let (0, 12..=18) = (row_index, result_cell_index) {
                    assert_eq!(result_cell.fg, Color::Yellow);
                    assert_eq!(result_cell.modifier, Modifier::BOLD);
                }
            }
        }

        // Note: State changes would be done through event system or test setup methods
        let fd = setup.fd.clone();
        setup
            .terminal
            .draw(|f| {
                super::draw(setup.area, setup.config.app_colors, f, &fd);
            })
            .unwrap();

        for (row_index, result_row) in get_result(&setup) {
            for (result_cell_index, result_cell) in result_row.iter().enumerate() {
                assert_eq!(result_cell.bg, Color::Reset);
                if let (0, 12..=18) = (row_index, result_cell_index) {
                    assert_eq!(result_cell.fg, Color::Red);
                    assert_eq!(result_cell.modifier, Modifier::BOLD);
                }
            }
        }
    }

    #[test]
    /// Custom colors applied to ports panel
    fn test_draw_blocks_ports_custom_colors() {
        let mut setup = test_setup(32, 8, true, true);

        let mut colors = AppColors::new();
        colors.chart_ports.background = Color::Black;
        colors.chart_ports.border = Color::Yellow;
        colors.chart_ports.headings = Color::Red;
        colors.chart_ports.text = Color::Green;
        colors.chart_ports.title = Color::Magenta;

        let fd = setup.fd.clone();
        setup
            .terminal
            .draw(|f| {
                super::draw(setup.area, colors, f, &fd);
            })
            .unwrap();

        assert_snapshot!(setup.terminal.backend());

        for (row_index, result_row) in get_result(&setup) {
            for (result_cell_index, result_cell) in result_row.iter().enumerate() {
                assert_eq!(result_cell.bg, Color::Black);

                match (row_index, result_cell_index) {
                    // title => {
                    (0, 12..=18) => {
                        assert_eq!(result_cell.fg, Color::Magenta);
                    }
                    // title
                    (1, 1..=24) => {
                        assert_eq!(result_cell.fg, Color::Red);
                    }
                    // text
                    (2, 1..=24) => {
                        assert_eq!(result_cell.fg, Color::Green);
                    }
                    // border & everything else
                    _ => {
                        assert_eq!(result_cell.fg, Color::Yellow);
                    }
                }
            }
        }
    }

    #[test]
    // Custom state color applied to ports panel title
    fn test_draw_blocks_ports_custom_colors_state() {
        let mut setup = test_setup(32, 8, true, true);

        let mut colors = AppColors::new();
        colors.container_state.dead = Color::Green;
        colors.container_state.exited = Color::Magenta;
        colors.container_state.paused = Color::Gray;
        colors.container_state.removing = COLOR_ORANGE;
        colors.container_state.restarting = COLOR_RX;
        colors.container_state.running_healthy = COLOR_TX;
        colors.container_state.running_unhealthy = Color::Cyan;
        colors.container_state.unknown = Color::LightMagenta;

        colors.chart_ports.title = Color::DarkGray;

        for i in [
            (State::Dead, Color::Green),
            (State::Exited, Color::Magenta),
            (State::Paused, Color::Gray),
            (State::Removing, COLOR_ORANGE),
            (State::Restarting, COLOR_RX),
            (State::Unknown, Color::LightMagenta),
            (State::Running(RunningState::Healthy), Color::DarkGray),
            (State::Running(RunningState::Unhealthy), Color::DarkGray),
        ] {
            // Note: State changes would be done through event system or test setup methods

            let fd = setup.fd.clone();
            setup
                .terminal
                .draw(|f| {
                    super::draw(setup.area, colors, f, &fd);
                })
                .unwrap();

            // assert_snapshot!(setup.terminal.backend());

            for (row_index, result_row) in get_result(&setup) {
                for (result_cell_index, result_cell) in result_row.iter().enumerate() {
                    if row_index == 0 && (12..=18).contains(&result_cell_index) {
                        assert_eq!(result_cell.fg, i.1);
                    }
                }
            }
        }
    }
}
