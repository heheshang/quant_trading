use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use data_layer::market_data_repo::MarketDataRepository;
use exchange_binance::ClientInterface;
use quant_common::config::DataPullerConfig;
use quant_common::error::Result;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{error, info, warn};

mod pulls;

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

/// Background task that periodically pulls market data from Binance and persists it.
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
            matches!(&err, Error::Network(msg) if msg == "always fail"),
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
