//! # Quant Trading — Services Layer
//!
//! Business logic orchestration. Each domain has a dedicated service
//! that encapsulates operations, logging, and metrics.
//!
//! Services are stateless (take explicit dependencies) and testable.
//! They do NOT depend on Tauri — all framework wiring stays in `src-tauri/`.

pub mod account_service;
pub mod app_service;
pub mod auth_service;
pub mod config_service;
pub mod error;
pub mod market_data_provider;
pub mod market_service;
pub mod okx_service;
pub mod risk_service;
pub mod strategy_service;

pub use account_service::AccountService;
pub use app_service::AppServices;
pub use auth_service::AuthService;
pub use config_service::ConfigService;
pub use error::{ServiceError, ServiceResult};
pub use market_service::MarketService;
pub use okx_service::OkxService;
pub use risk_service::RiskService;
pub use strategy_service::StrategyService;
