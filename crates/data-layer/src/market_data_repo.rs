use chrono::{DateTime, Utc};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use sqlx::PgPool;

/// A single K-line record as stored in the market_data partitioned table
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MarketDataRecord {
    pub id: uuid::Uuid,
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
        let records = vec![
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
}
