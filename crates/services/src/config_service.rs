use quant_common::config::AppConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration management service.
pub struct ConfigService {
    config: Arc<RwLock<AppConfig>>,
}

impl ConfigService {
    pub fn new(config: Arc<RwLock<AppConfig>>) -> Self {
        Self { config }
    }

    pub async fn get_config(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, new_config: AppConfig) {
        let mut cfg = self.config.write().await;
        *cfg = new_config;
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
