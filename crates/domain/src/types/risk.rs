//! 风险指标和告警模块

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ─── Risk Metrics ────────────────────────────────────────────────────────────

/// 风险指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub timestamp: DateTime<Utc>,
    pub var_95: Decimal,
    pub var_99: Decimal,
    pub portfolio_volatility: Decimal,
    pub beta: Decimal,
    pub concentration_risk: Decimal,
    pub leverage: Decimal,
}

impl RiskMetrics {
    /// Ratio of VaR(99) to VaR(95) — indicates tail risk.
    pub fn tail_ratio(&self) -> Decimal {
        if self.var_95.is_zero() {
            return Decimal::ZERO;
        }
        self.var_99 / self.var_95
    }

    /// True if leverage exceeds a given threshold.
    #[must_use]
    pub fn is_over_leveraged(&self, max_leverage: Decimal) -> bool {
        self.leverage > max_leverage
    }
}

// ─── Alerts ──────────────────────────────────────────────────────────────────

/// 告警级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// 告警
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: i64,
    pub level: AlertLevel,
    pub source: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

impl Alert {
    /// Create a new unacknowledged alert (alert_id = 0, assigned by DB on insert).
    pub fn new(level: AlertLevel, source: String, message: String) -> Self {
        Self {
            alert_id: 0,
            level,
            source,
            message,
            timestamp: Utc::now(),
            acknowledged: false,
        }
    }

    /// True if critical severity.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.level == AlertLevel::Critical
    }

    /// Age of the alert in seconds.
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.timestamp).num_seconds()
    }

    /// Mark as acknowledged.
    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }
}
