use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quant_domain::BacktestResult;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, instrument};

use crate::error::RepoError;

/// Database row type — maps 1:1 to `backtest_results` table columns.
#[derive(Debug, Clone, sqlx::FromRow)]
struct BacktestResultRow {
    id: i64,
    strategy_id: String,
    strategy_name: Option<String>,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    initial_capital: Decimal,
    final_capital: Decimal,
    total_return: Decimal,
    annual_return: Decimal,
    sharpe_ratio: Option<Decimal>,
    max_drawdown: Option<Decimal>,
    win_rate: Option<Decimal>,
    profit_loss_ratio: Option<Decimal>,
    total_trades: Option<i32>,
    winning_trades: Option<i32>,
    losing_trades: Option<i32>,
    equity_curve: Option<serde_json::Value>,
    symbols: Option<Vec<String>>,
    commission_rate: Option<Decimal>,
    slippage: Option<Decimal>,
    created_at: DateTime<Utc>,
}

impl BacktestResultRow {
    fn to_backtest_result(&self) -> BacktestResult {
        let equity_curve: Vec<(DateTime<Utc>, Decimal)> = self
            .equity_curve
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        BacktestResult {
            id: Some(self.id),
            strategy_id: self.strategy_id.clone(),
            start_date: self.start_date,
            end_date: self.end_date,
            initial_capital: self.initial_capital,
            final_capital: self.final_capital,
            total_return: self.total_return,
            annual_return: self.annual_return,
            sharpe_ratio: self.sharpe_ratio.unwrap_or(Decimal::ZERO),
            max_drawdown: self.max_drawdown.unwrap_or(Decimal::ZERO),
            win_rate: self.win_rate.unwrap_or(Decimal::ZERO),
            profit_loss_ratio: self.profit_loss_ratio.unwrap_or(Decimal::ZERO),
            total_trades: self.total_trades.unwrap_or(0),
            winning_trades: self.winning_trades.unwrap_or(0),
            losing_trades: self.losing_trades.unwrap_or(0),
            equity_curve,
        }
    }
}

/// Summary row — lightweight version for list queries (excludes equity_curve JSONB).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BacktestResultSummaryRow {
    pub id: i64,
    pub strategy_id: String,
    pub strategy_name: Option<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub total_return: Decimal,
    pub sharpe_ratio: Option<Decimal>,
    pub max_drawdown: Option<Decimal>,
    pub total_trades: Option<i32>,
    pub win_rate: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

/// Backtest data access trait.
#[async_trait]
pub trait BacktestRepository: Send + Sync + 'static {
    /// Query backtest results with pagination (sorted by created_at DESC).
    async fn find_all(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BacktestResultSummaryRow>, RepoError>;

    /// Query a single backtest result by ID (includes equity_curve).
    async fn find_by_id(&self, id: i64) -> Result<Option<BacktestResult>, RepoError>;

    /// Insert a backtest result into the database.
    async fn insert(
        &self,
        result: &BacktestResult,
        db_name: &str,
        symbols: &[String],
        commission_rate: Decimal,
        slippage: Decimal,
    ) -> Result<(), RepoError>;

    /// Delete a backtest result by ID. Returns true if a row was deleted.
    async fn delete_by_id(&self, id: i64) -> Result<bool, RepoError>;
}

/// PostgreSQL implementation of `BacktestRepository`.
#[derive(Debug, Clone)]
pub struct PgBacktestRepository {
    pool: Arc<PgPool>,
}

impl PgBacktestRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BacktestRepository for PgBacktestRepository {
    #[instrument(skip(self), fields(limit, offset))]
    async fn find_all(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BacktestResultSummaryRow>, RepoError> {
        sqlx::query_as::<_, BacktestResultSummaryRow>(
            r#"
            SELECT id, strategy_id, strategy_name,
                   start_date, end_date,
                   total_return, sharpe_ratio, max_drawdown,
                   total_trades, win_rate, created_at
            FROM backtest_results
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query backtest results: {}", e);
            RepoError::from(e)
        })
    }

    #[instrument(skip(self), fields(%id))]
    async fn find_by_id(&self, id: i64) -> Result<Option<BacktestResult>, RepoError> {
        let row = sqlx::query_as::<_, BacktestResultRow>(
            r#"
            SELECT id, strategy_id, strategy_name,
                   start_date, end_date,
                   initial_capital, final_capital,
                   total_return, annual_return,
                   sharpe_ratio, max_drawdown,
                   win_rate, profit_loss_ratio,
                   total_trades, winning_trades, losing_trades,
                   equity_curve, symbols,
                   commission_rate, slippage, created_at
            FROM backtest_results
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query backtest result {}: {}", id, e);
            RepoError::from(e)
        })?;

        Ok(row.map(|r| r.to_backtest_result()))
    }

    #[instrument(skip(self, result), fields(backtest_id = ?result.id, strategy_id = %result.strategy_id))]
    async fn insert(
        &self,
        result: &BacktestResult,
        db_name: &str,
        symbols: &[String],
        commission_rate: Decimal,
        slippage: Decimal,
    ) -> Result<(), RepoError> {
        let backtest_id = result.id.unwrap_or(0);
        let equity_json = serde_json::to_value(&result.equity_curve)
            .map_err(|e| RepoError::Database(format!("Failed to serialize equity curve: {}", e)))?;
        let symbols_str: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();

        sqlx::query(
            r#"
            INSERT INTO backtest_results (
                id, strategy_id, strategy_name,
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
        .bind(&result.strategy_id)
        .bind(db_name)
        .bind(result.start_date)
        .bind(result.end_date)
        .bind(result.initial_capital)
        .bind(result.final_capital)
        .bind(result.total_return)
        .bind(result.annual_return)
        .bind(result.sharpe_ratio)
        .bind(result.max_drawdown)
        .bind(result.win_rate)
        .bind(result.profit_loss_ratio)
        .bind(result.total_trades)
        .bind(result.winning_trades)
        .bind(result.losing_trades)
        .bind(&equity_json)
        .bind(&symbols_str)
        .bind(commission_rate)
        .bind(slippage)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to insert backtest result: {}", e);
            RepoError::from(e)
        })?;

        Ok(())
    }

    #[instrument(skip(self), fields(%id))]
    async fn delete_by_id(&self, id: i64) -> Result<bool, RepoError> {
        let affected = sqlx::query("DELETE FROM backtest_results WHERE id = $1")
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                error!("Failed to delete backtest result {}: {}", id, e);
                RepoError::from(e)
            })?;

        Ok(affected.rows_affected() > 0)
    }
}
