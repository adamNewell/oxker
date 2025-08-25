use std::{rc::Rc, sync::Arc};

use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Paragraph},
};

use super::{CONSTRAINT_100, MARGIN};
use crate::ui::{FrameViewModel, GuiState, Status, gui_state::Region};
use oxker_core::{AppColors, Header, Keymap, SortedOrder};
/// Generate a header paragraph with it's width
fn gen_header<'a>(
    colors: AppColors,
    fd: &FrameViewModel,
    header: Header,
    width: usize,
) -> (Paragraph<'a>, u16) {
    let block = gen_header_block(colors, fd, header);

    let text = format!(
        "{x:<width$}{MARGIN}",
        x = format!("{header}{ic}", ic = block.1),
    );
    let count = u16::try_from(text.chars().count()).unwrap_or_default();
    let status = Paragraph::new(text)
        .style(gen_style(None, block.0))
        .alignment(Alignment::Left);
    (status, count)
}

// Generate a block for the header, if the header is currently being used to sort a column, then highlight it white
fn gen_header_block<'a>(
    colors: AppColors,
    fd: &FrameViewModel,
    header: Header,
) -> (Color, &'a str) {
    let mut color = colors.headers_bar.text;
    let mut suffix = "";
    if let Some((a, b)) = &fd.sorted_by {
        if &header == a {
            match b {
                SortedOrder::Asc => suffix = " ▲",
                SortedOrder::Desc => suffix = " ▼",
            }
            color = colors.headers_bar.text_selected;
        }
    }

    (color, suffix)
}

fn gen_style(bg: Option<Color>, fg: Color) -> Style {
    bg.map_or_else(
        || Style::default().fg(fg),
        |bg| Style::default().bg(bg).fg(fg),
    )
}

/// Generate the text to display on the show help section, as can change with a custom keymap
fn gen_help_text(fd: &FrameViewModel, keymap: &Keymap) -> String {
    let suffix = if fd.status.contains(&Status::Help) {
        "exit"
    } else {
        "show"
    };

    if keymap.toggle_help == Keymap::new().toggle_help {
        format!("( h ) {suffix} help{MARGIN}")
    } else if let Some(secondary) = keymap.toggle_help.1 {
        format!(
            " ( {} | {secondary} ) {suffix} help{MARGIN}",
            keymap.toggle_help.0
        )
    } else {
        format!(" ( {} ) {suffix} help{MARGIN}", keymap.toggle_help.0)
    }
}

/// Draw the show/hide help section
fn draw_help(
    colors: AppColors,
    f: &mut Frame,
    fd: &FrameViewModel,
    help_text: String,
    gui_state: &Arc<Mutex<GuiState>>,
    split_bar: &Rc<[Rect]>,
) {
    let help_text_color = if fd.status.contains(&Status::Help) {
        colors.headers_bar.text
    } else {
        colors.headers_bar.text_selected
    };

    let help_paragraph = Paragraph::new(help_text)
        .style(gen_style(None, help_text_color))
        .alignment(Alignment::Right);

    // If no containers, don't display the headers, could maybe do this first?
    let help_index = if fd.has_containers { 2 } else { 0 };
    gui_state
        .lock()
        .update_region_map(Region::HelpPanel, split_bar[help_index]);
    f.render_widget(help_paragraph, split_bar[help_index]);
}

// Draw loading icon, or not, and a prefix with a single space
fn draw_loading_spinner(colors: AppColors, f: &mut Frame, fd: &FrameViewModel, rect: Rect) {
    let loading_paragraph = Paragraph::new(format!("{:>2}", fd.loading_icon))
        .style(gen_style(None, colors.headers_bar.loading_spinner))
        .alignment(Alignment::Left);
    f.render_widget(loading_paragraph, rect);
}

/// Draw the sortable column headers (name/state/status etc)
fn draw_columns(
    colors: AppColors,
    f: &mut Frame,
    fd: &FrameViewModel,
    gui_state: &Arc<Mutex<GuiState>>,
    split_bar: &Rc<[Rect]>,
) {
    if fd.has_containers {
        let header_section_width = split_bar[1].width;

        let mut counter = 0;

        // Meta data to iterate over to create blocks with correct widths
        let header_meta = [
            (Header::Name, fd.columns.name.1),
            (Header::State, fd.columns.state.1),
            (Header::Status, fd.columns.status.1),
            (Header::Cpu, fd.columns.cpu.1),
            (Header::Memory, fd.columns.mem.1 + fd.columns.mem.2 + 3),
            (Header::Id, fd.columns.id.1),
            (Header::Image, fd.columns.image.1),
            (Header::Rx, fd.columns.net_rx.1),
            (Header::Tx, fd.columns.net_tx.1),
        ];

        // Only show a header if the header cumulative header width is less than the header section width
        let header_data = header_meta
            .into_iter()
            .filter_map(|(header, width)| {
                let header_block = gen_header(colors, fd, header, usize::from(width));
                counter += header_block.1;
                if counter <= header_section_width {
                    Some((header_block.0, header, Constraint::Max(header_block.1)))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let headers_section = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(header_data.iter().map(|i| i.2))
            .split(split_bar[1]);

        for (index, (paragraph, header, _)) in header_data.into_iter().enumerate() {
            let rect = headers_section[index];
            gui_state
                .lock()
                .update_region_map(Region::Header(header), rect);
            f.render_widget(paragraph, rect);
        }
    }
}

// Draw heading bar at top of program, always visible
pub fn draw(
    area: Rect,
    colors: AppColors,
    f: &mut Frame,
    fd: &FrameViewModel,
    gui_state: &Arc<Mutex<GuiState>>,
    keymap: &Keymap,
) {
    let gen_style = |bg: Option<Color>, fg: Color| {
        bg.map_or_else(
            || Style::default().fg(fg),
            |bg| Style::default().bg(bg).fg(fg),
        )
    };

    f.render_widget(
        Block::default().style(gen_style(Some(colors.headers_bar.background), Color::Reset)),
        area,
    );

    let help_text = gen_help_text(fd, keymap);
    let help_width = help_text.chars().count();

    let column_width = usize::from(area.width).saturating_sub(help_width);
    let column_width = if column_width > 0 { column_width } else { 1 };

    let split_bar = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if fd.has_containers {
            vec![
                Constraint::Max(4),
                Constraint::Max(column_width.try_into().unwrap_or_default()),
                Constraint::Max(help_width.try_into().unwrap_or_default()),
            ]
        } else {
            CONSTRAINT_100.to_vec()
        })
        .split(area);

    draw_loading_spinner(colors, f, fd, split_bar[0]);
    draw_columns(colors, f, fd, gui_state, &split_bar);
    draw_help(colors, f, fd, help_text, gui_state, &split_bar);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::ops::RangeInclusive;

    use crossterm::event::KeyCode;
    use insta::assert_snapshot;
    use ratatui::style::Color;
    use uuid::Uuid;

    use crate::ui::FrameViewModel;
    use oxker_core::{AppColors, Header, Keymap, SortedOrder};
    // Placeholder test
    #[test]
    fn test_placeholder() {
        // Tests were truncated during import migration
    }
}
