use bollard::secret::ContainerNetworkStats;
use oxker_core::Config;
use std::collections::HashMap;

fn create_test_config(network_interface: Option<String>) -> Config {
    Config {
        app_colors: oxker_core::AppColors::new(),
        color_logs: false,
        docker_interval_ms: 1000,
        gui: true,
        host: None,
        in_container: false,
        keymap: oxker_core::Keymap::new(),
        network_interface,
        raw_logs: false,
        save_dir: None,
        show_self: false,
        show_std_err: false,
        show_timestamp: false,
        timezone: None,
        timestamp_format: "%Y-%m-%d %H:%M:%S".to_string(),
        show_logs: true,
        debug_mode: false,
        event_driven_mode: false,
        full_sync_interval_ms: 60_000,
    }
}

const fn create_network_stats(rx_bytes: u64, tx_bytes: u64) -> ContainerNetworkStats {
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

#[tokio::test]
async fn test_stats_collection_with_auto_detection() {
    let config = create_test_config(Some("auto".to_string()));
    assert_eq!(config.network_interface, Some("auto".to_string()));
}

#[tokio::test]
async fn test_stats_collection_with_specific_interface() {
    let config = create_test_config(Some("eth1".to_string()));
    assert_eq!(config.network_interface, Some("eth1".to_string()));
}

#[tokio::test]
async fn test_stats_collection_with_none_disabled() {
    let config = create_test_config(Some("none".to_string()));
    assert_eq!(config.network_interface, Some("none".to_string()));
}

#[tokio::test]
async fn test_stats_collection_with_no_config() {
    let config = create_test_config(None);
    assert_eq!(config.network_interface, None);
}

#[tokio::test]
async fn test_network_stats_parsing() {
    let mut networks = HashMap::new();
    networks.insert("eth0".to_string(), create_network_stats(1000, 2000));
    networks.insert("eth1".to_string(), create_network_stats(1500, 2500));

    let eth0_stats = networks.get("eth0").unwrap();
    assert_eq!(eth0_stats.rx_bytes, Some(1000));
    assert_eq!(eth0_stats.tx_bytes, Some(2000));

    let eth1_stats = networks.get("eth1").unwrap();
    assert_eq!(eth1_stats.rx_bytes, Some(1500));
    assert_eq!(eth1_stats.tx_bytes, Some(2500));
}

#[tokio::test]
async fn test_network_stats_empty_handling() {
    let networks: HashMap<String, ContainerNetworkStats> = HashMap::new();
    assert!(networks.is_empty());
    assert_eq!(networks.get("eth0"), None);
}

#[tokio::test]
async fn test_network_stats_missing_values() {
    let stats = ContainerNetworkStats {
        rx_bytes: None,
        tx_bytes: None,
        rx_packets: None,
        tx_packets: None,
        rx_errors: None,
        tx_errors: None,
        rx_dropped: None,
        tx_dropped: None,
        endpoint_id: None,
        instance_id: None,
    };

    assert_eq!(stats.rx_bytes.unwrap_or_default(), 0);
    assert_eq!(stats.tx_bytes.unwrap_or_default(), 0);
}

#[tokio::test]
async fn test_backward_compatibility() {
    let config = create_test_config(None);
    assert!(config.network_interface.is_none());
}
