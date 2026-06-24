use chrono::Utc;
use quant_common::config::RiskConfig;
use quant_common::types::{Account, Alert, AlertLevel};
use rust_decimal::Decimal;
use tracing::{info, instrument, warn};
use uuid::Uuid;

const DEFAULT_MAX_DRAWDOWN: f64 = 0.15;
const MARGIN_RATIO_CRITICAL: f64 = 0.9;

/// 实时风险监控器
pub struct RealTimeRiskMonitor {
    config: RiskConfig,
    alerts: Vec<Alert>,
}

impl RealTimeRiskMonitor {
    pub fn new(config: RiskConfig) -> Self {
        Self {
            config,
            alerts: Vec::new(),
        }
    }

    /// 监控账户风险
    #[instrument(skip(self, account), fields(risk_check = "real_time"))]
    pub fn monitor_account(&mut self, account: &Account) -> Vec<Alert> {
        let mut new_alerts = Vec::new();

        // 监控回撤
        if let Some(alert) = self.check_drawdown(account) {
            warn!("Drawdown limit breached");
            new_alerts.push(alert);
        }

        // 监控保证金比例
        if let Some(alert) = self.check_margin_ratio(account) {
            warn!("Margin ratio limit breached");
            new_alerts.push(alert);
        }

        // 监控每日盈亏
        if let Some(alert) = self.check_daily_pnl(account) {
            warn!("Daily PnL limit breached");
            new_alerts.push(alert);
        }

        if new_alerts.is_empty() {
            info!("All real-time risk checks passed");
        }

        self.alerts.extend(new_alerts.clone());
        new_alerts
    }

    fn check_drawdown(&self, account: &Account) -> Option<Alert> {
        let max_drawdown = Decimal::from_f64_retain(self.config.max_drawdown)
            .unwrap_or(Decimal::from_f64_retain(DEFAULT_MAX_DRAWDOWN).unwrap());

        // 简化：实际需要基于历史最高净值计算
        let current_drawdown = if account.total_assets > Decimal::ZERO {
            Decimal::ZERO // 实际应该计算真实回撤
        } else {
            Decimal::ZERO
        };

        if current_drawdown > max_drawdown {
            Some(Alert {
                alert_id: Uuid::new_v4(),
                level: AlertLevel::Critical,
                source: "RiskMonitor".to_string(),
                message: format!("Max drawdown exceeded: {}", current_drawdown),
                timestamp: Utc::now(),
                acknowledged: false,
            })
        } else {
            None
        }
    }

    fn check_margin_ratio(&self, account: &Account) -> Option<Alert> {
        if account.margin_ratio > Decimal::from_f64_retain(MARGIN_RATIO_CRITICAL).unwrap() {
            Some(Alert {
                alert_id: Uuid::new_v4(),
                level: AlertLevel::Warning,
                source: "RiskMonitor".to_string(),
                message: format!("High margin ratio: {}", account.margin_ratio),
                timestamp: Utc::now(),
                acknowledged: false,
            })
        } else {
            None
        }
    }

    fn check_daily_pnl(&self, account: &Account) -> Option<Alert> {
        let max_daily_loss =
            Decimal::from_f64_retain(self.config.max_daily_loss).unwrap_or(Decimal::ZERO);

        if account.daily_pnl < -max_daily_loss {
            Some(Alert {
                alert_id: Uuid::new_v4(),
                level: AlertLevel::Critical,
                source: "RiskMonitor".to_string(),
                message: format!("Daily loss limit exceeded: {}", account.daily_pnl),
                timestamp: Utc::now(),
                acknowledged: false,
            })
        } else {
            None
        }
    }

    pub fn get_alerts(&self) -> &[Alert] {
        &self.alerts
    }
}
