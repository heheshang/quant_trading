pub mod api_keys;
pub mod audit;
pub mod auth;
pub mod backtest;
pub mod binance;
pub mod binance_ws;
pub mod core;
pub mod market_data;
pub mod optimizer;
pub mod strategy_risk;
pub mod twofa;

/// 认证失败的 `ApiFailure`（未登录/会话失效）。
pub fn auth_err(e: String) -> quant_common::api::ApiFailure {
    quant_common::api::ApiFailure::new(quant_common::api::code::UNAUTHORIZED, e)
}

/// 服务未初始化的 `ApiFailure`。
pub fn not_init_err(msg: impl Into<String>) -> quant_common::api::ApiFailure {
    quant_common::api::ApiFailure::new(quant_common::api::code::NOT_INITIALIZED, msg)
}

pub use api_keys::*;
pub use audit::*;
pub use auth::*;
pub use backtest::*;
pub use binance::*;
pub use binance_ws::*;
pub use core::*;
pub use market_data::*;
pub use optimizer::*;
pub use strategy_risk::*;
pub use twofa::*;

#[cfg(test)]
mod tests;
