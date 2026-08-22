//! 各类数据拉取后台任务。

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use data_layer::market_data_repo::{
    MarketDataRepository, NewAccountSnapshot, NewFundingRate, NewMarkPrice, NewMarketDataRecord,
    NewPositionSnapshot, NewTickerSnapshot,
};
use exchange_okx::ClientInterface;
use quant_common::config::{CandlePullConfig, IntervalConfig, TickerPullConfig};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::{api_call_with_retry, parse_decimal, parse_timestamp, DataPuller};

impl DataPuller {
    /// Periodic candle pull loop for one symbol + bar combination.
    pub(super) async fn run_candle_pull(
        client_lock: Arc<RwLock<dyn ClientInterface + Send + Sync>>,
        repo: Arc<MarketDataRepository>,
        symbol: String,
        bar: String,
        config: CandlePullConfig,
    ) {
        let interval = Duration::from_secs(config.interval_secs);
        info!(
            "Starting candle pull: {} / {} every {:?}",
            symbol, bar, interval
        );

        loop {
            sleep(interval).await;

            let candles =
                match api_call_with_retry(&format!("get_candles/{}/{}", symbol, bar), 4, || async {
                    let client = client_lock.read().await;
                    client.get_candles(&symbol, &bar, Some(config.limit)).await
                })
                .await
                {
                    Ok(c) => c,
                    Err(e) => {
                        error!(
                            "Failed to fetch candles for {} / {} after retries: {}",
                            symbol, bar, e
                        );
                        continue;
                    }
                };

            let records: Vec<NewMarketDataRecord> = candles
                .into_iter()
                .filter_map(|c| {
                    let ts = parse_timestamp(&c.ts)?;
                    Some(NewMarketDataRecord {
                        instrument_id: symbol.clone(),
                        timeframe: bar.clone(),
                        timestamp: ts,
                        open: parse_decimal(&c.open)?,
                        high: parse_decimal(&c.high)?,
                        low: parse_decimal(&c.low)?,
                        close: parse_decimal(&c.close)?,
                        volume: parse_decimal(&c.vol)?,
                    })
                })
                .collect();

            if records.is_empty() {
                warn!("No valid candle records for {} / {}", symbol, bar);
                continue;
            }

            match repo.insert_batch(&records).await {
                Ok(count) => {
                    if count > 0 {
                        info!("Inserted {} candles for {} / {}", count, symbol, bar);
                    }
                }
                Err(e) => error!("Failed to insert candles for {} / {}: {}", symbol, bar, e),
            }
        }
    }

    /// Periodic ticker pull loop for one symbol.
    pub(super) async fn run_ticker_pull(
        client_lock: Arc<RwLock<dyn ClientInterface + Send + Sync>>,
        repo: Arc<MarketDataRepository>,
        symbol: String,
        config: TickerPullConfig,
    ) {
        let interval = Duration::from_secs(config.interval_secs);
        info!("Starting ticker pull: {} every {:?}", symbol, interval);

        loop {
            sleep(interval).await;

            let ticker = match api_call_with_retry(&format!("get_ticker/{}", symbol), 4, || async {
                let client = client_lock.read().await;
                client.get_ticker(&symbol).await
            })
            .await
            {
                Ok(t) => t,
                Err(e) => {
                    error!("Failed to fetch ticker for {} after retries: {}", symbol, e);
                    continue;
                }
            };

            let ts = match parse_timestamp(&ticker.ts) {
                Some(t) => t,
                None => {
                    error!("Invalid ticker timestamp: {}", ticker.ts);
                    continue;
                }
            };

            let record = NewTickerSnapshot {
                instrument_id: symbol.clone(),
                ts,
                last_px: parse_decimal(&ticker.last),
                open_24h: parse_decimal(&ticker.open_24h),
                high_24h: parse_decimal(&ticker.high_24h),
                low_24h: parse_decimal(&ticker.low_24h),
                vol_24h: parse_decimal(&ticker.vol_24h),
                vol_ccy_24h: parse_decimal(&ticker.vol_ccy_24h),
                change_24h: None,
            };

            match repo.insert_ticker_snapshot(&record).await {
                Ok(count) => {
                    if count > 0 {
                        info!("Inserted ticker snapshot for {}", symbol);
                    }
                }
                Err(e) => error!("Failed to insert ticker for {}: {}", symbol, e),
            }
        }
    }

    /// Periodic funding rate pull loop for one swap symbol.
    pub(super) async fn run_funding_rate_pull(
        client_lock: Arc<RwLock<dyn ClientInterface + Send + Sync>>,
        repo: Arc<MarketDataRepository>,
        symbol: String,
        config: IntervalConfig,
    ) {
        let interval = Duration::from_secs(config.interval_secs);
        info!(
            "Starting funding rate pull: {} every {:?}",
            symbol, interval
        );

        loop {
            sleep(interval).await;

            let rate =
                match api_call_with_retry(&format!("get_funding_rate/{}", symbol), 4, || async {
                    let client = client_lock.read().await;
                    client.get_funding_rate(&symbol).await
                })
                .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        error!(
                            "Failed to fetch funding rate for {} after retries: {}",
                            symbol, e
                        );
                        continue;
                    }
                };

            let record = NewFundingRate {
                inst_id: symbol.clone(),
                ts: Utc::now(),
                funding_rate: parse_decimal(&rate.funding_rate),
                next_funding_rate: parse_decimal(&rate.next_funding_rate),
                funding_time: parse_timestamp(&rate.funding_time),
            };

            match repo.insert_funding_rate(&record).await {
                Ok(count) => {
                    if count > 0 {
                        info!("Inserted funding rate for {}", symbol);
                    }
                }
                Err(e) => error!("Failed to insert funding rate for {}: {}", symbol, e),
            }
        }
    }

    /// Periodic mark price pull loop for one symbol.
    pub(super) async fn run_mark_price_pull(
        client_lock: Arc<RwLock<dyn ClientInterface + Send + Sync>>,
        repo: Arc<MarketDataRepository>,
        symbol: String,
        config: IntervalConfig,
    ) {
        let interval = Duration::from_secs(config.interval_secs);
        info!("Starting mark price pull: {} every {:?}", symbol, interval);

        loop {
            sleep(interval).await;

            let mp = match api_call_with_retry(&format!("get_mark_price/{}", symbol), 4, || async {
                let client = client_lock.read().await;
                client.get_mark_price(&symbol).await
            })
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    error!(
                        "Failed to fetch mark price for {} after retries: {}",
                        symbol, e
                    );
                    continue;
                }
            };

            let record = NewMarkPrice {
                inst_id: symbol.clone(),
                ts: Utc::now(),
                mark_px: parse_decimal(&mp.mark_px),
                idx_px: None,
            };

            match repo.insert_mark_price(&record).await {
                Ok(count) => {
                    if count > 0 {
                        info!("Inserted mark price for {}", symbol);
                    }
                }
                Err(e) => error!("Failed to insert mark price for {}: {}", symbol, e),
            }
        }
    }

    /// Periodic account balance snapshot pull.
    pub(super) async fn run_account_pull(
        client_lock: Arc<RwLock<dyn ClientInterface + Send + Sync>>,
        repo: Arc<MarketDataRepository>,
        config: IntervalConfig,
    ) {
        let interval = Duration::from_secs(config.interval_secs);
        info!("Starting account balance pull every {:?}", interval);

        loop {
            sleep(interval).await;

            let balances = match api_call_with_retry("get_account_balance", 4, || async {
                let client = client_lock.read().await;
                client.get_account_balance(None).await
            })
            .await
            {
                Ok(b) => b,
                Err(e) => {
                    error!("Failed to fetch account balance after retries: {}", e);
                    continue;
                }
            };

            let ts = Utc::now();
            let count = balances.len();
            for balance in &balances {
                let record = NewAccountSnapshot {
                    ccy: balance.ccy.clone(),
                    ts,
                    eq: parse_decimal(&balance.eq),
                    cash_bal: parse_decimal(&balance.cash_bal),
                    avail_eq: parse_decimal(&balance.avail_eq),
                    frozen_bal: parse_decimal(&balance.frozen_bal),
                };

                if let Err(e) = repo.insert_account_snapshot(&record).await {
                    error!(
                        "Failed to insert account snapshot for {}: {}",
                        balance.ccy, e
                    );
                }
            }
            info!("Inserted {} account balance snapshots", count);
        }
    }

    /// Periodic position snapshot pull.
    pub(super) async fn run_position_pull(
        client_lock: Arc<RwLock<dyn ClientInterface + Send + Sync>>,
        repo: Arc<MarketDataRepository>,
        config: IntervalConfig,
    ) {
        let interval = Duration::from_secs(config.interval_secs);
        info!("Starting position pull every {:?}", interval);

        loop {
            sleep(interval).await;

            let positions = match api_call_with_retry("get_positions", 4, || async {
                let client = client_lock.read().await;
                client.get_positions(None).await
            })
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to fetch positions after retries: {}", e);
                    continue;
                }
            };

            let ts = Utc::now();
            let count = positions.len();
            for pos in &positions {
                let record = NewPositionSnapshot {
                    inst_id: pos.inst_id.clone(),
                    ts,
                    pos: parse_decimal(&pos.pos),
                    avg_px: parse_decimal(&pos.avg_px),
                    upl: parse_decimal(&pos.upl),
                    upl_ratio: parse_decimal(&pos.upl_ratio),
                    mark_px: None,
                };

                if let Err(e) = repo.insert_position_snapshot(&record).await {
                    error!(
                        "Failed to insert position snapshot for {}: {}",
                        pos.inst_id, e
                    );
                }
            }
            info!("Inserted {} position snapshots", count);
        }
    }
}
