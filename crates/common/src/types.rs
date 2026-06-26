//! Type re-exports from the domain layer.
//!
//! Business types (`Instrument`, `Order`, `Position`, `Account`, etc.) are
//! defined in `quant_domain::types` — the single source of truth.
//! This module re-exports them for backward compatibility.
//!
//! New code should import from `quant_domain::types` directly.

pub use quant_domain::types::{
    Account, Alert, AlertLevel, BacktestResult, Exchange, Instrument, InstrumentType, MarketData,
    Order, OrderSide, OrderStatus, OrderType, Position, RiskMetrics, StrategyParams, StrategyType,
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
}
