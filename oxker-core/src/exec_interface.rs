use crate::{AppError, ContainerId};
use async_trait::async_trait;
use bollard::Docker;
use std::sync::Arc;

/// Represents the terminal dimensions for exec sessions
#[derive(Debug, Clone, Copy)]
pub struct TerminalDimensions {
    pub width: u16,
    pub height: u16,
}

/// Events emitted by the exec functionality
#[derive(Debug)]
pub enum ExecEvent {
    /// Data received from the container's stdout/stderr
    OutputData(Vec<u8>),
    /// Session started successfully
    SessionStarted,
    /// Session ended
    SessionEnded,
    /// Error occurred during session
    Error(String),
}

/// Interface for UI interactions during exec sessions
#[async_trait]
pub trait ExecInterface: Send + Sync {
    /// Called when output data is received from the container
    async fn handle_output(&self, data: Vec<u8>) -> Result<(), AppError>;

    /// Get input data from the user (returns None if no data available)
    async fn get_input(&self) -> Result<Option<Vec<u8>>, AppError>;

    /// Set the cursor position (used for terminal reset)
    async fn set_cursor_position(&self, row: u16, col: u16) -> Result<(), AppError>;

    /// Flush any pending output
    async fn flush_output(&self) -> Result<(), AppError>;

    /// Get current terminal dimensions
    fn get_terminal_dimensions(&self) -> Option<TerminalDimensions>;

    /// Check if the session should continue
    fn is_active(&self) -> bool;
}

/// Interface for terminal-specific operations
pub trait TerminalHandler: Send + Sync {
    /// Enable raw mode for terminal input
    ///
    /// # Errors
    ///
    /// Returns an error if terminal mode cannot be changed
    fn enable_raw_mode(&self) -> Result<(), AppError>;

    /// Disable raw mode and restore terminal state
    ///
    /// # Errors
    ///
    /// Returns an error if terminal mode cannot be restored
    fn disable_raw_mode(&self) -> Result<(), AppError>;

    /// Write data directly to terminal output
    ///
    /// # Errors
    ///
    /// Returns an error if writing to terminal fails
    fn write_to_terminal(&self, data: &[u8]) -> Result<(), AppError>;

    /// Read from terminal input (non-blocking)
    ///
    /// # Errors
    ///
    /// Returns an error if reading from terminal fails
    fn read_from_terminal(&self) -> Result<Option<Vec<u8>>, AppError>;

    /// Check if TTY is available
    fn is_tty_available(&self) -> bool;
}

/// Represents an exec session
#[derive(Debug, Clone)]
pub struct ExecSession {
    pub container_id: Arc<ContainerId>,
    pub docker: Arc<Docker>,
    pub use_cli: bool,
}

impl ExecSession {
    /// Create a new exec session
    #[must_use]
    pub const fn new(container_id: Arc<ContainerId>, docker: Arc<Docker>, use_cli: bool) -> Self {
        Self {
            container_id,
            docker,
            use_cli,
        }
    }
}

/// Mode of execution (internal via Bollard or external via Docker CLI)
#[derive(Debug, Clone)]
pub enum ExecMode {
    Internal(ExecSession),
    External(Arc<ContainerId>),
}

/// Security module for TTY input sanitization
pub mod security {
    use super::AppError;

    /// Sanitize TTY input to prevent injection attacks
    /// Filters out potentially dangerous escape sequences and control characters
    ///
    /// # Errors
    ///
    /// Currently never returns an error, but the Result type is preserved for future validation
    pub fn sanitize_tty_input(input: &[u8]) -> Result<Vec<u8>, AppError> {
        let mut sanitized = Vec::with_capacity(input.len());

        let mut i = 0;
        while i < input.len() {
            match input[i] {
                // Allow printable ASCII characters
                0x20..=0x7E |
                // Allow common control characters
                0x09 | 0x0A | 0x0D | // Tab, LF, CR
                0x08 | 0x7F => sanitized.push(input[i]), // Backspace, Delete
                // ESC sequences - filter potentially dangerous ones
                0x1B => {
                    if i + 1 < input.len() {
                        match input[i + 1] {
                            // Allow cursor movement and basic formatting
                            b'[' => {
                                // Check for safe CSI sequences
                                if is_safe_csi_sequence(&input[i..]) {
                                    let seq_len = find_csi_sequence_end(&input[i..]);
                                    sanitized.extend_from_slice(&input[i..i + seq_len]);
                                    i += seq_len - 1;
                                } else {
                                    // Skip dangerous sequence
                                    i += find_csi_sequence_end(&input[i..]) - 1;
                                }
                            }
                            // Allow simple escape sequences for arrow keys etc
                            b'O' => {
                                if i + 2 < input.len() && is_safe_ss3_sequence(input[i + 2]) {
                                    sanitized.extend_from_slice(&input[i..i + 3]);
                                    i += 2;
                                }
                            }
                            // OSC sequences - these are dangerous, skip them
                            b']' => {
                                // Find the end of OSC sequence (terminated by BEL or ST)
                                let mut j = i + 2;
                                while j < input.len() {
                                    if input[j] == 0x07 || // BEL
                                       (input[j] == 0x1B && j + 1 < input.len() && input[j + 1] == b'\\') { // ST
                                        i = j;
                                        if input[j] == 0x1B {
                                            i += 1; // Skip ST second byte
                                        }
                                        break;
                                    }
                                    j += 1;
                                }
                            }
                            _ => {} // Skip other escape sequences
                        }
                    }
                }
                _ => {} // Skip other control characters
            }
            i += 1;
        }

        Ok(sanitized)
    }

    /// Check if a CSI sequence is safe to pass through
    fn is_safe_csi_sequence(input: &[u8]) -> bool {
        if input.len() < 3 || input[0] != 0x1B || input[1] != b'[' {
            return false;
        }

        // Find the command character
        let mut i = 2;
        while i < input.len() && i < 20 {
            // Limit sequence length
            match input[i] {
                // Safe movement commands
                b'A' | b'B' | b'C' | b'D' | b'H' | b'F' |
                // Safe editing commands
                b'K' | b'J' | b'P' | b'@' |
                // Safe formatting commands
                b'm' => return true,
                // Parameter bytes and intermediates
                0x20..=0x3F => i += 1,
                _ => return false,
            }
        }
        false
    }

    /// Find the end of a CSI sequence
    fn find_csi_sequence_end(input: &[u8]) -> usize {
        if input.len() < 3 || input[0] != 0x1B || input[1] != b'[' {
            return 2;
        }

        let mut i = 2;
        while i < input.len() && i < 20 {
            match input[i] {
                0x40..=0x7E => return i + 1, // Final byte
                _ => i += 1,
            }
        }
        i
    }

    /// Check if SS3 sequence is safe (for function keys)
    const fn is_safe_ss3_sequence(byte: u8) -> bool {
        matches!(byte, b'P'..=b'S' | b'A'..=b'D')
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_sanitize_printable_ascii() {
            let input = b"Hello World!";
            let result = sanitize_tty_input(input).unwrap();
            assert_eq!(result, input.to_vec());
        }

        #[test]
        fn test_sanitize_control_chars() {
            let input = b"Hello\nWorld\r\n";
            let result = sanitize_tty_input(input).unwrap();
            assert_eq!(result, input.to_vec());
        }

        #[test]
        fn test_sanitize_dangerous_escape() {
            // Attempt to inject dangerous escape sequence
            let input = b"Hello\x1B]0;evil\x07World";
            let result = sanitize_tty_input(input).unwrap();
            assert_eq!(result, b"HelloWorld".to_vec());
        }

        #[test]
        fn test_allow_cursor_movement() {
            let input = b"\x1B[A\x1B[B\x1B[C\x1B[D";
            let result = sanitize_tty_input(input).unwrap();
            assert_eq!(result, input.to_vec());
        }

        #[test]
        fn test_backspace_delete_allowed() {
            let input = b"Test\x08\x7F";
            let result = sanitize_tty_input(input).unwrap();
            assert_eq!(result, input.to_vec());
        }

        #[test]
        fn test_filter_osc_sequences() {
            // OSC sequences can be dangerous
            let input = b"\x1B]2;title\x07test";
            let result = sanitize_tty_input(input).unwrap();
            assert_eq!(result, b"test".to_vec());
        }

        #[test]
        fn test_safe_color_sequences() {
            // Color sequences should be allowed
            let input = b"\x1B[31mRed\x1B[0m";
            let result = sanitize_tty_input(input).unwrap();
            assert_eq!(result, input.to_vec());
        }
    }
}

#[cfg(test)]
mod interface_tests {
    use super::*;
    use std::sync::Mutex;

    struct MockExecInterface {
        output: Arc<Mutex<Vec<Vec<u8>>>>,
        input: Arc<Mutex<Vec<Vec<u8>>>>,
        active: Arc<Mutex<bool>>,
        cursor_positions: Arc<Mutex<Vec<(u16, u16)>>>,
        flush_count: Arc<Mutex<u32>>,
        dimensions: Option<TerminalDimensions>,
    }

    impl MockExecInterface {
        fn new() -> Self {
            Self {
                output: Arc::new(Mutex::new(Vec::new())),
                input: Arc::new(Mutex::new(vec![
                    b"test input".to_vec(),
                    b"more data".to_vec(),
                ])),
                active: Arc::new(Mutex::new(true)),
                cursor_positions: Arc::new(Mutex::new(Vec::new())),
                flush_count: Arc::new(Mutex::new(0)),
                dimensions: Some(TerminalDimensions {
                    width: 80,
                    height: 24,
                }),
            }
        }

        fn set_active(&self, active: bool) {
            *self.active.lock().unwrap() = active;
        }

        fn get_output(&self) -> Vec<Vec<u8>> {
            self.output.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ExecInterface for MockExecInterface {
        async fn handle_output(&self, data: Vec<u8>) -> Result<(), AppError> {
            self.output.lock().unwrap().push(data);
            Ok(())
        }

        async fn get_input(&self) -> Result<Option<Vec<u8>>, AppError> {
            let mut input = self.input.lock().unwrap();
            if input.is_empty() {
                Ok(None)
            } else {
                Ok(Some(input.remove(0)))
            }
        }

        async fn set_cursor_position(&self, row: u16, col: u16) -> Result<(), AppError> {
            self.cursor_positions.lock().unwrap().push((row, col));
            Ok(())
        }

        async fn flush_output(&self) -> Result<(), AppError> {
            *self.flush_count.lock().unwrap() += 1;
            Ok(())
        }

        fn get_terminal_dimensions(&self) -> Option<TerminalDimensions> {
            self.dimensions
        }

        fn is_active(&self) -> bool {
            *self.active.lock().unwrap()
        }
    }

    struct MockTerminalHandler {
        raw_mode: Arc<Mutex<bool>>,
        terminal_output: Arc<Mutex<Vec<Vec<u8>>>>,
        terminal_input: Arc<Mutex<Vec<Vec<u8>>>>,
        tty_available: bool,
    }

    impl MockTerminalHandler {
        fn new(tty_available: bool) -> Self {
            Self {
                raw_mode: Arc::new(Mutex::new(false)),
                terminal_output: Arc::new(Mutex::new(Vec::new())),
                terminal_input: Arc::new(Mutex::new(vec![b"terminal input".to_vec()])),
                tty_available,
            }
        }

        fn get_terminal_output(&self) -> Vec<Vec<u8>> {
            self.terminal_output.lock().unwrap().clone()
        }
    }

    impl TerminalHandler for MockTerminalHandler {
        fn enable_raw_mode(&self) -> Result<(), AppError> {
            let mut raw_mode = self.raw_mode.lock().unwrap();
            if *raw_mode {
                return Err(AppError::Terminal);
            }
            *raw_mode = true;
            drop(raw_mode);
            Ok(())
        }

        fn disable_raw_mode(&self) -> Result<(), AppError> {
            let mut raw_mode = self.raw_mode.lock().unwrap();
            if !*raw_mode {
                return Err(AppError::Terminal);
            }
            *raw_mode = false;
            drop(raw_mode);
            Ok(())
        }

        fn write_to_terminal(&self, data: &[u8]) -> Result<(), AppError> {
            self.terminal_output.lock().unwrap().push(data.to_vec());
            Ok(())
        }

        fn read_from_terminal(&self) -> Result<Option<Vec<u8>>, AppError> {
            let mut input = self.terminal_input.lock().unwrap();
            if input.is_empty() {
                Ok(None)
            } else {
                Ok(Some(input.remove(0)))
            }
        }

        fn is_tty_available(&self) -> bool {
            self.tty_available
        }
    }

    #[test]
    fn test_terminal_dimensions() {
        let dims = TerminalDimensions {
            width: 120,
            height: 40,
        };
        assert_eq!(dims.width, 120);
        assert_eq!(dims.height, 40);

        // Test copy
        let dims2 = dims;
        assert_eq!(dims2.width, 120);
        assert_eq!(dims2.height, 40);
    }

    #[test]
    fn test_exec_mode() {
        use crate::app_data::ContainerId;
        let id = Arc::new(ContainerId::from("test-container"));

        // Test External mode
        let external = ExecMode::External(id);
        match &external {
            ExecMode::External(cid) => assert_eq!(cid.get(), "test-container"),
            ExecMode::Internal(_) => panic!("Wrong mode"),
        }
    }

    #[tokio::test]
    async fn test_exec_interface_handle_output() {
        let interface = MockExecInterface::new();

        // Test handling output data
        assert!(interface.handle_output(b"Hello".to_vec()).await.is_ok());
        assert!(interface.handle_output(b"World".to_vec()).await.is_ok());

        let output = interface.get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], b"Hello");
        assert_eq!(output[1], b"World");
    }

    #[tokio::test]
    async fn test_exec_interface_get_input() {
        let interface = MockExecInterface::new();

        // Test getting input data
        let input1 = interface.get_input().await.unwrap();
        assert_eq!(input1, Some(b"test input".to_vec()));

        let input2 = interface.get_input().await.unwrap();
        assert_eq!(input2, Some(b"more data".to_vec()));

        // No more input
        let input3 = interface.get_input().await.unwrap();
        assert_eq!(input3, None);
    }

    #[tokio::test]
    async fn test_exec_interface_cursor_position() {
        let interface = MockExecInterface::new();

        assert!(interface.set_cursor_position(10, 20).await.is_ok());
        assert!(interface.set_cursor_position(0, 0).await.is_ok());

        let positions = interface.cursor_positions.lock().unwrap().clone();
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], (10, 20));
        assert_eq!(positions[1], (0, 0));
    }

    #[tokio::test]
    async fn test_exec_interface_flush() {
        let interface = MockExecInterface::new();

        assert!(interface.flush_output().await.is_ok());
        assert!(interface.flush_output().await.is_ok());
        assert!(interface.flush_output().await.is_ok());

        assert_eq!(*interface.flush_count.lock().unwrap(), 3);
    }

    #[test]
    fn test_exec_interface_active_state() {
        let interface = MockExecInterface::new();

        assert!(interface.is_active());

        interface.set_active(false);
        assert!(!interface.is_active());

        interface.set_active(true);
        assert!(interface.is_active());
    }

    #[test]
    fn test_terminal_handler_raw_mode() {
        let handler = MockTerminalHandler::new(true);

        // Initially not in raw mode
        assert!(!*handler.raw_mode.lock().unwrap());

        // Enable raw mode
        assert!(handler.enable_raw_mode().is_ok());
        assert!(*handler.raw_mode.lock().unwrap());

        // Can't enable again
        assert!(handler.enable_raw_mode().is_err());

        // Disable raw mode
        assert!(handler.disable_raw_mode().is_ok());
        assert!(!*handler.raw_mode.lock().unwrap());

        // Can't disable again
        assert!(handler.disable_raw_mode().is_err());
    }

    #[test]
    fn test_terminal_handler_io() {
        let handler = MockTerminalHandler::new(true);

        // Test writing to terminal
        assert!(handler.write_to_terminal(b"Hello Terminal").is_ok());
        assert!(handler.write_to_terminal(b"\x1B[2J\x1B[H").is_ok());

        let output = handler.get_terminal_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], b"Hello Terminal");
        assert_eq!(output[1], b"\x1B[2J\x1B[H");

        // Test reading from terminal
        let input = handler.read_from_terminal().unwrap();
        assert_eq!(input, Some(b"terminal input".to_vec()));

        // No more input
        let input2 = handler.read_from_terminal().unwrap();
        assert_eq!(input2, None);
    }

    #[test]
    fn test_terminal_handler_tty_availability() {
        let handler_with_tty = MockTerminalHandler::new(true);
        assert!(handler_with_tty.is_tty_available());

        let handler_without_tty = MockTerminalHandler::new(false);
        assert!(!handler_without_tty.is_tty_available());
    }

    #[test]
    fn test_exec_event() {
        // Test OutputData event
        let event = ExecEvent::OutputData(b"output".to_vec());
        match event {
            ExecEvent::OutputData(data) => assert_eq!(data, b"output"),
            _ => panic!("Wrong event type"),
        }

        // Test SessionStarted event
        let event = ExecEvent::SessionStarted;
        match event {
            ExecEvent::SessionStarted => (),
            _ => panic!("Wrong event type"),
        }

        // Test SessionEnded event
        let event = ExecEvent::SessionEnded;
        match event {
            ExecEvent::SessionEnded => (),
            _ => panic!("Wrong event type"),
        }

        // Test Error event
        let event = ExecEvent::Error("Test error".to_string());
        match event {
            ExecEvent::Error(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Wrong event type"),
        }
    }
}
