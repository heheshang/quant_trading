//! 各类数据拉取后台任务。
//!
//! 本阶段市场数据改由 Binance 供给：candles / account_balance / positions
//! 从 Binance REST 拉取。Binance client 尚无 funding_rate / mark_price /
//! ticker 对等端点，对应拉取任务已移除（不伪造数据）。

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use data_layer::market_data_repo::{
    MarketDataRepository, NewAccountSnapshot, NewMarketDataRecord, NewPositionSnapshot,
};
use exchange_binance::types::{from_binance_symbol, to_binance_symbol};
use exchange_binance::ClientInterface;
use quant_common::config::{CandlePullConfig, IntervalConfig};
use quant_common::error::Result;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::{api_call_with_retry, DataPuller};

/// 拉取一批数据；失败时记录错误并返回 `None`（调用方 `continue`）。
///
/// 模板方法：统一“重试拉取 + 错误处理”的公共骨架，避免拉取循环重复该逻辑。
async fn fetch_or_continue<T, F, Fut>(name: &str, operation: &str, fetch: F) -> Option<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    match api_call_with_retry(operation, 4, fetch).await {
        Ok(data) => Some(data),
        Err(e) => {
            error!("Failed to fetch {} after retries: {}", name, e);
            None
        }
    }
}

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

            let Some(candles) = fetch_or_continue(
                &format!("candles {}/{}", symbol, bar),
                &format!("get_candles/{}/{}", symbol, bar),
                || async {
                    let client = client_lock.read().await;
                    client
                        .get_candles(
                            &to_binance_symbol(&symbol),
                            &bar.to_lowercase(),
                            Some(config.limit),
                        )
                        .await
                },
            )
            .await
            else {
                continue;
            };

            let records: Vec<NewMarketDataRecord> = candles
                .into_iter()
                .filter_map(|c| {
                    let ts = chrono::DateTime::from_timestamp_millis(c.open_time)?;
                    Some(NewMarketDataRecord {
                        instrument_id: symbol.clone(),
                        timeframe: bar.to_lowercase(),
                        timestamp: ts,
                        open: c.open,
                        high: c.high,
                        low: c.low,
                        close: c.close,
                        volume: c.volume,
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

            let Some(balances) =
                fetch_or_continue("account balance", "get_account_balance", || async {
                    let client = client_lock.read().await;
                    client.get_account_balance().await
                })
                .await
            else {
                continue;
            };

            let ts = Utc::now();
            let count = balances.len();
            for balance in &balances {
                let record = NewAccountSnapshot {
                    ccy: balance.asset.clone(),
                    ts,
                    eq: Some(balance.free + balance.locked),
                    cash_bal: Some(balance.free),
                    avail_eq: Some(balance.free),
                    frozen_bal: Some(balance.locked),
                };

                if let Err(e) = repo.insert_account_snapshot(&record).await {
                    error!(
                        "Failed to insert account snapshot for {}: {}",
                        balance.asset, e
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

            let Some(positions) = fetch_or_continue("positions", "get_positions", || async {
                let client = client_lock.read().await;
                client.get_positions(None).await
            })
            .await
            else {
                continue;
            };

            let ts = Utc::now();
            let count = positions.len();
            for pos in &positions {
                let record = NewPositionSnapshot {
                    inst_id: from_binance_symbol(&pos.symbol),
                    ts,
                    pos: Some(pos.position_amt),
                    avg_px: Some(pos.entry_price),
                    upl: Some(pos.un_realized_profit),
                    upl_ratio: None,
                    mark_px: Some(pos.mark_price),
                };

                if let Err(e) = repo.insert_position_snapshot(&record).await {
                    error!(
                        "Failed to insert position snapshot for {}: {}",
                        pos.symbol, e
                    );
                }
            }
            info!("Inserted {} position snapshots", count);
        }
    }
}
