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

use data_layer::market_data::DataSource;
use data_layer::market_data_repo::MarketDataRepository;
use exchange_binance::ClientInterface as BinanceClientInterface;
use monitor_engine::LogBuffer;
use quant_clients::RedisCache;
use quant_common::config::AppConfig;
use quant_common::MarketDataProvider;
use data_layer::{PgBacktestRepository, PgStrategyRepository, PostgresClient};
use std::path::PathBuf;
use std::sync::Arc;
use strategy_engine::registry::default_registry;
use strategy_engine::scheduler::StrategyScheduler;
use strategy_engine::PipelineExecutor;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use trading_engine::{BinanceExecutor, OrderManager};

type SharedBinance = Arc<RwLock<Option<Arc<dyn BinanceClientInterface + Send + Sync>>>>;

use crate::account_service::AccountService;
use crate::api_key_service::ApiKeyService;
use crate::auth_service::AuthService;
use crate::binance_service::BinanceService;
use crate::live_trades_service::LiveTradesService;
use crate::config_service::ConfigService;
use crate::market_data_provider::{
    resolve_default_timeframe, MarketDataStore, RepositoryMarketDataProvider,
};
use crate::market_service::MarketService;
use crate::optimizer::ParamOptimizer;
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
    pub market_data_source: Arc<RwLock<Option<Arc<dyn DataSource>>>>,
    pub live_market_data_provider: Option<Arc<dyn MarketDataProvider>>,
    pub binance_client: SharedBinance,
    pub binance_executor: Arc<RwLock<Option<Arc<BinanceExecutor>>>>,
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
    pub order_manager: Arc<OrderManager>,
    pub log_buffer: Arc<LogBuffer>,

    // Domain services
    pub config_service: ConfigService,
    pub auth_service: AuthService,
    pub account_service: Arc<AccountService>,
    pub market_service: Arc<MarketService>,
    pub strategy_service: StrategyService,
    pub binance_service: BinanceService,
    pub live_trades: LiveTradesService,
    pub risk_service: Arc<RiskService>,
    pub api_key_service: ApiKeyService,

    // Cross-service use-cases
    pub order_processor: Arc<OrderProcessor>,
    // Strategy scheduler — shared by `StrategyService` for lifecycle wiring and
    // configured here (provider + pipeline) so `start_strategy` actually trades.
    pub scheduler: Arc<StrategyScheduler>,
    // Parameter optimizer (GridSearch)
    pub optimizer: ParamOptimizer,
}

impl AppServices {
    fn build_strategy_service(
        postgres: &Option<Arc<PostgresClient>>,
        market_data: &Option<Arc<MarketDataRepository>>,
        default_timeframe: String,
        scheduler: Arc<StrategyScheduler>,
        live_market_data_provider: Option<Arc<dyn MarketDataProvider>>,
    ) -> StrategyService {
        // Build a single persistence-first market-data provider and hand it to
        // BOTH the StrategyService and the strategy scheduler so a started
        // strategy can fetch real historical data for signal generation.
        //
        // The provider reads Postgres `market_data` (written by the data-puller)
        // at the default timeframe first and falls back to the live Binance
        // source when the repository is unconfigured, empty, or errors.
        let repo: Option<Arc<dyn MarketDataStore>> = market_data
            .as_ref()
            .map(|r| r.clone() as Arc<dyn MarketDataStore>);
        let provider: Arc<dyn MarketDataProvider> = Arc::new(RepositoryMarketDataProvider::new(
            repo,
            live_market_data_provider,
            default_timeframe,
        ));
        scheduler.set_market_data_provider(provider.clone());
        // Wire the order pipeline so `trading_ready()` passes — without it the
        // scheduler refuses to start a strategy ("signal pipeline not wired").
        scheduler.set_pipeline(Arc::new(PipelineExecutor::new()));
        let mut strategy_service = StrategyService::new(
            postgres.clone(),
            Some(provider),
            postgres.as_ref().map(|pg| {
                Arc::new(PgBacktestRepository::new(Arc::new(pg.pool().clone())))
                    as Arc<dyn data_layer::BacktestRepository>
            }),
            postgres.as_ref().map(|pg| {
                Arc::new(PgStrategyRepository::new(Arc::new(pg.pool().clone())))
                    as Arc<dyn data_layer::StrategyRepository>
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
        let default_timeframe = {
            let bars = infra
                .config
                .try_read()
                .map(|c| c.data_puller.candle.bars.clone())
                .unwrap_or_default();
            resolve_default_timeframe(&bars)
        };
        let strategy_service = Self::build_strategy_service(
            &infra.postgres,
            &infra.market_data,
            default_timeframe,
            scheduler.clone(),
            infra.live_market_data_provider.clone(),
        );
        let config_service = ConfigService::new(infra.config.clone());
        Self::assemble(infra, strategy_service, config_service, scheduler)
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
        let default_timeframe = {
            let bars = infra
                .config
                .try_read()
                .map(|c| c.data_puller.candle.bars.clone())
                .unwrap_or_default();
            resolve_default_timeframe(&bars)
        };
        let strategy_service = Self::build_strategy_service(
            &infra.postgres,
            &infra.market_data,
            default_timeframe,
            scheduler.clone(),
            infra.live_market_data_provider.clone(),
        );
        let config_service = ConfigService::with_path(infra.config.clone(), config_path);
        Self::assemble(infra, strategy_service, config_service, scheduler)
    }

    /// Shared assembly used by both constructors.
    fn assemble(
        infra: SharedInfra,
        strategy_service: StrategyService,
        config_service: ConfigService,
        scheduler: Arc<StrategyScheduler>,
    ) -> Self {
        let SharedInfra {
            config,
            postgres,
            redis,
            market_data,
            market_data_source,
            live_market_data_provider: _live_market_data_provider,
            binance_client,
            binance_executor,
            order_manager,
            log_buffer,
        } = infra;

        let account_service = Arc::new(AccountService::new(postgres.clone()));
        let market_service = Arc::new(MarketService::new(market_data_source, market_data.clone()));
        let risk_service = Arc::new(RiskService::new(postgres.clone()));

        let order_processor = Arc::new(OrderProcessor::new(
            config.clone(),
            binance_executor.clone(),
            order_manager.clone(),
            log_buffer.clone(),
            market_service.clone(),
            risk_service.clone(),
            account_service.clone(),
        ));

        // Wire the strategy scheduler's pipeline so strategy-generated orders are
        // routed through `OrderProcessor` (risk + paper/real execution +
        // persistence). The scheduler was given its market-data provider in
        // `build_strategy_service`; the pipeline is supplied here once the
        // OrderProcessor exists.
        let live_pipeline = crate::pipeline::make_live_pipeline(order_processor.clone());
        scheduler.set_pipeline(Arc::new(live_pipeline));

        let api_key_repo = postgres.as_ref().map(|pg| {
            Arc::new(data_layer::PgApiKeyRepository::new(Arc::new(
                pg.pool().clone(),
            ))) as Arc<dyn data_layer::ApiKeyRepository>
        });
        let encryption_key = config
            .try_read()
            .map(|c| c.security.encryption_key.clone())
            .ok();
        let api_key_service = ApiKeyService::new(encryption_key, api_key_repo);
        let live_trades_repo = postgres.as_ref().map(|pg| {
            Arc::new(data_layer::LiveTradesRepository::new(pg.pool().clone()))
        });
        let live_trades = LiveTradesService::new(live_trades_repo);
        let param_optimizer_config = config
            .try_read()
            .map(|c| c.param_optimizer.clone())
            .unwrap_or_default();
        let optimizer = ParamOptimizer::new(Arc::new(default_registry()), param_optimizer_config);

        Self {
            config_service,
            auth_service: AuthService::new(config.clone(), postgres.clone()),
            account_service,
            market_service,
            strategy_service,
            binance_service: BinanceService::new(binance_client.clone()),
            live_trades,
            risk_service,
            order_processor,
            api_key_service,
            optimizer,
            scheduler,
            config,
            postgres,
            redis,
            market_data,
            order_manager,
            log_buffer,
        }
    }
}
