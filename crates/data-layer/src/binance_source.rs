use crate::market_data::DataSource;
use chrono::{DateTime, Utc};
use exchange_binance::types::{
    to_binance_symbol, BinanceKline, BinanceOrderBook, BinanceTicker24h,
};
use exchange_binance::ClientInterface;
use quant_common::types::MarketData;
use quant_common::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

/// Default candle timeframe used by the live Binance fallback.
///
/// Kept in sync with `services::market_data_provider::DEFAULT_TIMEFRAME`
/// ("1H") so the persistence-first repository query and this live fallback
/// agree on granularity. Binance's REST endpoint expects the lowercase form
/// ("1h"), which we normalize before issuing the request.
pub const DEFAULT_TIMEFRAME: &str = "1H";

/// Binance 数据源（市场数据经 Binance 供给）。
///
/// Historical klines are fetched from Binance REST and mapped to
/// [`MarketData`]. Realtime data is a Binance REST snapshot from
/// `ticker/24hr` and `depth`, mapped to [`MarketData`]; the push stream is
/// wired separately through the `binance:*` WebSocket events.
#[derive(Clone)]
pub struct BinanceDataSource {
    client: Arc<RwLock<dyn ClientInterface>>,
}

impl BinanceDataSource {
    /// 创建新的 Binance 数据源。
    pub fn new(client: Arc<RwLock<dyn ClientInterface>>) -> Self {
        Self { client }
    }

    /// 从 Binance K线数据转换为 MarketData。
    fn kline_to_market_data(symbol: &str, kline: &BinanceKline) -> Result<MarketData> {
        Ok(MarketData {
            symbol: symbol.to_string(),
            timestamp: DateTime::from_timestamp(kline.open_time / 1000, 0).unwrap_or_else(Utc::now),
            open: kline.open,
            high: kline.high,
            low: kline.low,
            close: kline.close,
            volume: kline.volume,
            turnover: kline.quote_volume,
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        })
    }

    /// 从 Binance 24h ticker + 订单簿快照构建实时 MarketData。
    fn ticker_to_market_data(
        symbol: &str,
        ticker: &BinanceTicker24h,
        book: &BinanceOrderBook,
    ) -> MarketData {
        MarketData {
            symbol: symbol.to_string(),
            timestamp: Utc::now(),
            open: ticker.open,
            high: ticker.high,
            low: ticker.low,
            close: ticker.last_price,
            volume: ticker.volume,
            turnover: ticker.quote_volume,
            open_interest: None,
            bid_prices: book.bids.iter().map(|(p, _)| *p).collect(),
            bid_volumes: book.bids.iter().map(|(_, q)| *q).collect(),
            ask_prices: book.asks.iter().map(|(p, _)| *p).collect(),
            ask_volumes: book.asks.iter().map(|(_, q)| *q).collect(),
        }
    }

    /// Binance kline intervals are lowercase ("1h", "1d"); the domain config
    /// uses uppercase ("1H", "1D"). Normalize before issuing the request.
    fn normalize_interval(interval: &str) -> String {
        interval.to_lowercase()
    }
}

#[async_trait::async_trait]
impl DataSource for BinanceDataSource {
    #[instrument(skip(self), fields(symbol = %symbol))]
    async fn get_realtime_data(&self, symbol: &str) -> Result<MarketData> {
        let binance_symbol = to_binance_symbol(symbol);
        let client = self.client.read().await;
        let ticker = client.get_ticker_24hr(&binance_symbol).await?;
        // Best-effort order book for bid/ask ladders; a failure degrades to an
        // empty ladder rather than failing the whole snapshot.
        let book = client
            .get_order_book(&binance_symbol, Some(10))
            .await
            .unwrap_or_else(|_| BinanceOrderBook {
                symbol: binance_symbol.clone(),
                bids: vec![],
                asks: vec![],
            });
        Ok(Self::ticker_to_market_data(symbol, &ticker, &book))
    }

    #[instrument(skip(self), fields(symbol = %symbol, %start, %end))]
    async fn get_historical_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>> {
        let binance_symbol = to_binance_symbol(symbol);
        let timeframe = Self::normalize_interval(DEFAULT_TIMEFRAME);
        let client = self.client.read().await;

        // Binance `get_candles(symbol, interval, limit)` returns the most
        // recent `limit` klines; request enough to cover the requested window
        // (capped at Binance's per-request max of 1000) and filter client-side.
        let hours = end.signed_duration_since(start).num_hours();
        let limit = (hours.max(1) as u32).min(1000);

        let candles = client
            .get_candles(&binance_symbol, &timeframe, Some(limit))
            .await?;

        let mut market_data = Vec::new();
        for kline in candles {
            match Self::kline_to_market_data(symbol, &kline) {
                Ok(data) => {
                    if data.timestamp >= start && data.timestamp <= end {
                        market_data.push(data);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to convert Binance kline to market data: {}", e);
                    continue;
                }
            }
        }

        Ok(market_data)
    }

    async fn subscribe(&self, _symbols: Vec<String>) -> Result<()> {
        // Binance WebSocket subscription is handled through the `binance:*`
        // event stream; this hook stays `Ok(())` to keep the trait honest.
        Ok(())
    }

    async fn unsubscribe(&self, _symbols: Vec<String>) -> Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl quant_common::MarketDataProvider for BinanceDataSource {
    async fn get_historical_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> std::result::Result<Vec<MarketData>, String> {
        <Self as DataSource>::get_historical_data(self, symbol, start, end)
            .await
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn sample_kline() -> BinanceKline {
        BinanceKline {
            open_time: 1_700_000_000_000,
            open: Decimal::new(50_000, 0),
            high: Decimal::new(51_000, 0),
            low: Decimal::new(49_000, 0),
            close: Decimal::new(50_500, 0),
            volume: Decimal::new(12, 0),
            close_time: 1_700_000_003_600_000,
            quote_volume: Decimal::new(605_000, 0),
            trades: 42,
        }
    }

    fn sample_ticker() -> BinanceTicker24h {
        BinanceTicker24h {
            symbol: "BTCUSDT".to_string(),
            last_price: Decimal::new(50_500, 0),
            price_change: Decimal::new(250, 0),
            price_change_percent: Decimal::new(50, 1),
            high: Decimal::new(51_000, 0),
            low: Decimal::new(49_000, 0),
            open: Decimal::new(50_250, 0),
            volume: Decimal::new(12, 0),
            quote_volume: Decimal::new(605_000, 0),
        }
    }

    fn sample_order_book() -> BinanceOrderBook {
        #[allow(clippy::inconsistent_digit_grouping)]
        BinanceOrderBook {
            symbol: "BTCUSDT".to_string(),
            bids: vec![(Decimal::new(50_400, 0), Decimal::new(1, 0))],
            asks: vec![(Decimal::new(50_600, 0), Decimal::new(2, 0))],
        }
    }

    #[test]
    fn kline_maps_to_market_data() {
        let md = BinanceDataSource::kline_to_market_data("BTC-USDT", &sample_kline()).unwrap();
        assert_eq!(md.symbol, "BTC-USDT");
        assert_eq!(md.open, Decimal::new(50_000, 0));
        assert_eq!(md.high, Decimal::new(51_000, 0));
        assert_eq!(md.low, Decimal::new(49_000, 0));
        assert_eq!(md.close, Decimal::new(50_500, 0));
        assert_eq!(md.volume, Decimal::new(12, 0));
        // Turnover maps Binance quote_volume; order-book / open-interest stay
        // at their honest defaults.
        assert_eq!(md.turnover, Decimal::new(605_000, 0));
        assert_eq!(md.open_interest, None);
        assert!(md.bid_prices.is_empty());
        assert!(md.ask_prices.is_empty());
        // open_time (ms) → Utc timestamp.
        assert_eq!(md.timestamp.timestamp_millis(), 1_700_000_000_000);
    }

    #[test]
    fn ticker_maps_to_realtime_market_data() {
        let md = BinanceDataSource::ticker_to_market_data(
            "BTC-USDT",
            &sample_ticker(),
            &sample_order_book(),
        );
        assert_eq!(md.symbol, "BTC-USDT");
        assert_eq!(md.open, Decimal::new(50_250, 0));
        assert_eq!(md.high, Decimal::new(51_000, 0));
        assert_eq!(md.low, Decimal::new(49_000, 0));
        assert_eq!(md.close, Decimal::new(50_500, 0));
        assert_eq!(md.volume, Decimal::new(12, 0));
        assert_eq!(md.turnover, Decimal::new(605_000, 0));
        assert_eq!(md.open_interest, None);
        assert_eq!(md.bid_prices, vec![Decimal::new(50_400, 0)]);
        assert_eq!(md.bid_volumes, vec![Decimal::new(1, 0)]);
        assert_eq!(md.ask_prices, vec![Decimal::new(50_600, 0)]);
        assert_eq!(md.ask_volumes, vec![Decimal::new(2, 0)]);
    }

    #[test]
    fn normalizes_interval_to_lowercase() {
        assert_eq!(BinanceDataSource::normalize_interval("1H"), "1h");
        assert_eq!(BinanceDataSource::normalize_interval("1m"), "1m");
        assert_eq!(BinanceDataSource::normalize_interval("1D"), "1d");
        assert_eq!(DEFAULT_TIMEFRAME, "1H");
    }
}
