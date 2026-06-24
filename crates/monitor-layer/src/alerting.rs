use chrono::Utc;
use quant_common::types::{Alert, AlertLevel};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub struct AlertManager {
    alerts: Arc<RwLock<Vec<Alert>>>,
    email_enabled: bool,
    webhook_urls: Vec<String>,
    http_client: Client,
    rate_limiter: Arc<RwLock<HashMap<String, Instant>>>,
    rate_limit_duration: Duration,
}

impl AlertManager {
    pub fn new(email_enabled: bool, webhook_urls: Vec<String>) -> Self {
        Self {
            alerts: Arc::new(RwLock::new(Vec::new())),
            email_enabled,
            webhook_urls,
            http_client: Client::new(),
            rate_limiter: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_duration: Duration::from_secs(60), // 1 minute rate limit
        }
    }

    pub async fn send_alert(&self, alert: Alert) {
        // Log the alert based on its level
        match alert.level {
            AlertLevel::Info => info!("[ALERT] {}", alert.message),
            AlertLevel::Warning => warn!("[ALERT] {}", alert.message),
            AlertLevel::Critical => error!("[ALERT] {}", alert.message),
        }

        // Add alert to internal storage
        {
            let mut alerts = self.alerts.write().await;
            alerts.push(alert.clone());

            // Keep only the last 1000 alerts to prevent memory issues
            let len = alerts.len();
            if len > 1000 {
                alerts.drain(0..len - 1000);
            }
        }

        // Rate limiting check
        if self.is_rate_limited(&alert.source).await {
            info!("Alert from source {} rate limited", alert.source);
            return;
        }

        // Update rate limiter
        self.update_rate_limit(&alert.source).await;

        // Send notifications
        if self.email_enabled {
            self.send_email(&alert).await;
        }

        for webhook_url in &self.webhook_urls {
            self.send_webhook(webhook_url, &alert).await;
        }
    }

    async fn is_rate_limited(&self, source: &str) -> bool {
        let rate_limiter = self.rate_limiter.read().await;
        if let Some(last_alert_time) = rate_limiter.get(source) {
            last_alert_time.elapsed() < self.rate_limit_duration
        } else {
            false
        }
    }

    async fn update_rate_limit(&self, source: &str) {
        let mut rate_limiter = self.rate_limiter.write().await;
        rate_limiter.insert(source.to_string(), Instant::now());
    }

    async fn send_email(&self, alert: &Alert) {
        // In a real implementation, this would connect to an email service
        info!(
            "Email alert sent: [{:?}] {} - {}",
            alert.level, alert.source, alert.message
        );
    }

    async fn send_webhook(&self, url: &str, alert: &Alert) {
        let payload = json!({
            "alert_id": alert.alert_id,
            "level": alert.level,
            "source": alert.source,
            "message": alert.message,
            "timestamp": alert.timestamp,
            "acknowledged": alert.acknowledged
        });

        match self.http_client.post(url).json(&payload).send().await {
            Ok(response) => {
                info!(
                    "Webhook alert sent to {}: Status {}",
                    url,
                    response.status()
                );
            }
            Err(e) => {
                error!("Failed to send webhook alert to {}: {}", url, e);
            }
        }
    }

    pub async fn get_alerts(&self) -> Vec<Alert> {
        self.alerts.read().await.clone()
    }

    pub async fn get_alerts_by_level(&self, level: AlertLevel) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts
            .iter()
            .filter(|alert| alert.level == level)
            .cloned()
            .collect()
    }

    pub async fn get_alerts_by_source(&self, source: &str) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts
            .iter()
            .filter(|alert| alert.source == source)
            .cloned()
            .collect()
    }

    pub async fn acknowledge_alert(&self, alert_id: uuid::Uuid) -> bool {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.acknowledged = true;
            true
        } else {
            false
        }
    }

    pub async fn clear_acknowledged_alerts(&self) {
        let mut alerts = self.alerts.write().await;
        alerts.retain(|alert| !alert.acknowledged);
    }

    pub async fn get_alert_count(&self) -> usize {
        self.alerts.read().await.len()
    }

    pub async fn get_unacknowledged_alert_count(&self) -> usize {
        let alerts = self.alerts.read().await;
        alerts.iter().filter(|alert| !alert.acknowledged).count()
    }

    /// Get alerts within a time range
    pub async fn get_alerts_by_time_range(
        &self,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts
            .iter()
            .filter(|alert| alert.timestamp >= start && alert.timestamp <= end)
            .cloned()
            .collect()
    }

    /// Get critical unacknowledged alerts
    pub async fn get_critical_unacknowledged_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts
            .iter()
            .filter(|alert| alert.level == AlertLevel::Critical && !alert.acknowledged)
            .cloned()
            .collect()
    }

    /// Acknowledge all alerts from a specific source
    pub async fn acknowledge_alerts_by_source(&self, source: &str) -> usize {
        let mut alerts = self.alerts.write().await;
        let mut count = 0;
        for alert in alerts.iter_mut() {
            if alert.source == source && !alert.acknowledged {
                alert.acknowledged = true;
                count += 1;
            }
        }
        count
    }

    /// Acknowledge all alerts of a specific level
    pub async fn acknowledge_alerts_by_level(&self, level: AlertLevel) -> usize {
        let mut alerts = self.alerts.write().await;
        let mut count = 0;
        for alert in alerts.iter_mut() {
            if alert.level == level && !alert.acknowledged {
                alert.acknowledged = true;
                count += 1;
            }
        }
        count
    }

    /// Get alert statistics
    pub async fn get_alert_statistics(&self) -> AlertStatistics {
        let alerts = self.alerts.read().await;

        let total = alerts.len();
        let unacknowledged = alerts.iter().filter(|a| !a.acknowledged).count();
        let info_count = alerts
            .iter()
            .filter(|a| a.level == AlertLevel::Info)
            .count();
        let warning_count = alerts
            .iter()
            .filter(|a| a.level == AlertLevel::Warning)
            .count();
        let critical_count = alerts
            .iter()
            .filter(|a| a.level == AlertLevel::Critical)
            .count();

        AlertStatistics {
            total,
            acknowledged: total - unacknowledged,
            unacknowledged,
            info: info_count,
            warning: warning_count,
            critical: critical_count,
        }
    }

    /// Set rate limit duration
    pub fn set_rate_limit_duration(&mut self, duration: Duration) {
        self.rate_limit_duration = duration;
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertStatistics {
    pub total: usize,
    pub acknowledged: usize,
    pub unacknowledged: usize,
    pub info: usize,
    pub warning: usize,
    pub critical: usize,
}
