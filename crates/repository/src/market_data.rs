use chrono::{DateTime, Utc};
use quant_common::{Error, Result};
use quant_domain::MarketData;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::Arc;

/// 市场数据存储（PostgreSQL 分区表）
///
/// 底层使用 market_data 分区表（按时间 RANGE 分区）。
pub struct MarketDataRepository {
    pool: Arc<PgPool>,
}

impl MarketDataRepository {
    /// 从已有的 PostgreSQL 连接池创建。
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// 写入单条市场数据。
    pub async fn write_market_data(&self, data: &MarketData) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO market_data
                (symbol, timestamp, open, high, low, close, volume, turnover,
                 open_interest, bid_prices, bid_volumes, ask_prices, ask_volumes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(&data.symbol)
        .bind(data.timestamp)
        .bind(data.open)
        .bind(data.high)
        .bind(data.low)
        .bind(data.close)
        .bind(data.volume)
        .bind(data.turnover)
        .bind(data.open_interest)
        .bind(&data.bid_prices)
        .bind(&data.bid_volumes)
        .bind(&data.ask_prices)
        .bind(&data.ask_volumes)
        .execute(&*self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// 批量写入市场数据。
    pub async fn write_market_data_batch(&self, data: &[MarketData]) -> Result<()> {
        for market_data in data {
            self.write_market_data(market_data).await?;
        }
        Ok(())
    }

    /// 按交易对和时间范围查询市场数据。
    pub async fn query_market_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>> {
        let rows = sqlx::query_as::<_, MarketDataRow>(
            r#"
            SELECT symbol, timestamp, open, high, low, close, volume, turnover,
                   open_interest, bid_prices, bid_volumes, ask_prices, ask_volumes
            FROM market_data
            WHERE symbol = $1 AND timestamp >= $2 AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
        )
        .bind(symbol)
        .bind(start)
        .bind(end)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(MarketData::from).collect())
    }
}

/// sqlx 行映射 — 匹配 market_data 表结构。
#[derive(sqlx::FromRow)]
struct MarketDataRow {
    symbol: String,
    timestamp: DateTime<Utc>,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
    turnover: Decimal,
    open_interest: Option<Decimal>,
    bid_prices: Vec<Decimal>,
    bid_volumes: Vec<Decimal>,
    ask_prices: Vec<Decimal>,
    ask_volumes: Vec<Decimal>,
}

impl From<MarketDataRow> for MarketData {
    fn from(row: MarketDataRow) -> Self {
        MarketData {
            symbol: row.symbol,
            timestamp: row.timestamp,
            open: row.open,
            high: row.high,
            low: row.low,
            close: row.close,
            volume: row.volume,
            turnover: row.turnover,
            open_interest: row.open_interest,
            bid_prices: row.bid_prices,
            bid_volumes: row.bid_volumes,
            ask_prices: row.ask_prices,
            ask_volumes: row.ask_volumes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_data_row_conversion() {
        let row = MarketDataRow {
            symbol: "BTC-USDT".into(),
            timestamp: Utc::now(),
            open: Decimal::new(50000, 0),
            high: Decimal::new(51000, 0),
            low: Decimal::new(49000, 0),
            close: Decimal::new(50500, 0),
            volume: Decimal::new(1000, 0),
            turnover: Decimal::new(50000000, 0),
            open_interest: Some(Decimal::new(100, 0)),
            bid_prices: vec![Decimal::new(50400, 0), Decimal::new(50300, 0)],
            bid_volumes: vec![Decimal::new(10, 0), Decimal::new(20, 0)],
            ask_prices: vec![Decimal::new(50600, 0), Decimal::new(50700, 0)],
            ask_volumes: vec![Decimal::new(15, 0), Decimal::new(25, 0)],
        };

        let md = MarketData::from(row);
        assert_eq!(md.symbol, "BTC-USDT");
        assert_eq!(md.open, Decimal::new(50000, 0));
        assert_eq!(md.close, Decimal::new(50500, 0));
        assert_eq!(md.bid_prices.len(), 2);
        assert_eq!(md.ask_volumes[1], Decimal::new(25, 0));
        assert_eq!(md.open_interest, Some(Decimal::new(100, 0)));
    }
}
