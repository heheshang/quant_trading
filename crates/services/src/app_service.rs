//! Application services container.
//!
//! Holds all domain services and their shared dependencies.
//! This is the single entry point for business logic.

use data_layer::OkxDataSource;
use exchange_okx::ClientInterface;
use quant_clients::RedisCache;
use quant_common::config::AppConfig;
use quant_repository::{MarketDataRepository, PgBacktestRepository, PostgresClient};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use trading_engine::OkxExecutor;

use crate::account_service::AccountService;
use crate::auth_service::AuthService;
use crate::config_service::ConfigService;
use crate::market_data_provider::LockingProvider;
use crate::market_service::MarketService;
use crate::okx_service::OkxService;
use crate::risk_service::RiskService;
use crate::strategy_service::StrategyService;

/// Shared application services container.
///
/// Each service receives only the dependencies it needs.
/// This struct owns the shared infrastructure and creates services from it.
pub struct AppServices {
    // Shared infrastructure
    pub config: Arc<RwLock<AppConfig>>,
    pub postgres: Option<Arc<PostgresClient>>,
    pub redis: Option<Arc<RedisCache>>,
    pub market_data: Option<Arc<MarketDataRepository>>,
    pub okx_client: Arc<RwLock<Option<Arc<RwLock<dyn ClientInterface + Send + Sync>>>>>,
    pub okx_executor: Arc<RwLock<Option<Arc<OkxExecutor>>>>,
    pub okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,

    // Domain services
    pub config_service: ConfigService,
    pub auth_service: AuthService,
    pub account_service: AccountService,
    pub market_service: MarketService,
    pub strategy_service: StrategyService,
    pub okx_service: OkxService,
    pub risk_service: RiskService,
}

impl AppServices {
    /// Construct AppServices from raw infrastructure references.
    ///
    /// Each service is wired with only the dependencies it needs.
    #[instrument(skip_all)]
    pub fn new(
        config: Arc<RwLock<AppConfig>>,
        postgres: Option<Arc<PostgresClient>>,
        redis: Option<Arc<RedisCache>>,
        market_data: Option<Arc<MarketDataRepository>>,
        okx_client: Arc<RwLock<Option<Arc<RwLock<dyn ClientInterface + Send + Sync>>>>>,
        okx_executor: Arc<RwLock<Option<Arc<OkxExecutor>>>>,
        okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
    ) -> Self {
        info!("Initializing AppServices");
        Self {
            config_service: ConfigService::new(config.clone()),
            auth_service: AuthService::new(config.clone(), postgres.clone()),
            account_service: AccountService::new(postgres.clone()),
            market_service: MarketService::new(okx_data_source.clone()),
            strategy_service: StrategyService::new(
                postgres.clone(),
                Some(Arc::new(LockingProvider::new(okx_data_source.clone()))),
                postgres
                    .as_ref()
                    .map(|pg| Arc::new(PgBacktestRepository::new(Arc::new(pg.pool().clone())))
                        as Arc<dyn quant_repository::BacktestRepository>),
            ),
            okx_service: OkxService::new(
                okx_client.clone(),
                okx_executor.clone(),
                okx_data_source.clone(),
            ),
            risk_service: RiskService::new(postgres.clone()),
            config,
            postgres,
            redis,
            market_data,
            okx_client,
            okx_executor,
            okx_data_source,
        }
    }

    /// Construct AppServices with a config file path for persistence.
    #[instrument(skip_all)]
    pub fn with_config_path(
        config: Arc<RwLock<AppConfig>>,
        config_path: PathBuf,
        postgres: Option<Arc<PostgresClient>>,
        redis: Option<Arc<RedisCache>>,
        market_data: Option<Arc<MarketDataRepository>>,
        okx_client: Arc<RwLock<Option<Arc<RwLock<dyn ClientInterface + Send + Sync>>>>>,
        okx_executor: Arc<RwLock<Option<Arc<OkxExecutor>>>>,
        okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
    ) -> Self {
        info!("Initializing AppServices with config path: {}", config_path.display());
        Self {
            config_service: ConfigService::with_path(config.clone(), config_path),
            auth_service: AuthService::new(config.clone(), postgres.clone()),
            account_service: AccountService::new(postgres.clone()),
            market_service: MarketService::new(okx_data_source.clone()),
            strategy_service: StrategyService::new(
                postgres.clone(),
                Some(Arc::new(LockingProvider::new(okx_data_source.clone()))),
                postgres
                    .as_ref()
                    .map(|pg| Arc::new(PgBacktestRepository::new(Arc::new(pg.pool().clone())))
                        as Arc<dyn quant_repository::BacktestRepository>),
            ),
            okx_service: OkxService::new(
                okx_client.clone(),
                okx_executor.clone(),
                okx_data_source.clone(),
            ),
            risk_service: RiskService::new(postgres.clone()),
            config,
            postgres,
            redis,
            market_data,
            okx_client,
            okx_executor,
            okx_data_source,
        }
    }
}
