//! Temporary stub types to replace UI dependencies
//! TODO: These should be removed once proper message passing is implemented

use std::sync::atomic::{AtomicBool, Ordering};

/// Stub for UI Status enum
#[derive(Debug, Clone, Copy)]
pub enum Status {
    Loading,
    Success,
    Error,
}

/// Stub for UI GuiState
pub struct GuiState {
    _dummy: bool,
}

impl GuiState {
    pub fn status_push(&self, _status: Status) {
        // No-op for now
    }
}

/// Stub for UI Rerender
pub struct Rerender {
    should_redraw: AtomicBool,
}

impl Rerender {
    pub fn new() -> Self {
        Self {
            should_redraw: AtomicBool::new(false),
        }
    }

    pub fn set_redraw(&self, value: bool) {
        self.should_redraw.store(value, Ordering::Relaxed);
    }
}

impl Default for Rerender {
    fn default() -> Self {
        Self::new()
    }
}
