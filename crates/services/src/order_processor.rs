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
use quant_common::types::{LogEntry, MarketData, Order, OrderSide, OrderType};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::instrument;
use risk_engine::PreTradeRiskChecker;
use trading_engine::{ExecutionEngine, OkxExecutor, OrderManager};

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

/// Order-submission orchestration use-case.
pub struct OrderProcessor {
    config: Arc<RwLock<AppConfig>>,
    okx_executor: Arc<RwLock<Option<Arc<OkxExecutor>>>>,
    order_manager: Arc<OrderManager>,
    log_buffer: Arc<LogBuffer>,
    market_service: Arc<MarketService>,
    risk_service: Arc<RiskService>,
    account_service: Arc<AccountService>,
}

impl OrderProcessor {
    pub fn new(
        config: Arc<RwLock<AppConfig>>,
        okx_executor: Arc<RwLock<Option<Arc<OkxExecutor>>>>,
        order_manager: Arc<OrderManager>,
        log_buffer: Arc<LogBuffer>,
        market_service: Arc<MarketService>,
        risk_service: Arc<RiskService>,
        account_service: Arc<AccountService>,
    ) -> Self {
        Self {
            config,
            okx_executor,
            order_manager,
            log_buffer,
            market_service,
            risk_service,
            account_service,
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

        // Prefer the persisted risk config (DB) when available.
        if let Ok(db_risk_config) = self.risk_service.get_risk_config().await {
            risk_config = db_risk_config;
        }

        // 1. Resolve market data with a conservative synthetic fallback so a
        //    data-source outage does not block order submission (limit orders).
        let market_data = self.resolve_market_data(&order).await;

        // 2. Pre-trade risk check (best-effort; requires account + positions).
        if enable_pre_trade {
            let checker = PreTradeRiskChecker::new(risk_config);
            if let (Ok(account), Ok(positions)) = (
                self.account_service.get_account_info().await,
                self.account_service.get_positions().await,
            ) {
                checker
                    .check_order_with_reference_price(
                        &order,
                        &account,
                        &positions,
                        Some(market_data.close),
                    )
                    .map_err(|e| ServiceError::Other(format!("Risk check failed: {}", e)))?;
            }
        }

        // 3. Build the execution engine (paper vs OKX chosen by config) and
        //    submit to the in-memory order manager.
        let okx_executor = self.okx_executor.read().await.clone();
        let execution_engine = ExecutionEngine::new(
            self.order_manager.clone(),
            trading_config,
            okx_executor,
        );

        let mut order = order;
        let order_id = self
            .order_manager
            .submit_order(order.clone())
            .await
            .map_err(|e| ServiceError::Other(e.to_string()))?;
        order.order_id = order_id;

        // 4. Log submission.
        self.log_order(&order).await;

        // 5. Persist to PostgreSQL (graceful degradation when DB unavailable).
        self.persist_order(&order).await;

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

        // 7. Dispatch execution on a background task.
        self.spawn_execution(execution_engine, order, market_data);

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
                message: format!("Order {} submitted for symbol {}", order.order_id, order.symbol),
                module: Some("trading".to_string()),
            })
            .await;
    }

    async fn persist_order(&self, order: &Order) {
        match self.account_service.get_account_info().await {
            Ok(account) => {
                if let Err(e) = self
                    .account_service
                    .persist_order(order, &account.account_id)
                    .await
                {
                    self.log_buffer
                        .add_entry(LogEntry {
                            timestamp: chrono::Utc::now(),
                            level: "warn".to_string(),
                            message: format!("Order persisted to DB failed: {}", e),
                            module: Some("commands".to_string()),
                        })
                        .await;
                }
            }
            Err(_) => {
                self.log_buffer
                    .add_entry(LogEntry {
                        timestamp: chrono::Utc::now(),
                        level: "warn".to_string(),
                        message: "Account not available for order persistence".to_string(),
                        module: Some("commands".to_string()),
                    })
                    .await;
            }
        }
    }

    /// Run execution asynchronously; failures are logged, not surfaced (best-effort).
    fn spawn_execution(&self, engine: ExecutionEngine, order: Order, market_data: MarketData) {
        let log = self.log_buffer.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Err(e) = engine.execute_order(order, &market_data).await {
                log.add_entry(LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: "error".to_string(),
                    message: format!("Order execution failed: {}", e),
                    module: Some("trading".to_string()),
                })
                .await;
            }
        });
    }
}
