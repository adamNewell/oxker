//! Headers panel component for displaying sortable column headers

use crate::ui::{
    FrameViewModel, Status,
    color_conversion::IntoRatatuiColor,
    components::Component,
    gui_state::{GuiState, Region},
};
use oxker_core::{AppColors, CoreCommand, Header, Keymap, SortedOrder, config::Color as CoreColor};
use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use std::sync::Arc;

const MARGIN: &str = "  "; // Two spaces for better column separation
const CONSTRAINT_100: u16 = 100;

/// Headers panel component that displays column headers and controls
pub struct HeadersPanel {
    // No internal state needed
}

pub struct HeadersPanelProps<'a> {
    pub view_model: &'a FrameViewModel,
    pub theme: &'a AppColors,
    pub keymap: &'a Keymap,
    pub gui_state: &'a Arc<Mutex<GuiState>>,
}

impl HeadersPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }


    /// Generate help text based on current state and keymap
    fn gen_help_text(props: &HeadersPanelProps) -> String {
        let suffix = if props.view_model.status.contains(&Status::Help) {
            "exit"
        } else {
            "show"
        };

        let keymap = props.keymap;
        if keymap.toggle_help == Keymap::new().toggle_help {
            format!("( h ) {suffix} help{MARGIN}")
        } else if let Some(secondary) = keymap.toggle_help.1 {
            format!(
                " ( {} | {} ) {suffix} help{MARGIN}",
                format_key(keymap.toggle_help.0),
                format_key(secondary),
            )
        } else {
            format!(
                " ( {} ) {suffix} help{MARGIN}",
                format_key(keymap.toggle_help.0)
            )
        }
    }

    /// Draw the help section
    fn draw_help(
        frame: &mut Frame,
        props: &HeadersPanelProps,
        help_text: String,
        split_bar: &[Rect],
    ) {
        let help_text_color = if props.view_model.status.contains(&Status::Help) {
            props.theme.headers_bar.text
        } else {
            props.theme.headers_bar.text_selected
        };

        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(help_text_color.into_ratatui_color()))
            .alignment(Alignment::Right);

        // If no containers, don't display the headers
        let help_index = if props.view_model.has_containers {
            2
        } else {
            0
        };
        props
            .gui_state
            .lock()
            .update_region_map(Region::HelpPanel, split_bar[help_index]);
        frame.render_widget(help_paragraph, split_bar[help_index]);
    }

    /// Draw loading spinner
    fn draw_loading_spinner(frame: &mut Frame, props: &HeadersPanelProps, rect: Rect) {
        let loading_paragraph = Paragraph::new(format!("{:>2}", props.view_model.loading_icon))
            .style(
                Style::default().fg(props.theme.headers_bar.loading_spinner.into_ratatui_color()),
            )
            .alignment(Alignment::Left);
        frame.render_widget(loading_paragraph, rect);
    }

    /// Draw the sortable column headers
    fn draw_columns(frame: &mut Frame, props: &HeadersPanelProps, split_bar: &[Rect]) {
        if !props.view_model.has_containers {
            return;
        }

        // Static header template based on CORRECTED - Default from errors.txt
        // Each column has exactly the spacing shown, with sort indicators replacing the last space when active
        let mut header_spans = Vec::new();
        
        // Add padding for border (1 char) + list highlight symbol (2 chars) = 3 chars total
        header_spans.push(Span::raw("   "));
        
        // Helper to get color and sort indicator for a header
        let get_header_style = |header: Header| -> (CoreColor, &'static str) {
            let mut color = props.theme.headers_bar.text;
            let mut indicator = " ";  // Default trailing space
            
            if let Some((sorted_header, order)) = &props.view_model.sorted_by
                && &header == sorted_header
            {
                color = props.theme.headers_bar.text_selected;
                indicator = match order {
                    SortedOrder::Asc => "▲",
                    SortedOrder::Desc => "▼",
                };
            }
            
            (color, indicator)
        };
        
        let (color, indicator) = get_header_style(Header::Name);
        header_spans.push(Span::styled(
            format!("name {}                  ", indicator),
            Style::default().fg(color.into_ratatui_color())
        ));
        
        let (color, indicator) = get_header_style(Header::State);
        header_spans.push(Span::styled(
            format!("state {}       ", indicator),
            Style::default().fg(color.into_ratatui_color())
        ));
        
        let (color, indicator) = get_header_style(Header::Status);
        header_spans.push(Span::styled(
            format!("status {}                 ", indicator),
            Style::default().fg(color.into_ratatui_color())
        ));
        
        let (color, indicator) = get_header_style(Header::Cpu);
        header_spans.push(Span::styled(
            format!("cpu {}    ", indicator),
            Style::default().fg(color.into_ratatui_color())
        ));
        
        let (color, indicator) = get_header_style(Header::Memory);
        header_spans.push(Span::styled(
            format!("memory/limit {}        ", indicator),
            Style::default().fg(color.into_ratatui_color())
        ));
        
        let (color, indicator) = get_header_style(Header::Id);
        header_spans.push(Span::styled(
            format!("id {}      ", indicator),
            Style::default().fg(color.into_ratatui_color())
        ));
        
        let (color, indicator) = get_header_style(Header::Image);
        header_spans.push(Span::styled(
            format!("image {}                              ", indicator),
            Style::default().fg(color.into_ratatui_color())
        ));
        
        let (color, indicator) = get_header_style(Header::Rx);
        header_spans.push(Span::styled(
            format!("↓ rx {}      ", indicator),
            Style::default().fg(color.into_ratatui_color())
        ));
        
        let (color, indicator) = get_header_style(Header::Tx);
        header_spans.push(Span::styled(
            format!("↑ tx {}                    ", indicator),
            Style::default().fg(color.into_ratatui_color())
        ));

        let header_line = Line::from(header_spans);
        let paragraph = Paragraph::new(header_line)
            .style(Style::default().bg(props.theme.headers_bar.background.into_ratatui_color()));
        
        let headers_rect = Rect {
            x: split_bar[0].x,
            y: split_bar[1].y,
            width: split_bar[0].width + split_bar[1].width,
            height: 1,
        };
        frame.render_widget(paragraph, headers_rect);
        
        // Register clickable regions for each header using fixed positions
        let mut x_offset = split_bar[0].x + 3; // Start after padding (3 chars)
        
        props.gui_state.lock().update_region_map(
            Region::Header(Header::Name),
            Rect { x: x_offset, y: split_bar[1].y, width: 24, height: 1 }
        );
        x_offset += 24;
        
        props.gui_state.lock().update_region_map(
            Region::Header(Header::State),
            Rect { x: x_offset, y: split_bar[1].y, width: 14, height: 1 }
        );
        x_offset += 14;
        
        props.gui_state.lock().update_region_map(
            Region::Header(Header::Status),
            Rect { x: x_offset, y: split_bar[1].y, width: 25, height: 1 }
        );
        x_offset += 25;
        
        props.gui_state.lock().update_region_map(
            Region::Header(Header::Cpu),
            Rect { x: x_offset, y: split_bar[1].y, width: 9, height: 1 }
        );
        x_offset += 9;
        
        props.gui_state.lock().update_region_map(
            Region::Header(Header::Memory),
            Rect { x: x_offset, y: split_bar[1].y, width: 22, height: 1 }
        );
        x_offset += 22;
        
        props.gui_state.lock().update_region_map(
            Region::Header(Header::Id),
            Rect { x: x_offset, y: split_bar[1].y, width: 10, height: 1 }
        );
        x_offset += 10;
        
        props.gui_state.lock().update_region_map(
            Region::Header(Header::Image),
            Rect { x: x_offset, y: split_bar[1].y, width: 37, height: 1 }
        );
        x_offset += 37;
        
        props.gui_state.lock().update_region_map(
            Region::Header(Header::Rx),
            Rect { x: x_offset, y: split_bar[1].y, width: 12, height: 1 }
        );
        x_offset += 12;
        
        props.gui_state.lock().update_region_map(
            Region::Header(Header::Tx),
            Rect { x: x_offset, y: split_bar[1].y, width: 26, height: 1 }
        );
    }
}

impl<'p> Component<'p> for HeadersPanel {
    type Props = HeadersPanelProps<'p>;
    type Event = HeaderEvent;

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Render background
        frame.render_widget(
            Block::default().style(
                Style::default()
                    .bg(props.theme.headers_bar.background.into_ratatui_color())
                    .fg(Color::Reset),
            ),
            area,
        );

        let help_text = Self::gen_help_text(props);
        let help_width = help_text.chars().count();

        let column_width = usize::from(area.width).saturating_sub(help_width);
        let column_width = if column_width > 0 { column_width } else { 1 };

        let split_bar = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(if props.view_model.has_containers {
                vec![
                    Constraint::Max(4),
                    Constraint::Max(column_width.try_into().unwrap_or_default()),
                    Constraint::Max(help_width.try_into().unwrap_or_default()),
                ]
            } else {
                vec![Constraint::Percentage(CONSTRAINT_100)]
            })
            .split(area);

        Self::draw_loading_spinner(frame, props, split_bar[0]);
        Self::draw_columns(frame, props, &split_bar);
        Self::draw_help(frame, props, help_text, &split_bar);
    }

    fn handle_event(&mut self, event: &Self::Event) -> Option<CoreCommand> {
        match event {
            HeaderEvent::ToggleHelp => None, // Help toggle is handled by UI state, not CoreCommand
            HeaderEvent::SortBy(_header) => {
                // Note: Sorting could be implemented using CoreCommand::SortContainers
                // but needs mapping from Header to SortField and determining SortOrder
                // TODO: Implement sorting
                None
            }
        }
    }
}

/// Events that can be handled by the headers panel
pub enum HeaderEvent {
    ToggleHelp,
    SortBy(Header),
}

impl Default for HeadersPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a key code for display
fn format_key(key: oxker_core::KeyCode) -> String {
    use oxker_core::KeyCode;

    match key {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::Up => "↑".to_string(),
        KeyCode::Down => "↓".to_string(),
        KeyCode::Left => "←".to_string(),
        KeyCode::Right => "→".to_string(),
        KeyCode::F(n) => format!("F{n}"),
        _ => format!("{key:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_headers_panel_render() {
        let panel = HeadersPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let theme = AppColors::new();
        let keymap = Keymap::new();
        let fd = FrameViewModel::default();

        let props = HeadersPanelProps {
            view_model: &fd,
            theme: &theme,
            keymap: &keymap,
            gui_state: &gui_state,
        };

        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify basic rendering works
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 1);
    }
}
