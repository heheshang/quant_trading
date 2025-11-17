use quant_common::config::AppConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
}
