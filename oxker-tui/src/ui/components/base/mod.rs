//! Basic UI components that serve as building blocks for more complex widgets

use crate::ui::components::Component;
use oxker_core::AppColors;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block as RatatuiBlock, Borders, Paragraph},
};

/// A styled block component with optional title and borders
pub struct Block {
    borders: Borders,
}

pub struct BlockProps<'a> {
    pub title: Option<&'a str>,
    pub selected: bool,
    pub theme: &'a AppColors,
    pub borders: Borders,
}

impl Block {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            borders: Borders::ALL,
        }
    }

    #[must_use]
    pub const fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }
}

impl<'p> Component<'p> for Block {
    type Props = BlockProps<'p>;
    type Event = ();

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        let border_color = if props.selected {
            props.theme.borders.selected
        } else {
            props.theme.borders.unselected
        };

        let mut block = RatatuiBlock::default()
            .borders(props.borders)
            .border_style(Style::default().fg(border_color));

        if let Some(title) = props.title {
            let title_style = if props.selected {
                Style::default()
                    .fg(props.theme.filter.highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(props.theme.filter.text)
            };

            block = block.title(Line::from(vec![
                Span::raw(" "),
                Span::styled(title, title_style),
                Span::raw(" "),
            ]));
        }

        frame.render_widget(block, area);
    }
}

/// A text component for rendering styled text
pub struct Text {
    alignment: Alignment,
    wrap: bool,
}

pub struct TextProps<'a> {
    pub content: &'a str,
    pub style: Style,
    pub alignment: Alignment,
}

impl Text {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alignment: Alignment::Left,
            wrap: true,
        }
    }

    #[must_use]
    pub const fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    #[must_use]
    pub const fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
}

impl<'p> Component<'p> for Text {
    type Props = TextProps<'p>;
    type Event = ();

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        let paragraph = Paragraph::new(props.content)
            .style(props.style)
            .alignment(props.alignment)
            .wrap(ratatui::widgets::Wrap { trim: self.wrap });

        frame.render_widget(paragraph, area);
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Text {
    fn default() -> Self {
        Self::new()
    }
}
