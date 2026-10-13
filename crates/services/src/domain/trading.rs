//! 交易领域服务组合
//!
//! 包含账户、订单处理等交易相关服务。

// 重新导出交易领域服务
pub use crate::account_service::AccountService;
pub use crate::order_processor::OrderProcessor;
