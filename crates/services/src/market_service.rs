use crate::error::{ServiceError, ServiceResult};
use data_layer::market_data::DataSource;
use data_layer::{
    AccountSnapshotRecord, BalanceRecord, FundingRateRecord, LastPriceRecord, MarketDataRecord,
    MarketDataRepository, NewMarketDataRecord, OrderbookSnapshotRecord, PositionSnapshotRecord,
    StreamTradeRecord, TickerSnapshotRecord, MarkPriceRecord,
};
use quant_common::types::MarketData;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, instrument};

/// Market data service — retrieves real-time and historical data, and reads
/// persisted snapshots/funding/mark-price series from the repository.
pub struct MarketService {
    data_source: Arc<RwLock<Option<Arc<dyn DataSource>>>>,
    market_data: Option<Arc<MarketDataRepository>>,
    /// Per-(symbol, timeframe) kickoff time for REST backfill, throttled to one
    /// attempt per minute to avoid hammering Binance on repeated empty polls.
    backfill_guard: Arc<Mutex<HashMap<(String, String), Instant>>>,
}

impl MarketService {
    pub fn new(
        data_source: Arc<RwLock<Option<Arc<dyn DataSource>>>>,
        market_data: Option<Arc<MarketDataRepository>>,
    ) -> Self {
        Self {
            data_source,
            market_data,
            backfill_guard: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_realtime_data(&self, symbol: &str) -> ServiceResult<MarketData> {
        let ds = self.data_source.read().await;
        match ds.as_ref() {
            Some(source) => source.get_realtime_data(symbol).await.map_err(|e| {
                error!(symbol = %symbol, "Failed to get realtime data: {}", e);
                ServiceError::DataSource(e.to_string())
            }),
            None => {
                error!("market data source not configured for realtime data");
                Err(ServiceError::Other(
                    "market data source not configured (check exchange API configuration)".into(),
                ))
            }
        }
    }

    /// Distinct symbols available in the market_data store (dropdown source).
    #[instrument(skip(self))]
    pub async fn list_symbols(&self) -> ServiceResult<Vec<String>> {
        let repo = self.repo_or_err("symbols not available (no database)")?;
        repo.list_symbols().await.map_err(|e| {
            error!("Failed to list symbols: {}", e);
            ServiceError::Other(e.to_string())
        })
    }

    /// Latest N klines for an instrument/timeframe, read from DB, REST-backfilled
    /// on cold start.
    ///
    /// DB-first read path for the K-line chart: when the `market_data` table has
    /// no rows (or fewer than `limit`) for the requested timeframe — a timeframe
    /// the WebSocket has not been streaming yet — fetch `limit` historical klines
    /// from the Binance REST source, persist them, and re-read. Throttled to one
    /// REST backfill per (symbol, timeframe) per minute.
    #[instrument(skip(self), fields(symbol = %symbol, timeframe = %timeframe))]
    pub async fn get_klines(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: i64,
    ) -> ServiceResult<Vec<MarketDataRecord>> {
        let repo = self.repo_or_err("klines not available (no database)")?;
        let mut rows = repo
            .query_latest_klines(symbol, timeframe, limit)
            .await
            .map_err(|e| {
                error!("Failed to read klines from DB: {}", e);
                ServiceError::Other(e.to_string())
            })?;

        // 冷启动回填：库内该周期行数不足时才补（每分钟每组合最多一次）。
        if rows.len() < limit as usize {
            self.backfill_klines(symbol, timeframe, limit, &repo).await;
            rows = repo
                .query_latest_klines(symbol, timeframe, limit)
                .await
                .map_err(|e| {
                    error!("Failed to re-read klines from DB: {}", e);
                    ServiceError::Other(e.to_string())
                })?;
        }
        Ok(rows)
    }

    /// Fetch `limit` historical klines from the REST source and persist them to
    /// `market_data` (best-effort; failures log and leave the DB as-is).
    async fn backfill_klines(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: i64,
        repo: &Arc<MarketDataRepository>,
    ) {
        let key = (symbol.to_string(), timeframe.to_string());
        {
            let guard = self.backfill_guard.lock().unwrap();
            if let Some(last) = guard.get(&key) {
                if last.elapsed() < Duration::from_secs(60) {
                    return;
                }
            }
        }
        {
            let mut guard = self.backfill_guard.lock().unwrap();
            guard.insert(key, Instant::now());
        }

        let ds = self.data_source.read().await;
        let Some(source) = ds.as_ref() else {
            return;
        };
        let data = match source.get_klines_history(symbol, timeframe, limit).await {
            Ok(d) => d,
            Err(e) => {
                error!(symbol = %symbol, timeframe = %timeframe, "kline backfill fetch failed: {}", e);
                return;
            }
        };
        if data.is_empty() {
            return;
        }

        let records: Vec<NewMarketDataRecord> = data
            .into_iter()
            .map(|m| NewMarketDataRecord {
                instrument_id: m.symbol.clone(),
                timeframe: timeframe.to_string(),
                timestamp: m.timestamp,
                open: m.open,
                high: m.high,
                low: m.low,
                close: m.close,
                volume: m.volume,
            })
            .collect();

        match repo.insert_batch(&records).await {
            Ok(n) => info!(symbol = %symbol, timeframe = %timeframe, inserted = n, "kline backfill persisted"),
            Err(e) => error!(symbol = %symbol, timeframe = %timeframe, "kline backfill persist failed: {}", e),
        }
    }

    /// Latest N stream trades for a symbol, read from DB (`stream_trades`).
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_trades_from_db(&self, symbol: &str, limit: i64) -> ServiceResult<Vec<StreamTradeRecord>> {
        let repo = self.repo_or_err("trades not available (no database)")?;
        repo.query_latest_trades(symbol, limit)
            .await
            .map_err(|e| {
                error!("Failed to read stream trades from DB: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Latest orderbook snapshot for a symbol, read from DB (`orderbook_snapshots`).
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_orderbook_from_db(&self, symbol: &str) -> ServiceResult<Option<OrderbookSnapshotRecord>> {
        let repo = self.repo_or_err("orderbook not available (no database)")?;
        repo.query_latest_orderbook(symbol)
            .await
            .map_err(|e| {
                error!("Failed to read orderbook from DB: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Latest per-asset balances, read from DB (`balances`).
    #[instrument(skip(self))]
    pub async fn get_balances_from_db(&self) -> ServiceResult<Vec<BalanceRecord>> {
        let repo = self.repo_or_err("balances not available (no database)")?;
        repo.query_latest_balances()
            .await
            .map_err(|e| {
                error!("Failed to read balances from DB: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Latest prices for all symbols, read from DB (`last_prices`).
    #[instrument(skip(self))]
    pub async fn get_last_prices_from_db(&self) -> ServiceResult<Vec<LastPriceRecord>> {
        let repo = self.repo_or_err("last_prices not available (no database)")?;
        repo.query_all_last_prices()
            .await
            .map_err(|e| {
                error!("Failed to read last_prices from DB: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Point-in-time行情快照，从 DB 读（优先最新 `ticker_snapshots`，缺失时回退 `last_prices` 取价）。
    ///
    /// remote WS 导入的行情入库后，前端不再直连币安 REST。
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_market_data_from_db(&self, symbol: &str) -> ServiceResult<MarketData> {
        let repo = self.repo_or_err("market data not available (no database)")?;
        let rows = repo
            .query_ticker_snapshots(symbol, None, None, Some(1))
            .await
            .map_err(|e| {
                error!("Failed to read ticker snapshot from DB: {}", e);
                ServiceError::Other(e.to_string())
            })?;
        if let Some(r) = rows.into_iter().next() {
            return Ok(MarketData {
                symbol: r.instrument_id,
                timestamp: r.ts,
                open: r.open_24h.unwrap_or(Decimal::ZERO),
                high: r.high_24h.unwrap_or(Decimal::ZERO),
                low: r.low_24h.unwrap_or(Decimal::ZERO),
                close: r.last_px.unwrap_or(Decimal::ZERO),
                volume: r.vol_24h.unwrap_or(Decimal::ZERO),
                turnover: r.vol_ccy_24h.unwrap_or(Decimal::ZERO),
                open_interest: None,
                bid_prices: vec![],
                bid_volumes: vec![],
                ask_prices: vec![],
                ask_volumes: vec![],
            });
        }
        // 无 ticker 快照（未订阅）：回退 last_prices 取现价，OHLC 均记现值。
        let lp = repo
            .query_latest_price(symbol)
            .await
            .map_err(|e| {
                error!("Failed to read last_price from DB: {}", e);
                ServiceError::Other(e.to_string())
            })?
            .ok_or_else(|| ServiceError::NotFound(format!("No market data for {}", symbol)))?;
        Ok(MarketData {
            symbol: symbol.to_string(),
            timestamp: lp.ts,
            open: lp.price,
            high: lp.price,
            low: lp.price,
            close: lp.price,
            volume: Decimal::ZERO,
            turnover: Decimal::ZERO,
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        })
    }

    /// Read persisted ticker snapshots for an instrument.
    #[instrument(skip(self), fields(inst_id = %inst_id))]
    pub async fn get_ticker_snapshots(
        &self,
        inst_id: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<TickerSnapshotRecord>> {
        let repo = self.repo_or_err("ticker snapshots not available (no database)")?;
        repo.query_ticker_snapshots(inst_id, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query ticker snapshots: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Read persisted account snapshots for a currency.
    #[instrument(skip(self), fields(ccy = %ccy))]
    pub async fn get_account_snapshots(
        &self,
        ccy: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<AccountSnapshotRecord>> {
        let repo = self.repo_or_err("account snapshots not available (no database)")?;
        repo.query_account_snapshots(ccy, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query account snapshots: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Read persisted position snapshots for an instrument.
    #[instrument(skip(self), fields(inst_id = %inst_id))]
    pub async fn get_position_snapshots(
        &self,
        inst_id: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<PositionSnapshotRecord>> {
        let repo = self.repo_or_err("position snapshots not available (no database)")?;
        repo.query_position_snapshots(inst_id, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query position snapshots: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Read persisted funding rates for an instrument.
    #[instrument(skip(self), fields(inst_id = %inst_id))]
    pub async fn get_funding_rates(
        &self,
        inst_id: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<FundingRateRecord>> {
        let repo = self.repo_or_err("funding rates not available (no database)")?;
        repo.query_funding_rates(inst_id, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query funding rates: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Read persisted mark prices for an instrument.
    #[instrument(skip(self), fields(inst_id = %inst_id))]
    pub async fn get_mark_prices(
        &self,
        inst_id: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<MarkPriceRecord>> {
        let repo = self.repo_or_err("mark prices not available (no database)")?;
        repo.query_mark_prices(inst_id, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query mark prices: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    fn repo_or_err(&self, msg: &str) -> ServiceResult<Arc<MarketDataRepository>> {
        self.market_data.clone().ok_or_else(|| {
            error!("{}", msg);
            ServiceError::Other(msg.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_realtime_data_no_datasource() {
        let svc = MarketService::new(Arc::new(RwLock::new(None)), None);
        let result = svc.get_realtime_data("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Other(_)));
    }

    #[tokio::test]
    async fn test_get_funding_rates_no_repo() {
        let svc = MarketService::new(Arc::new(RwLock::new(None)), None);
        let result = svc
            .get_funding_rates("BTC-USDT", None, None, Some(10))
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Other(_)));
    }

    #[tokio::test]
    async fn test_get_mark_prices_no_repo() {
        let svc = MarketService::new(Arc::new(RwLock::new(None)), None);
        let result = svc.get_mark_prices("BTC-USDT", None, None, Some(10)).await;
        assert!(result.is_err());
    }
}
