use async_trait::async_trait;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use oxker_core::{
    AppError,
    exec_interface::{ExecInterface, TerminalDimensions, TerminalHandler},
};
use parking_lot::Mutex;
use std::io::{Read, Write, stdout};
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::mpsc;

/// Implementation of ExecInterface for the TUI
pub struct TuiExecInterface {
    output_tx: mpsc::Sender<Vec<u8>>,
    input_rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    active: Arc<Mutex<bool>>,
    terminal_dimensions: Arc<Mutex<Option<TerminalDimensions>>>,
}

impl TuiExecInterface {
    pub fn new(
        output_tx: mpsc::Sender<Vec<u8>>,
        input_rx: mpsc::Receiver<Vec<u8>>,
        terminal_dimensions: Option<TerminalDimensions>,
    ) -> Self {
        Self {
            output_tx,
            input_rx: Arc::new(Mutex::new(input_rx)),
            active: Arc::new(Mutex::new(true)),
            terminal_dimensions: Arc::new(Mutex::new(terminal_dimensions)),
        }
    }

    pub fn set_active(&self, active: bool) {
        *self.active.lock() = active;
    }

    pub fn set_dimensions(&self, dimensions: Option<TerminalDimensions>) {
        *self.terminal_dimensions.lock() = dimensions;
    }
}

#[async_trait]
impl ExecInterface for TuiExecInterface {
    async fn handle_output(&self, data: Vec<u8>) -> Result<(), AppError> {
        self.output_tx
            .send(data)
            .await
            .map_err(|_| AppError::Terminal)
    }

    async fn get_input(&self) -> Result<Option<Vec<u8>>, AppError> {
        let mut rx = self.input_rx.lock();
        match rx.try_recv() {
            Ok(data) => Ok(Some(data)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(_) => Err(AppError::Terminal),
        }
    }

    async fn set_cursor_position(&self, _row: u16, _col: u16) -> Result<(), AppError> {
        // In TUI mode, cursor position is handled by the terminal output handler
        let mut stdout = stdout();
        stdout
            .write_all(b"\x1B[J\x1B[H")
            .map_err(|_| AppError::Terminal)?;
        Ok(())
    }

    async fn flush_output(&self) -> Result<(), AppError> {
        let mut stdout = stdout();
        stdout.flush().map_err(|_| AppError::Terminal)
    }

    fn get_terminal_dimensions(&self) -> Option<TerminalDimensions> {
        *self.terminal_dimensions.lock()
    }

    fn is_active(&self) -> bool {
        *self.active.lock()
    }
}

/// Implementation of TerminalHandler for the TUI
pub struct TuiTerminalHandler {
    cleanup_needed: Arc<AtomicBool>,
}

impl TuiTerminalHandler {
    pub fn new() -> Self {
        Self {
            cleanup_needed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Performs the keyboard protocol cleanup sequence
    fn perform_cleanup(&self) -> Result<(), AppError> {
        const KEYBOARD_PROTO: &str = "\x1B[?u\x1B[c";
        const TTY: &str = "/dev/tty";

        let waiting = Arc::new(AtomicBool::new(true));
        let waiting_thread = Arc::clone(&waiting);

        std::thread::spawn(move || {
            let mut bytes = Vec::with_capacity(26);
            while waiting_thread.load(std::sync::atomic::Ordering::SeqCst) {
                let mut buf = [0];
                if let Ok(mut f) = std::fs::File::open(TTY) {
                    if f.read_exact(&mut buf).is_err() {
                        waiting_thread.store(false, std::sync::atomic::Ordering::SeqCst);
                    }
                    bytes.push(buf[0]);
                    if byte_sequence_valid(&bytes) {
                        waiting_thread.store(false, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
        });

        let mut stdout = stdout();
        stdout
            .write_all(KEYBOARD_PROTO.as_bytes())
            .map_err(|_| AppError::Terminal)?;
        stdout.flush().map_err(|_| AppError::Terminal)?;

        let start = std::time::Instant::now();
        while waiting.load(std::sync::atomic::Ordering::SeqCst) {
            if start.elapsed().as_millis() > 1500 {
                waiting.store(false, std::sync::atomic::Ordering::SeqCst);
                return Err(AppError::Terminal);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }
}

/// ByteOutput enum for terminal cleanup validation
enum ByteOutput {
    Arm,
    X86,
}

impl ByteOutput {
    const fn len(&self) -> usize {
        match self {
            Self::Arm => 26,
            Self::X86 => 6,
        }
    }
    const fn last(&self) -> &[u8] {
        match self {
            Self::Arm => &[50],
            Self::X86 => &[99],
        }
    }
}

/// Check the output from tty to see if it matches known sequence
fn byte_sequence_valid(bytes: &[u8]) -> bool {
    [ByteOutput::Arm, ByteOutput::X86]
        .iter()
        .any(|i| i.len() == bytes.len() && bytes.ends_with(i.last()))
}

impl TerminalHandler for TuiTerminalHandler {
    fn enable_raw_mode(&self) -> Result<(), AppError> {
        self.cleanup_needed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        enable_raw_mode().map_err(|_| AppError::Terminal)
    }

    fn disable_raw_mode(&self) -> Result<(), AppError> {
        // First disable raw mode
        disable_raw_mode().map_err(|_| AppError::Terminal)?;

        // Then perform keyboard protocol cleanup if needed
        if self
            .cleanup_needed
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            self.cleanup_needed
                .store(false, std::sync::atomic::Ordering::SeqCst);
            self.perform_cleanup()?;
        }
        Ok(())
    }

    fn write_to_terminal(&self, data: &[u8]) -> Result<(), AppError> {
        let mut stdout = stdout();
        stdout.write_all(data).map_err(|_| AppError::Terminal)?;
        stdout.flush().map_err(|_| AppError::Terminal)
    }

    fn read_from_terminal(&self) -> Result<Option<Vec<u8>>, AppError> {
        // In TUI mode, terminal reading is handled by the input event system
        // This is primarily used for the cleanup sequence
        Ok(None)
    }

    fn is_tty_available(&self) -> bool {
        oxker_core::tty_readable()
    }
}

/// Drop implementation to ensure terminal cleanup on panic
impl Drop for TuiTerminalHandler {
    fn drop(&mut self) {
        // Best effort cleanup - ignore errors
        if self.cleanup_needed.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = disable_raw_mode();
            let _ = self.perform_cleanup();
        }
    }
}

/// Helper to convert ratatui Terminal dimensions to our TerminalDimensions
pub fn convert_terminal_size(
    terminal: &ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
) -> Option<TerminalDimensions> {
    terminal.size().ok().map(|size| TerminalDimensions {
        width: size.width,
        height: size.height,
    })
}
