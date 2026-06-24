use crate::error::{ServiceError, ServiceResult};
use data_layer::market_data::DataSource;
use data_layer::OkxDataSource;
use quant_common::types::{BacktestResult, StrategyParams, StrategyType};
use quant_repository::PostgresClient;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use sqlx::Row;
use std::sync::Arc;
use std::time::Instant;
use strategy_engine::strategy::MeanReversionStrategy;
use strategy_engine::{BacktestEngine, Strategy};
use tokio::sync::RwLock;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

pub struct StrategyService {
    postgres: Option<Arc<PostgresClient>>,
    okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
}

impl StrategyService {
    pub fn new(
        postgres: Option<Arc<PostgresClient>>,
        okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
    ) -> Self {
        Self {
            postgres,
            okx_data_source,
        }
    }

    #[instrument(skip_all)]
    pub async fn get_strategies(&self) -> ServiceResult<Vec<StrategyParams>> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let rows = sqlx::query(
            r#"
            SELECT strategy_id, strategy_name, strategy_type, params,
                   enabled, max_position, max_daily_loss, created_at, updated_at
            FROM strategies
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(ServiceError::from)?;

        let strategies: Vec<StrategyParams> = rows
            .iter()
            .map(|row| {
                let ty_str: String = row.get("strategy_type");
                let strategy_type: StrategyType =
                    serde_json::from_value(serde_json::Value::String(ty_str))
                        .unwrap_or(StrategyType::MeanReversion);
                StrategyParams {
                    strategy_id: row.get("strategy_id"),
                    strategy_name: row.get("strategy_name"),
                    strategy_type,
                    params: row.get("params"),
                    enabled: row.get("enabled"),
                    max_position: row.get("max_position"),
                    max_daily_loss: row.get("max_daily_loss"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }
            })
            .collect();

        info!(count = strategies.len(), "Strategies retrieved");
        Ok(strategies)
    }

    #[instrument(skip(self, strategy), fields(strategy_id = %strategy.strategy_id))]
    pub async fn save_strategy(&self, strategy: &StrategyParams) -> ServiceResult<String> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let type_str = serde_json::to_value(&strategy.strategy_type)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "MeanReversion".to_string());

        sqlx::query(
            r#"
            INSERT INTO strategies (strategy_id, strategy_name, strategy_type, params,
                                    enabled, max_position, max_daily_loss, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (strategy_id) DO UPDATE SET
                strategy_name = EXCLUDED.strategy_name,
                strategy_type = EXCLUDED.strategy_type,
                params = EXCLUDED.params,
                enabled = EXCLUDED.enabled,
                max_position = EXCLUDED.max_position,
                max_daily_loss = EXCLUDED.max_daily_loss,
                updated_at = NOW()
            "#,
        )
        .bind(&strategy.strategy_id)
        .bind(&strategy.strategy_name)
        .bind(&type_str)
        .bind(&strategy.params)
        .bind(strategy.enabled)
        .bind(strategy.max_position)
        .bind(strategy.max_daily_loss)
        .bind(strategy.created_at)
        .bind(strategy.updated_at)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to save strategy {}: {}", strategy.strategy_id, e);
            ServiceError::from(e)
        })?;

        info!(strategy_id = %strategy.strategy_id, strategy_name = %strategy.strategy_name, "Strategy saved");
        Ok(strategy.strategy_id.clone())
    }

    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn delete_strategy(&self, strategy_id: &str) -> ServiceResult<bool> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let affected = sqlx::query("DELETE FROM strategies WHERE strategy_id = $1")
            .bind(strategy_id)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("Failed to delete strategy {}: {}", strategy_id, e);
                ServiceError::from(e)
            })?;

        let deleted = affected.rows_affected() > 0;
        if deleted {
            info!(strategy_id = %strategy_id, "Strategy deleted");
        } else {
            warn!(strategy_id = %strategy_id, "Strategy not found for deletion");
        }
        Ok(deleted)
    }

    #[instrument(skip(self), fields(strategy_id = %strategy_id, enabled))]
    pub async fn toggle_strategy(&self, strategy_id: &str, enabled: bool) -> ServiceResult<bool> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let affected = sqlx::query(
            "UPDATE strategies SET enabled = $1, updated_at = NOW() WHERE strategy_id = $2",
        )
        .bind(enabled)
        .bind(strategy_id)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to toggle strategy {}: {}", strategy_id, e);
            ServiceError::from(e)
        })?;

        let toggled = affected.rows_affected() > 0;
        info!(strategy_id = %strategy_id, enabled, "Strategy toggled");
        Ok(toggled)
    }

    #[instrument(skip(self), fields(strategy_id = %strategy_id, initial_capital, symbols = ?symbols))]
    pub async fn run_backtest(
        &self,
        strategy_id: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        initial_capital: f64,
        commission_rate: f64,
        slippage: f64,
        symbols: &[String],
    ) -> ServiceResult<BacktestResult> {
        let start_time = Instant::now();
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();
        let row = sqlx::query(
            r#"
            SELECT strategy_type, params, strategy_name, max_position, max_daily_loss
            FROM strategies
            WHERE strategy_id = $1
            "#,
        )
        .bind(strategy_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch strategy {} for backtest: {}", strategy_id, e);
            ServiceError::from(e)
        })?
        .ok_or_else(|| {
            error!("Strategy '{}' not found for backtest", strategy_id);
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let db_type: String = row.get("strategy_type");
        let db_params: serde_json::Value = row.get("params");
        let db_name: String = row.get("strategy_name");
        let db_max_pos: Decimal = row.get("max_position");
        let db_max_loss: Decimal = row.get("max_daily_loss");

        let symbol = db_params
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("BTC-USDT")
            .to_string();

        let market_data = {
            let ds = self.okx_data_source.read().await;
            match ds.as_ref() {
                Some(source) => source
                    .get_historical_data(&symbol, start, end)
                    .await
                    .map_err(|e| {
                        error!("Failed to fetch market data for {}: {}", symbol, e);
                        ServiceError::DataSource(e.to_string())
                    })?,
                None => {
                    error!("OKX data source not initialized for backtest");
                    return Err(ServiceError::OkxDataSourceNotInitialized);
                }
            }
        };

        if market_data.is_empty() {
            error!(symbol = %symbol, "No market data for backtest date range");
            return Err(ServiceError::Other(
                "No market data returned for the specified date range".into(),
            ));
        }

        let strategy_type: StrategyType =
            serde_json::from_value(serde_json::Value::String(db_type.clone()))
                .unwrap_or(StrategyType::MeanReversion);

        if strategy_type != StrategyType::MeanReversion {
            return Err(ServiceError::Strategy(format!(
                "Strategy type '{:?}' is not supported for backtesting. Only MeanReversion is implemented.",
                strategy_type
            )));
        }

        let strategy_params = StrategyParams {
            strategy_id: strategy_id.to_string(),
            strategy_name: db_name.clone(),
            strategy_type: strategy_type.clone(),
            params: db_params,
            enabled: true,
            max_position: db_max_pos,
            max_daily_loss: db_max_loss,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let mut strategy = MeanReversionStrategy::new();
        strategy
            .initialize(strategy_params)
            .await
            .map_err(|e| {
                error!("Strategy initialization failed: {}", e);
                ServiceError::Strategy(e.to_string())
            })?;

        let initial_capital_dec = Decimal::from_f64(initial_capital).unwrap_or(Decimal::ZERO);
        let commission_rate_dec = Decimal::from_f64(commission_rate).unwrap_or(Decimal::ZERO);
        let slippage_dec = Decimal::from_f64(slippage).unwrap_or(Decimal::ZERO);

        let mut engine =
            BacktestEngine::new(initial_capital_dec, commission_rate_dec, slippage_dec);
        let result = engine
            .run(&strategy, market_data)
            .await
            .map_err(|e| {
                error!("Backtest execution failed: {}", e);
                ServiceError::Backtest(e.to_string())
            })?;

        let backtest_result = BacktestResult {
            strategy_id: strategy_id.to_string(),
            ..result
        };

        let backtest_id = Uuid::new_v4();
        let equity_json = serde_json::to_value(&backtest_result.equity_curve).map_err(|e| {
            ServiceError::Serialization {
                what: "equity curve",
                source: e,
            }
        })?;
        let symbols_str: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();

        sqlx::query(
            r#"
            INSERT INTO backtest_results (
                backtest_id, strategy_id, strategy_name,
                start_date, end_date,
                initial_capital, final_capital,
                total_return, annual_return,
                sharpe_ratio, max_drawdown,
                win_rate, profit_loss_ratio,
                total_trades, winning_trades, losing_trades,
                equity_curve, symbols,
                commission_rate, slippage
            ) VALUES (
                $1, $2, $3,
                $4, $5,
                $6, $7,
                $8, $9,
                $10, $11,
                $12, $13,
                $14, $15, $16,
                $17, $18,
                $19, $20
            )
            "#,
        )
        .bind(backtest_id)
        .bind(&backtest_result.strategy_id)
        .bind(&db_name)
        .bind(backtest_result.start_date)
        .bind(backtest_result.end_date)
        .bind(backtest_result.initial_capital)
        .bind(backtest_result.final_capital)
        .bind(backtest_result.total_return)
        .bind(backtest_result.annual_return)
        .bind(backtest_result.sharpe_ratio)
        .bind(backtest_result.max_drawdown)
        .bind(backtest_result.win_rate)
        .bind(backtest_result.profit_loss_ratio)
        .bind(backtest_result.total_trades)
        .bind(backtest_result.winning_trades)
        .bind(backtest_result.losing_trades)
        .bind(&equity_json)
        .bind(&symbols_str)
        .bind(commission_rate_dec)
        .bind(slippage_dec)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to persist backtest result: {}", e);
            ServiceError::from(e)
        })?;

        let duration_ms = start_time.elapsed().as_millis();
        info!(
            strategy_id = %backtest_result.strategy_id,
            total_return = %backtest_result.total_return,
            sharpe_ratio = %backtest_result.sharpe_ratio,
            max_drawdown = %backtest_result.max_drawdown,
            total_trades = backtest_result.total_trades,
            duration_ms,
            "Backtest completed"
        );
        Ok(backtest_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service_no_db() -> StrategyService {
        StrategyService::new(None, Arc::new(RwLock::new(None)))
    }

    #[tokio::test]
    async fn get_strategies_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc.get_strategies().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn save_strategy_no_db_returns_error() {
        let svc = make_service_no_db();
        let strategy = StrategyParams {
            strategy_id: "test_001".to_string(),
            strategy_name: "Test".to_string(),
            strategy_type: StrategyType::MeanReversion,
            params: serde_json::json!({}),
            enabled: true,
            max_position: Decimal::ZERO,
            max_daily_loss: Decimal::ZERO,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let result = svc.save_strategy(&strategy).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn delete_strategy_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc.delete_strategy("test_001").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn toggle_strategy_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc.toggle_strategy("test_001", true).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn run_backtest_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc
            .run_backtest(
                "test_001",
                chrono::Utc::now() - chrono::Duration::days(7),
                chrono::Utc::now(),
                100000.0,
                0.001,
                0.0005,
                &["BTC-USDT".to_string()],
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[test]
    fn new_with_none_creates_service_with_no_deps() {
        let svc = make_service_no_db();
        assert!(svc.postgres.is_none());
    }
}
