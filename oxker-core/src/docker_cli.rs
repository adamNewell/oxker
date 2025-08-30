use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum DockerCliStatus {
    Available,
    NotFound,
    NotInPath(Vec<String>),
    PermissionDenied(String),
    DaemonNotRunning,
}

pub struct DockerCliDetector {
    cache: Option<(Instant, DockerCliStatus)>,
}

impl DockerCliDetector {
    const DEFAULT_TTL: Duration = Duration::from_secs(30);
    const ERROR_TTL: Duration = Duration::from_secs(5);

    #[must_use]
    pub const fn new() -> Self {
        Self { cache: None }
    }

    pub fn detect(&mut self) -> DockerCliStatus {
        if let Some((timestamp, ref status)) = self.cache {
            let ttl = match status {
                DockerCliStatus::Available => Self::DEFAULT_TTL,
                _ => Self::ERROR_TTL,
            };
            
            if timestamp.elapsed() < ttl {
                return status.clone();
            }
        }

        let status = Self::detect_impl();
        self.cache = Some((Instant::now(), status.clone()));
        status
    }

    pub fn invalidate_cache(&mut self) {
        self.cache = None;
    }

    fn detect_impl() -> DockerCliStatus {
        #[cfg(unix)]
        {
            Self::detect_unix()
        }
        #[cfg(windows)]
        {
            Self::detect_windows()
        }
    }

    #[cfg(unix)]
    fn detect_unix() -> DockerCliStatus {
        match Command::new("docker").arg("version").output() {
            Ok(output) => {
                if output.status.success() {
                    DockerCliStatus::Available
                } else {
                    // Docker exists but might have issues
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.contains("Cannot connect to the Docker daemon") ||
                       stderr.contains("Is the docker daemon running") {
                        DockerCliStatus::DaemonNotRunning
                    } else if stderr.contains("permission denied") ||
                              stderr.contains("Permission denied") {
                        DockerCliStatus::PermissionDenied(stderr.to_string())
                    } else {
                        DockerCliStatus::DaemonNotRunning
                    }
                }
            }
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => {
                        // Try to find docker in common locations
                        let common_paths = vec![
                            "/usr/local/bin/docker",
                            "/usr/bin/docker",
                            "/opt/homebrew/bin/docker",
                            "/snap/bin/docker",
                        ];
                        
                        let mut checked_paths = Vec::new();
                        for path in &common_paths {
                            checked_paths.push((*path).to_string());
                            if std::path::Path::new(path).exists() {
                                return DockerCliStatus::NotInPath(checked_paths);
                            }
                        }
                        
                        DockerCliStatus::NotFound
                    }
                    io::ErrorKind::PermissionDenied => {
                        DockerCliStatus::PermissionDenied("Permission denied executing docker".to_string())
                    }
                    _ => DockerCliStatus::NotFound
                }
            }
        }
    }

    #[cfg(windows)]
    fn detect_windows() -> DockerCliStatus {
        match Command::new("docker").arg("version").output() {
            Ok(output) => {
                if output.status.success() {
                    DockerCliStatus::Available
                } else {
                    // Docker exists but might have issues
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.contains("Cannot connect to the Docker daemon") ||
                       stderr.contains("Is the docker daemon running") ||
                       stderr.contains("error during connect") {
                        DockerCliStatus::DaemonNotRunning
                    } else if stderr.contains("Access is denied") {
                        DockerCliStatus::PermissionDenied(stderr.to_string())
                    } else {
                        DockerCliStatus::DaemonNotRunning
                    }
                }
            }
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => {
                        if let Ok(output) = Command::new("docker.exe").arg("version").output() {
                            if output.status.success() {
                                return DockerCliStatus::Available;
                            }
                        }
                        
                        // Check common Windows paths
                        let common_paths = vec![
                            r"C:\Program Files\Docker\Docker\resources\bin\docker.exe",
                            r"C:\ProgramData\DockerDesktop\version-bin\docker.exe",
                            r"C:\Program Files\Docker\Docker\resources\docker.exe",
                        ];
                        
                        let mut checked_paths = Vec::new();
                        for path in &common_paths {
                            checked_paths.push((*path).to_string());
                            if std::path::Path::new(path).exists() {
                                return DockerCliStatus::NotInPath(checked_paths);
                            }
                        }
                        
                        DockerCliStatus::NotFound
                    }
                    _ => DockerCliStatus::NotFound
                }
            }
        }
    }
}

impl Default for DockerCliDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_cli_detector_creation() {
        let detector = DockerCliDetector::new();
        assert!(detector.cache.is_none());
    }

    #[test]
    fn test_cache_invalidation() {
        let mut detector = DockerCliDetector::new();
        detector.cache = Some((Instant::now(), DockerCliStatus::Available));
        detector.invalidate_cache();
        assert!(detector.cache.is_none());
    }

    #[test]
    fn test_docker_cli_status_clone() {
        let status = DockerCliStatus::NotFound;
        let _ = status;
    }
}
