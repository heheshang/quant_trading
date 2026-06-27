//! Type re-exports from the domain layer.
//!
//! Business types (`Instrument`, `Order`, `Position`, `Account`, etc.) are
//! defined in `quant_domain::types` — the single source of truth.
//! This module re-exports them for backward compatibility.
//!
//! New code should import from `quant_domain::types` directly.

pub use quant_domain::types::{
    Account, Alert, AlertLevel, BacktestResult, Exchange, Instrument, InstrumentType, MarketData,
    Order, OrderSide, OrderStatus, OrderType, Position, RiskMetrics,
    SchedulerTaskInfo, StrategyError, StrategyGuard, StrategyParams, StrategyStatus,
    StrategyType, StatusTransition, allowed_transitions,
};

// ─── LogEntry ────────────────────────────────────────────────────────────
// LogEntry is a utility type unique to the common layer — not a business
// domain concept. Defined here rather than in quant_domain.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Log entry for structured logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub module: Option<String>,
}

impl LogEntry {
    pub fn new(
        level: impl Into<String>,
        message: impl Into<String>,
        module: Option<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            level: level.into(),
            message: message.into(),
            module,
        }
    }
}

// ─── Strategy Lifecycle Types ──────────────────────────────────────────────
// Re-exported from quant_domain::types (single source of truth).

/// Performance scorecard computed for a strategy run or backtest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyScorecard {
    pub total_return: f64,
    pub annual_return: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub profit_loss_ratio: f64,
    pub total_trades: u32,
}

/// The type of a parameter for schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    Number,
    String,
    Select(Vec<String>),
}

/// Range constraint for a numeric parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamRange {
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,
}

/// Describes a single configurable parameter of a strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    pub name: String,
    pub param_type: ParamType,
    pub default: serde_json::Value,
    pub range: Option<ParamRange>,
    pub description: String,
}

// ─── Signal Pipeline Types ─────────────────────────────────────────────────

/// A named step in the signal processing pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStepDef {
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
}

/// Configuration for a complete signal pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalPipelineConfig {
    pub steps: Vec<PipelineStepDef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logentry_new_with_module() {
        let entry = LogEntry::new("INFO", "Server started", Some("main".into()));
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.message, "Server started");
        assert_eq!(entry.module, Some("main".into()));
    }

    #[test]
    fn test_logentry_new_without_module() {
        let entry = LogEntry::new("WARN", "Disk usage high", None);
        assert_eq!(entry.level, "WARN");
        assert_eq!(entry.message, "Disk usage high");
        assert_eq!(entry.module, None);
    }

    // ── StrategyStatus ─────────────────────────────────────────────────────

    #[test]
    fn test_strategy_status_allowed_transitions() {
        use StrategyStatus::*;
        assert!(Draft.can_transition_to(Backtesting));
        assert!(Draft.can_transition_to(Archived));
        assert!(Backtesting.can_transition_to(Deployed));
        assert!(Backtesting.can_transition_to(Draft));
        assert!(Deployed.can_transition_to(Running));
        assert!(Deployed.can_transition_to(Draft));
        assert!(Running.can_transition_to(Paused));
        assert!(Running.can_transition_to(Archived));
        assert!(Paused.can_transition_to(Running));
        assert!(Paused.can_transition_to(Archived));
    }

    #[test]
    fn test_strategy_status_forbidden_transitions() {
        use StrategyStatus::*;
        assert!(!Draft.can_transition_to(Running));
        assert!(!Draft.can_transition_to(Paused));
        assert!(!Draft.can_transition_to(Draft));
        assert!(!Backtesting.can_transition_to(Running));
        assert!(!Backtesting.can_transition_to(Paused));
        assert!(!Backtesting.can_transition_to(Archived));
        assert!(!Deployed.can_transition_to(Paused));
        assert!(!Deployed.can_transition_to(Archived));
        assert!(!Deployed.can_transition_to(Backtesting));
        assert!(!Running.can_transition_to(Draft));
        assert!(!Running.can_transition_to(Backtesting));
        assert!(!Running.can_transition_to(Running));
        assert!(!Paused.can_transition_to(Draft));
        assert!(!Paused.can_transition_to(Backtesting));
        assert!(!Paused.can_transition_to(Deployed));
        assert!(!Archived.can_transition_to(Archived));
        assert!(!Archived.can_transition_to(Draft));
    }

    #[test]
    fn test_allowed_transitions_list() {
        let transitions = allowed_transitions();
        assert_eq!(transitions.len(), 10);
        for t in &transitions {
            assert!(t.from.can_transition_to(t.to),
                "transition {:?} → {:?} should be valid", t.from, t.to);
        }
    }

    // ── StrategyScorecard ──────────────────────────────────────────────────

    #[test]
    fn test_strategy_scorecard_default_values() {
        let sc = StrategyScorecard {
            total_return: 0.15,
            annual_return: 0.12,
            sharpe_ratio: 1.5,
            max_drawdown: 0.08,
            win_rate: 0.55,
            profit_loss_ratio: 1.8,
            total_trades: 100,
        };
        assert!((sc.sharpe_ratio - 1.5).abs() < f64::EPSILON);
        assert_eq!(sc.total_trades, 100);
    }

    // ── ParameterSchema ────────────────────────────────────────────────────

    #[test]
    fn test_parameter_schema_number_type() {
        let schema = ParameterSchema {
            name: "lookback_period".into(),
            param_type: ParamType::Number,
            default: serde_json::json!(20),
            range: Some(ParamRange { min: 5.0, max: 100.0, step: Some(1.0) }),
            description: "Number of bars for lookback".into(),
        };
        assert_eq!(schema.name, "lookback_period");
    }

    #[test]
    fn test_parameter_schema_select_type() {
        let schema = ParameterSchema {
            name: "method".into(),
            param_type: ParamType::Select(vec!["sma".into(), "ema".into()]),
            default: serde_json::json!("sma"),
            range: None,
            description: "Moving average method".into(),
        };
        if let ParamType::Select(options) = &schema.param_type {
            assert_eq!(options.len(), 2);
        } else {
            panic!("expected Select variant");
        }
    }

    // ── Signal Pipeline ────────────────────────────────────────────────────

    #[test]
    fn test_signal_pipeline_config_roundtrip() {
        let config = SignalPipelineConfig {
            steps: vec![
                PipelineStepDef {
                    name: "risk_check".into(),
                    enabled: true,
                    config: serde_json::json!({"max_position_usd": 100000}),
                },
                PipelineStepDef {
                    name: "order_convert".into(),
                    enabled: true,
                    config: serde_json::json!({}),
                },
            ],
        };
        assert_eq!(config.steps.len(), 2);
        let first = &config.steps[0];
        assert_eq!(first.name, "risk_check");
        assert!(first.enabled);
    }

    // ── SchedulerTaskInfo ──────────────────────────────────────────────────

    #[test]
    fn test_scheduler_task_info_default() {
        let info = SchedulerTaskInfo {
            strategy_id: "s001".into(),
            strategy_name: "Test Strategy".into(),
            status: StrategyStatus::Running,
            interval_secs: 60,
            last_run_at: None,
            error_count: 0,
        };
        assert_eq!(info.strategy_id, "s001");
        assert_eq!(info.error_count, 0);
        assert!(info.last_run_at.is_none());
    }
}
