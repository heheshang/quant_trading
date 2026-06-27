use quant_common::config::AppConfig;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};

/// Configuration management service.
#[derive(Debug)]
pub struct ConfigService {
    config: Arc<RwLock<AppConfig>>,
    config_path: Option<PathBuf>,
}

impl ConfigService {
    pub fn new(config: Arc<RwLock<AppConfig>>) -> Self {
        Self {
            config,
            config_path: None,
        }
    }

    /// Create ConfigService with a known config file path for persistence.
    pub fn with_path(config: Arc<RwLock<AppConfig>>, config_path: PathBuf) -> Self {
        Self {
            config,
            config_path: Some(config_path),
        }
    }

    #[instrument(skip_all)]
    pub async fn get_config(&self) -> AppConfig {
        let cfg = self.config.read().await.clone();
        info!("Config retrieved");
        cfg
    }

    #[instrument(skip(self, new_config))]
    pub async fn update_config(&self, new_config: AppConfig) -> String {
        // Update in-memory state
        {
            let mut cfg = self.config.write().await;
            *cfg = new_config.clone();
        }
        info!("Config updated in memory");

        // Persist to file if a config path is configured
        if let Some(ref path) = self.config_path {
            let toml_str = toml::to_string(&new_config)
                .unwrap_or_else(|e| {
                    warn!("Failed to serialize config: {}", e);
                    String::new()
                });
            if toml_str.is_empty() {
                return "Config updated in memory, but serialization failed".to_string();
            }
            if let Err(e) = tokio::fs::write(path, &toml_str).await {
                warn!("Failed to persist config to {}: {}", path.display(), e);
                return format!("Config updated in memory, but file write failed: {}", e);
            }
            info!("Config persisted to {}", path.display());
            "Config updated and persisted".to_string()
        } else {
            "Config updated in memory (no persistence path configured)".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_config_returns_default() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let service = ConfigService::new(config);
        let cfg = service.get_config().await;
        assert_eq!(cfg.database.host, "localhost");
        assert_eq!(cfg.database.port, 5432);
    }

    #[tokio::test]
    async fn test_update_config_mutates_value() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let service = ConfigService::new(config);

        let mut new_cfg = AppConfig::default();
        new_cfg.trading.enable_paper_trading = !new_cfg.trading.enable_paper_trading;
        service.update_config(new_cfg.clone()).await;

        let cfg = service.get_config().await;
        assert_eq!(
            cfg.trading.enable_paper_trading,
            new_cfg.trading.enable_paper_trading
        );
    }

    #[tokio::test]
    async fn test_concurrent_read_access() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let service = Arc::new(ConfigService::new(config));

        let mut handles = vec![];
        for _ in 0..10 {
            let svc = service.clone();
            handles.push(tokio::spawn(async move {
                let cfg = svc.get_config().await;
                assert_eq!(cfg.database.host, "localhost");
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_concurrent_read_and_write() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let service = Arc::new(ConfigService::new(config));

        let mut handles = vec![];
        for _ in 0..5 {
            let svc = service.clone();
            handles.push(tokio::spawn(async move {
                let cfg = svc.get_config().await;
                let _ = cfg.database.host;
            }));
        }

        {
            let svc = service.clone();
            handles.push(tokio::spawn(async move {
                let mut new_cfg = AppConfig::default();
                new_cfg.trading.max_orders_per_second = 999;
                svc.update_config(new_cfg).await;
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let final_cfg = service.get_config().await;
        assert_eq!(final_cfg.trading.max_orders_per_second, 999);
    }
}
