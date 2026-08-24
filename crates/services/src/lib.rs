//! # Quant Trading — Services Layer
//!
//! Business logic orchestration. Each domain has a dedicated service
//! that encapsulates operations, logging, and metrics.
//!
//! Services are stateless (take explicit dependencies) and testable.
//! They do NOT depend on Tauri — all framework wiring stays in `src-tauri/`.

pub mod account_service;
pub mod api_key_service;
pub mod app_service;
pub mod auth_service;
pub mod binance_service;
pub mod config_service;
pub mod error;
pub mod live_trades_service;
pub mod market_data_provider;
pub mod market_service;
pub mod optimizer;
pub mod order_processor;
pub mod pipeline;
pub mod risk_service;
pub mod strategy_service;

pub use account_service::AccountService;
pub use api_key_service::{ApiKeyService, MaskedApiKey};
pub use app_service::{AppServices, SharedInfra};
pub use auth_service::{AuthService, Enable2faResult};
pub use binance_service::BinanceService;
pub use config_service::ConfigService;
pub use error::{ServiceError, ServiceResult};
pub use live_trades_service::LiveTradesService;
pub use market_service::MarketService;
pub use optimizer::{
    expand_grid, OptimizationAlgorithm, OptimizationMetric, OptimizationResult, ParamOptimizer,
    ParameterCombo,
};
pub use risk_service::RiskService;
pub use strategy_service::StrategyService;
