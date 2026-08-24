//! Remote Binance WebSocket → PostgreSQL import pipeline (market data).
//!
//! A batching importer that consumes [`BinanceWsMessage`], converts each message
//! into a `data_layer` insert/upsert record, and flushes accumulated batches to
//! the database on a short timer or once a per-type batch fills. The writer runs
//! on its own task; callers hand messages over through a bounded channel
//! (`try_send`), so a slow database never blocks the WebSocket forwarding loop
//! (and, by backpressure, the socket read loop).
//!
//! This crate intentionally has no Tauri dependency: the pipeline is a plain
//! `Sink` consumer usable by the desktop app or a headless ingestion service.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

use chrono::{DateTime, Utc};
use data_layer::{
    MarketDataRepository, NewMarketDataRecord, NewOrderbookSnapshot, NewStreamTrade,
    NewTickerSnapshot,
};
use exchange_binance::websocket::{
    BinanceWsDepth, BinanceWsKline, BinanceWsMessage, BinanceWsTicker, BinanceWsTrade,
};
use quant_common::error::Result;
use tokio::sync::mpsc::{self, error::TrySendError};
use tracing::{debug, info, warn};

/// One accumulated group of import records, keyed by target table.
#[derive(Debug, Clone, Default)]
pub struct ImportBatch {
    pub klines: Vec<NewMarketDataRecord>,
    pub tickers: Vec<NewTickerSnapshot>,
    pub trades: Vec<NewStreamTrade>,
    pub orderbooks: Vec<NewOrderbookSnapshot>,
}

impl ImportBatch {
    fn is_empty(&self) -> bool {
        self.klines.is_empty()
            && self.tickers.is_empty()
            && self.trades.is_empty()
            && self.orderbooks.is_empty()
    }

    /// Number of rows buffered across all groups.
    fn len(&self) -> usize {
        self.klines.len() + self.tickers.len() + self.trades.len() + self.orderbooks.len()
    }

    fn clear(&mut self) {
        self.klines.clear();
        self.tickers.clear();
        self.trades.clear();
        self.orderbooks.clear();
    }
}

/// Persistence backend for the market import pipeline.
///
/// Implemented by [`MarketDataRepository`]; split out as a trait so the batching
/// writer can be unit-tested against a mock backend without a live PostgreSQL.
#[async_trait::async_trait]
pub trait MarketSink: Send + Sync + 'static {
    /// Write one accumulated batch. Flushes every non-empty group; the first
    /// error is returned after the remaining groups are still attempted.
    async fn flush_batch(&self, batch: &ImportBatch) -> Result<()>;
}

#[async_trait::async_trait]
impl MarketSink for MarketDataRepository {
    async fn flush_batch(&self, batch: &ImportBatch) -> Result<()> {
        // Independent groups: keep them isolated so one failing table doesn't
        // silently block the others. Capture the first error, flush the rest,
        // then surface it to the caller for observability.
        let mut first_err: Option<quant_common::Error> = None;

        if !batch.klines.is_empty() {
            if let Err(e) = self.upsert_klines_batch(&batch.klines).await {
                first_err.get_or_insert(e);
            }
        }
        if !batch.tickers.is_empty() {
            if let Err(e) = self.upsert_tickers_batch(&batch.tickers).await {
                first_err.get_or_insert(e);
            }
        }
        if !batch.trades.is_empty() {
            if let Err(e) = self.insert_trades_batch(&batch.trades).await {
                first_err.get_or_insert(e);
            }
        }
        if !batch.orderbooks.is_empty() {
            if let Err(e) = self.upsert_orderbooks_batch(&batch.orderbooks).await {
                first_err.get_or_insert(e);
            }
        }

        match first_err {
            Some(e) => {
                warn!(error = %e, "market import batch group failed");
                Err(e)
            }
            None => Ok(()),
        }
    }
}

#[async_trait::async_trait]
impl<S> MarketSink for Arc<S>
where
    S: MarketSink,
{
    async fn flush_batch(&self, batch: &ImportBatch) -> Result<()> {
        (**self).flush_batch(batch).await
    }
}

/// Configuration for the import pipeline.
#[derive(Debug, Clone)]
pub struct ImportOpts {
    /// Max rows held per type before an early flush (guards against a flood).
    pub batch_capacity: usize,
    /// Max time a batch is held before flushing (drains a steady trickle).
    pub flush_interval: Duration,
    /// Bounded channel capacity between producer and writer.
    pub channel_capacity: usize,
}

impl Default for ImportOpts {
    fn default() -> Self {
        Self {
            batch_capacity: 256,
            flush_interval: Duration::from_millis(50),
            channel_capacity: 1024,
        }
    }
}

/// Internal message handed from [`MarketImporter::try_send`] to the writer task.
enum ImportMsg {
    Kline(NewMarketDataRecord),
    Ticker(NewTickerSnapshot),
    Trade(NewStreamTrade),
    OrderBook(NewOrderbookSnapshot),
}

/// Producer handle used to feed WS messages into the import pipeline.
///
/// `try_send` is non-blocking: if the bounded channel is full the message is
/// dropped and a drop counter is incremented (see [`MarketImporter::dropped`]).
/// This is the safety valve that keeps the WebSocket forward loop (and the
/// socket read loop behind it) stall-free under a slow database.
#[derive(Clone)]
pub struct MarketImporter {
    tx: mpsc::Sender<ImportMsg>,
    dropped: Arc<AtomicU64>,
}

impl MarketImporter {
    /// Create an importer bound to `sink`, spawning the batching writer task.
    pub fn new<S>(sink: S) -> Self
    where
        S: MarketSink,
    {
        Self::with_options(sink, ImportOpts::default())
    }

    /// Create an importer with explicit pipeline tuning.
    pub fn with_options<S>(sink: S, opts: ImportOpts) -> Self
    where
        S: MarketSink,
    {
        let (tx, rx) = mpsc::channel::<ImportMsg>(opts.channel_capacity);
        let dropped = Arc::new(AtomicU64::new(0));
        tokio::spawn(writer_task(sink, rx, opts, dropped.clone()));
        Self { tx, dropped }
    }

    /// Feed a parsed WS message.
    ///
    /// Returns `true` when accepted (unsupported message kinds are a no-op that
    /// still counts as accepted), `false` when dropped because the channel was
    /// full or the writer has shut down. The Tauri forward loop treats `false`
    /// as "skip, don't block".
    pub fn try_send(&self, msg: &BinanceWsMessage) -> bool {
        let record = match msg {
            BinanceWsMessage::Kline(k) => Some(ImportMsg::Kline(kline_to_record(k))),
            BinanceWsMessage::Ticker(t) => Some(ImportMsg::Ticker(ticker_to_record(t))),
            BinanceWsMessage::Trade(t) => Some(ImportMsg::Trade(trade_to_record(t))),
            BinanceWsMessage::Depth(d) | BinanceWsMessage::OrderBook(d) => {
                Some(ImportMsg::OrderBook(depth_to_record(d)))
            }
            // Account/order updates flow through the user-data stream, not here.
            BinanceWsMessage::AccountPosition(_)
            | BinanceWsMessage::OrderUpdate(_)
            | BinanceWsMessage::ConnectionStatus(_)
            | BinanceWsMessage::Error(_) => None,
        };

        match record {
            None => true,
            Some(m) => match self.tx.try_send(m) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                    false
                }
                Err(TrySendError::Closed(_)) => false,
            },
        }
    }

    /// Messages dropped because the bounded channel was full.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

/// Batching writer task: accumulates records and flushes on a timer or capacity.
async fn writer_task<S>(
    sink: S,
    mut rx: mpsc::Receiver<ImportMsg>,
    opts: ImportOpts,
    dropped: Arc<AtomicU64>,
) where
    S: MarketSink,
{
    let mut batch = ImportBatch::default();
    let mut deadline = Instant::now() + opts.flush_interval;
    let mut flushes = 0u64;
    let mut drop_warned = false;
    info!(
        capacity = opts.batch_capacity,
        interval_ms = opts.flush_interval.as_millis(),
        channel = opts.channel_capacity,
        "market import writer started"
    );

    loop {
        let timer = tokio::time::sleep_until(deadline);
        tokio::pin!(timer);

        tokio::select! {
            _ = &mut timer => {
                deadline = Instant::now() + opts.flush_interval;
                finish_flush(&sink, &mut batch, &mut flushes).await;
            }
            msg = rx.recv() => {
                match msg {
                    Some(ImportMsg::Kline(k)) => { batch.klines.push(k); }
                    Some(ImportMsg::Ticker(t)) => { batch.tickers.push(t); }
                    Some(ImportMsg::Trade(t)) => { batch.trades.push(t); }
                    Some(ImportMsg::OrderBook(d)) => { batch.orderbooks.push(d); }
                    None => break,
                }
                // Early flush once a per-type batch fills (bound memory on floods).
                if batch.klines.len() >= opts.batch_capacity
                    || batch.tickers.len() >= opts.batch_capacity
                    || batch.trades.len() >= opts.batch_capacity
                    || batch.orderbooks.len() >= opts.batch_capacity
                {
                    deadline = Instant::now() + opts.flush_interval;
                    finish_flush(&sink, &mut batch, &mut flushes).await;
                }
            }
        }

        let d = dropped.load(Ordering::Relaxed);
        if d > 0 && !drop_warned {
            warn!(dropped = d, "market import backpressure: messages dropped (channel full)");
            drop_warned = true;
        }
    }

    finish_flush(&sink, &mut batch, &mut flushes).await;
    info!(flushes, "market import writer stopped");
}

/// Flush a non-empty batch to the sink, then reset it.
async fn finish_flush<S>(sink: &S, batch: &mut ImportBatch, flushes: &mut u64)
where
    S: MarketSink,
{
    if batch.is_empty() {
        return;
    }
    let rows = batch.len();
    match sink.flush_batch(batch).await {
        Ok(()) => {
            *flushes += 1;
            debug!(rows, "market import flushed");
        }
        Err(e) => warn!(error = %e, rows, "market import flush failed"),
    }
    batch.clear();
}

/// Snap an instant to the top of its minute (seconds/nanos cleared).
///
/// Keeps at most one `ticker_snapshots` row per instrument per minute, updated
/// in place by the upsert, so the high-frequency ticker stream cannot grow the
/// table without bound.
fn floor_minute(dt: DateTime<Utc>) -> DateTime<Utc> {
    let secs = dt.timestamp();
    DateTime::from_timestamp(secs - (secs % 60), 0).unwrap_or(dt)
}

/// Map a `@kline` WS message to a `market_data` row (domain symbol = instrument_id).
///
/// The interval from Binance is lower-case; normalize so storage, the REST pull,
/// and the read path all agree on one canonical case.
fn kline_to_record(k: &BinanceWsKline) -> NewMarketDataRecord {
    NewMarketDataRecord {
        instrument_id: k.symbol.clone(),
        timeframe: k.interval.to_lowercase(),
        timestamp: DateTime::from_timestamp_millis(k.open_time).unwrap_or_else(Utc::now),
        open: k.open,
        high: k.high,
        low: k.low,
        close: k.close,
        volume: k.volume,
    }
}

/// Map a `@ticker` WS message to a `ticker_snapshots` row (minute-snapped ts).
fn ticker_to_record(t: &BinanceWsTicker) -> NewTickerSnapshot {
    let ts = DateTime::from_timestamp_millis(t.event_time)
        .map(floor_minute)
        .unwrap_or_else(Utc::now);
    NewTickerSnapshot {
        instrument_id: t.symbol.clone(),
        ts,
        last_px: Some(t.last_price),
        open_24h: Some(t.open),
        high_24h: Some(t.high),
        low_24h: Some(t.low),
        vol_24h: Some(t.volume),
        vol_ccy_24h: Some(t.quote_volume),
        change_24h: Some(t.price_change),
    }
}

/// Map a `@trade` WS message to a `stream_trades` row (append-only).
fn trade_to_record(t: &BinanceWsTrade) -> NewStreamTrade {
    NewStreamTrade {
        symbol: t.symbol.clone(),
        price: t.price,
        quantity: t.quantity,
        trade_time: DateTime::from_timestamp_millis(t.trade_time).unwrap_or_else(Utc::now),
        is_buyer_maker: t.is_buyer_maker,
    }
}

/// Map a `@depth`/`@orderbook` WS message to an `orderbook_snapshots` row.
///
/// The book levels are serialized to JSON strings for the single-row-per-symbol
/// snapshot table.
fn depth_to_record(d: &BinanceWsDepth) -> NewOrderbookSnapshot {
    NewOrderbookSnapshot {
        symbol: d.symbol.clone(),
        bids: serde_json::to_string(&d.bids).unwrap_or_else(|_| "[]".to_string()),
        asks: serde_json::to_string(&d.asks).unwrap_or_else(|_| "[]".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::Result;
    use rust_decimal::Decimal;

    /// Mock backend recording every flushed batch for deterministic assertions.
    struct MockSink {
        batches: std::sync::Mutex<Vec<ImportBatch>>,
    }

    impl MockSink {
        fn new() -> Self {
            Self {
                batches: std::sync::Mutex::new(Vec::new()),
            }
        }
        fn snapshot(&self) -> Vec<usize> {
            self.batches
                .lock()
                .unwrap()
                .iter()
                .map(|b| b.len())
                .collect()
        }
    }

    #[async_trait::async_trait]
    impl MarketSink for MockSink {
        async fn flush_batch(&self, batch: &ImportBatch) -> Result<()> {
            self.batches.lock().unwrap().push(batch.clone());
            Ok(())
        }
    }

    fn ticker(symbol: &str, last: &str) -> BinanceWsTicker {
        BinanceWsTicker {
            symbol: symbol.to_string(),
            last_price: Decimal::from_str_exact(last).unwrap(),
            price_change: Decimal::ZERO,
            price_change_percent: Decimal::ZERO,
            high: Decimal::ZERO,
            low: Decimal::ZERO,
            open: Decimal::ZERO,
            volume: Decimal::ZERO,
            quote_volume: Decimal::ZERO,
            event_time: 0,
        }
    }

    #[test]
    fn floor_minute_clears_seconds_and_nanos() {
        let dt = DateTime::<Utc>::from_timestamp_millis(1_700_000_000_000 + 12_345).unwrap();
        let floored = floor_minute(dt);
        assert_eq!(floored.timestamp() % 60, 0);
    }

    #[test]
    fn kline_to_record_normalizes_interval_case() {
        let k = BinanceWsKline {
            symbol: "BTC-USDT".to_string(),
            interval: "1H".to_string(),
            open_time: 1_700_000_000_000,
            open: Decimal::ONE,
            high: Decimal::ONE,
            low: Decimal::ONE,
            close: Decimal::ONE,
            volume: Decimal::ONE,
            is_closed: false,
        };
        let rec = kline_to_record(&k);
        assert_eq!(rec.instrument_id, "BTC-USDT");
        assert_eq!(rec.timeframe, "1h");
    }

    #[tokio::test]
    async fn try_send_rejects_unsupported_messages_as_noop() {
        let importer = MarketImporter::new(MockSink::new());
        // Account/order/status/error messages are not import records; try_send
        // must treat them as accepted no-ops and never allocate a record slot.
        assert!(importer.try_send(&BinanceWsMessage::ConnectionStatus("ok".into())));
        assert!(importer.try_send(&BinanceWsMessage::Error("boom".into())));
        assert_eq!(importer.dropped(), 0);
    }

    #[tokio::test]
    async fn try_send_forwards_a_ticker_to_the_sink() {
        let sink = Arc::new(MockSink::new());
        let importer = MarketImporter::new(sink.clone());
        assert!(importer.try_send(&BinanceWsMessage::Ticker(ticker("BTC-USDT", "50000"))));
        // Give the writer task a moment to consume and flush on the 50ms timer.
        tokio::time::sleep(Duration::from_millis(120)).await;
        let counts = sink.snapshot();
        assert_eq!(counts, vec![1]);
    }
}
