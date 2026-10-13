//! 策略领域模块
//!
//! 包含策略相关的领域模型、状态机和错误类型。

pub mod error;
pub mod params;
pub mod status;

// 重新导出公共类型
pub use error::StrategyError;
pub use params::{StrategyParams, StrategyParamsBuilder, StrategyType};
pub use status::{allowed_transitions, SchedulerTaskInfo, StatusTransition, StrategyGuard, StrategyStatus};
