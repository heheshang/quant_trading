//! 策略和风控命令（向后兼容模块）
//!
//! # 已废弃
//! 此模块已废弃，请使用 `crate::commands::strategy` 和 `crate::commands::risk` 模块。
//!
//! ## 迁移指南
//! ```ignore
//! // 旧代码
//! use crate::commands::strategy_risk::{get_strategies, get_risk_metrics};
//!
//! // 新代码
//! use crate::commands::{get_strategies, get_risk_metrics};
//! // 或者
//! use crate::commands::strategy;
//! use crate::commands::risk;
//! ```

// 向后兼容重导出 - 所有命令现在直接在 commands 模块中可用
pub use crate::commands::strategy;
pub use crate::commands::risk;
