use crate::error::{ServiceError, ServiceResult};
use quant_common::config::RiskConfig;
use quant_common::types::{Account, Order, Position};
use quant_repository::PostgresClient;
use risk_engine::{PreTradeRiskChecker, VaRCalculator};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

/// Risk management service.
///
/// Handles risk configuration, metrics computation, and pre-trade checks.
pub struct RiskService {
    postgres: Option<Arc<PostgresClient>>,
}

impl RiskService {
    pub fn new(postgres: Option<Arc<PostgresClient>>) -> Self {
        Self { postgres }
    }

    #[instrument(skip_all)]
    pub async fn get_risk_metrics(&self) -> ServiceResult<HashMap<String, f64>> {
        let mut metrics = HashMap::new();

        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        let pool = client.pool();

        // Read risk config from DB for VaR confidence level
        let row = sqlx::query(
            r#"
            SELECT var_confidence_level, max_position_size, max_daily_loss, max_drawdown, max_concentration
            FROM risk_config WHERE id = 1
            "#,
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch risk config for metrics: {}", e);
            ServiceError::from(e)
        })?
        .ok_or_else(|| {
            error!("Risk config not found");
            ServiceError::NotFound("Risk config not found".into())
        })?;

        let var_conf: f64 = row.get("var_confidence_level");
        let max_pos: f64 = row.get("max_position_size");
        let max_loss: f64 = row.get("max_daily_loss");
        let max_dd: f64 = row.get("max_drawdown");
        let max_conc: f64 = row.get("max_concentration");

        // Query accounts table for historical daily_pnl and total_assets
        let account_rows = sqlx::query(
            r#"
            SELECT daily_pnl, total_assets FROM accounts
            WHERE daily_pnl IS NOT NULL AND total_assets > 0
            ORDER BY updated_at DESC
            LIMIT 100
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch account history for VaR: {}", e);
            ServiceError::from(e)
        })?;

        // Compute percentage returns: daily_pnl / total_assets
        let returns: Vec<Decimal> = account_rows
            .iter()
            .map(|r| {
                let daily_pnl: Decimal = r.get("daily_pnl");
                let total_assets: Decimal = r.get("total_assets");
                daily_pnl / total_assets
            })
            .collect();

        // Historical VaR needs >= 2 data points; degrade gracefully so a fresh
        // account with no history doesn't fail the whole risk-metrics request.
        let (var_95, var_99) = if returns.len() >= 2 {
            (
                VaRCalculator::historical_simulation(&returns, 0.95),
                VaRCalculator::historical_simulation(&returns, 0.99),
            )
        } else {
            warn!(
                "Insufficient account history for VaR (need >= 2, got {}); VaR unavailable",
                returns.len()
            );
            (Decimal::ZERO, Decimal::ZERO)
        };

        metrics.insert("var_95".to_string(), var_95.to_f64().unwrap_or(0.0));
        metrics.insert("var_99".to_string(), var_99.to_f64().unwrap_or(0.0));
        metrics.insert("max_position_size".to_string(), max_pos);
        metrics.insert("max_daily_loss".to_string(), max_loss);
        metrics.insert("max_drawdown".to_string(), max_dd);
        metrics.insert("max_concentration".to_string(), max_conc);
        metrics.insert("var_confidence_level".to_string(), var_conf);

        info!(
            var_95 = metrics["var_95"],
            var_99 = metrics["var_99"],
            "Risk metrics computed"
        );
        Ok(metrics)
    }

    #[instrument(skip_all)]
    pub async fn get_risk_config(&self) -> ServiceResult<RiskConfig> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        let row = sqlx::query(
            r#"
            SELECT max_position_size, max_daily_loss, max_drawdown,
                   max_concentration,
                   enable_pre_trade_check, enable_real_time_monitor, var_confidence_level
            FROM risk_config
            WHERE id = 1
            "#,
        )
        .fetch_one(client.pool())
        .await
        .map_err(|e| {
            error!("Failed to fetch risk config: {}", e);
            ServiceError::Database(e)
        })?;

        let config = RiskConfig {
            max_position_size: row.get("max_position_size"),
            max_daily_loss: row.get("max_daily_loss"),
            max_drawdown: row.get("max_drawdown"),
            max_concentration: row.get("max_concentration"),
            enable_pre_trade_check: row.get("enable_pre_trade_check"),
            enable_real_time_monitor: row.get("enable_real_time_monitor"),
            var_confidence_level: row.get("var_confidence_level"),
        };
        info!("Risk config retrieved");
        Ok(config)
    }

    #[instrument(skip(self, config))]
    pub async fn update_risk_config(&self, config: &RiskConfig) -> ServiceResult<bool> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        let affected = sqlx::query(
            r#"
            UPDATE risk_config SET
                max_position_size = $1, max_daily_loss = $2, max_drawdown = $3,
                enable_pre_trade_check = $4, enable_real_time_monitor = $5,
                max_concentration = $6, var_confidence_level = $7, updated_at = NOW()
            WHERE id = 1
            "#,
        )
        .bind(config.max_position_size)
        .bind(config.max_daily_loss)
        .bind(config.max_drawdown)
        .bind(config.enable_pre_trade_check)
        .bind(config.enable_real_time_monitor)
        .bind(config.max_concentration)
        .bind(config.var_confidence_level)
        .execute(client.pool())
        .await
        .map_err(|e| {
            error!("Failed to update risk config: {}", e);
            ServiceError::Database(e)
        })?;

        let updated = affected.rows_affected() > 0;
        info!("Risk config updated");
        Ok(updated)
    }

    /// Run pre-trade risk check. Returns (passed, RiskConfig) where passed indicates
    /// whether the check succeeded. The RiskConfig is returned so callers can log it.
    pub async fn pre_trade_check(
        &self,
        order: &Order,
        account: &Account,
        positions: &[Position],
    ) -> ServiceResult<(bool, RiskConfig)> {
        let config = match self.get_risk_config().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Falling back to default RiskConfig: {}", e);
                RiskConfig {
                    max_position_size: 0.2,
                    max_daily_loss: 0.05,
                    max_drawdown: 0.15,
                    max_concentration: 0.2,
                    enable_pre_trade_check: true,
                    enable_real_time_monitor: true,
                    var_confidence_level: 0.95,
                }
            }
        };

        let checker = PreTradeRiskChecker::new(config.clone());
        match checker.check_order(order, account, positions) {
            Ok(_) => Ok((true, config)),
            Err(_) => Ok((false, config)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::config::RiskConfig;
    use quant_common::types::{OrderSide, OrderStatus, OrderType};
    use rust_decimal_macros::dec;

    // ── test fixtures ──────────────────────────────────────────────────────────

    fn sample_order_buy() -> Order {
        Order {
            order_id: 0,
            strategy_id: "test_strat".into(),
            symbol: "BTC-USDT".into(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: Some(dec!(50000)),
            quantity: dec!(0.1),
            filled_quantity: dec!(0),
            status: OrderStatus::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            commission: dec!(0),
            slippage: dec!(0),
        }
    }

    fn sample_order_sell_small() -> Order {
        Order {
            order_id: 0,
            strategy_id: "test_strat".into(),
            symbol: "BTC-USDT".into(),
            order_type: OrderType::Limit,
            side: OrderSide::Sell,
            price: Some(dec!(50000)),
            quantity: dec!(0.1),
            filled_quantity: dec!(0),
            status: OrderStatus::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            commission: dec!(0),
            slippage: dec!(0),
        }
    }

    fn sample_account_healthy() -> Account {
        Account {
            account_id: 0,
            total_assets: dec!(1000000),
            available_cash: dec!(500000),
            frozen_cash: dec!(0),
            market_value: dec!(500000),
            total_pnl: dec!(50000),
            daily_pnl: dec!(1000),
            margin: dec!(0),
            margin_ratio: dec!(0),
            updated_at: chrono::Utc::now(),
        }
    }

    fn sample_account_broke() -> Account {
        Account {
            account_id: 0,
            total_assets: dec!(1000),
            available_cash: dec!(10),
            frozen_cash: dec!(0),
            market_value: dec!(990),
            total_pnl: dec!(-50000),
            daily_pnl: dec!(-5000),
            margin: dec!(0),
            margin_ratio: dec!(0),
            updated_at: chrono::Utc::now(),
        }
    }

    fn sample_position(symbol: &str, quantity: Decimal) -> Position {
        Position {
            symbol: symbol.into(),
            quantity,
            available_quantity: quantity,
            avg_price: dec!(50000),
            market_value: quantity * dec!(50000),
            unrealized_pnl: dec!(0),
            realized_pnl: dec!(0),
            updated_at: chrono::Utc::now(),
        }
    }

    // ── get_risk_metrics (postgres: None → error) ───────────────────────────────

    #[tokio::test]
    async fn get_risk_metrics_no_db_returns_error() {
        let svc = RiskService::new(None);
        let result = svc.get_risk_metrics().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    // ── get_risk_config (postgres: None → error) ───────────────────────────────

    #[tokio::test]
    async fn get_risk_config_returns_error_when_no_db() {
        let svc = RiskService::new(None);
        let result = svc.get_risk_config().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    // ── update_risk_config (postgres: None → error) ────────────────────────────

    #[tokio::test]
    async fn update_risk_config_returns_error_when_no_db() {
        let svc = RiskService::new(None);
        let config = RiskConfig {
            max_position_size: 0.3,
            max_daily_loss: 0.1,
            max_drawdown: 0.2,
            max_concentration: 0.25,
            enable_pre_trade_check: true,
            enable_real_time_monitor: true,
            var_confidence_level: 0.99,
        };
        let result = svc.update_risk_config(&config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    // ── pre_trade_check (postgres: None → falls back to default config) ────────

    #[tokio::test]
    async fn pre_trade_check_passes_for_safe_sell_order() {
        let svc = RiskService::new(None);
        let order = sample_order_sell_small();
        let account = sample_account_healthy();
        // 持有 0.5 BTC，卖出 0.1 BTC 有足够可用持仓
        let positions = vec![sample_position("BTC-USDT", dec!(0.5))];
        let (passed, config) = svc
            .pre_trade_check(&order, &account, &positions)
            .await
            .unwrap();
        assert!(passed);
        assert_eq!(config.max_position_size, 0.2);
    }

    #[tokio::test]
    async fn pre_trade_check_fails_for_insufficient_cash() {
        let svc = RiskService::new(None);
        let order = sample_order_buy(); // 0.1 BTC × 50000 = 5000 USDT needed
        let account = sample_account_broke(); // only 10 USDT available
        let positions: Vec<Position> = vec![];
        let (passed, _) = svc
            .pre_trade_check(&order, &account, &positions)
            .await
            .unwrap();
        assert!(!passed);
    }

    #[tokio::test]
    async fn pre_trade_check_fails_for_position_limit_exceeded() {
        let svc = RiskService::new(None);
        let order = sample_order_buy(); // 买入 0.1 BTC @ 50000 = 5000 USDT
        let account = sample_account_healthy(); // 总资产 1,000,000
                                                // 现有 4.0 BTC（市值 200,000）+ 新买 0.1 BTC = 4.1 BTC（205,000），
                                                // 占总资产 20.5% > max_position_size 20%
        let positions = vec![sample_position("BTC-USDT", dec!(4.0))];
        let (passed, _) = svc
            .pre_trade_check(&order, &account, &positions)
            .await
            .unwrap();
        assert!(!passed);
    }

    #[tokio::test]
    async fn pre_trade_check_fails_for_daily_loss_limit() {
        let svc = RiskService::new(None);
        let order = sample_order_sell_small();
        let account = Account {
            daily_pnl: dec!(-60000),
            ..sample_account_healthy()
        };
        let positions: Vec<Position> = vec![];
        let (passed, _) = svc
            .pre_trade_check(&order, &account, &positions)
            .await
            .unwrap();
        assert!(!passed);
    }

    #[tokio::test]
    async fn pre_trade_check_falls_back_to_default_config() {
        let svc = RiskService::new(None);
        let order = sample_order_sell_small();
        let account = sample_account_healthy();
        let positions: Vec<Position> = vec![];
        let (_, config) = svc
            .pre_trade_check(&order, &account, &positions)
            .await
            .unwrap();
        assert_eq!(config.max_position_size, 0.2);
        assert_eq!(config.max_daily_loss, 0.05);
        assert_eq!(config.max_concentration, 0.2);
        assert!(config.enable_pre_trade_check);
    }
}
