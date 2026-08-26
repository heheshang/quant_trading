use chrono::{DateTime, Utc};
use exchange_binance::types::BinanceKline;
use exchange_binance::websocket::{BinanceWsKline, BinanceWsTrade};
use quant_common::types::MarketData;
use quant_common::utils::{datetime_from_millis, datetime_from_millis_or_now};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::instrument;

/// A single K-line record as stored in the market_data partitioned table
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct MarketDataRecord {
    pub id: i64,
    pub instrument_id: String,
    pub timeframe: String,
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub created_at: Option<DateTime<Utc>>,
}
/// A single ticker snapshot row — maps 1:1 to `ticker_snapshots`.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TickerSnapshotRecord {
    pub instrument_id: String,
    pub ts: DateTime<Utc>,
    pub last_px: Option<Decimal>,
    pub open_24h: Option<Decimal>,
    pub high_24h: Option<Decimal>,
    pub low_24h: Option<Decimal>,
    pub vol_24h: Option<Decimal>,
    pub vol_ccy_24h: Option<Decimal>,
    pub change_24h: Option<Decimal>,
    pub created_at: Option<DateTime<Utc>>,
}

/// A single account snapshot row — maps 1:1 to `account_snapshots`.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AccountSnapshotRecord {
    pub ccy: String,
    pub ts: DateTime<Utc>,
    pub eq: Option<Decimal>,
    pub cash_bal: Option<Decimal>,
    pub avail_eq: Option<Decimal>,
    pub frozen_bal: Option<Decimal>,
    pub created_at: Option<DateTime<Utc>>,
}

/// A single position snapshot row — maps 1:1 to `position_snapshots`.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct PositionSnapshotRecord {
    pub inst_id: String,
    pub ts: DateTime<Utc>,
    pub pos: Option<Decimal>,
    pub avg_px: Option<Decimal>,
    pub upl: Option<Decimal>,
    pub upl_ratio: Option<Decimal>,
    pub mark_px: Option<Decimal>,
    pub created_at: Option<DateTime<Utc>>,
}

/// A single funding rate row — maps 1:1 to `funding_rates`.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct FundingRateRecord {
    pub inst_id: String,
    pub ts: DateTime<Utc>,
    pub funding_rate: Option<Decimal>,
    pub next_funding_rate: Option<Decimal>,
    pub funding_time: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

/// A single mark price row — maps 1:1 to `mark_prices`.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct MarkPriceRecord {
    pub inst_id: String,
    pub ts: DateTime<Utc>,
    pub mark_px: Option<Decimal>,
    pub idx_px: Option<Decimal>,
    pub created_at: Option<DateTime<Utc>>,
}

/// A single stream trade row (remote WS `@trade`, appended).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct StreamTradeRecord {
    pub id: i64,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub trade_time: DateTime<Utc>,
    pub is_buyer_maker: bool,
    pub created_at: Option<DateTime<Utc>>,
}

/// Latest orderbook snapshot row (remote WS `@depth`, per-symbol one row).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct OrderbookSnapshotRecord {
    pub symbol: String,
    pub bids: String,
    pub asks: String,
    pub ts: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
}

/// Per-asset balance row (snapshot writer, latest per asset).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BalanceRecord {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
    pub ts: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
}

/// Latest price row for a symbol (snapshot writer, domain symbol PK).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct LastPriceRecord {
    pub symbol: String,
    pub price: Decimal,
    pub ts: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
}

/// Repository for market_data partitioned table
pub struct MarketDataRepository {
    pool: PgPool,
}

impl MarketDataRepository {
    /// Create a new repository from a PgPool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a batch of K-line records in a single transaction
    #[instrument(skip(self, items), fields(count = items.len()))]
    pub async fn insert_batch(&self, items: &[NewMarketDataRecord]) -> Result<u64> {
        if items.is_empty() {
            return Ok(0);
        }

        let mut total = 0u64;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(format!("Failed to start transaction: {}", e)))?;

        for item in items {
            let rows_affected = sqlx::query(
                r#"
                INSERT INTO market_data (instrument_id, timeframe, timestamp, open, high, low, close, volume)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (instrument_id, timeframe, timestamp) DO NOTHING
                "#,
            )
            .bind(&item.instrument_id)
            .bind(&item.timeframe)
            .bind(item.timestamp)
            .bind(item.open)
            .bind(item.high)
            .bind(item.low)
            .bind(item.close)
            .bind(item.volume)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::Database(format!("Failed to insert market_data row: {}", e)))?;

            total += rows_affected.rows_affected() as u64;
        }

        tx.commit()
            .await
            .map_err(|e| Error::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(total)
    }

    /// Upsert a batch of klines in a single transaction (live forming-bar updates).
    ///
    /// Batching collapses the high-frequency per-message round-trips into one
    /// tx; each row still `DO UPDATE`s the forming bar in place, so repeated
    /// emissions of the same `(instrument_id, timeframe, timestamp)` converge.
    #[instrument(skip(self, items), fields(count = items.len()))]
    pub async fn upsert_klines_batch(&self, items: &[NewMarketDataRecord]) -> Result<u64> {
        if items.is_empty() {
            return Ok(0);
        }
        let mut total = 0u64;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(format!("Failed to start transaction: {}", e)))?;
        for item in items {
            let rows_affected = sqlx::query(
                r#"
                INSERT INTO market_data (instrument_id, timeframe, timestamp, open, high, low, close, volume)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (instrument_id, timeframe, timestamp) DO UPDATE SET
                    open = EXCLUDED.open,
                    high = EXCLUDED.high,
                    low = EXCLUDED.low,
                    close = EXCLUDED.close,
                    volume = EXCLUDED.volume
                "#,
            )
            .bind(&item.instrument_id)
            .bind(&item.timeframe)
            .bind(item.timestamp)
            .bind(item.open)
            .bind(item.high)
            .bind(item.low)
            .bind(item.close)
            .bind(item.volume)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::Database(format!("Failed to upsert market_data kline: {}", e)))?;
            total += rows_affected.rows_affected() as u64;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(format!("Failed to commit transaction: {}", e)))?;
        Ok(total)
    }

    /// Upsert a batch of ticker snapshots in a single transaction.
    ///
    /// Each row upserts on `(instrument_id, ts)`; the minute snapped ts keeps at
    /// most one row per instrument per minute that is updated in place.
    #[instrument(skip(self, items), fields(count = items.len()))]
    pub async fn upsert_tickers_batch(&self, items: &[NewTickerSnapshot]) -> Result<u64> {
        if items.is_empty() {
            return Ok(0);
        }
        let mut total = 0u64;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(format!("Failed to start transaction: {}", e)))?;
        for item in items {
            let rows_affected = sqlx::query(
                r#"
                INSERT INTO ticker_snapshots (instrument_id, ts, last_px, open_24h, high_24h, low_24h, vol_24h, vol_ccy_24h, change_24h)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (instrument_id, ts) DO UPDATE SET
                    last_px = EXCLUDED.last_px,
                    open_24h = EXCLUDED.open_24h,
                    high_24h = EXCLUDED.high_24h,
                    low_24h = EXCLUDED.low_24h,
                    vol_24h = EXCLUDED.vol_24h,
                    vol_ccy_24h = EXCLUDED.vol_ccy_24h,
                    change_24h = EXCLUDED.change_24h
                "#,
            )
            .bind(&item.instrument_id)
            .bind(item.ts)
            .bind(item.last_px)
            .bind(item.open_24h)
            .bind(item.high_24h)
            .bind(item.low_24h)
            .bind(item.vol_24h)
            .bind(item.vol_ccy_24h)
            .bind(item.change_24h)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::Database(format!("Failed to upsert ticker_snapshot: {}", e)))?;
            total += rows_affected.rows_affected() as u64;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(format!("Failed to commit transaction: {}", e)))?;
        Ok(total)
    }

    /// Append a batch of stream trades in a single transaction.
    #[instrument(skip(self, items), fields(count = items.len()))]
    pub async fn insert_trades_batch(&self, items: &[NewStreamTrade]) -> Result<u64> {
        if items.is_empty() {
            return Ok(0);
        }
        let mut total = 0u64;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(format!("Failed to start transaction: {}", e)))?;
        for item in items {
            let rows_affected = sqlx::query(
                r#"
                INSERT INTO stream_trades (symbol, price, quantity, trade_time, is_buyer_maker)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(&item.symbol)
            .bind(item.price)
            .bind(item.quantity)
            .bind(item.trade_time)
            .bind(item.is_buyer_maker)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::Database(format!("Failed to insert stream_trade: {}", e)))?;
            total += rows_affected.rows_affected() as u64;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(format!("Failed to commit transaction: {}", e)))?;
        Ok(total)
    }

    /// Upsert a batch of latest orderbook snapshots in a single transaction (per symbol).
    #[instrument(skip(self, items), fields(count = items.len()))]
    pub async fn upsert_orderbooks_batch(&self, items: &[NewOrderbookSnapshot]) -> Result<u64> {
        if items.is_empty() {
            return Ok(0);
        }
        let mut total = 0u64;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(format!("Failed to start transaction: {}", e)))?;
        for item in items {
            let rows_affected = sqlx::query(
                r#"
                INSERT INTO orderbook_snapshots (symbol, bids, asks, ts)
                VALUES ($1, $2, $3, now())
                ON CONFLICT (symbol) DO UPDATE SET
                    bids = EXCLUDED.bids,
                    asks = EXCLUDED.asks,
                    ts = now()
                "#,
            )
            .bind(&item.symbol)
            .bind(&item.bids)
            .bind(&item.asks)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::Database(format!("Failed to upsert orderbook_snapshot: {}", e)))?;
            total += rows_affected.rows_affected() as u64;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(format!("Failed to commit transaction: {}", e)))?;
        Ok(total)
    }

    /// Query market data by instrument, timeframe, and time range
    #[instrument(skip(self), fields(instrument_id = %instrument_id, timeframe = %timeframe, %from, %to))]
    pub async fn query_by_range(
        &self,
        instrument_id: &str,
        timeframe: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<MarketDataRecord>> {
        let max_rows = limit.unwrap_or(1000);

        let records = sqlx::query_as::<_, MarketDataRecord>(
            r#"
            SELECT id, instrument_id, timeframe, timestamp, open, high, low, close, volume, created_at
            FROM market_data
            WHERE instrument_id = $1
              AND timeframe = $2
              AND timestamp >= $3
              AND timestamp < $4
            ORDER BY timestamp ASC
            LIMIT $5
            "#,
        )
        .bind(instrument_id)
        .bind(timeframe)
        .bind(from)
        .bind(to)
        .bind(max_rows)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query market_data: {}", e)))?;

        Ok(records)
    }

    /// Latest N klines for an instrument/timeframe (newest first), read from DB.
    ///
    /// Remote WS import path complements: the frontend reads persisted klines
    /// from here instead of the live stream.
    #[instrument(skip(self), fields(instrument_id = %instrument_id, timeframe = %timeframe))]
    pub async fn query_latest_klines(
        &self,
        instrument_id: &str,
        timeframe: &str,
        limit: i64,
    ) -> Result<Vec<MarketDataRecord>> {
        let records = sqlx::query_as::<_, MarketDataRecord>(
            r#"
            SELECT id, instrument_id, timeframe, timestamp, open, high, low, close, volume, created_at
            FROM market_data
            WHERE instrument_id = $1 AND timeframe = $2
            ORDER BY timestamp DESC
            LIMIT $3
            "#,
        )
        .bind(instrument_id)
        .bind(timeframe)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query latest klines: {}", e)))?;
        Ok(records)
    }

    /// Distinct instruments present in `market_data` (the symbol dropdown source).
    #[instrument(skip(self))]
    pub async fn list_symbols(&self) -> Result<Vec<String>> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT DISTINCT instrument_id FROM market_data ORDER BY instrument_id")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| Error::Database(format!("Failed to list market_data symbols: {}", e)))?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    }

    /// Insert a single account snapshot record
    #[instrument(skip(self))]
    pub async fn insert_account_snapshot(&self, item: &NewAccountSnapshot) -> Result<u64> {
        let rows_affected = sqlx::query(
            r#"
            INSERT INTO account_snapshots (ccy, ts, eq, cash_bal, avail_eq, frozen_bal)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&item.ccy)
        .bind(item.ts)
        .bind(item.eq)
        .bind(item.cash_bal)
        .bind(item.avail_eq)
        .bind(item.frozen_bal)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to insert account_snapshot: {}", e)))?;

        Ok(rows_affected.rows_affected() as u64)
    }

    /// Insert a single position snapshot record
    #[instrument(skip(self))]
    pub async fn insert_position_snapshot(&self, item: &NewPositionSnapshot) -> Result<u64> {
        let rows_affected = sqlx::query(
            r#"
            INSERT INTO position_snapshots (inst_id, ts, pos, avg_px, upl, upl_ratio, mark_px)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&item.inst_id)
        .bind(item.ts)
        .bind(item.pos)
        .bind(item.avg_px)
        .bind(item.upl)
        .bind(item.upl_ratio)
        .bind(item.mark_px)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to insert position_snapshot: {}", e)))?;

        Ok(rows_affected.rows_affected() as u64)
    }

    /// Query ticker snapshots by instrument, optional time range, newest first.
    #[instrument(skip(self), fields(instrument_id = %instrument_id, from = ?from, to = ?to))]
    pub async fn query_ticker_snapshots(
        &self,
        instrument_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<i64>,
    ) -> Result<Vec<TickerSnapshotRecord>> {
        let max_rows = limit.unwrap_or(1000);
        let records = sqlx::query_as::<_, TickerSnapshotRecord>(
            r#"
            SELECT instrument_id, ts, last_px, open_24h, high_24h, low_24h,
                   vol_24h, vol_ccy_24h, change_24h, created_at
            FROM ticker_snapshots
            WHERE instrument_id = $1
              AND ($2::timestamptz IS NULL OR ts >= $2)
              AND ($3::timestamptz IS NULL OR ts <= $3)
            ORDER BY ts DESC
            LIMIT $4
            "#,
        )
        .bind(instrument_id)
        .bind(from)
        .bind(to)
        .bind(max_rows)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query ticker_snapshots: {}", e)))?;
        Ok(records)
    }

    /// Query account snapshots by currency, optional time range, newest first.
    #[instrument(skip(self), fields(ccy = %ccy, from = ?from, to = ?to))]
    pub async fn query_account_snapshots(
        &self,
        ccy: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<i64>,
    ) -> Result<Vec<AccountSnapshotRecord>> {
        let max_rows = limit.unwrap_or(1000);
        let records = sqlx::query_as::<_, AccountSnapshotRecord>(
            r#"
            SELECT ccy, ts, eq, cash_bal, avail_eq, frozen_bal, created_at
            FROM account_snapshots
            WHERE ccy = $1
              AND ($2::timestamptz IS NULL OR ts >= $2)
              AND ($3::timestamptz IS NULL OR ts <= $3)
            ORDER BY ts DESC
            LIMIT $4
            "#,
        )
        .bind(ccy)
        .bind(from)
        .bind(to)
        .bind(max_rows)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query account_snapshots: {}", e)))?;
        Ok(records)
    }

    /// Query position snapshots by instrument, optional time range, newest first.
    #[instrument(skip(self), fields(inst_id = %inst_id, from = ?from, to = ?to))]
    pub async fn query_position_snapshots(
        &self,
        inst_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<i64>,
    ) -> Result<Vec<PositionSnapshotRecord>> {
        let max_rows = limit.unwrap_or(1000);
        let records = sqlx::query_as::<_, PositionSnapshotRecord>(
            r#"
            SELECT inst_id, ts, pos, avg_px, upl, upl_ratio, mark_px, created_at
            FROM position_snapshots
            WHERE inst_id = $1
              AND ($2::timestamptz IS NULL OR ts >= $2)
              AND ($3::timestamptz IS NULL OR ts <= $3)
            ORDER BY ts DESC
            LIMIT $4
            "#,
        )
        .bind(inst_id)
        .bind(from)
        .bind(to)
        .bind(max_rows)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query position_snapshots: {}", e)))?;
        Ok(records)
    }

    /// Query funding rates by instrument, optional time range, newest first.
    #[instrument(skip(self), fields(inst_id = %inst_id, from = ?from, to = ?to))]
    pub async fn query_funding_rates(
        &self,
        inst_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<i64>,
    ) -> Result<Vec<FundingRateRecord>> {
        let max_rows = limit.unwrap_or(1000);
        let records = sqlx::query_as::<_, FundingRateRecord>(
            r#"
            SELECT inst_id, ts, funding_rate, next_funding_rate, funding_time, created_at
            FROM funding_rates
            WHERE inst_id = $1
              AND ($2::timestamptz IS NULL OR ts >= $2)
              AND ($3::timestamptz IS NULL OR ts <= $3)
            ORDER BY ts DESC
            LIMIT $4
            "#,
        )
        .bind(inst_id)
        .bind(from)
        .bind(to)
        .bind(max_rows)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query funding_rates: {}", e)))?;
        Ok(records)
    }

    /// Query mark prices by instrument, optional time range, newest first.
    #[instrument(skip(self), fields(inst_id = %inst_id, from = ?from, to = ?to))]
    pub async fn query_mark_prices(
        &self,
        inst_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<i64>,
    ) -> Result<Vec<MarkPriceRecord>> {
        let max_rows = limit.unwrap_or(1000);
        let records = sqlx::query_as::<_, MarkPriceRecord>(
            r#"
            SELECT inst_id, ts, mark_px, idx_px, created_at
            FROM mark_prices
            WHERE inst_id = $1
              AND ($2::timestamptz IS NULL OR ts >= $2)
              AND ($3::timestamptz IS NULL OR ts <= $3)
            ORDER BY ts DESC
            LIMIT $4
            "#,
        )
        .bind(inst_id)
        .bind(from)
        .bind(to)
        .bind(max_rows)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query mark_prices: {}", e)))?;
        Ok(records)
    }

    /// Latest N stream trades for a symbol (newest first), read from DB.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn query_latest_trades(&self, symbol: &str, limit: i64) -> Result<Vec<StreamTradeRecord>> {
        let records = sqlx::query_as::<_, StreamTradeRecord>(
            r#"
            SELECT id, symbol, price, quantity, trade_time, is_buyer_maker, created_at
            FROM stream_trades
            WHERE symbol = $1
            ORDER BY trade_time DESC
            LIMIT $2
            "#,
        )
        .bind(symbol)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query stream_trades: {}", e)))?;
        Ok(records)
    }

    /// Latest orderbook snapshot for a symbol, read from DB.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn query_latest_orderbook(&self, symbol: &str) -> Result<Option<OrderbookSnapshotRecord>> {
        let record = sqlx::query_as::<_, OrderbookSnapshotRecord>(
            r#"
            SELECT symbol, bids, asks, ts, created_at
            FROM orderbook_snapshots
            WHERE symbol = $1
            "#,
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query orderbook_snapshot: {}", e)))?;
        Ok(record)
    }

    /// Upsert latest per-asset balances (snapshot writer, per asset one row).
    #[instrument(skip(self), fields(count = items.len()))]
    pub async fn upsert_balances(&self, items: &[NewBalance]) -> Result<u64> {
        let mut total = 0u64;
        for item in items {
            let r = sqlx::query(
                r#"
                INSERT INTO balances (asset, free, locked, ts)
                VALUES ($1, $2, $3, now())
                ON CONFLICT (asset) DO UPDATE SET
                    free = EXCLUDED.free,
                    locked = EXCLUDED.locked,
                    ts = now()
                "#,
            )
            .bind(&item.asset)
            .bind(item.free)
            .bind(item.locked)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to upsert balance: {}", e)))?;
            total += r.rows_affected() as u64;
        }
        Ok(total)
    }

    /// All latest per-asset balances, read from DB.
    #[instrument(skip(self))]
    pub async fn query_latest_balances(&self) -> Result<Vec<BalanceRecord>> {
        let records = sqlx::query_as::<_, BalanceRecord>(
            r#"SELECT asset, free, locked, ts, created_at FROM balances ORDER BY asset"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query balances: {}", e)))?;
        Ok(records)
    }

    /// Upsert latest price for a symbol (snapshot writer, domain symbol PK).
    #[instrument(skip(self), fields(symbol = %item.symbol))]
    pub async fn upsert_last_price(&self, item: &NewLastPrice) -> Result<u64> {
        let r = sqlx::query(
            r#"
            INSERT INTO last_prices (symbol, price, ts)
            VALUES ($1, $2, now())
            ON CONFLICT (symbol) DO UPDATE SET
                price = EXCLUDED.price,
                ts = now()
            "#,
        )
        .bind(&item.symbol)
        .bind(item.price)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to upsert last_price: {}", e)))?;
        Ok(r.rows_affected() as u64)
    }

    /// Latest prices for all symbols, read from DB.
    #[instrument(skip(self))]
    pub async fn query_all_last_prices(&self) -> Result<Vec<LastPriceRecord>> {
        let records = sqlx::query_as::<_, LastPriceRecord>(
            r#"SELECT symbol, price, ts, created_at FROM last_prices ORDER BY symbol"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query last_prices: {}", e)))?;
        Ok(records)
    }

    /// Latest price for a single symbol, read from DB.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn query_latest_price(&self, symbol: &str) -> Result<Option<LastPriceRecord>> {
        let record = sqlx::query_as::<_, LastPriceRecord>(
            r#"SELECT symbol, price, ts, created_at FROM last_prices WHERE symbol = $1"#,
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query last_price: {}", e)))?;
        Ok(record)
    }
}

/// Input struct for inserting new K-line data
#[derive(Debug, Clone)]
pub struct NewMarketDataRecord {
    pub instrument_id: String,
    pub timeframe: String,
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

#[derive(Debug, Clone)]
pub struct NewTickerSnapshot {
    pub instrument_id: String,
    pub ts: DateTime<Utc>,
    pub last_px: Option<Decimal>,
    pub open_24h: Option<Decimal>,
    pub high_24h: Option<Decimal>,
    pub low_24h: Option<Decimal>,
    pub vol_24h: Option<Decimal>,
    pub vol_ccy_24h: Option<Decimal>,
    pub change_24h: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct NewAccountSnapshot {
    pub ccy: String,
    pub ts: DateTime<Utc>,
    pub eq: Option<Decimal>,
    pub cash_bal: Option<Decimal>,
    pub avail_eq: Option<Decimal>,
    pub frozen_bal: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct NewPositionSnapshot {
    pub inst_id: String,
    pub ts: DateTime<Utc>,
    pub pos: Option<Decimal>,
    pub avg_px: Option<Decimal>,
    pub upl: Option<Decimal>,
    pub upl_ratio: Option<Decimal>,
    pub mark_px: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct NewStreamTrade {
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub trade_time: DateTime<Utc>,
    pub is_buyer_maker: bool,
}

#[derive(Debug, Clone)]
pub struct NewOrderbookSnapshot {
    pub symbol: String,
    pub bids: String, // JSON array string
    pub asks: String, // JSON array string
}

impl NewMarketDataRecord {
    /// 由 REST kline（`/api/v3/klines`）构造；时间戳非法返回 `None`。
    pub fn from_kline(k: &BinanceKline, instrument_id: &str, interval: &str) -> Option<Self> {
        Some(Self {
            instrument_id: instrument_id.to_string(),
            timeframe: interval.to_string(),
            timestamp: datetime_from_millis(k.open_time)?,
            open: k.open,
            high: k.high,
            low: k.low,
            close: k.close,
            volume: k.volume,
        })
    }

    /// 由 WS `@kline` 消息构造；时间戳非法回退当前时间。
    pub fn from_ws_kline(k: &BinanceWsKline) -> Self {
        Self {
            instrument_id: k.symbol.clone(),
            timeframe: k.interval.to_lowercase(),
            timestamp: datetime_from_millis_or_now(k.open_time),
            open: k.open,
            high: k.high,
            low: k.low,
            close: k.close,
            volume: k.volume,
        }
    }

    /// 由领域行情构造（backfill 场景；时间戳已是 `DateTime`）。
    pub fn from_market_data(m: &MarketData, timeframe: &str) -> Self {
        Self {
            instrument_id: m.symbol.clone(),
            timeframe: timeframe.to_string(),
            timestamp: m.timestamp,
            open: m.open,
            high: m.high,
            low: m.low,
            close: m.close,
            volume: m.volume,
        }
    }
}

impl NewStreamTrade {
    /// 由 WS `@trade` 消息构造；时间戳非法回退当前时间。
    pub fn from_ws_trade(t: &BinanceWsTrade) -> Self {
        Self {
            symbol: t.symbol.clone(),
            price: t.price,
            quantity: t.quantity,
            trade_time: datetime_from_millis_or_now(t.trade_time),
            is_buyer_maker: t.is_buyer_maker,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewBalance {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
}

#[derive(Debug, Clone)]
pub struct NewLastPrice {
    pub symbol: String,
    pub price: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_market_data_record_creation() {
        let now = Utc::now();
        let record = NewMarketDataRecord {
            instrument_id: "BTC-USDT".to_string(),
            timeframe: "1h".to_string(),
            timestamp: now,
            open: Decimal::new(50000, 2),
            high: Decimal::new(51000, 2),
            low: Decimal::new(49000, 2),
            close: Decimal::new(50500, 2),
            volume: Decimal::new(1000, 0),
        };

        assert_eq!(record.instrument_id, "BTC-USDT");
        assert_eq!(record.timeframe, "1h");
        assert_eq!(record.open, Decimal::new(50000, 2));
    }

    #[test]
    fn test_market_data_record_roundtrip() {
        let now = Utc::now();
        let records = [
            NewMarketDataRecord {
                instrument_id: "ETH-USDT".to_string(),
                timeframe: "15m".to_string(),
                timestamp: now,
                open: Decimal::new(3000, 2),
                high: Decimal::new(3100, 2),
                low: Decimal::new(2950, 2),
                close: Decimal::new(3050, 2),
                volume: Decimal::new(5000, 0),
            },
            NewMarketDataRecord {
                instrument_id: "ETH-USDT".to_string(),
                timeframe: "15m".to_string(),
                timestamp: now,
                open: Decimal::new(3050, 2),
                high: Decimal::new(3150, 2),
                low: Decimal::new(3000, 2),
                close: Decimal::new(3100, 2),
                volume: Decimal::new(4500, 0),
            },
        ];

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].instrument_id, records[1].instrument_id);
    }
    #[test]
    fn test_ticker_snapshot_record_fields() {
        let now = Utc::now();
        let rec = TickerSnapshotRecord {
            instrument_id: "BTC-USDT".to_string(),
            ts: now,
            last_px: Some(Decimal::new(50000, 2)),
            open_24h: Some(Decimal::new(49900, 2)),
            high_24h: Some(Decimal::new(51000, 2)),
            low_24h: Some(Decimal::new(49000, 2)),
            vol_24h: Some(Decimal::new(1000, 0)),
            vol_ccy_24h: Some(Decimal::new(50, 0)),
            change_24h: Some(Decimal::new(2, 2)),
            created_at: Some(now),
        };
        assert_eq!(rec.instrument_id, "BTC-USDT");
        assert_eq!(rec.last_px, Some(Decimal::new(50000, 2)));
        assert!(rec.created_at.is_some());
    }

    #[test]
    fn test_funding_rate_record_fields() {
        let now = Utc::now();
        let rec = FundingRateRecord {
            inst_id: "BTC-USDT-SWAP".to_string(),
            ts: now,
            funding_rate: Some(Decimal::new(1, 4)),
            next_funding_rate: Some(Decimal::new(2, 4)),
            funding_time: Some(now),
            created_at: Some(now),
        };
        assert_eq!(rec.inst_id, "BTC-USDT-SWAP");
        assert!(rec.funding_rate.is_some());
    }

    #[test]
    fn test_mark_price_record_fields() {
        let now = Utc::now();
        let rec = MarkPriceRecord {
            inst_id: "ETH-USDT".to_string(),
            ts: now,
            mark_px: Some(Decimal::new(3050, 2)),
            idx_px: Some(Decimal::new(3049, 2)),
            created_at: Some(now),
        };
        assert_eq!(rec.mark_px, Some(Decimal::new(3050, 2)));
    }

    #[test]
    fn test_account_snapshot_record_fields() {
        let now = Utc::now();
        let rec = AccountSnapshotRecord {
            ccy: "USDT".to_string(),
            ts: now,
            eq: Some(Decimal::new(100000, 0)),
            cash_bal: Some(Decimal::new(90000, 0)),
            avail_eq: Some(Decimal::new(85000, 0)),
            frozen_bal: Some(Decimal::new(5000, 0)),
            created_at: Some(now),
        };
        assert_eq!(rec.ccy, "USDT");
        assert_eq!(rec.eq, Some(Decimal::new(100000, 0)));
    }

    #[test]
    fn test_position_snapshot_record_fields() {
        let now = Utc::now();
        let rec = PositionSnapshotRecord {
            inst_id: "BTC-USDT-SWAP".to_string(),
            ts: now,
            pos: Some(Decimal::new(2, 0)),
            avg_px: Some(Decimal::new(50000, 2)),
            upl: Some(Decimal::new(1000, 0)),
            upl_ratio: Some(Decimal::new(1, 2)),
            mark_px: Some(Decimal::new(50500, 2)),
            created_at: Some(now),
        };
        assert_eq!(rec.inst_id, "BTC-USDT-SWAP");
        assert_eq!(rec.pos, Some(Decimal::new(2, 0)));
    }
}
