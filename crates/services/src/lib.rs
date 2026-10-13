//! # Quant Trading — Services Layer
//!
//! Business logic orchestration. Each domain has a dedicated service
//! that encapsulates operations, logging, and metrics.
//!
//! Services are stateless (take explicit dependencies) and testable.
//! They do NOT depend on Tauri — all framework wiring stays in `src-tauri/`.
//!
//! ## 模块结构
//! - `domain/` - 按业务域组织服务组合
//!   - `trading.rs` - 交易领域（账户、订单处理）
//!   - `strategy.rs` - 策略领域（策略服务、优化器）
//!   - `market.rs` - 市场领域（行情、实时交易）
//! - `infra/` - 基础设施服务（配置、认证、API 密钥）
//! - `shared/` - 共享组件（AppServices、SharedInfra）

// 模块定义
pub mod account_service;
pub mod api_key_service;
pub mod app_service;
pub mod auth_service;
pub mod binance_service;
pub mod config_service;
pub mod domain;
pub mod error;
pub mod infra;
pub mod live_trades_service;
pub mod market_data_provider;
pub mod market_service;
pub mod optimizer;
pub mod order_processor;
pub mod pipeline;
pub mod risk_service;
pub mod shared;
pub mod strategy_service;

// 重新导出公共 API（保持向后兼容）
pub use account_service::AccountService;
pub use api_key_service::{ApiKeyService, MaskedApiKey};
pub use app_service::{AppServices, SharedInfra};
pub use auth_service::{AuthService, Enable2faResult};
pub use binance_service::BinanceService;
pub use config_service::ConfigService;
pub use domain::{market, strategy, trading};
pub use error::{ServiceError, ServiceResult};
pub use infra::{AuthService as InfraAuthService, ConfigService as InfraConfigService};
pub use live_trades_service::LiveTradesService;
pub use market_service::MarketService;
pub use optimizer::{
    expand_grid, OptimizationAlgorithm, OptimizationMetric, OptimizationResult, ParamOptimizer,
    ParameterCombo,
};
pub use risk_service::RiskService;
pub use strategy_service::StrategyService;
