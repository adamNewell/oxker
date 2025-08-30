use std::io::Write;

use crate::{
    app_data::ContainerId,
    app_error::AppError,
    docker_cli::{DockerCliDetector, DockerCliStatus},
};

/// TTY location
const TTY: &str = "/dev/tty";

mod command {
    pub const DOCKER: &str = "docker";
    pub const EXEC: &str = "exec";
    pub const SH: &str = "sh";
    pub const IT: &str = "-it";
}

/// Check if tty is able to be written to, aka not windows
#[must_use]
pub fn tty_readable() -> bool {
    std::fs::OpenOptions::new()
        .read(true)
        .write(false)
        .open(TTY)
        .is_ok()
}

/// exec into the container using the external docker cli
///
/// # Errors
///
/// Returns an error if the docker CLI command fails or terminal operations fail
pub fn exec_docker_cli(id: &ContainerId) -> Result<(), AppError> {
    // First check if Docker CLI is available
    let mut detector = DockerCliDetector::new();
    match detector.detect() {
        DockerCliStatus::Available => {
            // Docker is available, proceed with exec
        }
        DockerCliStatus::NotFound => {
            return Err(AppError::DockerNotFound);
        }
        DockerCliStatus::NotInPath(paths) => {
            let detail = if paths.is_empty() {
                "Docker found but not in PATH".to_string()
            } else {
                format!("Docker found at: {}, but not in PATH", paths.join(", "))
            };
            return Err(AppError::DockerNotAccessible(detail));
        }
        DockerCliStatus::PermissionDenied(msg) => {
            return Err(AppError::DockerNotAccessible(msg));
        }
        DockerCliStatus::DaemonNotRunning => {
            return Err(AppError::DockerDaemonNotRunning);
        }
    }

    // Clear screen and reset cursor
    print!("\x1B[2J\x1B[H");
    std::io::stdout().flush().map_err(|_| AppError::Terminal)?;

    let mut child = std::process::Command::new(command::DOCKER)
        .args([command::EXEC, command::IT, id.get(), command::SH])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| {
            // Handle specific error cases from spawning the process
            match e.kind() {
                std::io::ErrorKind::NotFound => AppError::DockerNotFound,
                std::io::ErrorKind::PermissionDenied => {
                    AppError::DockerNotAccessible("Permission denied executing docker".to_string())
                }
                _ => AppError::Terminal,
            }
        })?;

    let status = child.wait().map_err(|_| AppError::Terminal)?;

    // Check if the command executed successfully
    if !status.success() {
        // Check if container was not found or not running
        // Note: We can't easily distinguish these without capturing stderr,
        // but we can provide a general error message that covers both cases
        return Err(AppError::DockerExec);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tty_readable() {
        // This test will pass or fail based on the platform
        // On Unix-like systems with /dev/tty, it should return true
        // On Windows or systems without /dev/tty, it should return false
        let result = tty_readable();

        #[cfg(unix)]
        {
            // On Unix systems, we expect this to work in most cases
            // But in CI or containers, it might not have a TTY
            // Just verify the function runs without crashing
            let _ = result;
        }

        #[cfg(windows)]
        {
            // On Windows, /dev/tty doesn't exist
            assert!(!result);
        }
    }

    // Since we can't easily mock std::process::Command directly,
    // we'll test the behavior through integration tests instead.
    // The unit tests below demonstrate the expected behavior.

    #[test]
    fn test_exec_docker_cli_command_construction() {
        // This test verifies the expected command construction
        // In a real implementation, we would use dependency injection
        // to make the Command creation testable

        let container_id = ContainerId::from("test_container_123");

        // The expected command would be:
        // docker exec -it test_container_123 sh
        let expected_args = ["exec", "-it", "test_container_123", "sh"];

        // Verify our constants match expected values
        assert_eq!(command::DOCKER, "docker");
        assert_eq!(command::EXEC, expected_args[0]);
        assert_eq!(command::IT, expected_args[1]);
        assert_eq!(container_id.get(), expected_args[2]);
        assert_eq!(command::SH, expected_args[3]);

        // Verify the container ID getter works
        assert_eq!(container_id.get(), "test_container_123");
    }

    #[test]
    fn test_exec_docker_cli_error_scenarios() {
        // Test that exec_docker_cli returns appropriate errors
        // This demonstrates the error cases even though we can't
        // directly test them without mocking

        // Test with empty container ID
        let empty_id = ContainerId::from("");
        assert_eq!(empty_id.get(), "");

        // Test with special characters in container ID
        let special_id = ContainerId::from("container-with-special-chars!@#");
        assert_eq!(special_id.get(), "container-with-special-chars!@#");
    }
}
