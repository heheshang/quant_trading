use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use data_layer::market_data_repo::{
    MarketDataRepository, NewAccountSnapshot, NewFundingRate, NewMarkPrice, NewMarketDataRecord,
    NewPositionSnapshot, NewTickerSnapshot,
};
use exchange_okx::ClientInterface;
use quant_common::config::{CandlePullConfig, DataPullerConfig, IntervalConfig, TickerPullConfig};
use quant_common::error::Result;
use rust_decimal::Decimal;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Retry an async API call with exponential backoff.
///
/// Calls `f()` up to `max_attempts` times. Between attempts, sleeps for
/// `2^(attempt-1)` seconds (1s, 2s, 4s, ...). Logs a warning on each failed
/// attempt and an error when all attempts are exhausted.
async fn api_call_with_retry<F, Fut, T>(operation: &str, max_attempts: usize, f: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_err = None;
    for attempt in 1..=max_attempts {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                warn!(
                    "{operation} attempt {attempt}/{max_attempts} failed: {e}",
                    operation = operation,
                    attempt = attempt,
                    max_attempts = max_attempts,
                    e = e,
                );
                last_err = Some(e);
                if attempt < max_attempts {
                    let delay = Duration::from_secs(1 << (attempt - 1));
                    sleep(delay).await;
                }
            }
        }
    }
    error!(
        "{operation} failed after {max_attempts} attempts",
        operation = operation,
        max_attempts = max_attempts,
    );
    Err(last_err.unwrap_or_else(|| {
        quant_common::Error::Internal(format!("{operation} failed before any retry attempt"))
    }))
}

/// Background task that periodically pulls market data from OKX and persists it.
pub struct DataPuller {
    config: DataPullerConfig,
    client: Arc<RwLock<dyn ClientInterface + Send + Sync>>,
    repo: Arc<MarketDataRepository>,
}

impl DataPuller {
    pub fn new(
        config: DataPullerConfig,
        client: Arc<RwLock<dyn ClientInterface + Send + Sync>>,
        repo: Arc<MarketDataRepository>,
    ) -> Self {
        Self {
            config,
            client,
            repo,
        }
    }

    /// Start all configured pull tasks. Returns when all tasks exit (normally runs indefinitely).
    pub async fn run(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Data puller is disabled, no tasks will be started");
            return Ok(());
        }

        let symbols = self.config.symbols.clone();
        if symbols.is_empty() {
            warn!("Data puller is enabled but no symbols configured");
            return Ok(());
        }

        info!(
            "Starting data puller for {} symbols: {:?}",
            symbols.len(),
            symbols
        );

        let mut handles = Vec::new();

        // Candle pull tasks
        if self.config.candle.enabled {
            for symbol in &symbols {
                for bar in &self.config.candle.bars {
                    let handle = tokio::spawn(Self::run_candle_pull(
                        Arc::clone(&self.client),
                        Arc::clone(&self.repo),
                        symbol.clone(),
                        bar.clone(),
                        self.config.candle.clone(),
                    ));
                    handles.push(handle);
                }
            }
        }

        // Ticker pull tasks
        if self.config.ticker.enabled {
            for symbol in &symbols {
                let handle = tokio::spawn(Self::run_ticker_pull(
                    Arc::clone(&self.client),
                    Arc::clone(&self.repo),
                    symbol.clone(),
                    self.config.ticker.clone(),
                ));
                handles.push(handle);
            }
        }

        // Funding rate pull tasks (perpetual swaps only — append -SWAP suffix)
        if self.config.funding_rate.enabled {
            for symbol in &symbols {
                let swap_symbol = format!("{}-SWAP", symbol);
                let handle = tokio::spawn(Self::run_funding_rate_pull(
                    Arc::clone(&self.client),
                    Arc::clone(&self.repo),
                    swap_symbol,
                    self.config.funding_rate.clone(),
                ));
                handles.push(handle);
            }
        }

        // Mark price pull tasks
        if self.config.mark_price.enabled {
            for symbol in &symbols {
                let handle = tokio::spawn(Self::run_mark_price_pull(
                    Arc::clone(&self.client),
                    Arc::clone(&self.repo),
                    symbol.clone(),
                    self.config.mark_price.clone(),
                ));
                handles.push(handle);
            }
        }

        // Account balance pull tasks
        if self.config.account_balance.enabled {
            let handle = tokio::spawn(Self::run_account_pull(
                Arc::clone(&self.client),
                Arc::clone(&self.repo),
                self.config.account_balance.clone(),
            ));
            handles.push(handle);
        }

        // Position pull tasks
        if self.config.positions.enabled {
            let handle = tokio::spawn(Self::run_position_pull(
                Arc::clone(&self.client),
                Arc::clone(&self.repo),
                self.config.positions.clone(),
            ));
            handles.push(handle);
        }

        info!("All data puller tasks started ({} total)", handles.len());

        // Wait for all tasks (they run indefinitely until cancelled)
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Data puller task panicked: {}", e);
            }
        }

        Ok(())
    }

    /// Periodic candle pull loop for one symbol + bar combination.
    async fn run_candle_pull(
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
    async fn run_ticker_pull(
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
    async fn run_funding_rate_pull(
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
    async fn run_mark_price_pull(
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
    async fn run_account_pull(
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
    async fn run_position_pull(
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

/// Parse a string timestamp (milliseconds since epoch) into DateTime<Utc>.
fn parse_timestamp(ts: &str) -> Option<DateTime<Utc>> {
    let millis: i64 = ts.parse().ok()?;
    DateTime::from_timestamp_millis(millis)
}

/// Parse a string decimal value into Decimal. Returns None for empty/invalid strings.
fn parse_decimal(s: &str) -> Option<Decimal> {
    if s.is_empty() {
        return None;
    }
    s.parse::<Decimal>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::error::Error;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Helper: a test error that is distinct for assertion purposes.
    fn test_error(msg: &str) -> Error {
        Error::Network(msg.into())
    }

    #[tokio::test]
    async fn test_retry_first_attempt_succeeds() {
        // A closure that always succeeds — no retries needed.
        let result = api_call_with_retry("test_ok", 4, || async { Ok::<_, Error>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_all_attempts_fail() {
        // A closure that always fails — expect Err after max_attempts.
        let result: Result<i32> = api_call_with_retry("test_all_fail", 3, || async {
            Err(test_error("always fail"))
        })
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::Network(ref msg) if msg == "always fail"),
            "expected Network error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_two_failures() {
        // Fails twice, then succeeds on the 3rd attempt (0-indexed).
        let attempt = Arc::new(AtomicUsize::new(0));

        let result = api_call_with_retry("test_retry_ok", 5, || {
            let attempt = Arc::clone(&attempt);
            async move {
                let current = attempt.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    Err(test_error(&format!("attempt {current}")))
                } else {
                    Ok(current)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 2);
        assert_eq!(attempt.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_max_attempts_exhausted_count() {
        // Fails on every attempt; verify the function actually calls the
        // closure exactly `max_attempts` times.
        let attempt = Arc::new(AtomicUsize::new(0));

        let _: Result<i32> = api_call_with_retry("test_exhausted", 2, || {
            let attempt = Arc::clone(&attempt);
            async move {
                attempt.fetch_add(1, Ordering::SeqCst);
                Err(test_error("persistent"))
            }
        })
        .await;

        assert_eq!(attempt.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_single_attempt_no_retry() {
        // With max_attempts = 1, the closure is called exactly once (no retry).
        let attempt = Arc::new(AtomicUsize::new(0));

        let result: Result<i32> = api_call_with_retry("test_single", 1, || {
            let attempt = Arc::clone(&attempt);
            async move {
                attempt.fetch_add(1, Ordering::SeqCst);
                Err(test_error("single"))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempt.load(Ordering::SeqCst), 1);
    }
}
