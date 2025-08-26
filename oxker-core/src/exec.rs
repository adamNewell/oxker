// This module provides exec functionality with conditional compilation
// When exec_refactor feature is enabled, uses the new refactored implementation
// Otherwise, uses the original implementation

#[cfg(feature = "exec_refactor")]
pub use crate::exec_refactored::{ExecMode, tty_readable};

#[cfg(not(feature = "exec_refactor"))]
pub use crate::exec_original::{ExecMode, TerminalSize, tty_readable};
