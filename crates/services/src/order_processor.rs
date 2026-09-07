//! Order-placement use-case.
//!
//! Encapsulates the end-to-end order placement pipeline:
//!
//! ```text
//! resolve market data → pre-trade risk check → in-memory submission
//!     → persistence (required) → event descriptor → database-led execution
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
    /// Returns the assigned order id plus the UI event descriptor only after
    /// persistence succeeds. The scheduler executes persisted paper orders.
    #[instrument(skip(self, order), fields(symbol = %order.symbol, side = ?order.side))]
    pub async fn place_order(&self, order: Order) -> ServiceResult<OrderPlacement> {
        let trading_config = self.config.read().await.trading.clone();
        let mut risk_config = self.config.read().await.risk.clone();

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

        // Only an explicitly disconnected service may use the supplied offline config.
        match self.risk_service.get_risk_config().await {
            Ok(db_risk_config) => risk_config = db_risk_config,
            Err(ServiceError::DatabaseNotConnected) => {}
            Err(e) => return Err(e),
        }
        let enable_pre_trade = risk_config.enable_pre_trade_check;
        if !matches!(order.exchange.as_str(), "paper" | "algorithm") {
            return Err(ServiceError::InvalidParameter(
                "OrderProcessor only accepts paper/algorithm orders".to_string(),
            ));
        }
        // No database means there is no durable work for the scheduler to execute.
        let account = self.account_service.get_account_info().await?;

        // 1. Resolve market data。市价单在数据源中断时 fail-closed（无法定价风控）；
        //    限价单用限价作参考价。
        let market_data = match self.resolve_market_data(&order).await {
            Ok(data) => data,
            Err(()) => {
                if matches!(order.order_type, OrderType::Market) {
                    return Err(ServiceError::Other(
                        "市价单无法获取行情参考价，拒绝下单（fail-closed）".to_string(),
                    ));
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
        };

        // 2. Pre-trade risk check. When enabled, this is fail-closed: an
        //    unavailable account/positions rejects the order rather than
        //    silently letting it through without risk validation.
        if enable_pre_trade {
            let checker = PreTradeRiskChecker::new(risk_config);
            // 纸面账号：持仓从已成交单净额推导（positions 表是静态账本，纸面成交不更新）。
            let positions = self.account_service.get_paper_positions(account.account_id).await.map_err(|e| {
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

        // Reserve an in-memory id, then persist; failures must not leave an active ghost.
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

        // 5. Persist to PostgreSQL。活跃订单以数据库为准：落库失败则下单失败（fail-closed），
        //    避免订单只在内存、重启/活跃列表丢失。
        if let Err(e) = self
            .account_service
            .persist_order(&order, &account.account_id, &order.exchange)
            .await
        {
            if let Err(sync_error) = self
                .order_manager
                .update_order_status(order_id, OrderStatus::Rejected)
                .await
            {
                error!(order_id, %sync_error, "Failed to reject unpersisted order");
            }
            return Err(e);
        }
        self.log_order(&order).await;

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

    /// Realtime market data；若数据源中断返回 `Err(())`，由调用方决定降级/拒绝。
    async fn resolve_market_data(&self, order: &Order) -> Result<MarketData, ()> {
        if let Ok(data) = self.market_service.get_realtime_data(&order.symbol).await {
            return Ok(data);
        }
        // 数据中断：不在此处伪造价格。市价单调用方将 fail-closed；限价单用限价作参考。
        Err(())
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

    /// Commit cancellation before publishing it to the in-memory view.
    pub async fn cancel_order(&self, order_id: i64) -> ServiceResult<()> {
        self.account_service.cancel_order(order_id).await?;
        Self::sync_order_status(&self.order_manager, order_id, OrderStatus::Cancelled).await;
        Ok(())
    }

    async fn sync_order_status(order_manager: &OrderManager, order_id: i64, status: OrderStatus) {
        match order_manager.update_order_status(order_id, status).await {
            Ok(()) | Err(quant_common::Error::NotFound(_)) => {}
            Err(e) => error!(order_id, %e, "Database committed but memory status sync failed"),
        }
    }

    async fn execute_paper_order(
        account_service: &AccountService,
        order_manager: &OrderManager,
        engine: &ExecutionEngine,
        order: Order,
        market_data: &MarketData,
    ) -> ServiceResult<()> {
        let exchange = order.exchange.clone();
        let result = engine
            .calculate_paper_fill(order, market_data)
            .await
            .map_err(|e| ServiceError::Other(e.to_string()))?;
        account_service
            .update_order_status(
                result.order_id,
                &exchange,
                result.status.clone(),
                result.filled_quantity,
                Some(result.commission),
            )
            .await?;
        match order_manager.apply_execution_result(&result).await {
            Ok(()) | Err(quant_common::Error::NotFound(_)) => {}
            Err(e) => {
                error!(order_id = result.order_id, %e, "Database committed but memory fill sync failed")
            }
        }
        Ok(())
    }

    /// 启动纸面订单执行调度器（后台定时任务）。
    ///
    /// 周期性扫描 `orders` 表中已到期（`created_at + simulation_delay_ms <= now`）
    /// 且未成交的纸面单，纯计算后条件更新 DB，成功后才同步内存。
    /// 以 DB 为事实来源 → App 重启后可恢复处理，不再丢单。
    pub async fn start_paper_execution_scheduler(&self) {
        // 纸面调度器**永远**用纸面引擎（即使 enable_paper_trading=false）——它只处理
        // 纸面/算法单，绝不下真实 Binance 单。实盘单由 live_order_monitor/place_binance_order 处理。
        let mut trading = self.config.read().await.trading.clone();
        trading.enable_paper_trading = true;
        let delay_ms = trading.simulation_delay_ms;
        let timeout = std::time::Duration::from_secs(trading.order_timeout_seconds);
        let engine = ExecutionEngine::new_binance(
            self.order_manager.clone(),
            trading,
            self.binance_executor.read().await.clone(),
        );
        let account_service = self.account_service.clone();
        let market_service = self.market_service.clone();
        let log = self.log_buffer.clone();
        let order_manager = self.order_manager.clone();

        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(1);
            loop {
                tokio::time::sleep(interval).await;
                let Ok(orders) = account_service.get_active_orders(None).await else {
                    continue;
                };
                let now = chrono::Utc::now();
                for order in orders {
                    // 只处理纸面/算法单；实盘单（exchange='live'）走 Binance 实时路径，跳过。
                    if !matches!(order.exchange.as_str(), "paper" | "algorithm") {
                        continue;
                    }
                    // 订单超时未成交（如限价长时间未触及）→ 标记 Expired，避免永挂活跃。
                    let created_clock = order.created_at
                        + chrono::Duration::from_std(timeout)
                            .unwrap_or_else(|_| chrono::Duration::seconds(30));
                    if now > created_clock {
                        match account_service
                            .update_order_status(
                                order.order_id,
                                &order.exchange,
                                OrderStatus::Expired,
                                order.filled_quantity,
                                Some(order.commission),
                            )
                            .await
                        {
                            Ok(()) => {
                                Self::sync_order_status(
                                    &order_manager,
                                    order.order_id,
                                    OrderStatus::Expired,
                                )
                                .await
                            }
                            Err(e) => {
                                error!(order_id = order.order_id, %e, "Order expiry was not committed")
                            }
                        }
                        continue;
                    }
                    let due =
                        order.created_at + chrono::Duration::milliseconds(delay_ms as i64) <= now;
                    if !due {
                        continue;
                    }
                    let market_data = match market_service.get_realtime_data(&order.symbol).await {
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
                    match Self::execute_paper_order(
                        &account_service,
                        &order_manager,
                        &engine,
                        order,
                        &market_data,
                    )
                    .await
                    {
                        Ok(()) => {}
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
    use data_layer::market_data::DataSource;
    use quant_common::types::{MarketData, OrderStatus};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    /// 固定价行情源，供算法切片等测试注入（无真实数据源也能定价）。
    struct FakeDataSource;
    #[async_trait::async_trait]
    impl DataSource for FakeDataSource {
        async fn get_realtime_data(&self, symbol: &str) -> quant_common::Result<MarketData> {
            Ok(MarketData {
                symbol: symbol.to_string(),
                timestamp: chrono::Utc::now(),
                open: dec!(100),
                high: dec!(110),
                low: dec!(90),
                close: dec!(100),
                volume: dec!(0),
                turnover: dec!(0),
                open_interest: None,
                bid_prices: vec![],
                bid_volumes: vec![],
                ask_prices: vec![],
                ask_volumes: vec![],
            })
        }
        async fn get_historical_data(
            &self,
            _symbol: &str,
            _start: chrono::DateTime<chrono::Utc>,
            _end: chrono::DateTime<chrono::Utc>,
        ) -> quant_common::Result<Vec<MarketData>> {
            Ok(vec![])
        }
        async fn get_klines_history(
            &self,
            _symbol: &str,
            _timeframe: &str,
            _limit: i64,
        ) -> quant_common::Result<Vec<MarketData>> {
            Ok(vec![])
        }
        async fn subscribe(&self, _symbols: Vec<String>) -> quant_common::Result<()> {
            Ok(())
        }
        async fn unsubscribe(&self, _symbols: Vec<String>) -> quant_common::Result<()> {
            Ok(())
        }
    }

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
    async fn test_submission_without_database_is_rejected() {
        let processor = make_order_processor();
        assert!(matches!(
            processor.place_order(sample_order()).await,
            Err(ServiceError::DatabaseNotConnected)
        ));
        assert!(processor.order_manager.get_active_orders().await.is_empty());
        processor.config.write().await.risk.enable_pre_trade_check = false;
        assert!(matches!(
            processor.place_order(sample_order()).await,
            Err(ServiceError::DatabaseNotConnected)
        ));
        assert!(processor.order_manager.get_active_orders().await.is_empty());
    }

    #[test]
    fn test_rate_limit_rejects_second_order_in_window() {
        let limiter = RateLimiter::new(1.0);
        assert!(limiter.try_acquire(1.0));
        assert!(!limiter.try_acquire(1.0));
    }

    // ── Algorithmic orders (TWAP / VWAP / Iceberg) ──────────────────────────

    /// Explicit offline configuration; placement must fail without durable storage.
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
            Arc::new(MarketService::new(
                Arc::new(RwLock::new(Some(
                    Arc::new(FakeDataSource) as Arc<dyn DataSource>
                ))),
                None,
            )),
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
    #[ignore = "requires AUDIT_DATABASE_URL; uses connection-local TEMP tables"]
    async fn test_place_algorithmic_order_twap_generates_equal_slices() {
        let (processor, _) = make_database_processor().await;
        let order_manager = processor.order_manager.clone();
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
    #[ignore = "requires AUDIT_DATABASE_URL; uses connection-local TEMP tables"]
    async fn test_place_algorithmic_order_iceberg_places_limit_slices() {
        let (processor, _) = make_database_processor().await;
        let order_manager = processor.order_manager.clone();
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
        assert!(matches!(result, Err(ServiceError::InvalidParameter(_))));
    }

    #[tokio::test]
    async fn test_place_algorithmic_order_invalid_slices_rejected() {
        let (processor, _) = make_paper_order_processor().await;
        let result = processor
            .place_algorithmic_order(big_order(), "TWAP", &twap_params(Some(60), Some(0)))
            .await;
        assert!(matches!(result, Err(ServiceError::InvalidParameter(_))));
    }

    #[tokio::test]
    async fn test_place_algorithmic_order_unknown_algorithm_rejected() {
        let (processor, _) = make_paper_order_processor().await;
        let result = processor
            .place_algorithmic_order(big_order(), "FOO", &twap_params(Some(60), Some(10)))
            .await;
        assert!(matches!(result, Err(ServiceError::InvalidParameter(_))));
    }

    #[tokio::test]
    #[ignore = "requires AUDIT_DATABASE_URL; uses connection-local TEMP tables"]
    async fn test_live_pipeline_routes_order_to_in_memory_order_manager() {
        use strategy_engine::signals::{Signal, SignalSource, SignalType};

        let (processor, _) = make_database_processor().await;
        let order_manager = processor.order_manager.clone();
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

    async fn make_database_processor() -> (OrderProcessor, Arc<data_layer::PostgresClient>) {
        let url = std::env::var("AUDIT_DATABASE_URL").expect("set AUDIT_DATABASE_URL explicitly");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        // All names shadow production tables on this one connection; no migrations or .env.
        sqlx::raw_sql(r#"
            CREATE TEMP TABLE accounts (
                account_id bigint PRIMARY KEY, total_assets numeric DEFAULT 100000,
                available_cash numeric DEFAULT 100000, frozen_cash numeric DEFAULT 0,
                market_value numeric DEFAULT 0, total_pnl numeric DEFAULT 0,
                daily_pnl numeric DEFAULT 0, margin numeric DEFAULT 0,
                margin_ratio numeric DEFAULT 0, updated_at timestamptz DEFAULT now()
            );
            INSERT INTO accounts (account_id) VALUES (1);
            CREATE TEMP TABLE orders (
                order_id bigint PRIMARY KEY, account_id bigint, strategy_id text,
                symbol text, order_type text, side text, price numeric, quantity numeric,
                filled_quantity numeric, commission numeric, slippage numeric, status text,
                exchange text, created_at timestamptz, updated_at timestamptz
            );
            CREATE TEMP TABLE positions (
                account_id bigint, symbol text, quantity numeric, available_quantity numeric,
                avg_price numeric, market_value numeric, unrealized_pnl numeric,
                realized_pnl numeric, updated_at timestamptz DEFAULT now()
            );
            CREATE TEMP TABLE risk_config (
                id integer PRIMARY KEY, max_position_size float8 DEFAULT 1,
                max_daily_loss float8 DEFAULT 1, max_drawdown float8 DEFAULT 1,
                max_concentration float8 DEFAULT 1, enable_pre_trade_check boolean DEFAULT false,
                enable_real_time_monitor boolean DEFAULT false, var_confidence_level float8 DEFAULT 0.95
            );
            INSERT INTO risk_config (id) VALUES (1);
        "#).execute(&pool).await.unwrap();
        let client = Arc::new(data_layer::PostgresClient::from_pool(pool));
        let (mut processor, _) = make_paper_order_processor().await;
        processor.account_service = Arc::new(AccountService::new(Some(client.clone())));
        processor.risk_service = Arc::new(RiskService::new(Some(client.clone())));
        (processor, client)
    }

    #[tokio::test]
    #[ignore = "requires AUDIT_DATABASE_URL; uses connection-local TEMP tables"]
    async fn test_database_cancellation_blocks_stale_fill_and_live_cancellation() {
        let (processor, client) = make_database_processor().await;
        let placement = processor.place_order(sample_order()).await.unwrap();
        let stale = processor
            .account_service
            .get_active_orders(None)
            .await
            .unwrap()
            .pop()
            .unwrap();
        processor.cancel_order(placement.order_id).await.unwrap();
        let engine = ExecutionEngine::new_binance(
            processor.order_manager.clone(),
            processor.config.read().await.trading.clone(),
            None,
        );
        let market = FakeDataSource
            .get_realtime_data(&stale.symbol)
            .await
            .unwrap();
        assert!(matches!(
            OrderProcessor::execute_paper_order(
                &processor.account_service,
                &processor.order_manager,
                &engine,
                stale,
                &market
            )
            .await,
            Err(ServiceError::Conflict(_))
        ));
        let memory = processor
            .order_manager
            .get_order(placement.order_id)
            .await
            .unwrap();
        assert_eq!(memory.status, OrderStatus::Cancelled);
        assert_eq!(memory.filled_quantity, Decimal::ZERO);
        let persisted: (String, Decimal) =
            sqlx::query_as("SELECT status, filled_quantity FROM orders WHERE order_id = $1")
                .bind(placement.order_id)
                .fetch_one(client.pool())
                .await
                .unwrap();
        assert_eq!(persisted, ("Cancelled".to_string(), Decimal::ZERO));
        assert!(matches!(
            processor.cancel_order(-1).await,
            Err(ServiceError::NotFound(_))
        ));
        for (id, exchange) in [(41, "live"), (42, "unknown")] {
            let mut order = sample_order();
            order.order_id = id;
            processor
                .account_service
                .persist_order(&order, &1, exchange)
                .await
                .unwrap();
            assert!(matches!(
                processor.cancel_order(id).await,
                Err(ServiceError::NotFound(_))
            ));
            let status: String =
                sqlx::query_scalar("SELECT status FROM orders WHERE order_id = $1")
                    .bind(id)
                    .fetch_one(client.pool())
                    .await
                    .unwrap();
            assert_eq!(status, "Pending");
        }
        let memory_only = processor
            .order_manager
            .submit_order(sample_order())
            .await
            .unwrap();
        assert!(matches!(
            processor.cancel_order(memory_only).await,
            Err(ServiceError::NotFound(_))
        ));
        assert_eq!(
            processor
                .order_manager
                .get_order(memory_only)
                .await
                .unwrap()
                .status,
            OrderStatus::Submitted
        );
        let durable = processor.place_order(sample_order()).await.unwrap();
        sqlx::query("ALTER TABLE orders ADD CONSTRAINT reject_cancel CHECK (status <> 'Cancelled') NOT VALID")
            .execute(client.pool()).await.unwrap();
        assert!(matches!(
            processor.cancel_order(durable.order_id).await,
            Err(ServiceError::Database(_))
        ));
        assert_eq!(
            processor
                .order_manager
                .get_order(durable.order_id)
                .await
                .unwrap()
                .status,
            OrderStatus::Submitted
        );
    }

    #[tokio::test]
    #[ignore = "requires AUDIT_DATABASE_URL; uses connection-local TEMP tables"]
    async fn test_database_restored_order_executes_without_memory_entry() {
        let (processor, client) = make_database_processor().await;
        let mut order = sample_order();
        order.order_id = 71;
        order.status = OrderStatus::Submitted;
        processor
            .account_service
            .persist_order(&order, &1, "paper")
            .await
            .unwrap();
        let engine = ExecutionEngine::new_binance(
            processor.order_manager.clone(),
            processor.config.read().await.trading.clone(),
            None,
        );
        let market = FakeDataSource
            .get_realtime_data(&order.symbol)
            .await
            .unwrap();
        OrderProcessor::execute_paper_order(
            &processor.account_service,
            &processor.order_manager,
            &engine,
            order,
            &market,
        )
        .await
        .unwrap();
        let persisted: (String, Decimal) =
            sqlx::query_as("SELECT status, filled_quantity FROM orders WHERE order_id = 71")
                .fetch_one(client.pool())
                .await
                .unwrap();
        assert_eq!(persisted, ("Filled".to_string(), dec!(1)));
        assert!(processor.order_manager.get_active_orders().await.is_empty());
        assert!(matches!(
            processor
                .account_service
                .update_order_status(
                    -1,
                    "paper",
                    OrderStatus::Filled,
                    dec!(1),
                    Some(Decimal::ZERO)
                )
                .await,
            Err(ServiceError::Conflict(_))
        ));
    }

    #[tokio::test]
    #[ignore = "requires AUDIT_DATABASE_URL; uses connection-local TEMP tables"]
    async fn test_database_paper_positions_include_cancelled_fills_and_isolate_accounts() {
        let (processor, client) = make_database_processor().await;
        for (id, account, exchange, status, quantity, side) in [
            (
                1,
                1,
                "paper",
                OrderStatus::Cancelled,
                dec!(3),
                OrderSide::Buy,
            ),
            (
                2,
                1,
                "algorithm",
                OrderStatus::PartiallyFilled,
                dec!(2),
                OrderSide::Buy,
            ),
            (
                3,
                1,
                "paper",
                OrderStatus::Cancelled,
                dec!(1),
                OrderSide::Sell,
            ),
            (4, 1, "live", OrderStatus::Filled, dec!(100), OrderSide::Buy),
            (
                5,
                2,
                "paper",
                OrderStatus::Filled,
                dec!(100),
                OrderSide::Buy,
            ),
            (
                6,
                1,
                "unknown",
                OrderStatus::Filled,
                dec!(100),
                OrderSide::Buy,
            ),
        ] {
            let mut order = sample_order();
            order.order_id = id;
            order.status = status;
            order.quantity = quantity;
            order.filled_quantity = quantity;
            order.side = side;
            processor
                .account_service
                .persist_order(&order, &account, exchange)
                .await
                .unwrap();
        }
        let positions = processor
            .account_service
            .get_paper_positions(1)
            .await
            .unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].quantity, dec!(4));
        assert_eq!(positions[0].available_quantity, dec!(4));
        sqlx::raw_sql(
            "INSERT INTO positions (account_id, symbol, quantity, available_quantity, avg_price,
            market_value, unrealized_pnl, realized_pnl) VALUES
            (1, 'BTC-USDT', 8, 2, 100, 800, 0, 0), (2, 'ETH-USDT', 99, 99, 100, 9900, 0, 0)",
        )
        .execute(client.pool())
        .await
        .unwrap();
        let positions = processor.account_service.get_positions().await.unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "BTC-USDT");
        assert_eq!(positions[0].available_quantity, dec!(2));
    }

    #[tokio::test]
    #[ignore = "requires AUDIT_DATABASE_URL; uses connection-local TEMP tables"]
    async fn test_database_risk_toggle_and_persistence_failure_fail_closed() {
        let (processor, client) = make_database_processor().await;
        sqlx::query("UPDATE risk_config SET enable_pre_trade_check = true, max_position_size = 0")
            .execute(client.pool())
            .await
            .unwrap();
        assert!(processor.place_order(sample_order()).await.is_err());
        assert!(processor.order_manager.get_active_orders().await.is_empty());
        sqlx::query("ALTER TABLE risk_config RENAME COLUMN max_position_size TO unavailable")
            .execute(client.pool())
            .await
            .unwrap();
        assert!(matches!(
            processor.place_order(sample_order()).await,
            Err(ServiceError::Database(_))
        ));
        assert!(processor.order_manager.get_active_orders().await.is_empty());
        sqlx::query("ALTER TABLE risk_config RENAME COLUMN unavailable TO max_position_size")
            .execute(client.pool())
            .await
            .unwrap();
        sqlx::query("DELETE FROM risk_config")
            .execute(client.pool())
            .await
            .unwrap();
        assert!(matches!(
            processor.place_order(sample_order()).await,
            Err(ServiceError::NotFound(_))
        ));
        sqlx::query("INSERT INTO risk_config (id) VALUES (1)")
            .execute(client.pool())
            .await
            .unwrap();
        sqlx::query("ALTER TABLE orders ADD CONSTRAINT reject_insert CHECK (quantity < 0)")
            .execute(client.pool())
            .await
            .unwrap();
        assert!(matches!(
            processor.place_order(sample_order()).await,
            Err(ServiceError::Database(_))
        ));
        assert!(processor.order_manager.get_active_orders().await.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires AUDIT_DATABASE_URL; uses connection-local TEMP tables"]
    async fn test_database_status_updates_preserve_terminal_and_cumulative_facts() {
        let (processor, client) = make_database_processor().await;
        let service = &processor.account_service;
        let mut order = sample_order();
        order.order_id = 81;
        order.quantity = dec!(10);
        service.persist_order(&order, &1, "live").await.unwrap();
        service
            .update_order_status(
                81,
                "live",
                OrderStatus::PartiallyFilled,
                dec!(2),
                Some(dec!(1)),
            )
            .await
            .unwrap();
        assert!(matches!(
            service
                .update_order_status(81, "live", OrderStatus::Submitted, dec!(1), None)
                .await,
            Err(ServiceError::Conflict(_))
        ));
        assert!(matches!(
            service
                .update_order_status(81, "paper", OrderStatus::Filled, dec!(10), None)
                .await,
            Err(ServiceError::Conflict(_))
        ));
        service
            .update_order_status(81, "live", OrderStatus::Cancelled, dec!(2), None)
            .await
            .unwrap();
        service
            .update_order_status(81, "live", OrderStatus::Cancelled, dec!(2), Some(dec!(1)))
            .await
            .unwrap();
        assert!(matches!(
            service
                .update_order_status(81, "live", OrderStatus::Filled, dec!(10), None)
                .await,
            Err(ServiceError::Conflict(_))
        ));
        assert!(matches!(
            service
                .update_order_status(81, "live", OrderStatus::Cancelled, dec!(3), None)
                .await,
            Err(ServiceError::Conflict(_))
        ));
        let facts: (String, Decimal, Decimal) = sqlx::query_as(
            "SELECT status, filled_quantity, commission FROM orders WHERE order_id=81",
        )
        .fetch_one(client.pool())
        .await
        .unwrap();
        assert_eq!(facts, ("Cancelled".to_string(), dec!(2), dec!(1)));
    }
}
