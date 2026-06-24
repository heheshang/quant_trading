use data_layer::{OkxDataSource, PostgresClient, RedisCache};
use exchange_okx::Client as OkxClient;
use monitor_layer::{AlertManager, LogBuffer};
use quant_common::config::AppConfig;
use std::sync::Arc;
use tokio::sync::RwLock;
use trading_layer::OkxExecutor;

#[expect(dead_code)]
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub alert_manager: Arc<AlertManager>,
    pub log_buffer: Arc<LogBuffer>,
    pub pg_client: Option<PostgresClient>,
    pub redis_cache: Option<RedisCache>,
    pub okx_client: Arc<RwLock<Option<Arc<RwLock<OkxClient>>>>>,
    pub okx_executor: Arc<RwLock<Option<OkxExecutor>>>,
    pub okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
}
