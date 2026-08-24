//! Order-placement use-case.
//!
//! Encapsulates the end-to-end order placement pipeline:
//!
//! ```text
//! resolve market data → pre-trade risk check → in-memory submission
//!     → persistence (graceful) → event descriptor → async execution
//! ```
//!
//! This is the **single place** that orchestrates order submission, so the
//! Tauri command layer stays a thin adapter (SRP) and never reaches into the
//! domain / engine / infrastructure layers directly (layering + DIP).
//!
//! `OrderProcessor` is stateful only in that it *owns* the collaborators it
//! needs (constructed via dependency injection); it holds no mutable order
//! state and is safe to share across requests.

use crate::account_service::AccountService;
use crate::error::{ServiceError, ServiceResult};
use crate::market_service::MarketService;
use crate::risk_service::RiskService;
use monitor_engine::LogBuffer;
use quant_common::config::AppConfig;
use quant_common::types::{LogEntry, MarketData, Order, OrderSide, OrderStatus, OrderType};
use risk_engine::PreTradeRiskChecker;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{error, instrument};
use trading_engine::algorithms::{AlgorithmicOrderSlicer, TWAPParams, VWAPParams};
use trading_engine::{BinanceExecutor, ExecutionEngine, OrderManager};

/// Subscription payload emitted to the UI after a successful submission.
///
/// Kept as a typed struct so the command layer can serialize it verbatim
/// (the keys below mirror the historical `order:submitted` event, preserving
/// wire-format compatibility).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrderSubmittedEvent {
    pub order_id: i64,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub status: String,
    pub timestamp: String,
}

/// Outcome of [`OrderProcessor::place_order`].
#[derive(Debug, Clone)]
pub struct OrderPlacement {
    pub order_id: i64,
    pub event: OrderSubmittedEvent,
}

/// Parameters accepted by [`OrderProcessor::place_algorithmic_order`].
///
/// `duration_minutes` / `num_slices` drive TWAP; `volume_profile` drives VWAP;
/// `display_quantity` drives Iceberg. Only the fields relevant to the chosen
/// algorithm are required; the rest default to `None`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AlgorithmicOrderParams {
    #[serde(default)]
    pub duration_minutes: Option<i64>,
    #[serde(default)]
    pub num_slices: Option<usize>,
    #[serde(default)]
    pub display_quantity: Option<Decimal>,
    #[serde(default)]
    pub volume_profile: Option<Vec<(chrono::DateTime<chrono::Utc>, Decimal)>>,
}

/// Token bucket used to enforce `max_orders_per_second`.
///
/// Starts full and refills at `capacity` tokens per second. Calls that cannot
/// obtain a token are rejected (fail-closed) rather than silently dropped.
struct TokenBucket {
    tokens: f64,
    last: Instant,
    capacity: f64,
    refill_per_sec: f64,
}

impl TokenBucket {
    fn new(capacity: f64) -> Self {
        Self {
            tokens: capacity,
            last: Instant::now(),
            capacity,
            refill_per_sec: capacity,
        }
    }

    fn set_capacity(&mut self, capacity: f64) {
        if (self.capacity - capacity).abs() < f64::EPSILON {
            return;
        }
        self.capacity = capacity;
        self.refill_per_sec = capacity;
        self.tokens = self.tokens.min(capacity);
    }

    fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        self.last = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Thread-safe wrapper around [`TokenBucket`].
struct RateLimiter {
    bucket: Mutex<TokenBucket>,
}

impl RateLimiter {
    fn new(capacity: f64) -> Self {
        Self {
            bucket: Mutex::new(TokenBucket::new(capacity)),
        }
    }

    /// Attempt to consume one token at the given capacity.
    ///
    /// On a poisoned lock we fail closed (return `false` → reject) so a
    /// panicked limiter can never silently let orders through unthrottled.
    fn try_acquire(&self, capacity: f64) -> bool {
        match self.bucket.lock() {
            Ok(mut b) => {
                b.set_capacity(capacity);
                b.try_acquire()
            }
            Err(_) => false,
        }
    }
}

/// Order-submission orchestration use-case.
pub struct OrderProcessor {
    config: Arc<RwLock<AppConfig>>,
    binance_executor: Arc<RwLock<Option<Arc<BinanceExecutor>>>>,
    order_manager: Arc<OrderManager>,
    log_buffer: Arc<LogBuffer>,
    market_service: Arc<MarketService>,
    risk_service: Arc<RiskService>,
    account_service: Arc<AccountService>,
    rate_limiter: RateLimiter,
}

impl OrderProcessor {
    pub fn new(
        config: Arc<RwLock<AppConfig>>,
        binance_executor: Arc<RwLock<Option<Arc<BinanceExecutor>>>>,
        order_manager: Arc<OrderManager>,
        log_buffer: Arc<LogBuffer>,
        market_service: Arc<MarketService>,
        risk_service: Arc<RiskService>,
        account_service: Arc<AccountService>,
    ) -> Self {
        let max_rate = config
            .try_read()
            .map(|c| c.trading.max_orders_per_second.max(1) as f64)
            .unwrap_or(100.0);
        Self {
            config,
            binance_executor,
            order_manager,
            log_buffer,
            market_service,
            risk_service,
            account_service,
            rate_limiter: RateLimiter::new(max_rate),
        }
    }

    /// Submit an order through the full pipeline.
    ///
    /// Returns the assigned order id plus the UI event descriptor. Order
    /// execution is dispatched on a background task after the event is built,
    /// matching the previous behaviour.
    #[instrument(skip(self, order), fields(symbol = %order.symbol, side = ?order.side))]
    pub async fn place_order(&self, order: Order) -> ServiceResult<OrderPlacement> {
        let trading_config = self.config.read().await.trading.clone();
        let mut risk_config = self.config.read().await.risk.clone();
        let enable_pre_trade = risk_config.enable_pre_trade_check;

        // Enforce `max_orders_per_second` (fail-closed: reject when throttled,
        // so an over-limit order is never silently dropped).
        if !self
            .rate_limiter
            .try_acquire(trading_config.max_orders_per_second.max(1) as f64)
        {
            return Err(ServiceError::RateLimited(format!(
                "max_orders_per_second={}",
                trading_config.max_orders_per_second
            )));
        }

        // Prefer the persisted risk config (DB) when available.
        if let Ok(db_risk_config) = self.risk_service.get_risk_config().await {
            risk_config = db_risk_config;
        }

        // 1. Resolve market data with a conservative synthetic fallback so a
        //    data-source outage does not block order submission (limit orders).
        let market_data = self.resolve_market_data(&order).await;

        // 2. Pre-trade risk check. When enabled, this is fail-closed: an
        //    unavailable account/positions rejects the order rather than
        //    silently letting it through without risk validation.
        if enable_pre_trade {
            let checker = PreTradeRiskChecker::new(risk_config);
            let account = self.account_service.get_account_info().await.map_err(|e| {
                ServiceError::Other(format!(
                    "Pre-trade risk check failed: unable to fetch account info (fail-closed): {}",
                    e
                ))
            })?;
            // 纸面账号：持仓从已成交单净额推导（positions 表是静态账本，纸面成交不更新）。
            let positions = self.account_service.get_paper_positions().await.map_err(|e| {
                ServiceError::Other(format!(
                    "Pre-trade risk check failed: unable to fetch paper positions (fail-closed): {}",
                    e
                ))
            })?;
            checker
                .check_order_with_reference_price(
                    &order,
                    &account,
                    &positions,
                    Some(market_data.close),
                )
                .map_err(|e| ServiceError::Other(format!("Risk check failed: {}", e)))?;
        }

        // 3. 提交到内存 OrderManager（Paper 由后台调度器在到期后撮合；现场由
        //    execution engine 在 `place_order` 之外处理）。这里只记录为 Submitted。
        let mut order = order;
        let order_id = self
            .order_manager
            .submit_order(order.clone())
            .await
            .map_err(|e| ServiceError::Other(e.to_string()))?;
        order.order_id = order_id;
        // `submit_order(..)` 只修改内存副本；把提交后的状态同步回原订单，
        // 否则 `persist_order` 会落库成入参的 `Pending`（待提交）。
        order.status = OrderStatus::Submitted;

        // 4. Log submission.
        self.log_order(&order).await;

        // 5. Persist to PostgreSQL。活跃订单以数据库为准：落库失败则下单失败（fail-closed），
        //    避免订单只在内存、重启/活跃列表丢失。
        self.persist_order(&order).await?;

        // 6. Build the UI event descriptor.
        // Clone event fields so the order can still be moved into execution.
        let event = OrderSubmittedEvent {
            order_id,
            symbol: order.symbol.clone(),
            side: order.side.clone(),
            order_type: order.order_type.clone(),
            price: order.price,
            quantity: order.quantity,
            status: "Submitted".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        monitor_engine::MetricsCollector::inc_orders_total();
        Ok(OrderPlacement { order_id, event })
    }

    /// Best-effort realtime market data resolution with a synthetic fallback.
    async fn resolve_market_data(&self, order: &Order) -> MarketData {
        if let Ok(data) = self.market_service.get_realtime_data(&order.symbol).await {
            return data;
        }
        let fallback_price = order.price.unwrap_or(dec!(100));
        MarketData {
            symbol: order.symbol.clone(),
            timestamp: chrono::Utc::now(),
            open: fallback_price,
            high: fallback_price,
            low: fallback_price,
            close: fallback_price,
            volume: dec!(0),
            turnover: dec!(0),
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        }
    }

    async fn log_order(&self, order: &Order) {
        self.log_buffer
            .add_entry(LogEntry {
                timestamp: chrono::Utc::now(),
                level: "info".to_string(),
                message: format!(
                    "Order {} submitted for symbol {}",
                    order.order_id, order.symbol
                ),
                module: Some("trading".to_string()),
            })
            .await;
    }

    async fn persist_order(&self, order: &Order) -> ServiceResult<()> {
        match self.account_service.get_account_info().await {
            Ok(account) => {
                self.account_service
                    .persist_order(order, &account.account_id, &order.exchange)
                    .await
                    .map_err(|e| {
                        error!("Order persisted to DB failed: {}", e);
                        ServiceError::Other(format!("无法下单：订单写入数据库失败（{}）", e))
                    })?;
                Ok(())
            }
            Err(ServiceError::DatabaseNotConnected) => {
                // 无数据库（测试/离线降级）→ 尽力而为，不阻塞内存下单。
                Ok(())
            }
            Err(e) => Err(ServiceError::Other(format!(
                "无法下单：账户信息不可用（未落库）：{}",
                e
            ))),
        }
    }

    /// Run execution asynchronously; failures are logged, not surfaced (best-effort).
    /// 启动纸面订单执行调度器（后台定时任务）。
    ///
    /// 周期性扫描 `orders` 表中已到期（`created_at + simulation_delay_ms <= now`）
    /// 且未成交的纸面单，用 `ExecutionEngine::fill_order` 立即撮合并回写 DB。
    /// 以 DB 为事实来源 → App 重启后可恢复处理，不再丢单。
    pub async fn start_paper_execution_scheduler(&self) {
        let trading = self.config.read().await.trading.clone();
        let delay_ms = trading.simulation_delay_ms;
        let engine = ExecutionEngine::new_binance(
            self.order_manager.clone(),
            trading,
            self.binance_executor.read().await.clone(),
        );
        let account_service = self.account_service.clone();
        let market_service = self.market_service.clone();
        let log = self.log_buffer.clone();

        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(1);
            loop {
                tokio::time::sleep(interval).await;
                let Ok(orders) = account_service.get_active_orders().await else {
                    continue;
                };
                let now = chrono::Utc::now();
                for order in orders {
                    let due = order.created_at
                        + chrono::Duration::milliseconds(delay_ms as i64)
                        <= now;
                    if !due {
                        continue;
                    }
                    let market_data = match market_service.get_realtime_data(&order.symbol).await
                    {
                        Ok(md) => md,
                        Err(e) => {
                            log.add_entry(LogEntry {
                                timestamp: chrono::Utc::now(),
                                level: "warn".to_string(),
                                message: format!(
                                    "Scheduler skip order {} (market data): {}",
                                    order.order_id, e
                                ),
                                module: Some("trading".to_string()),
                            })
                            .await;
                            continue;
                        }
                    };
                    // 限价单：仅当市价触及限价才成交（避免活跃单到期即被撮合消失）。
                    if matches!(&order.order_type, OrderType::Limit) {
                        if let Some(limit) = order.price {
                            let crossed = match &order.side {
                                OrderSide::Buy => market_data.close <= limit,
                                OrderSide::Sell => market_data.close >= limit,
                            };
                            if !crossed {
                                continue;
                            }
                        }
                    }
                    match engine.fill_order(order, &market_data).await {
                        Ok(result) => {
                            let _ = account_service
                                .update_order_status(
                                    result.order_id,
                                    result.status,
                                    result.filled_quantity,
                                    result.commission,
                                )
                                .await;
                        }
                        Err(e) => {
                            log.add_entry(LogEntry {
                                timestamp: chrono::Utc::now(),
                                level: "error".to_string(),
                                message: format!("Order execution failed: {}", e),
                                module: Some("trading".to_string()),
                            })
                            .await;
                        }
                    }
                }
            }
        });
    }

    /// Submit an algorithmic (TWAP / VWAP / Iceberg) order by slicing it into
    /// plain Market / Limit sub-orders and placing each through the full
    /// [`place_order`] use-case (risk + paper/real execution + persistence).
    ///
    /// The slicers emit sub-orders typed `Market` (TWAP / VWAP) or `Limit`
    /// (Iceberg) — never `OrderType::TWAP/VWAP/Iceberg` — so live exchange
    /// execution accepts them as ordinary orders instead of rejecting the
    /// algorithmic type as unsupported.
    ///
    /// # Errors
    ///
    /// Returns an error when the algorithm is unknown, a required parameter is
    /// invalid (e.g. `duration_minutes <= 0`, `num_slices == 0`), the slicer
    /// rejects the inputs, or any intermediate `place_order` fails.
    #[instrument(skip(self, order, params), fields(algorithm, symbol = %order.symbol))]
    pub async fn place_algorithmic_order(
        &self,
        order: Order,
        algorithm: &str,
        params: &AlgorithmicOrderParams,
    ) -> ServiceResult<Vec<OrderPlacement>> {
        let symbol = order.symbol.clone();
        let side = order.side.clone();
        let total_quantity = order.quantity;
        let base_strategy_id = order.strategy_id.clone();
        let base_price = order.price;

        let slices = self.slice_algorithmic_order(
            algorithm,
            &symbol,
            &side,
            total_quantity,
            base_price,
            params,
        )?;

        let mut placements = Vec::with_capacity(slices.len());
        for mut slice in slices {
            // Propagate the caller's strategy id so all slices are attributed
            // to the same strategy (the slicer only assigns a labeled default).
            slice.strategy_id = base_strategy_id.clone();
            let placement = self.place_order(slice).await?;
            placements.push(placement);
        }
        Ok(placements)
    }

    /// Generate the sub-order list for an algorithmic order.
    fn slice_algorithmic_order(
        &self,
        algorithm: &str,
        symbol: &str,
        side: &OrderSide,
        total_quantity: Decimal,
        price: Option<Decimal>,
        params: &AlgorithmicOrderParams,
    ) -> ServiceResult<Vec<Order>> {
        let algo = algorithm.to_uppercase();
        let slices = match algo.as_str() {
            "TWAP" => {
                let duration_minutes =
                    params.duration_minutes.filter(|d| *d > 0).ok_or_else(|| {
                        ServiceError::InvalidParameter("duration_minutes must be > 0".to_string())
                    })?;
                let num_slices = params.num_slices.filter(|s| *s > 0).ok_or_else(|| {
                    ServiceError::InvalidParameter("num_slices must be > 0".to_string())
                })?;
                AlgorithmicOrderSlicer::twap(
                    symbol.to_string(),
                    side.clone(),
                    TWAPParams {
                        total_quantity,
                        duration_minutes,
                        num_slices,
                    },
                    chrono::Utc::now(),
                )
            }
            "VWAP" => {
                let duration_minutes =
                    params.duration_minutes.filter(|d| *d > 0).ok_or_else(|| {
                        ServiceError::InvalidParameter("duration_minutes must be > 0".to_string())
                    })?;
                let volume_profile = params.volume_profile.clone().unwrap_or_default();
                AlgorithmicOrderSlicer::vwap(
                    symbol.to_string(),
                    side.clone(),
                    VWAPParams {
                        total_quantity,
                        duration_minutes,
                    },
                    volume_profile,
                )
            }
            "ICE" | "ICEBERG" => {
                let display_quantity = params.display_quantity.ok_or_else(|| {
                    ServiceError::InvalidParameter(
                        "display_quantity is required for iceberg".to_string(),
                    )
                })?;
                AlgorithmicOrderSlicer::iceberg(
                    symbol.to_string(),
                    side.clone(),
                    total_quantity,
                    display_quantity,
                    price,
                )
            }
            other => {
                return Err(ServiceError::InvalidParameter(format!(
                    "Unknown algorithmic order type: {} (expected TWAP, VWAP, or Iceberg)",
                    other
                )));
            }
        }
        .map_err(|e| ServiceError::InvalidParameter(e.to_string()))?;

        Ok(slices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::types::OrderStatus;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn make_order_processor() -> OrderProcessor {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let order_manager = Arc::new(OrderManager::new());
        let log_buffer = Arc::new(LogBuffer::new(1000));
        let market_service = Arc::new(MarketService::new(Arc::new(RwLock::new(None)), None));
        let risk_service = Arc::new(RiskService::new(None));
        let account_service = Arc::new(AccountService::new(None));
        OrderProcessor::new(
            config,
            Arc::new(RwLock::new(None)),
            order_manager,
            log_buffer,
            market_service,
            risk_service,
            account_service,
        )
    }

    fn sample_order() -> Order {
        Order {
            order_id: 0,
            strategy_id: "test".to_string(),
            symbol: "BTC-USDT".to_string(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: Some(dec!(100)),
            quantity: dec!(1),
            filled_quantity: dec!(0),
            status: OrderStatus::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            commission: dec!(0),
            slippage: dec!(0),
            exchange: "paper".to_string(),
        }
    }

    #[tokio::test]
    async fn test_pre_trade_risk_fails_closed_when_account_unavailable() {
        let processor = make_order_processor();
        // Default config has enable_pre_trade_check = true and no DB, so
        // account/positions are unavailable → the order must be rejected.
        let result = processor.place_order(sample_order()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("fail-closed"), "unexpected error: {}", err);
    }

    #[tokio::test]
    async fn test_rate_limit_rejects_second_order_in_window() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        {
            let mut cfg = config.write().await;
            cfg.trading.max_orders_per_second = 1;
            cfg.risk.enable_pre_trade_check = false;
        }
        let processor = OrderProcessor::new(
            config,
            Arc::new(RwLock::new(None)),
            Arc::new(OrderManager::new()),
            Arc::new(LogBuffer::new(1000)),
            Arc::new(MarketService::new(Arc::new(RwLock::new(None)), None)),
            Arc::new(RiskService::new(None)),
            Arc::new(AccountService::new(None)),
        );

        // First order within the window is admitted.
        let first = processor.place_order(sample_order()).await;
        assert!(
            first.is_ok(),
            "first order should pass, got {:?}",
            first.err()
        );

        // Immediately placing a second one exceeds the 1/sec cap → rejected.
        let second = processor.place_order(sample_order()).await;
        assert!(second.is_err());
        assert!(matches!(second.unwrap_err(), ServiceError::RateLimited(_)));
    }

    // ── Algorithmic orders (TWAP / VWAP / Iceberg) ──────────────────────────

    /// A processor configured for paper trading without the pre-trade gate
    /// (so orders go through in a DB-less environment) plus the shared
    /// in-memory [`OrderManager`] we assert against.
    async fn make_paper_order_processor() -> (OrderProcessor, Arc<OrderManager>) {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        {
            let mut cfg = config.write().await;
            cfg.risk.enable_pre_trade_check = false;
            cfg.trading.enable_paper_trading = true;
            cfg.trading.simulation_delay_ms = 0;
            cfg.trading.max_orders_per_second = 1000;
        }
        let order_manager = Arc::new(OrderManager::new());
        let processor = OrderProcessor::new(
            config,
            Arc::new(RwLock::new(None)),
            order_manager.clone(),
            Arc::new(LogBuffer::new(1000)),
            Arc::new(MarketService::new(Arc::new(RwLock::new(None)), None)),
            Arc::new(RiskService::new(None)),
            Arc::new(AccountService::new(None)),
        );
        (processor, order_manager)
    }

    fn big_order() -> Order {
        Order {
            order_id: 0,
            strategy_id: "algo-strat".to_string(),
            symbol: "BTC-USDT".to_string(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: Some(dec!(100)),
            quantity: dec!(1000),
            filled_quantity: dec!(0),
            status: OrderStatus::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            commission: dec!(0),
            slippage: dec!(0),
            exchange: "paper".to_string(),
        }
    }

    fn twap_params(duration: Option<i64>, slices: Option<usize>) -> AlgorithmicOrderParams {
        AlgorithmicOrderParams {
            duration_minutes: duration,
            num_slices: slices,
            display_quantity: None,
            volume_profile: None,
        }
    }

    #[tokio::test]
    async fn test_place_algorithmic_order_twap_generates_equal_slices() {
        let (processor, order_manager) = make_paper_order_processor().await;
        let placements = processor
            .place_algorithmic_order(big_order(), "TWAP", &twap_params(Some(60), Some(10)))
            .await
            .expect("TWAP should place its slices");

        assert_eq!(placements.len(), 10, "TWAP(10 slices) must yield 10 orders");
        for placement in &placements {
            let placed = order_manager
                .get_order(placement.order_id)
                .await
                .expect("each slice should be in the in-memory order manager");
            assert_eq!(placed.quantity, dec!(100), "each TWAP slice must be equal");
            assert_eq!(placed.strategy_id, "algo-strat");
            assert_eq!(placed.order_type, OrderType::Market);
            assert_eq!(placed.side, OrderSide::Buy);
        }
    }

    #[tokio::test]
    async fn test_place_algorithmic_order_iceberg_places_limit_slices() {
        let (processor, order_manager) = make_paper_order_processor().await;
        let params = AlgorithmicOrderParams {
            duration_minutes: None,
            num_slices: None,
            display_quantity: Some(dec!(100)),
            volume_profile: None,
        };
        let placements = processor
            .place_algorithmic_order(big_order(), "Iceberg", &params)
            .await
            .expect("Iceberg should place its slices");

        assert_eq!(placements.len(), 10, "1000/100 display = 10 iceberg slices");
        for placement in &placements {
            let placed = order_manager
                .get_order(placement.order_id)
                .await
                .expect("each iceberg slice should be in the order manager");
            assert_eq!(placed.quantity, dec!(100));
            assert_eq!(placed.order_type, OrderType::Limit);
            assert_eq!(placed.price, Some(dec!(100)));
        }
    }

    #[tokio::test]
    async fn test_place_algorithmic_order_invalid_duration_rejected() {
        let (processor, _) = make_paper_order_processor().await;
        let result = processor
            .place_algorithmic_order(big_order(), "TWAP", &twap_params(Some(0), Some(10)))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("duration_minutes"),
            "unexpected error: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_place_algorithmic_order_invalid_slices_rejected() {
        let (processor, _) = make_paper_order_processor().await;
        let result = processor
            .place_algorithmic_order(big_order(), "TWAP", &twap_params(Some(60), Some(0)))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("num_slices"), "unexpected error: {}", err);
    }

    #[tokio::test]
    async fn test_place_algorithmic_order_unknown_algorithm_rejected() {
        let (processor, _) = make_paper_order_processor().await;
        let result = processor
            .place_algorithmic_order(big_order(), "FOO", &twap_params(Some(60), Some(10)))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Unknown algorithmic"),
            "unexpected error: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_live_pipeline_routes_order_to_in_memory_order_manager() {
        use strategy_engine::signals::{Signal, SignalSource, SignalType};

        let (processor, order_manager) = make_paper_order_processor().await;
        // A minimal "real" pipeline wired to OrderProcessor (passthrough risk so
        // the authoritative pre-trade check in `place_order` is not duplicated).
        let pipeline = crate::pipeline::make_live_pipeline(Arc::new(processor));

        let signal = Signal {
            signal_type: SignalType::Buy,
            symbol: "BTC-USDT".to_string(),
            strength: 1.0,
            price: Some(dec!(100)),
            quantity: Some(dec!(2)),
            id: "sig-1".to_string(),
            strategy_id: "strat-1".to_string(),
            source: SignalSource::Strategy,
            generated_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };
        let order = signal
            .to_order("strat-1")
            .expect("buy signal converts to order");

        let ctx = pipeline
            .execute(order)
            .await
            .expect("pipeline should approve and execute");
        assert!(ctx.risk_approved);
        assert_eq!(
            ctx.execution_status,
            strategy_engine::pipeline::ExecutionStatus::Confirmed,
            "the execution step must confirm successful placement"
        );

        // The order was submitted to the in-memory OrderManager by place_order.
        let active = order_manager.get_active_orders().await;
        assert_eq!(active.len(), 1, "exactly one order should be placed");
        assert_eq!(active[0].symbol, "BTC-USDT");
        assert_eq!(active[0].quantity, dec!(2));
        assert_eq!(active[0].strategy_id, "strat-1");
    }
}
