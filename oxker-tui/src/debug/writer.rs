//! Debug writer to file for tracking events

use super::{EventCategory, tracker};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// Write debug events to a file
pub fn init_debug_file() {
    let path = Path::new("/tmp/oxker_debug.log");

    // Clear the file on startup
    let _ = std::fs::write(path, "=== Oxker Debug Log ===\n");
}

/// Write a debug event to file
pub fn write_debug(category: EventCategory, component: &str, action: &str, details: &str) {
    let path = Path::new("/tmp/oxker_debug.log");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();

        let millis = timestamp.as_millis();
        let seconds = millis / 1000;
        let ms = millis % 1000;

        let line = format!(
            "[{:06}.{:03}] {:8} | {:12} | {:10} | {}\n",
            seconds % 100_000,
            ms,
            category.as_str(),
            component,
            action,
            details
        );

        let _ = file.write_all(line.as_bytes());
    }

    // Also log to tracker
    tracker().log(category, component, action, details);
}

/// Convenience macro for debug writing
#[macro_export]
macro_rules! debug_write {
    ($category:expr, $component:expr, $action:expr, $details:expr) => {
        $crate::debug::writer::write_debug($category, $component, $action, $details)
    };
    ($category:expr, $component:expr, $action:expr) => {
        $crate::debug::writer::write_debug($category, $component, $action, "")
    };
}
