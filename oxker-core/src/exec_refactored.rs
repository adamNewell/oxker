use std::sync::{Arc, mpsc::Sender};

use bollard::{
    Docker,
    exec::{CreateExecOptions, ResizeExecOptions, StartExecOptions, StartExecResults},
};
use futures_util::StreamExt;
use parking_lot::Mutex;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use tokio_util::sync::CancellationToken;

use crate::{
    app_data::{AppData, ContainerId, RunningState, State},
    app_error::AppError,
    exec_interface::{ExecInterface, ExecSession, TerminalDimensions, TerminalHandler, security},
};

/// TTY location
const TTY: &str = "/dev/tty";

/// This will be the start of a docker exec message if one is unable to actually exec into the container
const OCI_ERROR: &str = "OCI runtime exec failed";

mod command {
    pub const PWD: &str = "pwd";
    pub const DOCKER: &str = "docker";
    pub const EXEC: &str = "exec";
    pub const SH: &str = "sh";
    pub const IT: &str = "-it";
}

/// Check if tty is able to be written to, aka not windows
pub fn tty_readable() -> bool {
    std::fs::OpenOptions::new()
        .read(true)
        .write(false)
        .open(TTY)
        .is_ok()
}

struct AsyncTTY {
    rx: std::sync::mpsc::Receiver<u8>,
}

impl AsyncTTY {
    /// Use an async timeout to read data from the file, and send to the "main" thread
    async fn read_loop(mut f: File, tx: Sender<u8>) {
        loop {
            let mut buf = [0];
            if tokio::time::timeout(std::time::Duration::from_millis(10), f.read_exact(&mut buf))
                .await
                .is_ok()
                && tx.send(buf[0]).is_err()
            {
                break;
            }
        }
    }

    /// Async tty reading, spawned into its own tokio thread
    fn get(cancel_token: &CancellationToken) -> Option<Self> {
        if tty_readable() {
            let (tx, rx) = std::sync::mpsc::channel();
            let cancel_token = cancel_token.to_owned();
            tokio::spawn(async move {
                if let Ok(f) = tokio::fs::File::open(TTY).await {
                    tokio::select! {
                    () = cancel_token.cancelled() => (),
                    () = Self::read_loop(f, tx) => cancel_token.cancel(),
                    }
                }
            });
            Some(Self { rx })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExecMode {
    // use Bollard Rust library
    Internal(ExecSession),
    // use the external `docker-cli`
    External(Arc<ContainerId>),
}

impl ExecMode {
    /// Test if we can exec into the selected container, first via the Internal methods, then by the External
    /// If the container is oxker, it will always return None
    pub async fn new(app_data: &Arc<Mutex<AppData>>, docker: &Arc<Docker>) -> Option<Self> {
        let is_oxker = app_data.lock().is_oxker();
        if is_oxker {
            return None;
        }

        let use_cli = app_data.lock().config.use_cli;
        let container = app_data.lock().get_selected_container_id_state_name();

        if let Some((id, state, _)) = container {
            if [
                State::Running(RunningState::Healthy),
                State::Running(RunningState::Unhealthy),
            ]
            .contains(&state)
            {
                if tty_readable() && !use_cli {
                    if let Ok(exec) = docker
                        .create_exec(
                            id.get(),
                            CreateExecOptions {
                                attach_stdout: Some(true),
                                attach_stderr: Some(true),
                                cmd: Some(vec![command::PWD]),
                                ..Default::default()
                            },
                        )
                        .await
                    {
                        if let Ok(StartExecResults::Attached { mut output, .. }) =
                            docker.start_exec(&exec.id, None).await
                        {
                            if let Some(Ok(msg)) = output.next().await {
                                if !msg.to_string().starts_with(OCI_ERROR) {
                                    return Some(Self::Internal(ExecSession::new(
                                        Arc::new(id),
                                        Arc::clone(docker),
                                        use_cli,
                                    )));
                                }
                            }
                        }
                    }
                }

                if let Ok(output) = std::process::Command::new(command::DOCKER)
                    .args([command::EXEC, id.get(), command::PWD])
                    .output()
                {
                    if let Ok(output) = String::from_utf8(output.stdout) {
                        if !output.starts_with(OCI_ERROR) {
                            return Some(Self::External(Arc::new(id)));
                        }
                    }
                }
            }
        }
        None
    }

    /// exec into the container using the external docker cli
    pub async fn exec_external<T: TerminalHandler>(
        id: &ContainerId,
        terminal_handler: &T,
    ) -> Result<(), AppError> {
        // Set cursor position
        terminal_handler.write_to_terminal(b"\x1B[J\x1B[H")?;

        let mut child = std::process::Command::new(command::DOCKER)
            .args([command::EXEC, command::IT, id.get(), command::SH])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|_| AppError::Terminal)?;

        child.wait().map_err(|_| AppError::Terminal)?;
        if child.kill().is_err() {
            return Err(AppError::Terminal);
        }
        Ok(())
    }

    /// Exec into the container via the Bollard library
    pub async fn exec_internal<I: ExecInterface + 'static, T: TerminalHandler + 'static>(
        &self,
        session: &ExecSession,
        interface: Arc<I>,
        terminal_handler: Arc<T>,
        terminal_dimensions: Option<TerminalDimensions>,
    ) -> Result<(), AppError> {
        let cancel_token = CancellationToken::new();
        let id = &session.container_id;
        let docker = &session.docker;

        let exec_result = docker
            .create_exec(
                id.get(),
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    attach_stdin: Some(true),
                    tty: Some(true),
                    cmd: Some(vec![command::SH]),
                    ..Default::default()
                },
            )
            .await
            .map_err(|_| AppError::DockerExec)?;

        match docker
            .start_exec(
                &exec_result.id,
                Some(StartExecOptions {
                    detach: false,
                    ..Default::default()
                }),
            )
            .await
        {
            Ok(StartExecResults::Attached {
                mut output,
                mut input,
            }) => {
                // Enable raw mode
                terminal_handler.enable_raw_mode()?;

                // Set cursor position
                interface.set_cursor_position(0, 0).await?;
                interface.flush_output().await?;

                // Spawn output handler
                let interface_output = Arc::clone(&interface);
                let cancel_output = cancel_token.clone();
                tokio::spawn(async move {
                    while let Some(Ok(x)) = output.next().await {
                        if interface_output
                            .handle_output(x.into_bytes().to_vec())
                            .await
                            .is_err()
                        {
                            break;
                        }
                        if !interface_output.is_active() {
                            break;
                        }
                    }
                    cancel_output.cancel();
                });

                // Handle terminal resizing if dimensions provided
                if let Some(dimensions) = terminal_dimensions {
                    docker
                        .resize_exec(
                            &exec_result.id,
                            ResizeExecOptions {
                                height: dimensions.height,
                                width: dimensions.width,
                            },
                        )
                        .await
                        .ok();
                }

                // Handle input with sanitization
                while interface.is_active() && !cancel_token.is_cancelled() {
                    if let Ok(Some(data)) = interface.get_input().await {
                        // Sanitize input to prevent TTY injection attacks (SEC-001)
                        match security::sanitize_tty_input(&data) {
                            Ok(sanitized) => {
                                if input.write_all(&sanitized).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => {
                                // Log error but continue - don't break the session
                                // In production, this should be logged properly
                            }
                        }
                    } else {
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }

                // Disable raw mode
                terminal_handler.disable_raw_mode()?;

                Ok(())
            }
            _ => Err(AppError::Terminal),
        }
    }

    pub async fn run<I: ExecInterface + 'static, T: TerminalHandler + 'static>(
        &self,
        interface: Arc<I>,
        terminal_handler: Arc<T>,
    ) -> Result<(), AppError> {
        match self {
            Self::External(id) => Self::exec_external(id, terminal_handler.as_ref()).await,
            Self::Internal(session) => {
                let dimensions = interface.get_terminal_dimensions();
                self.exec_internal(session, interface, terminal_handler, dimensions)
                    .await
            }
        }
    }
}
