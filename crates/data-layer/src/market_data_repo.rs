use chrono::{DateTime, Utc};
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

    /// Insert a single ticker snapshot record
    #[instrument(skip(self))]
    pub async fn insert_ticker_snapshot(&self, item: &NewTickerSnapshot) -> Result<u64> {
        let rows_affected = sqlx::query(
            r#"
            INSERT INTO ticker_snapshots (instrument_id, ts, last_px, open_24h, high_24h, low_24h, vol_24h, vol_ccy_24h, change_24h)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT DO NOTHING
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
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to insert ticker_snapshot: {}", e)))?;

        Ok(rows_affected.rows_affected() as u64)
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

    /// Insert a single funding rate record
    #[instrument(skip(self))]
    pub async fn insert_funding_rate(&self, item: &NewFundingRate) -> Result<u64> {
        let rows_affected = sqlx::query(
            r#"
            INSERT INTO funding_rates (inst_id, ts, funding_rate, next_funding_rate, funding_time)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&item.inst_id)
        .bind(item.ts)
        .bind(item.funding_rate)
        .bind(item.next_funding_rate)
        .bind(item.funding_time)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to insert funding_rate: {}", e)))?;

        Ok(rows_affected.rows_affected() as u64)
    }

    /// Insert a single mark price record
    #[instrument(skip(self))]
    pub async fn insert_mark_price(&self, item: &NewMarkPrice) -> Result<u64> {
        let rows_affected = sqlx::query(
            r#"
            INSERT INTO mark_prices (inst_id, ts, mark_px, idx_px)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&item.inst_id)
        .bind(item.ts)
        .bind(item.mark_px)
        .bind(item.idx_px)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to insert mark_price: {}", e)))?;

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
pub struct NewFundingRate {
    pub inst_id: String,
    pub ts: DateTime<Utc>,
    pub funding_rate: Option<Decimal>,
    pub next_funding_rate: Option<Decimal>,
    pub funding_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewMarkPrice {
    pub inst_id: String,
    pub ts: DateTime<Utc>,
    pub mark_px: Option<Decimal>,
    pub idx_px: Option<Decimal>,
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
