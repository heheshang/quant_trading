//! 领域服务模块
//!
//! 按业务域组织服务组合，便于理解和维护。
//!
//! ## 结构
//! - `trading.rs` - 交易领域（账户、订单处理）
//! - `strategy.rs` - 策略领域（策略服务、优化器）
//! - `market.rs` - 市场领域（行情、实时交易）

pub mod market;
pub mod strategy;
pub mod trading;
