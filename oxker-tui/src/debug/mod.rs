//! Debug module for tracking and displaying system events

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

pub mod panel;
pub mod writer;

/// Maximum number of events to keep in the buffer
const MAX_EVENTS: usize = 1000;

/// Categories of events for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventCategory {
    Render,
    Container,
    Logs,
    State,
    GUI,
    Selection,
    System,
    Error,
}

impl EventCategory {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Render => "RENDER",
            Self::Container => "CONTAINER",
            Self::Logs => "LOGS",
            Self::State => "STATE",
            Self::GUI => "GUI",
            Self::Selection => "SELECT",
            Self::System => "SYSTEM",
            Self::Error => "ERROR",
        }
    }

    #[must_use]
    pub const fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Render => Color::Cyan,
            Self::Container => Color::Blue,
            Self::Logs => Color::Green,
            Self::State => Color::Magenta,
            Self::GUI => Color::LightBlue,
            Self::Selection => Color::Yellow,
            Self::System => Color::Gray,
            Self::Error => Color::Red,
        }
    }
}

/// A single debug event
#[derive(Debug, Clone)]
pub struct DebugEvent {
    pub timestamp: Instant,
    pub category: EventCategory,
    pub component: String,
    pub action: String,
    pub details: String,
    pub sequence_id: Option<u64>,
}

impl DebugEvent {
    pub fn new(
        category: EventCategory,
        component: impl Into<String>,
        action: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: Instant::now(),
            category,
            component: component.into(),
            action: action.into(),
            details: details.into(),
            sequence_id: None,
        }
    }

    /// Format the event for display
    #[must_use]
    pub fn format(&self, start_time: Instant) -> String {
        let elapsed = self.timestamp.duration_since(start_time);
        let millis = elapsed.as_millis();
        let seconds = millis / 1000;
        let ms = millis % 1000;

        format!(
            "{:02}:{:02}.{:03} {:8} {:12} {:10} {}",
            seconds / 60,
            seconds % 60,
            ms,
            self.category.as_str(),
            self.component,
            self.action,
            self.details
        )
    }
}

/// Filter configuration for debug events
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct EventFilters {
    pub render: bool,
    pub container: bool,
    pub logs: bool,
    pub state: bool,
    pub gui: bool,
    pub selection: bool,
    pub system: bool,
    pub error: bool,
    pub text_filter: String,
}

impl Default for EventFilters {
    fn default() -> Self {
        Self {
            render: true,
            container: true,
            logs: true,
            state: true,
            gui: true,
            selection: true,
            system: false, // Off by default to reduce noise
            error: true,
            text_filter: String::new(),
        }
    }
}

impl EventFilters {
    #[must_use]
    pub const fn is_enabled(&self, category: EventCategory) -> bool {
        match category {
            EventCategory::Render => self.render,
            EventCategory::Container => self.container,
            EventCategory::Logs => self.logs,
            EventCategory::State => self.state,
            EventCategory::GUI => self.gui,
            EventCategory::Selection => self.selection,
            EventCategory::System => self.system,
            EventCategory::Error => self.error,
        }
    }

    pub const fn toggle(&mut self, category: EventCategory) {
        match category {
            EventCategory::Render => self.render = !self.render,
            EventCategory::Container => self.container = !self.container,
            EventCategory::Logs => self.logs = !self.logs,
            EventCategory::State => self.state = !self.state,
            EventCategory::GUI => self.gui = !self.gui,
            EventCategory::Selection => self.selection = !self.selection,
            EventCategory::System => self.system = !self.system,
            EventCategory::Error => self.error = !self.error,
        }
    }
}

/// Debug event tracker
#[derive(Debug)]
pub struct DebugTracker {
    events: Arc<Mutex<VecDeque<DebugEvent>>>,
    filters: Arc<Mutex<EventFilters>>,
    paused: Arc<Mutex<bool>>,
    start_time: Instant,
    sequence_counter: Arc<Mutex<u64>>,
}

impl DebugTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_EVENTS))),
            filters: Arc::new(Mutex::new(EventFilters::default())),
            paused: Arc::new(Mutex::new(false)),
            start_time: Instant::now(),
            sequence_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Log a debug event
    pub fn log_event(&self, mut event: DebugEvent) {
        if *self.paused.lock() {
            return;
        }

        // Assign sequence ID
        let mut counter = self.sequence_counter.lock();
        *counter += 1;
        event.sequence_id = Some(*counter);
        drop(counter);

        let mut events = self.events.lock();

        // Maintain max buffer size
        if events.len() >= MAX_EVENTS {
            events.pop_front();
        }

        events.push_back(event);
    }

    /// Log a simple event
    pub fn log(
        &self,
        category: EventCategory,
        component: impl Into<String>,
        action: impl Into<String>,
        details: impl Into<String>,
    ) {
        self.log_event(DebugEvent::new(category, component, action, details));
    }

    /// Get filtered events
    #[must_use]
    pub fn get_filtered_events(&self) -> Vec<DebugEvent> {
        let events = self.events.lock();
        let filters = self.filters.lock();

        events
            .iter()
            .filter(|e| {
                // Check category filter
                if !filters.is_enabled(e.category) {
                    return false;
                }

                // Check text filter
                if !filters.text_filter.is_empty() {
                    let search = filters.text_filter.to_lowercase();
                    let matches = e.component.to_lowercase().contains(&search)
                        || e.action.to_lowercase().contains(&search)
                        || e.details.to_lowercase().contains(&search);
                    if !matches {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    /// Clear all events
    pub fn clear(&self) {
        self.events.lock().clear();
        *self.sequence_counter.lock() = 0;
    }

    /// Toggle pause state
    pub fn toggle_pause(&self) {
        let mut paused = self.paused.lock();
        *paused = !*paused;
    }

    /// Check if paused
    #[must_use]
    pub fn is_paused(&self) -> bool {
        *self.paused.lock()
    }

    /// Get filters for modification
    #[must_use]
    pub fn filters(&self) -> Arc<Mutex<EventFilters>> {
        Arc::clone(&self.filters)
    }

    /// Get start time for formatting
    #[must_use]
    pub const fn start_time(&self) -> Instant {
        self.start_time
    }
}

impl Default for DebugTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Global debug tracker instance
static TRACKER: std::sync::LazyLock<DebugTracker> = std::sync::LazyLock::new(DebugTracker::new);

/// Get the global debug tracker
#[must_use]
pub fn tracker() -> &'static DebugTracker {
    &TRACKER
}

/// Convenience macro for logging debug events
#[macro_export]
macro_rules! debug_event {
    ($category:expr, $component:expr, $action:expr, $details:expr) => {
        $crate::debug::tracker().log($category, $component, $action, $details)
    };
    ($category:expr, $component:expr, $action:expr) => {
        $crate::debug::tracker().log($category, $component, $action, "")
    };
}
