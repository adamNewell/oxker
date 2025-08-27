//! Help panel component for displaying keyboard shortcuts and app info

use crate::ui::components::{
    Component,
    constants::{DESCRIPTION, NAME_TEXT, REPO, VERSION},
};
use jiff::tz::TimeZone;
use oxker_core::{AppColors, Keymap};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

/// Help panel component that displays keyboard shortcuts and app information
pub struct HelpPanel {}

pub struct HelpPanelProps {
    pub theme: AppColors,
    pub keymap: Keymap,
    pub show_timestamp: bool,
    pub timezone: Option<TimeZone>,
}

impl HelpPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Generate an empty line
    fn empty_line() -> Line<'static> {
        Line::from(String::new())
    }

    /// Generate a styled span
    fn span(text: String, color: Color) -> Span<'static> {
        Span::styled(text, Style::default().fg(color))
    }

    /// Generate normal text span
    fn text_span(text: &str, theme: &AppColors) -> Span<'static> {
        Self::span(text.to_string(), theme.popup_help.text)
    }

    /// Generate highlighted text span
    fn highlighted_span(text: &str, theme: &AppColors) -> Span<'static> {
        Self::span(text.to_string(), theme.popup_help.text_highlight)
    }

    /// Generate button item span
    fn button_span(key: &str, theme: &AppColors) -> Span<'static> {
        Self::highlighted_span(&format!(" ( {key} ) "), theme)
    }

    /// Generate the app name section
    fn generate_name_section(theme: &AppColors) -> Vec<Line<'static>> {
        let mut lines = NAME_TEXT
            .lines()
            .map(|line| Line::from(Self::highlighted_span(line, theme)))
            .collect::<Vec<_>>();
        lines.insert(0, Self::empty_line());
        lines
    }

    /// Generate the description section
    fn generate_description_section(theme: &AppColors) -> Vec<Line<'static>> {
        vec![
            Self::empty_line(),
            Line::from(Self::text_span(DESCRIPTION, theme)),
        ]
    }

    /// Generate the keymap section
    fn generate_keymap_section(props: &HelpPanelProps) -> Vec<Line<'static>> {
        let theme = &props.theme;

        let or = || Self::text_span(" or ", theme);
        let space = || Self::text_span(" ", theme);
        let desc = |text: &str| Self::text_span(text, theme);
        let button = |key: &str| Self::button_span(key, theme);

        vec![
            Line::from(vec![
                space(),
                button("tab"),
                or(),
                button("shift+tab"),
                desc("change panels"),
            ]),
            Line::from(vec![
                space(),
                button("↑ ↓"),
                or(),
                button("j k"),
                or(),
                button("PgUp PgDown"),
                or(),
                button("Home End"),
                desc("scroll vertically"),
            ]),
            Line::from(vec![
                space(),
                button("← →"),
                desc("horizontal scroll across logs"),
            ]),
            Line::from(vec![
                space(),
                button("ctrl"),
                desc("increase scroll speed, used in conjuction scroll keys"),
            ]),
            Line::from(vec![
                space(),
                button("enter"),
                desc("send docker container command"),
            ]),
            Line::from(vec![
                space(),
                button("e"),
                desc("exec into a container"),
                #[cfg(target_os = "windows")]
                desc(" - not available on Windows"),
            ]),
            Line::from(vec![
                space(),
                button("f"),
                desc("force clear the screen & redraw the gui"),
            ]),
            Line::from(vec![
                space(),
                button("h"),
                desc("toggle this help information - or click heading"),
            ]),
            Line::from(vec![space(), button("s"), desc("save logs to file")]),
            Line::from(vec![
                space(),
                button("m"),
                desc("toggle mouse capture - if disabled, text on screen can be selected & copied"),
            ]),
            Line::from(vec![
                space(),
                button("F1"),
                or(),
                button("/"),
                desc("enter filter mode"),
            ]),
            Line::from(vec![space(), button("0"), desc("stop sort")]),
            Line::from(vec![
                space(),
                button("1 - 9"),
                desc("sort by header - or click header"),
            ]),
            Line::from(vec![
                space(),
                button("- ="),
                desc("change log section height"),
            ]),
            Line::from(vec![
                space(),
                button("\\"),
                desc("toggle log section visibility"),
            ]),
            Line::from(vec![space(), button("esc"), desc("close dialog")]),
            Line::from(vec![space(), button("q"), desc("quit at any time")]),
        ]
    }

    /// Generate info section with version and repository
    fn generate_info_section(props: &HelpPanelProps) -> Vec<Line<'static>> {
        let theme = &props.theme;

        vec![
            Self::empty_line(),
            Line::from(vec![Self::text_span(
                "currently an early work in progress, all and any input appreciated",
                theme,
            )]),
            Line::from(vec![Span::styled(
                REPO,
                Style::default()
                    .fg(theme.popup_help.text_highlight)
                    .add_modifier(Modifier::UNDERLINED),
            )]),
        ]
    }

    /// Generate timezone line
    fn generate_timezone_line(props: &HelpPanelProps) -> Line<'static> {
        let theme = &props.theme;
        let zone = props
            .timezone
            .as_ref()
            .and_then(|tz| tz.iana_name())
            .unwrap_or("Etc/UTC");

        Line::from(vec![
            Self::text_span("logs timezone: ", theme),
            Self::highlighted_span(zone, theme),
        ])
    }
}

impl Component<'_> for HelpPanel {
    type Props = HelpPanelProps;
    type Event = ();

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Generate sections separately for different alignments
        let name_lines = Self::generate_name_section(&props.theme);
        let description_lines = Self::generate_description_section(&props.theme);
        let keymap_lines = Self::generate_keymap_section(props);
        let info_lines = Self::generate_info_section(props);

        // Add timezone display if timestamps are shown
        let timezone_lines = if props.show_timestamp {
            vec![
                Self::empty_line(),
                Self::generate_timezone_line(props),
                Self::empty_line(),
            ]
        } else {
            vec![]
        };

        // Calculate total size needed
        let all_sections = [
            &name_lines,
            &description_lines,
            &timezone_lines,
            &keymap_lines,
            &info_lines,
        ];
        let width = all_sections
            .iter()
            .flat_map(|section| section.iter())
            .map(ratatui::prelude::Line::width)
            .max()
            .unwrap_or(60)
            + 4; // Add padding

        let total_height = name_lines.len()
            + description_lines.len()
            + timezone_lines.len()
            + keymap_lines.len()
            + info_lines.len()
            + 3;

        // Calculate the help panel area
        let help_area = {
            let x = area.x
                + (area
                    .width
                    .saturating_sub(u16::try_from(width).unwrap_or(u16::MAX)))
                    / 2;
            let y = area.y
                + (area
                    .height
                    .saturating_sub(u16::try_from(total_height).unwrap_or(u16::MAX)))
                    / 2;
            Rect::new(
                x,
                y,
                u16::try_from(width).unwrap_or(u16::MAX),
                u16::try_from(total_height).unwrap_or(u16::MAX),
            )
        };

        // Create layout constraints for sections
        let constraints = vec![
            Constraint::Length(u16::try_from(name_lines.len()).unwrap_or(10)),
            Constraint::Length(u16::try_from(description_lines.len()).unwrap_or(10)),
            Constraint::Length(u16::try_from(timezone_lines.len()).unwrap_or(2)),
            Constraint::Length(u16::try_from(keymap_lines.len()).unwrap_or(20)),
            Constraint::Length(u16::try_from(info_lines.len()).unwrap_or(5)),
        ];

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(help_area);

        // Render the clear widget
        frame.render_widget(Clear, help_area);

        // Create block for the whole area
        let title = format!(" {VERSION} ");
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(
                Style::default()
                    .fg(props.theme.popup_help.text)
                    .bg(props.theme.popup_help.background),
            )
            .style(
                Style::default()
                    .bg(props.theme.popup_help.background)
                    .fg(props.theme.popup_help.text),
            );
        frame.render_widget(block, help_area);

        // Render each section with appropriate alignment
        // Name (ASCII art) - centered
        let name_paragraph = Paragraph::new(name_lines)
            .style(
                Style::default()
                    .bg(props.theme.popup_help.background)
                    .fg(props.theme.popup_help.text_highlight),
            )
            .alignment(Alignment::Center);
        frame.render_widget(name_paragraph, sections[0]);

        // Description - centered
        let description_paragraph = Paragraph::new(description_lines)
            .style(
                Style::default()
                    .bg(props.theme.popup_help.background)
                    .fg(props.theme.popup_help.text_highlight),
            )
            .alignment(Alignment::Center);
        frame.render_widget(description_paragraph, sections[1]);

        // Timezone (if shown) - centered
        if !timezone_lines.is_empty() {
            let timezone_paragraph = Paragraph::new(timezone_lines)
                .style(
                    Style::default()
                        .bg(props.theme.popup_help.background)
                        .fg(props.theme.popup_help.text),
                )
                .alignment(Alignment::Center);
            frame.render_widget(timezone_paragraph, sections[2]);
        }

        // Keymap - left aligned
        let keymap_paragraph = Paragraph::new(keymap_lines)
            .style(
                Style::default()
                    .bg(props.theme.popup_help.background)
                    .fg(props.theme.popup_help.text),
            )
            .alignment(Alignment::Left);
        frame.render_widget(keymap_paragraph, sections[3]);

        // Info - centered
        let info_paragraph = Paragraph::new(info_lines)
            .style(
                Style::default()
                    .bg(props.theme.popup_help.background)
                    .fg(props.theme.popup_help.text),
            )
            .alignment(Alignment::Center);
        frame.render_widget(info_paragraph, sections[4]);
    }
}

impl Default for HelpPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_help_panel_render() {
        let help_panel = HelpPanel::new();
        let props = HelpPanelProps {
            theme: AppColors::new(),
            keymap: Keymap::new(),
            show_timestamp: true,
            timezone: None,
        };

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                help_panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify the widget rendered without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 120);
        assert_eq!(buffer.area.height, 40);
    }
}
