use quant_common::config::AppConfig;
use std::sync::Arc;
use tokio::sync::RwLock;
use monitor_layer::{AlertManager, LogBuffer};
use exchange_okx::Client as OkxClient;
use trading_layer::OkxExecutor;
use data_layer::OkxDataSource;

pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub alert_manager: Arc<AlertManager>,
    pub log_buffer: Arc<LogBuffer>,
    pub okx_client: Arc<RwLock<Option<Arc<RwLock<OkxClient>>>>>,
    pub okx_executor: Arc<RwLock<Option<OkxExecutor>>>,
    pub okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
}
