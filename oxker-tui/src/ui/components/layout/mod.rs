//! Layout components for organizing UI structure

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout as RatatuiLayout, Rect},
};

/// A layout component that splits an area into multiple sections
pub struct SplitLayout {
    direction: Direction,
    constraints: Vec<Constraint>,
    margin: u16,
}

impl SplitLayout {
    #[must_use]
    pub const fn new(direction: Direction) -> Self {
        Self {
            direction,
            constraints: vec![],
            margin: 0,
        }
    }

    #[must_use]
    pub fn constraints(mut self, constraints: Vec<Constraint>) -> Self {
        self.constraints = constraints;
        self
    }

    #[must_use]
    pub const fn margin(mut self, margin: u16) -> Self {
        self.margin = margin;
        self
    }

    /// Split the given area according to the layout configuration
    #[must_use]
    pub fn split(&self, area: Rect) -> Vec<Rect> {
        RatatuiLayout::default()
            .direction(self.direction)
            .margin(self.margin)
            .constraints(&self.constraints)
            .split(area)
            .to_vec()
    }
}

/// Position for modal overlays
#[derive(Clone, Copy, Debug)]
pub enum Position {
    Center,
    Top,
    Bottom,
}

/// A modal overlay component for dialogs and popups
pub struct ModalOverlay {
    width_percent: u16,
    height_percent: u16,
    position: Position,
}

impl ModalOverlay {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            width_percent: 50,
            height_percent: 50,
            position: Position::Center,
        }
    }

    #[must_use]
    pub fn width_percent(mut self, percent: u16) -> Self {
        self.width_percent = percent.min(100);
        self
    }

    #[must_use]
    pub fn height_percent(mut self, percent: u16) -> Self {
        self.height_percent = percent.min(100);
        self
    }

    #[must_use]
    pub const fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Calculate the modal area within the parent area
    #[must_use]
    pub fn area(&self, parent: Rect) -> Rect {
        let width = (parent.width * self.width_percent / 100).max(1);
        let height = (parent.height * self.height_percent / 100).max(1);

        let x = parent.x + (parent.width.saturating_sub(width)) / 2;

        let y = match self.position {
            Position::Top => parent.y + 2,
            Position::Center => parent.y + (parent.height.saturating_sub(height)) / 2,
            Position::Bottom => parent.y + parent.height.saturating_sub(height + 2),
        };

        Rect::new(x, y, width, height)
    }

    /// Render a dimmed background behind the modal
    pub fn render_background(&self, area: Rect, frame: &mut Frame) {
        use ratatui::widgets::{Block, Clear};

        // Clear the entire area first
        frame.render_widget(Clear, area);

        // Render a semi-transparent background
        let dim_block = Block::default()
            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black));

        frame.render_widget(dim_block, area);
    }
}

impl Default for ModalOverlay {
    fn default() -> Self {
        Self::new()
    }
}
