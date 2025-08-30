use std::collections::HashMap;

use bollard::secret::ContainerNetworkStats;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct NetworkInterfaceDetector {
    cache: HashMap<String, Option<String>>,
}

impl NetworkInterfaceDetector {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn detect_interface(
        &mut self,
        container_id: &str,
        networks: &HashMap<String, ContainerNetworkStats>,
        configured_interface: Option<&str>,
    ) -> Option<String> {
        if let Some(cached) = self.cache.get(container_id) {
            return cached.clone();
        }

        let interface = match configured_interface {
            Some("none") => {
                debug!("Network stats disabled for container {}", container_id);
                None
            }
            Some("auto") | None => {
                let detected = Self::auto_detect_interface(networks);
                if let Some(ref iface) = detected {
                    debug!(
                        "Auto-detected interface '{}' for container {}",
                        iface, container_id
                    );
                } else {
                    debug!("No network interface found for container {}", container_id);
                }
                detected
            }
            Some(specific) => {
                if networks.contains_key(specific) {
                    debug!(
                        "Using configured interface '{}' for container {}",
                        specific, container_id
                    );
                    Some(specific.to_string())
                } else {
                    debug!(
                        "Configured interface '{}' not found for container {}, falling back to auto-detection",
                        specific, container_id
                    );
                    let detected = Self::auto_detect_interface(networks);
                    if let Some(ref iface) = detected {
                        debug!(
                            "Auto-detected fallback interface '{}' for container {}",
                            iface, container_id
                        );
                    }
                    detected
                }
            }
        };

        self.cache
            .insert(container_id.to_string(), interface.clone());
        interface
    }

    fn auto_detect_interface(networks: &HashMap<String, ContainerNetworkStats>) -> Option<String> {
        if networks.is_empty() {
            return None;
        }

        if networks.contains_key("eth0") {
            return Some("eth0".to_string());
        }

        let mut interfaces: Vec<&String> = networks.keys().collect();
        interfaces.sort();

        for interface in &interfaces {
            if interface.starts_with("eth") {
                return Some((*interface).to_string());
            }
        }

        for interface in &interfaces {
            if interface.starts_with("wlan") {
                return Some((*interface).to_string());
            }
        }

        interfaces.first().map(|i| (*i).to_string())
    }

    #[allow(dead_code)]
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    #[allow(dead_code)]
    pub fn remove_from_cache(&mut self, container_id: &str) {
        self.cache.remove(container_id);
    }
}

impl Default for NetworkInterfaceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_network_stats(rx_bytes: u64, tx_bytes: u64) -> ContainerNetworkStats {
        ContainerNetworkStats {
            rx_bytes: Some(rx_bytes),
            tx_bytes: Some(tx_bytes),
            rx_packets: None,
            tx_packets: None,
            rx_errors: None,
            tx_errors: None,
            rx_dropped: None,
            tx_dropped: None,
            endpoint_id: None,
            instance_id: None,
        }
    }

    #[test]
    fn test_detect_interface_with_eth0() {
        let mut detector = NetworkInterfaceDetector::new();
        let mut networks = HashMap::new();
        networks.insert("eth0".to_string(), create_network_stats(100, 200));
        networks.insert("eth1".to_string(), create_network_stats(150, 250));

        let result = detector.detect_interface("container1", &networks, Some("auto"));
        assert_eq!(result, Some("eth0".to_string()));
    }

    #[test]
    fn test_detect_interface_without_eth0() {
        let mut detector = NetworkInterfaceDetector::new();
        let mut networks = HashMap::new();
        networks.insert("eth1".to_string(), create_network_stats(100, 200));
        networks.insert("eth2".to_string(), create_network_stats(150, 250));

        let result = detector.detect_interface("container1", &networks, Some("auto"));
        assert_eq!(result, Some("eth1".to_string()));
    }

    #[test]
    fn test_detect_interface_wlan_priority() {
        let mut detector = NetworkInterfaceDetector::new();
        let mut networks = HashMap::new();
        networks.insert("wlan0".to_string(), create_network_stats(100, 200));
        networks.insert("docker0".to_string(), create_network_stats(150, 250));

        let result = detector.detect_interface("container1", &networks, Some("auto"));
        assert_eq!(result, Some("wlan0".to_string()));
    }

    #[test]
    fn test_detect_interface_fallback_to_first() {
        let mut detector = NetworkInterfaceDetector::new();
        let mut networks = HashMap::new();
        networks.insert("docker0".to_string(), create_network_stats(100, 200));
        networks.insert("br0".to_string(), create_network_stats(150, 250));

        let result = detector.detect_interface("container1", &networks, Some("auto"));
        assert_eq!(result, Some("br0".to_string()));
    }

    #[test]
    fn test_detect_interface_empty_networks() {
        let mut detector = NetworkInterfaceDetector::new();
        let networks = HashMap::new();

        let result = detector.detect_interface("container1", &networks, Some("auto"));
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_interface_specific_exists() {
        let mut detector = NetworkInterfaceDetector::new();
        let mut networks = HashMap::new();
        networks.insert("eth0".to_string(), create_network_stats(100, 200));
        networks.insert("custom0".to_string(), create_network_stats(150, 250));

        let result = detector.detect_interface("container1", &networks, Some("custom0"));
        assert_eq!(result, Some("custom0".to_string()));
    }

    #[test]
    fn test_detect_interface_specific_missing_fallback() {
        let mut detector = NetworkInterfaceDetector::new();
        let mut networks = HashMap::new();
        networks.insert("eth0".to_string(), create_network_stats(100, 200));

        let result = detector.detect_interface("container1", &networks, Some("custom0"));
        assert_eq!(result, Some("eth0".to_string()));
    }

    #[test]
    fn test_detect_interface_none_config() {
        let mut detector = NetworkInterfaceDetector::new();
        let mut networks = HashMap::new();
        networks.insert("eth0".to_string(), create_network_stats(100, 200));

        let result = detector.detect_interface("container1", &networks, Some("none"));
        assert_eq!(result, None);
    }

    #[test]
    fn test_cache_functionality() {
        let mut detector = NetworkInterfaceDetector::new();
        let mut networks = HashMap::new();
        networks.insert("eth0".to_string(), create_network_stats(100, 200));

        let result1 = detector.detect_interface("container1", &networks, Some("auto"));
        assert_eq!(result1, Some("eth0".to_string()));

        networks.clear();
        let result2 = detector.detect_interface("container1", &networks, Some("auto"));
        assert_eq!(result2, Some("eth0".to_string()));

        detector.remove_from_cache("container1");
        let result3 = detector.detect_interface("container1", &networks, Some("auto"));
        assert_eq!(result3, None);
    }

    #[test]
    fn test_clear_cache() {
        let mut detector = NetworkInterfaceDetector::new();
        let mut networks = HashMap::new();
        networks.insert("eth0".to_string(), create_network_stats(100, 200));

        detector.detect_interface("container1", &networks, Some("auto"));
        detector.detect_interface("container2", &networks, Some("auto"));

        assert!(!detector.cache.is_empty());

        detector.clear_cache();
        assert!(detector.cache.is_empty());
    }
}
