//! 策略错误模块

use crate::types::strategy::status::StrategyStatus;

/// Strategy lifecycle error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StrategyError {
    #[error("Invalid status transition: {from:?} → {to:?} is not allowed")]
    InvalidTransition {
        from: StrategyStatus,
        to: StrategyStatus,
    },
}
