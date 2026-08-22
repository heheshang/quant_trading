//! Application services container.
//!
//! Holds all domain services and their shared dependencies.
//! This is the single entry point for business logic.
//!
//! **Design note (DIP / SRP):** `AppServices` is a *composition* (assembly)
//! root only — it owns the shared infrastructure and exposes a narrow set of
//! domain services. Business orchestration that spans multiple services
//! (e.g. [`OrderProcessor`]) lives in dedicated use-cases, not in the command
//! (Tauri) layer.

use data_layer::market_data_repo::MarketDataRepository;
use data_layer::OkxDataSource;
use exchange_binance::ClientInterface as BinanceClientInterface;
use exchange_okx::ClientInterface;
use monitor_engine::LogBuffer;
use quant_clients::RedisCache;
use quant_common::config::AppConfig;
use quant_repository::{PgBacktestRepository, PgStrategyRepository, PostgresClient};
use std::path::PathBuf;
use std::sync::Arc;
use strategy_engine::registry::default_registry;
use strategy_engine::scheduler::StrategyScheduler;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use trading_engine::{OkxExecutor, OrderManager};

type SharedClient = Arc<RwLock<dyn ClientInterface + Send + Sync>>;
type SharedBinance = Arc<RwLock<Option<Arc<dyn BinanceClientInterface + Send + Sync>>>>;

use crate::account_service::AccountService;
use crate::auth_service::AuthService;
use crate::binance_service::BinanceService;
use crate::config_service::ConfigService;
use crate::market_data_provider::LockingProvider;
use crate::market_service::MarketService;
use crate::okx_service::OkxService;
use crate::order_processor::OrderProcessor;
use crate::risk_service::RiskService;
use crate::strategy_service::StrategyService;

/// Bundle of shared infrastructure references used to wire [`AppServices`].
///
/// Grouping these into a single value keeps the composition-root constructors
/// small (SRP) and makes the dependency footprint explicit. Constructed at the
/// call site (e.g. `src-tauri/src/main.rs`) via a struct literal.
pub struct SharedInfra {
    pub config: Arc<RwLock<AppConfig>>,
    pub postgres: Option<Arc<PostgresClient>>,
    pub redis: Option<Arc<RedisCache>>,
    pub market_data: Option<Arc<MarketDataRepository>>,
    pub okx_client: Arc<RwLock<Option<SharedClient>>>,
    pub okx_executor: Arc<RwLock<Option<Arc<OkxExecutor>>>>,
    pub okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
    pub binance_client: SharedBinance,
    pub order_manager: Arc<OrderManager>,
    pub log_buffer: Arc<LogBuffer>,
}

/// Shared application services container.
///
/// Each service receives only the dependencies it needs.
/// This struct owns the shared infrastructure and creates services from it.
pub struct AppServices {
    // Shared infrastructure (owned by the composition root)
    pub config: Arc<RwLock<AppConfig>>,
    pub postgres: Option<Arc<PostgresClient>>,
    pub redis: Option<Arc<RedisCache>>,
    pub market_data: Option<Arc<MarketDataRepository>>,
    pub okx_client: Arc<RwLock<Option<SharedClient>>>,
    pub okx_executor: Arc<RwLock<Option<Arc<OkxExecutor>>>>,
    pub okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
    pub order_manager: Arc<OrderManager>,
    pub log_buffer: Arc<LogBuffer>,

    // Domain services
    pub config_service: ConfigService,
    pub auth_service: AuthService,
    pub account_service: Arc<AccountService>,
    pub market_service: Arc<MarketService>,
    pub strategy_service: StrategyService,
    pub okx_service: OkxService,
    pub binance_service: BinanceService,
    pub risk_service: Arc<RiskService>,

    // Cross-service use-cases
    pub order_processor: OrderProcessor,
}

impl AppServices {
    fn build_strategy_service(
        postgres: &Option<Arc<PostgresClient>>,
        okx_data_source: &Arc<RwLock<Option<OkxDataSource>>>,
        scheduler: Arc<StrategyScheduler>,
    ) -> StrategyService {
        let mut strategy_service = StrategyService::new(
            postgres.clone(),
            Some(Arc::new(LockingProvider::new(okx_data_source.clone()))),
            postgres.as_ref().map(|pg| {
                Arc::new(PgBacktestRepository::new(Arc::new(pg.pool().clone())))
                    as Arc<dyn quant_repository::BacktestRepository>
            }),
            postgres.as_ref().map(|pg| {
                Arc::new(PgStrategyRepository::new(Arc::new(pg.pool().clone())))
                    as Arc<dyn quant_repository::StrategyRepository>
            }),
            Some(scheduler),
        );
        strategy_service.set_registry(Arc::new(default_registry()));
        strategy_service
    }

    /// Construct `AppServices` from a [`SharedInfra`] bundle (in-memory config).
    #[instrument(skip_all)]
    pub fn new(infra: SharedInfra) -> Self {
        info!("Initializing AppServices");
        // Non-blocking read: `new` may be called from an async runtime (e.g.
        // Tauri's `main`), where `blocking_read()` would panic.
        let scheduler_config = infra
            .config
            .try_read()
            .map(|c| c.scheduler.clone())
            .unwrap_or_default();
        let scheduler = Arc::new(StrategyScheduler::new(scheduler_config));
        let strategy_service =
            Self::build_strategy_service(&infra.postgres, &infra.okx_data_source, scheduler);
        let config_service = ConfigService::new(infra.config.clone());
        Self::assemble(infra, strategy_service, config_service)
    }

    /// Construct `AppServices` with a config file path for persistence.
    #[instrument(skip_all)]
    pub fn with_config_path(infra: SharedInfra, config_path: PathBuf) -> Self {
        info!(
            "Initializing AppServices with config path: {}",
            config_path.display()
        );
        let scheduler_config = infra
            .config
            .try_read()
            .map(|c| c.scheduler.clone())
            .unwrap_or_default();
        let scheduler = Arc::new(StrategyScheduler::new(scheduler_config));
        let strategy_service =
            Self::build_strategy_service(&infra.postgres, &infra.okx_data_source, scheduler);
        let config_service = ConfigService::with_path(infra.config.clone(), config_path);
        Self::assemble(infra, strategy_service, config_service)
    }

    /// Shared assembly used by both constructors.
    fn assemble(
        infra: SharedInfra,
        strategy_service: StrategyService,
        config_service: ConfigService,
    ) -> Self {
        let SharedInfra {
            config,
            postgres,
            redis,
            market_data,
            okx_client,
            okx_executor,
            okx_data_source,
            binance_client,
            order_manager,
            log_buffer,
        } = infra;

        let account_service = Arc::new(AccountService::new(postgres.clone()));
        let market_service = Arc::new(MarketService::new(okx_data_source.clone()));
        let risk_service = Arc::new(RiskService::new(postgres.clone()));

        let order_processor = OrderProcessor::new(
            config.clone(),
            okx_executor.clone(),
            order_manager.clone(),
            log_buffer.clone(),
            market_service.clone(),
            risk_service.clone(),
            account_service.clone(),
        );

        Self {
            config_service,
            auth_service: AuthService::new(config.clone(), postgres.clone()),
            account_service,
            market_service,
            strategy_service,
            okx_service: OkxService::new(
                okx_client.clone(),
                okx_executor.clone(),
                okx_data_source.clone(),
            ),
            binance_service: BinanceService::new(binance_client.clone()),
            risk_service,
            order_processor,
            config,
            postgres,
            redis,
            market_data,
            okx_client,
            okx_executor,
            okx_data_source,
            order_manager,
            log_buffer,
        }
    }
}
