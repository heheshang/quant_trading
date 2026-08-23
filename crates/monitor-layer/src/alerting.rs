use chrono::Utc;
use quant_common::types::{Alert, AlertLevel};
use quant_repository::AlertRepository;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub struct AlertManager {
    alerts: Arc<RwLock<Vec<Alert>>>,
    repo: Option<Arc<dyn AlertRepository>>,
    email_enabled: bool,
    webhook_urls: Vec<String>,
    http_client: Option<Client>,
    rate_limiter: Arc<RwLock<HashMap<String, Instant>>>,
    rate_limit_duration: Duration,
}

impl AlertManager {
    pub fn new(email_enabled: bool, webhook_urls: Vec<String>) -> Self {
        let http_client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| {
                error!("Failed to build alert HTTP client: {}", e);
                e
            })
            .ok();

        Self {
            alerts: Arc::new(RwLock::new(Vec::new())),
            repo: None,
            email_enabled,
            webhook_urls,
            http_client,
            rate_limiter: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_duration: Duration::from_secs(60), // 1 minute rate limit
        }
    }

    /// Inject a persistent alert repository. Reads/acknowledgements prefer the
    /// repository, falling back to in-memory storage on any DB error.
    pub fn with_repository(mut self, repo: Option<Arc<dyn AlertRepository>>) -> Self {
        self.repo = repo;
        self
    }

    pub async fn send_alert(&self, alert: Alert) {
        // Log the alert based on its level
        match &alert.level {
            AlertLevel::Info => info!("[ALERT] {}", alert.message),
            AlertLevel::Warning => warn!("[ALERT] {}", alert.message),
            AlertLevel::Critical => error!("[ALERT] {}", alert.message),
        }

        // Add alert to internal storage (using the DB-assigned id when a
        // repository is wired; persistence failure never blocks delivery).
        {
            let mut alerts = self.alerts.write().await;
            let mut stored = alert.clone();
            if let Some(repo) = self.repo.as_ref() {
                match repo.insert(&alert).await {
                    Ok(persisted) => stored.alert_id = persisted.alert_id,
                    Err(e) => error!("Failed to persist alert to repository: {}", e),
                }
            }
            alerts.push(stored);

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

        let Some(client) = self.http_client.as_ref() else {
            error!("Alert HTTP client unavailable; webhook skipped: {}", url);
            return;
        };

        match client.post(url).json(&payload).send().await {
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
        if let Some(repo) = self.repo.as_ref() {
            match repo.find_all(1000, 0).await {
                Ok(alerts) => return alerts,
                Err(e) => error!("Failed to read alerts from repository: {}", e),
            }
        }
        self.alerts.read().await.clone()
    }

    pub async fn get_alerts_by_level(&self, level: AlertLevel) -> Vec<Alert> {
        if let Some(repo) = self.repo.as_ref() {
            let level_str = match level {
                AlertLevel::Info => "Info",
                AlertLevel::Warning => "Warning",
                AlertLevel::Critical => "Critical",
            };
            match repo.find_by_level(level_str, 1000, 0).await {
                Ok(alerts) => return alerts,
                Err(e) => error!("Failed to read alerts by level: {}", e),
            }
        }
        let alerts = self.alerts.read().await;
        alerts
            .iter()
            .filter(|alert| alert.level == level)
            .cloned()
            .collect()
    }

    pub async fn get_alerts_by_source(&self, source: &str) -> Vec<Alert> {
        if let Some(repo) = self.repo.as_ref() {
            match repo.find_by_source(source, 1000, 0).await {
                Ok(alerts) => return alerts,
                Err(e) => error!("Failed to read alerts by source: {}", e),
            }
        }
        let alerts = self.alerts.read().await;
        alerts
            .iter()
            .filter(|alert| alert.source == source)
            .cloned()
            .collect()
    }

    pub async fn acknowledge_alert(&self, alert_id: i64) -> bool {
        let mut acknowledged = false;
        if let Some(repo) = self.repo.as_ref() {
            match repo.acknowledge(alert_id).await {
                Ok(updated) => acknowledged = updated,
                Err(e) => error!(
                    "Failed to acknowledge alert {} in repository: {}",
                    alert_id, e
                ),
            }
        }
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.acknowledged = true;
            acknowledged = true;
        }
        acknowledged
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
        if let Some(repo) = self.repo.as_ref() {
            match repo.find_by_time_range(start, end, 1000, 0).await {
                Ok(alerts) => return alerts,
                Err(e) => error!("Failed to read alerts by time range: {}", e),
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use parking_lot::Mutex;
    use quant_repository::{AlertRepository, RepoError};

    /// In-memory [`AlertRepository`] used to exercise repo-backed alert flows.
    struct InMemoryAlertRepository {
        alerts: Mutex<Vec<Alert>>,
    }

    impl InMemoryAlertRepository {
        fn new() -> Self {
            Self {
                alerts: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl AlertRepository for InMemoryAlertRepository {
        async fn insert(&self, alert: &Alert) -> Result<Alert, RepoError> {
            let mut alerts = self.alerts.lock();
            let next_id = alerts.len() as i64 + 1;
            let mut stored = alert.clone();
            stored.alert_id = next_id;
            alerts.push(stored.clone());
            Ok(stored)
        }

        async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Alert>, RepoError> {
            let alerts = self.alerts.lock();
            Ok(alerts
                .iter()
                .rev()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect())
        }

        async fn find_by_level(
            &self,
            level: &str,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<Alert>, RepoError> {
            let alerts = self.alerts.lock();
            Ok(alerts
                .iter()
                .filter(|a| format!("{:?}", a.level) == level)
                .rev()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect())
        }

        async fn find_by_source(
            &self,
            source: &str,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<Alert>, RepoError> {
            let alerts = self.alerts.lock();
            Ok(alerts
                .iter()
                .filter(|a| a.source == source)
                .rev()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect())
        }

        async fn find_by_time_range(
            &self,
            start: DateTime<Utc>,
            end: DateTime<Utc>,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<Alert>, RepoError> {
            let alerts = self.alerts.lock();
            Ok(alerts
                .iter()
                .filter(|a| a.timestamp >= start && a.timestamp <= end)
                .rev()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect())
        }

        async fn acknowledge(&self, alert_id: i64) -> Result<bool, RepoError> {
            let mut alerts = self.alerts.lock();
            if let Some(alert) = alerts.iter_mut().find(|a| a.alert_id == alert_id) {
                alert.acknowledged = true;
                Ok(true)
            } else {
                Ok(false)
            }
        }

        async fn count(&self) -> Result<i64, RepoError> {
            Ok(self.alerts.lock().len() as i64)
        }
    }

    #[tokio::test]
    async fn test_send_alert_persists_to_repository() {
        let repo = Arc::new(InMemoryAlertRepository::new());
        let manager = AlertManager::new(false, vec![]).with_repository(Some(repo.clone()));

        let alert = Alert::new(AlertLevel::Warning, "test".to_string(), "hello".to_string());
        manager.send_alert(alert).await;

        // Repository-backed read returns the alert with a DB-assigned id.
        let alerts = manager.get_alerts().await;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].message, "hello");
        assert_eq!(alerts[0].alert_id, 1);
    }

    #[tokio::test]
    async fn test_repo_acknowledge_reflected_in_reads() {
        let repo = Arc::new(InMemoryAlertRepository::new());
        let manager = AlertManager::new(false, vec![]).with_repository(Some(repo.clone()));

        let alert = Alert::new(
            AlertLevel::Critical,
            "risk".to_string(),
            "breach".to_string(),
        );
        manager.send_alert(alert).await;

        let alerts = manager.get_alerts().await;
        let id = alerts[0].alert_id;
        assert!(manager.acknowledge_alert(id).await);

        let refreshed = manager.get_alerts().await;
        assert!(refreshed[0].acknowledged);
    }

    #[tokio::test]
    async fn test_memory_fallback_when_repo_errors() {
        // No repository wired → pure in-memory path still works.
        let manager = AlertManager::new(false, vec![]);
        let alert = Alert::new(AlertLevel::Info, "s".to_string(), "m".to_string());
        manager.send_alert(alert).await;
        assert_eq!(manager.get_alerts().await.len(), 1);
    }
}
