//! Terminal state protection to prevent corruption during Docker command execution

use crossterm::{
    cursor, execute,
    style::ResetColor,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

/// Guard to save and restore terminal state
/// Implements Drop for automatic cleanup even on panic
pub struct TerminalStateGuard {
    // We don't actually need to store the terminal state as crossterm handles it
    _marker: std::marker::PhantomData<()>,
    #[cfg(test)]
    skip_terminal_ops: bool,
}

impl TerminalStateGuard {
    /// Create a new terminal state guard that saves the current terminal state
    ///
    /// # Errors
    /// Returns an error if stdout cannot be flushed
    pub fn new() -> io::Result<Self> {
        // Flush any pending output before switching modes
        io::stdout().flush()?;

        // Save terminal state is implicit with crossterm's alternate screen
        tracing::debug!("Terminal state saved");

        Ok(Self {
            _marker: std::marker::PhantomData,
            #[cfg(test)]
            skip_terminal_ops: false,
        })
    }

    #[cfg(test)]
    /// Create a test guard that skips terminal operations
    #[must_use]
    pub const fn new_for_test() -> Self {
        Self {
            _marker: std::marker::PhantomData,
            skip_terminal_ops: true,
        }
    }

    /// Temporarily leave raw mode for command execution
    ///
    /// # Errors
    /// Returns an error if terminal operations fail
    pub fn suspend_raw_mode() -> io::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, cursor::Show, ResetColor)?;
        io::stdout().flush()?;
        tracing::debug!("Raw mode suspended for command execution");
        Ok(())
    }

    /// Re-enter raw mode after command execution
    ///
    /// # Errors
    /// Returns an error if terminal operations fail
    pub fn resume_raw_mode() -> io::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        io::stdout().flush()?;
        tracing::debug!("Raw mode resumed after command execution");
        Ok(())
    }

    /// Execute a function with terminal protection
    ///
    /// # Errors
    /// Returns an error if terminal operations fail
    pub async fn with_protected_terminal<F, Fut, T>(f: F) -> io::Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        // Suspend raw mode
        Self::suspend_raw_mode()?;

        // Execute the function
        let result = f().await;

        // Resume raw mode
        Self::resume_raw_mode()?;

        Ok(result)
    }
}

impl Drop for TerminalStateGuard {
    fn drop(&mut self) {
        #[cfg(test)]
        if self.skip_terminal_ops {
            return;
        }

        // Always try to restore terminal state on drop
        // This ensures cleanup even on panic
        if let Err(e) = Self::resume_raw_mode() {
            // Log but don't panic in Drop
            eprintln!("Failed to restore terminal state: {e}");
        }
        tracing::debug!("Terminal state restored via Drop");
    }
}

impl Default for TerminalStateGuard {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            eprintln!("Failed to create TerminalStateGuard: {e}");
            Self {
                _marker: std::marker::PhantomData,
                #[cfg(test)]
                skip_terminal_ops: false,
            }
        })
    }
}

/// Panic handler to restore terminal on unexpected errors
pub fn install_panic_handler() {
    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        // Try to restore terminal state
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show, ResetColor);
        let _ = io::stdout().flush();

        // Call the original panic handler
        original_hook(panic_info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_guard_creation() {
        // Use test-specific constructor that skips terminal operations
        let _guard = TerminalStateGuard::new_for_test();
        // Test passes if no panic occurs
    }

    #[test]
    fn test_panic_handler_installation() {
        // Just verify the panic handler can be installed without error
        // We can't actually test terminal restoration in unit tests
        install_panic_handler();
        // Test passes if no panic occurs
    }
}
