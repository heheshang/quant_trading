use crate::market_data::DataSource;
use chrono::{DateTime, Utc};
use exchange_okx::Client as OkxClient;
use quant_common::types::MarketData;
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

/// OKX 数据源
#[derive(Clone)]
pub struct OkxDataSource {
    client: Arc<RwLock<OkxClient>>,
}

impl OkxDataSource {
    /// 创建新的 OKX 数据源
    pub fn new(client: Arc<RwLock<OkxClient>>) -> Self {
        Self { client }
    }

    /// 从 OKX K线数据转换为 MarketData
    fn candle_to_market_data(
        symbol: &str,
        candle: &exchange_okx::types::OkxCandle,
    ) -> Result<MarketData> {
        Ok(MarketData {
            symbol: symbol.to_string(),
            timestamp: DateTime::from_timestamp(
                candle
                    .ts
                    .parse::<i64>()
                    .map_err(|e| Error::Internal(e.to_string()))?
                    / 1000,
                0,
            )
            .unwrap_or_else(Utc::now),
            open: Decimal::from_str(&candle.open).map_err(|e| Error::Internal(e.to_string()))?,
            high: Decimal::from_str(&candle.high).map_err(|e| Error::Internal(e.to_string()))?,
            low: Decimal::from_str(&candle.low).map_err(|e| Error::Internal(e.to_string()))?,
            close: Decimal::from_str(&candle.close).map_err(|e| Error::Internal(e.to_string()))?,
            volume: Decimal::from_str(&candle.vol).map_err(|e| Error::Internal(e.to_string()))?,
            turnover: Decimal::from_str(&candle.vol_ccy)
                .map_err(|e| Error::Internal(e.to_string()))?,
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        })
    }
}

#[async_trait::async_trait]
impl DataSource for OkxDataSource {
    #[instrument(skip(self), fields(symbol = %symbol))]
    async fn get_realtime_data(&self, symbol: &str) -> Result<MarketData> {
        let client = self.client.read().await;

        // Get the latest candle (1m)
        let candles = client.get_candles(symbol, "1m", Some(1)).await?;

        if let Some(candle) = candles.first() {
            Self::candle_to_market_data(symbol, candle)
        } else {
            Err(Error::NotFound(format!(
                "No realtime data found for {}",
                symbol
            )))
        }
    }

    #[instrument(skip(self), fields(symbol = %symbol, %start, %end))]
    async fn get_historical_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>> {
        let client = self.client.read().await;

        // Calculate the number of candles needed (using 1H intervals)
        let duration = end.signed_duration_since(start);
        let hours = duration.num_hours();
        let limit = hours.min(300) as u32; // OKX has a limit

        let candles = client.get_candles(symbol, "1H", Some(limit)).await?;

        let mut market_data = Vec::new();
        for candle in candles {
            match Self::candle_to_market_data(symbol, &candle) {
                Ok(data) => {
                    // Filter by date range
                    if data.timestamp >= start && data.timestamp <= end {
                        market_data.push(data);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to convert candle to market data: {}", e);
                    continue;
                }
            }
        }

        Ok(market_data)
    }

    async fn subscribe(&self, _symbols: Vec<String>) -> Result<()> {
        // WebSocket subscription would be implemented here
        // For now, we'll return success
        Ok(())
    }

    async fn unsubscribe(&self, _symbols: Vec<String>) -> Result<()> {
        // WebSocket unsubscription would be implemented here
        Ok(())
    }
}
