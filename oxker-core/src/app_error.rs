use crate::app_data::DockerCommand;
use std::fmt;

/// app errors to set in global state
#[derive(Debug, Clone)]
pub enum AppError {
    DockerCommand(DockerCommand),
    DockerExec,
    DockerLogs,
    DockerConnect,
    DockerNotFound,
    DockerNotAccessible(String),
    ContainerNotFound(String),
    ContainerNotRunning(String),
    DockerDaemonNotRunning,
    IO(String),
    MouseCapture(bool),
    Parse(String),
    Terminal,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::DockerCommand(s) => write!(f, "Unable to {s} container"),
            Self::DockerExec => write!(f, "Unable to exec into container"),
            Self::DockerLogs => write!(f, "Unable to save logs"),
            Self::DockerConnect => write!(f, "Unable to access docker daemon"),
            Self::DockerNotFound => write!(
                f,
                "Docker CLI not found.\n\
                Please install Docker Desktop:\n\
                - macOS/Windows: docker.com/products/docker-desktop\n\
                - Linux: docs.docker.com/engine/install/"
            ),
            Self::DockerNotAccessible(detail) => write!(
                f,
                "Docker is installed but not accessible.\n{detail}\n\
                Try:\n\
                - Adding Docker to your PATH\n\
                - Reinstalling Docker Desktop\n\
                - Checking Docker Desktop permissions"
            ),
            Self::ContainerNotFound(id) => write!(
                f,
                "Container '{id}' not found.\n\
                The container may have been removed or the ID may be incorrect.\n\
                Use the container list to verify available containers."
            ),
            Self::ContainerNotRunning(name) => write!(
                f,
                "Container '{name}' is not running.\n\
                Exec only works with running containers. Start the container first."
            ),
            Self::DockerDaemonNotRunning => write!(
                f,
                "Cannot connect to Docker daemon.\n\
                Is Docker Desktop running? Try:\n\
                - Starting Docker Desktop application\n\
                - Checking system tray for Docker icon\n\
                - Running 'docker version' to verify connectivity"
            ),
            Self::IO(msg) => write!(f, "IO error with: {msg}"),
            Self::MouseCapture(x) => {
                let reason = if *x { "en" } else { "dis" };
                write!(f, "Unable to {reason}able mouse capture")
            }
            Self::Parse(msg) => write!(f, "Parsing error: {msg}"),
            Self::Terminal => write!(f, "Unable to fully render to terminal"),
        }
    }
}
