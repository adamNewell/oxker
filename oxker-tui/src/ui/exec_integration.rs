use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::Stdout;
use std::io::Write;
use std::sync::Arc;

use {
    crate::handlers::exec_handler::{TuiExecInterface, TuiTerminalHandler, convert_terminal_size},
    oxker_core::TerminalHandler,
    tokio::sync::mpsc,
};

/// Execute the exec mode with proper interface handling
pub async fn run_exec_mode(
    mode: oxker_core::ExecMode,
    terminal: &Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), oxker_core::AppError> {
    // Set up channels for output and input handling
    let (output_tx, mut output_rx) = mpsc::channel(1024);
    let (_input_tx, input_rx) = mpsc::channel(1024);

    // Create the interface and handler
    let dimensions = convert_terminal_size(terminal);
    let interface = Arc::new(TuiExecInterface::new(output_tx, input_rx, dimensions));
    let terminal_handler = Arc::new(TuiTerminalHandler::new());

    // Spawn output handler to write to stdout
    let interface_clone = Arc::clone(&interface);
    tokio::spawn(async move {
        while let Some(data) = output_rx.recv().await {
            let mut stdout = std::io::stdout();
            if stdout.write_all(&data).is_err() {
                interface_clone.set_active(false);
                break;
            }
            if stdout.flush().is_err() {
                interface_clone.set_active(false);
                break;
            }
        }
    });

    // Spawn input handler if we need to handle TTY input
    if terminal_handler.is_tty_available() {
        tokio::spawn(async move {
            // In a real implementation, this would read from the terminal input
            // and send it through input_tx
            // For now, this is a placeholder
        });
    }

    // Run the exec mode
    mode.run(interface, terminal_handler).await
}
