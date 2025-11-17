use quant_common::types::{Alert, AlertLevel};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub struct AlertManager {
    alerts: Arc<RwLock<Vec<Alert>>>,
    email_enabled: bool,
    webhook_url: Option<String>,
}

impl AlertManager {
    pub fn new(email_enabled: bool, webhook_url: Option<String>) -> Self {
        Self {
            alerts: Arc::new(RwLock::new(Vec::new())),
            email_enabled,
            webhook_url,
        }
    }

    pub async fn send_alert(&self, alert: Alert) {
        match alert.level {
            AlertLevel::Info => info!("{}", alert.message),
            AlertLevel::Warning => warn!("{}", alert.message),
            AlertLevel::Critical => error!("{}", alert.message),
        }

        let mut alerts = self.alerts.write().await;
        alerts.push(alert.clone());

        // 发送通知
        if self.email_enabled {
            self.send_email(&alert).await;
        }

        if let Some(ref webhook) = self.webhook_url {
            self.send_webhook(webhook, &alert).await;
        }
    }

    async fn send_email(&self, alert: &Alert) {
        // TODO: 实现邮件发送
        info!("Email alert: {}", alert.message);
    }

    async fn send_webhook(&self, url: &str, alert: &Alert) {
        // TODO: 实现Webhook调用
        info!("Webhook alert to {}: {}", url, alert.message);
    }

    pub async fn get_alerts(&self) -> Vec<Alert> {
        self.alerts.read().await.clone()
    }
}
