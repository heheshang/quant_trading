//! Repository for live (Binance) order fills / metadata.
//!
//! Persists each Binance order placed through the app (strategy link + fill
//! price/qty) so the UI can show strategy info and compute real P&L locally
//! instead of re-querying Binance per-asset (rate-limit avoidance).
use chrono::{DateTime, Utc};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::instrument;

/// A single live-trade record — maps 1:1 to `live_trades`.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct LiveTrade {
    pub id: i64,
    pub order_id: i64,
    pub symbol: String,
    pub strategy_id: String,
    pub side: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Repository for `live_trades`.
pub struct LiveTradesRepository {
    pool: PgPool,
}

impl LiveTradesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert a live trade by `order_id` (idempotent).
    #[instrument(skip(self), fields(order_id = %order_id, symbol = %symbol))]
    pub async fn upsert(
        &self,
        order_id: i64,
        symbol: &str,
        strategy_id: &str,
        side: &str,
        price: Decimal,
        quantity: Decimal,
        filled_quantity: Decimal,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO live_trades
                (order_id, symbol, strategy_id, side, price, quantity, filled_quantity, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (order_id) DO UPDATE SET
                strategy_id = EXCLUDED.strategy_id,
                side = EXCLUDED.side,
                price = EXCLUDED.price,
                quantity = EXCLUDED.quantity,
                filled_quantity = EXCLUDED.filled_quantity,
                status = EXCLUDED.status,
                updated_at = now()
            "#,
        )
        .bind(order_id)
        .bind(symbol)
        .bind(strategy_id)
        .bind(side)
        .bind(price)
        .bind(quantity)
        .bind(filled_quantity)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to upsert live_trade: {}", e)))?;
        Ok(())
    }

    /// All live trades, newest first.
    #[instrument(skip(self))]
    pub async fn list_all(&self) -> Result<Vec<LiveTrade>> {
        let rows = sqlx::query_as::<_, LiveTrade>(
            r#"
            SELECT id, order_id, symbol, strategy_id, side, price, quantity,
                   filled_quantity, status, created_at, updated_at
            FROM live_trades
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to list live_trades: {}", e)))?;
        Ok(rows)
    }
}
