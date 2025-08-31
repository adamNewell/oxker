//! Panel components that represent major UI sections

pub mod charts;
pub mod commands;
pub mod confirmation_modal;
pub mod containers;
pub mod debug;
pub mod delete_confirm;
pub mod error;
pub mod filter;
pub mod headers;
pub mod help;
pub mod info;
pub mod logs;
pub mod ports;

// Re-export panel components
pub use charts::ChartsPanel;
pub use commands::CommandsPanel;
pub use confirmation_modal::{ConfirmationButton, ConfirmationModal};
pub use containers::ContainersPanel;
pub use debug::DebugPanel;
pub use delete_confirm::DeleteConfirmPanel;
pub use error::ErrorPanel;
pub use filter::FilterPanel;
pub use headers::HeadersPanel;
pub use help::HelpPanel;
pub use logs::LogsPanel;
pub use ports::PortsPanel;
