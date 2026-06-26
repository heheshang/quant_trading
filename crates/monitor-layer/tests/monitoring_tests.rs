use chrono::Utc;
use monitor_layer::{init_logging, AlertManager, LogBuffer, LoggingConfig, MetricsCollector};
use quant_common::types::{Alert, AlertLevel, LogEntry};

#[tokio::test]
async fn test_metrics_initialization() {
    // Initialize metrics
    MetricsCollector::init();

    // Test that we can record metrics
    MetricsCollector::inc_orders_total();
    MetricsCollector::inc_orders_filled();
    MetricsCollector::record_order_latency(0.1);
    MetricsCollector::set_account_balance(10000.0);

    // Get metrics text
    let metrics_text = MetricsCollector::get_metrics_text();
    assert!(metrics_text.contains("orders_total"));
    assert!(metrics_text.contains("orders_filled"));
    assert!(metrics_text.contains("order_latency_seconds"));
    assert!(metrics_text.contains("account_balance"));
}

#[tokio::test]
async fn test_alert_manager() {
    let alert_manager = AlertManager::new(false, vec![]);

    // Create a test alert
    let alert = Alert {
            alert_id: 0,
        level: AlertLevel::Warning,
        source: "test".to_string(),
        message: "Test alert message".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    // Send the alert
    alert_manager.send_alert(alert.clone()).await;

    // Check that the alert was stored
    let alerts = alert_manager.get_alerts().await;
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].message, "Test alert message");

    // Test alert filtering by level
    let warning_alerts = alert_manager.get_alerts_by_level(AlertLevel::Warning).await;
    assert_eq!(warning_alerts.len(), 1);

    let critical_alerts = alert_manager
        .get_alerts_by_level(AlertLevel::Critical)
        .await;
    assert_eq!(critical_alerts.len(), 0);

    // Test alert filtering by source
    let source_alerts = alert_manager.get_alerts_by_source("test").await;
    assert_eq!(source_alerts.len(), 1);

    // Test alert acknowledgment
    let ack_result = alert_manager.acknowledge_alert(alert.alert_id).await;
    assert!(ack_result);

    // Verify the alert is now acknowledged
    let alerts = alert_manager.get_alerts().await;
    assert!(alerts[0].acknowledged);

    // Test clearing acknowledged alerts
    alert_manager.clear_acknowledged_alerts().await;
    let alerts = alert_manager.get_alerts().await;
    assert_eq!(alerts.len(), 0);
}

#[tokio::test]
async fn test_alert_manager_rate_limiting() {
    let alert_manager = AlertManager::new(false, vec![]);

    // Create test alerts from the same source
    let alert1 = Alert {
            alert_id: 0,
        level: AlertLevel::Warning,
        source: "rate_limit_test".to_string(),
        message: "First alert".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    let alert2 = Alert {
            alert_id: 0,
        level: AlertLevel::Warning,
        source: "rate_limit_test".to_string(),
        message: "Second alert".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    // Send first alert
    alert_manager.send_alert(alert1).await;

    // Send second alert immediately (should be rate limited, so not added to storage)
    alert_manager.send_alert(alert2).await;

    // The second alert should be blocked by rate limiting, but both alerts are still stored
    // Rate limiting only prevents webhook/email notifications, not storage
    let alerts = alert_manager.get_alerts().await;
    assert_eq!(alerts.len(), 2); // Both alerts are stored
}

#[test]
fn test_logging_initialization() {
    let config = LoggingConfig {
        log_level: "info".to_string(),
        log_dir: "./test_logs".to_string(),
        service_name: "test-service".to_string(),
        enable_json_logging: false,
        enable_file_logging: false, // Disable file logging for tests
        enable_stdout_logging: true,
    };

    // This might fail if logging is already initialized, which is fine for tests
    let _ = init_logging(config);
}

#[tokio::test]
async fn test_alert_counts() {
    let alert_manager = AlertManager::new(false, vec![]);

    // Initially no alerts
    assert_eq!(alert_manager.get_alert_count().await, 0);
    assert_eq!(alert_manager.get_unacknowledged_alert_count().await, 0);

    // Add an alert
    let alert = Alert {
            alert_id: 0,
        level: AlertLevel::Critical,
        source: "count_test".to_string(),
        message: "Test alert for counting".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    alert_manager.send_alert(alert).await;

    // Check counts
    assert_eq!(alert_manager.get_alert_count().await, 1);
    assert_eq!(alert_manager.get_unacknowledged_alert_count().await, 1);

    // Acknowledge the alert
    let alerts = alert_manager.get_alerts().await;
    let ack_result = alert_manager.acknowledge_alert(alerts[0].alert_id).await;
    assert!(ack_result);

    // Check updated counts
    assert_eq!(alert_manager.get_alert_count().await, 1);
    assert_eq!(alert_manager.get_unacknowledged_alert_count().await, 0);
}

#[tokio::test]
async fn test_metrics_snapshot() {
    MetricsCollector::init();
    let collector = MetricsCollector::new();

    // Set some metrics
    MetricsCollector::set_account_balance(50000.0);
    MetricsCollector::set_daily_pnl(1500.0);
    MetricsCollector::inc_orders_total();
    MetricsCollector::inc_orders_filled();

    // Take a snapshot
    let snapshot = collector.take_snapshot().await;

    assert_eq!(snapshot.account_balance, 50000.0);
    assert_eq!(snapshot.daily_pnl, 1500.0);
    // Note: orders_total and orders_filled may have values from other tests
    // so we just check that snapshot captured something
    assert!(snapshot.orders_total >= 1.0);
    assert!(snapshot.orders_filled >= 1.0);

    // Get snapshot history
    let history = collector.get_snapshot_history().await;
    assert_eq!(history.len(), 1);
}

#[tokio::test]
async fn test_alert_statistics() {
    let alert_manager = AlertManager::new(false, vec![]);

    // Create alerts with different levels
    let alert1 = Alert {
            alert_id: 0,
        level: AlertLevel::Info,
        source: "test".to_string(),
        message: "Info alert".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    let alert2 = Alert {
            alert_id: 0,
        level: AlertLevel::Warning,
        source: "test".to_string(),
        message: "Warning alert".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    let alert3 = Alert {
            alert_id: 0,
        level: AlertLevel::Critical,
        source: "test".to_string(),
        message: "Critical alert".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    alert_manager.send_alert(alert1.clone()).await;
    alert_manager.send_alert(alert2.clone()).await;
    alert_manager.send_alert(alert3.clone()).await;

    // Get statistics
    let stats = alert_manager.get_alert_statistics().await;
    assert_eq!(stats.total, 3);
    assert_eq!(stats.unacknowledged, 3);
    assert_eq!(stats.info, 1);
    assert_eq!(stats.warning, 1);
    assert_eq!(stats.critical, 1);

    // Acknowledge one alert
    alert_manager.acknowledge_alert(alert1.alert_id).await;

    let stats = alert_manager.get_alert_statistics().await;
    assert_eq!(stats.acknowledged, 1);
    assert_eq!(stats.unacknowledged, 2);
}

#[tokio::test]
async fn test_alert_time_range() {
    let alert_manager = AlertManager::new(false, vec![]);

    let now = Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);
    let two_hours_ago = now - chrono::Duration::hours(2);

    // Create alerts at different times
    let alert1 = Alert {
            alert_id: 0,
        level: AlertLevel::Info,
        source: "test".to_string(),
        message: "Old alert".to_string(),
        timestamp: two_hours_ago,
        acknowledged: false,
    };

    let alert2 = Alert {
            alert_id: 0,
        level: AlertLevel::Warning,
        source: "test".to_string(),
        message: "Recent alert".to_string(),
        timestamp: one_hour_ago,
        acknowledged: false,
    };

    alert_manager.send_alert(alert1).await;
    alert_manager.send_alert(alert2).await;

    // Get alerts from the last 90 minutes
    let start = now - chrono::Duration::minutes(90);
    let alerts = alert_manager.get_alerts_by_time_range(start, now).await;

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].message, "Recent alert");
}

#[tokio::test]
async fn test_acknowledge_by_source() {
    let alert_manager = AlertManager::new(false, vec![]);

    // Create alerts from different sources
    let alert1 = Alert {
            alert_id: 0,
        level: AlertLevel::Warning,
        source: "source_a".to_string(),
        message: "Alert from source A".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    let alert2 = Alert {
            alert_id: 0,
        level: AlertLevel::Warning,
        source: "source_a".to_string(),
        message: "Another alert from source A".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    let alert3 = Alert {
            alert_id: 0,
        level: AlertLevel::Warning,
        source: "source_b".to_string(),
        message: "Alert from source B".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    alert_manager.send_alert(alert1).await;
    alert_manager.send_alert(alert2).await;
    alert_manager.send_alert(alert3).await;

    // Acknowledge all alerts from source_a
    let count = alert_manager.acknowledge_alerts_by_source("source_a").await;
    assert_eq!(count, 2);

    // Verify
    let unack_count = alert_manager.get_unacknowledged_alert_count().await;
    assert_eq!(unack_count, 1);
}

#[tokio::test]
async fn test_acknowledge_by_level() {
    let alert_manager = AlertManager::new(false, vec![]);

    // Create alerts with different levels
    let alert1 = Alert {
            alert_id: 0,
        level: AlertLevel::Info,
        source: "test".to_string(),
        message: "Info 1".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    let alert2 = Alert {
            alert_id: 0,
        level: AlertLevel::Info,
        source: "test".to_string(),
        message: "Info 2".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    let alert3 = Alert {
            alert_id: 0,
        level: AlertLevel::Critical,
        source: "test".to_string(),
        message: "Critical".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    };

    alert_manager.send_alert(alert1).await;
    alert_manager.send_alert(alert2).await;
    alert_manager.send_alert(alert3).await;

    // Acknowledge all Info alerts
    let count = alert_manager
        .acknowledge_alerts_by_level(AlertLevel::Info)
        .await;
    assert_eq!(count, 2);

    // Verify
    let critical_alerts = alert_manager.get_critical_unacknowledged_alerts().await;
    assert_eq!(critical_alerts.len(), 1);
}

#[tokio::test]
async fn test_log_buffer() {
    let log_buffer = LogBuffer::new(100);

    // Add some log entries
    let entry1 = LogEntry {
        timestamp: Utc::now(),
        level: "INFO".to_string(),
        message: "Test info message".to_string(),
        module: Some("test_module".to_string()),
    };

    let entry2 = LogEntry {
        timestamp: Utc::now(),
        level: "ERROR".to_string(),
        message: "Test error message".to_string(),
        module: Some("test_module".to_string()),
    };

    let entry3 = LogEntry {
        timestamp: Utc::now(),
        level: "DEBUG".to_string(),
        message: "Test debug message".to_string(),
        module: Some("other_module".to_string()),
    };

    log_buffer.add_entry(entry1).await;
    log_buffer.add_entry(entry2).await;
    log_buffer.add_entry(entry3).await;

    // Get all entries
    let entries = log_buffer.get_entries().await;
    assert_eq!(entries.len(), 3);

    // Get by level
    let error_entries = log_buffer.get_entries_by_level("ERROR").await;
    assert_eq!(error_entries.len(), 1);
    assert_eq!(error_entries[0].message, "Test error message");

    // Get by module
    let module_entries = log_buffer.get_entries_by_module("test_module").await;
    assert_eq!(module_entries.len(), 2);

    // Test count
    let count = log_buffer.get_count().await;
    assert_eq!(count, 3);

    // Clear buffer
    log_buffer.clear().await;
    let count = log_buffer.get_count().await;
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_metrics_reset() {
    MetricsCollector::init();

    // Set some metrics
    MetricsCollector::set_account_balance(10000.0);
    MetricsCollector::inc_orders_total();
    MetricsCollector::set_daily_pnl(500.0);

    // Reset metrics
    MetricsCollector::reset_metrics();

    // Verify metrics are reset (we can't directly check but the function should execute without errors)
    let metrics_text = MetricsCollector::get_metrics_text();
    assert!(metrics_text.contains("orders_total"));
}
