//! 策略状态模块
//!
//! 定义策略生命周期状态和状态机逻辑。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Strategy lifecycle status.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyStatus {
    #[default]
    Draft,
    Backtesting,
    Deployed,
    Running,
    Paused,
    Archived,
}

impl std::str::FromStr for StrategyStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "Backtesting" => Ok(Self::Backtesting),
            "Deployed" => Ok(Self::Deployed),
            "Running" => Ok(Self::Running),
            "Paused" => Ok(Self::Paused),
            "Archived" => Ok(Self::Archived),
            _ => Ok(Self::Draft),
        }
    }
}

impl StrategyStatus {
    /// Check whether a transition from `self` to `to` is allowed.
    #[must_use]
    pub fn can_transition_to(&self, to: StrategyStatus) -> bool {
        matches!(
            (*self, to),
            (
                Self::Draft,
                Self::Backtesting | Self::Deployed | Self::Archived
            ) | (Self::Backtesting, Self::Deployed | Self::Draft)
                | (Self::Deployed, Self::Running | Self::Draft)
                | (Self::Running, Self::Paused | Self::Archived)
                | (Self::Paused, Self::Running | Self::Archived)
        )
    }
}

/// Guard predicate for status transitions.
pub type StrategyGuard = Box<dyn Fn(&crate::types::strategy::StrategyParams) -> bool + Send + Sync>;

/// A permitted status transition with an optional guard predicate.
pub struct StatusTransition {
    pub from: StrategyStatus,
    pub to: StrategyStatus,
    pub guard: Option<StrategyGuard>,
}

impl std::fmt::Debug for StatusTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusTransition")
            .field("from", &self.from)
            .field("to", &self.to)
            .finish()
    }
}

/// All allowed status transitions (without guards).
#[must_use]
pub fn allowed_transitions() -> Vec<StatusTransition> {
    use StrategyStatus::*;
    vec![
        StatusTransition {
            from: Draft,
            to: Backtesting,
            guard: None,
        },
        StatusTransition {
            from: Draft,
            to: Deployed,
            guard: None,
        },
        StatusTransition {
            from: Draft,
            to: Archived,
            guard: None,
        },
        StatusTransition {
            from: Backtesting,
            to: Deployed,
            guard: None,
        },
        StatusTransition {
            from: Backtesting,
            to: Draft,
            guard: None,
        },
        StatusTransition {
            from: Deployed,
            to: Running,
            guard: None,
        },
        StatusTransition {
            from: Deployed,
            to: Draft,
            guard: None,
        },
        StatusTransition {
            from: Running,
            to: Paused,
            guard: None,
        },
        StatusTransition {
            from: Running,
            to: Archived,
            guard: None,
        },
        StatusTransition {
            from: Paused,
            to: Running,
            guard: None,
        },
        StatusTransition {
            from: Paused,
            to: Archived,
            guard: None,
        },
    ]
}

/// Information about a running scheduler task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerTaskInfo {
    pub strategy_id: String,
    pub strategy_name: String,
    pub status: StrategyStatus,
    pub interval_secs: u64,
    pub last_run_at: Option<DateTime<Utc>>,
    pub error_count: u32,
}
