use prometheus::{Counter, Gauge, Histogram, HistogramOpts, Registry, TextEncoder, Encoder};
use lazy_static::lazy_static;
use tracing::{info, error};
use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    
    // Order metrics
    pub static ref ORDERS_TOTAL: Counter = Counter::new(
        "orders_total",
        "Total number of orders"
    ).unwrap();
    
    pub static ref ORDERS_FILLED: Counter = Counter::new(
        "orders_filled",
        "Number of filled orders"
    ).unwrap();
    
    pub static ref ORDERS_CANCELLED: Counter = Counter::new(
        "orders_cancelled",
        "Number of cancelled orders"
    ).unwrap();
    
    pub static ref ORDERS_REJECTED: Counter = Counter::new(
        "orders_rejected",
        "Number of rejected orders"
    ).unwrap();
    
    // Latency metrics
    pub static ref ORDER_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new("order_latency_seconds", "Order execution latency")
    ).unwrap();
    
    pub static ref STRATEGY_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new("strategy_latency_seconds", "Strategy calculation latency")
    ).unwrap();
    
    // Account metrics
    pub static ref ACCOUNT_BALANCE: Gauge = Gauge::new(
        "account_balance",
        "Current account balance"
    ).unwrap();
    
    pub static ref POSITION_VALUE: Gauge = Gauge::new(
        "position_value",
        "Total position value"
    ).unwrap();
    
    pub static ref DAILY_PNL: Gauge = Gauge::new(
        "daily_pnl",
        "Daily profit and loss"
    ).unwrap();
    
    pub static ref TOTAL_PNL: Gauge = Gauge::new(
        "total_pnl",
        "Total profit and loss"
    ).unwrap();
    
    // Risk metrics
    pub static ref MARGIN_RATIO: Gauge = Gauge::new(
        "margin_ratio",
        "Current margin ratio"
    ).unwrap();
    
    pub static ref DRAWDOWN: Gauge = Gauge::new(
        "drawdown",
        "Current drawdown percentage"
    ).unwrap();
    
    pub static ref VaR_95: Gauge = Gauge::new(
        "var_95",
        "Value at Risk at 95% confidence"
    ).unwrap();
    
    pub static ref VaR_99: Gauge = Gauge::new(
        "var_99",
        "Value at Risk at 99% confidence"
    ).unwrap();
    
    // Strategy metrics
    pub static ref ACTIVE_STRATEGIES: Gauge = Gauge::new(
        "active_strategies",
        "Number of active strategies"
    ).unwrap();
    
    pub static ref STRATEGY_SHARPE_RATIO: Gauge = Gauge::new(
        "strategy_sharpe_ratio",
        "Sharpe ratio of strategies"
    ).unwrap();
    
    // System metrics
    pub static ref SYSTEM_CPU_USAGE: Gauge = Gauge::new(
        "system_cpu_usage_percent",
        "System CPU usage percentage"
    ).unwrap();
    
    pub static ref SYSTEM_MEMORY_USAGE: Gauge = Gauge::new(
        "system_memory_usage_bytes",
        "System memory usage in bytes"
    ).unwrap();
    
    // OKX specific metrics
    pub static ref OKX_API_CALLS: Counter = Counter::new(
        "okx_api_calls_total",
        "Total number of OKX API calls"
    ).unwrap();
    
    pub static ref OKX_API_ERRORS: Counter = Counter::new(
        "okx_api_errors_total",
        "Total number of OKX API errors"
    ).unwrap();
    
    pub static ref OKX_ORDERS_PLACED: Counter = Counter::new(
        "okx_orders_placed_total",
        "Total number of orders placed on OKX"
    ).unwrap();
    
    pub static ref OKX_ORDERS_CANCELLED: Counter = Counter::new(
        "okx_orders_cancelled_total",
        "Total number of orders cancelled on OKX"
    ).unwrap();
    
    pub static ref OKX_BALANCE_USDT: Gauge = Gauge::new(
        "okx_balance_usdt",
        "OKX USDT balance"
    ).unwrap();
    
    pub static ref OKX_API_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new("okx_api_latency_seconds", "OKX API call latency")
    ).unwrap();
}

pub struct MetricsCollector {
    snapshot_history: Arc<RwLock<Vec<MetricsSnapshot>>>,
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub orders_total: f64,
    pub orders_filled: f64,
    pub account_balance: f64,
    pub daily_pnl: f64,
    pub margin_ratio: f64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            snapshot_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn init() {
        // Register all metrics - silently ignore if already registered
        let _ = REGISTRY.register(Box::new(ORDERS_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(ORDERS_FILLED.clone()));
        let _ = REGISTRY.register(Box::new(ORDERS_CANCELLED.clone()));
        let _ = REGISTRY.register(Box::new(ORDERS_REJECTED.clone()));
        let _ = REGISTRY.register(Box::new(ORDER_LATENCY.clone()));
        let _ = REGISTRY.register(Box::new(STRATEGY_LATENCY.clone()));
        let _ = REGISTRY.register(Box::new(ACCOUNT_BALANCE.clone()));
        let _ = REGISTRY.register(Box::new(POSITION_VALUE.clone()));
        let _ = REGISTRY.register(Box::new(DAILY_PNL.clone()));
        let _ = REGISTRY.register(Box::new(TOTAL_PNL.clone()));
        let _ = REGISTRY.register(Box::new(MARGIN_RATIO.clone()));
        let _ = REGISTRY.register(Box::new(DRAWDOWN.clone()));
        let _ = REGISTRY.register(Box::new(VaR_95.clone()));
        let _ = REGISTRY.register(Box::new(VaR_99.clone()));
        let _ = REGISTRY.register(Box::new(ACTIVE_STRATEGIES.clone()));
        let _ = REGISTRY.register(Box::new(STRATEGY_SHARPE_RATIO.clone()));
        let _ = REGISTRY.register(Box::new(SYSTEM_CPU_USAGE.clone()));
        let _ = REGISTRY.register(Box::new(SYSTEM_MEMORY_USAGE.clone()));
        
        // Register OKX metrics
        let _ = REGISTRY.register(Box::new(OKX_API_CALLS.clone()));
        let _ = REGISTRY.register(Box::new(OKX_API_ERRORS.clone()));
        let _ = REGISTRY.register(Box::new(OKX_ORDERS_PLACED.clone()));
        let _ = REGISTRY.register(Box::new(OKX_ORDERS_CANCELLED.clone()));
        let _ = REGISTRY.register(Box::new(OKX_BALANCE_USDT.clone()));
        let _ = REGISTRY.register(Box::new(OKX_API_LATENCY.clone()));
        
        info!("Metrics collector initialized");
    }
    
    /// Increment total orders counter
    pub fn inc_orders_total() {
        ORDERS_TOTAL.inc();
    }
    
    /// Increment filled orders counter
    pub fn inc_orders_filled() {
        ORDERS_FILLED.inc();
    }
    
    /// Increment cancelled orders counter
    pub fn inc_orders_cancelled() {
        ORDERS_CANCELLED.inc();
    }
    
    /// Increment rejected orders counter
    pub fn inc_orders_rejected() {
        ORDERS_REJECTED.inc();
    }
    
    /// Record order latency
    pub fn record_order_latency(duration: f64) {
        ORDER_LATENCY.observe(duration);
    }
    
    /// Record strategy latency
    pub fn record_strategy_latency(duration: f64) {
        STRATEGY_LATENCY.observe(duration);
    }
    
    /// Set account balance
    pub fn set_account_balance(balance: f64) {
        ACCOUNT_BALANCE.set(balance);
    }
    
    /// Set position value
    pub fn set_position_value(value: f64) {
        POSITION_VALUE.set(value);
    }
    
    /// Set daily PnL
    pub fn set_daily_pnl(pnl: f64) {
        DAILY_PNL.set(pnl);
    }
    
    /// Set total PnL
    pub fn set_total_pnl(pnl: f64) {
        TOTAL_PNL.set(pnl);
    }
    
    /// Set margin ratio
    pub fn set_margin_ratio(ratio: f64) {
        MARGIN_RATIO.set(ratio);
    }
    
    /// Set drawdown
    pub fn set_drawdown(drawdown: f64) {
        DRAWDOWN.set(drawdown);
    }
    
    /// Set VaR at 95% confidence
    pub fn set_var_95(var: f64) {
        VaR_95.set(var);
    }
    
    /// Set VaR at 99% confidence
    pub fn set_var_99(var: f64) {
        VaR_99.set(var);
    }
    
    /// Set active strategies count
    pub fn set_active_strategies(count: f64) {
        ACTIVE_STRATEGIES.set(count);
    }
    
    /// Set strategy Sharpe ratio
    pub fn set_strategy_sharpe_ratio(sharpe: f64) {
        STRATEGY_SHARPE_RATIO.set(sharpe);
    }
    
    /// Set system CPU usage
    pub fn set_system_cpu_usage(usage: f64) {
        SYSTEM_CPU_USAGE.set(usage);
    }
    
    /// Set system memory usage
    pub fn set_system_memory_usage(usage: f64) {
        SYSTEM_MEMORY_USAGE.set(usage);
    }
    
    /// Take a snapshot of current metrics
    pub async fn take_snapshot(&self) -> MetricsSnapshot {
        let snapshot = MetricsSnapshot {
            timestamp: chrono::Utc::now(),
            orders_total: ORDERS_TOTAL.get(),
            orders_filled: ORDERS_FILLED.get(),
            account_balance: ACCOUNT_BALANCE.get(),
            daily_pnl: DAILY_PNL.get(),
            margin_ratio: MARGIN_RATIO.get(),
        };
        
        let mut history = self.snapshot_history.write().await;
        history.push(snapshot.clone());
        
        // Keep only the last 1000 snapshots
        let len = history.len();
        if len > 1000 {
            history.drain(0..len - 1000);
        }
        
        snapshot
    }
    
    /// Get historical snapshots
    pub async fn get_snapshot_history(&self) -> Vec<MetricsSnapshot> {
        self.snapshot_history.read().await.clone()
    }
    
    /// Get metrics in Prometheus text format
    pub fn get_metrics_text() -> String {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        let metric_families = REGISTRY.gather();
        match encoder.encode(&metric_families, &mut buffer) {
            Ok(_) => String::from_utf8(buffer).unwrap_or_else(|e| {
                error!("Failed to convert metrics to UTF8: {}", e);
                String::new()
            }),
            Err(e) => {
                error!("Failed to encode metrics: {}", e);
                String::new()
            }
        }
    }
    
    /// Reset all metrics (useful for testing)
    pub fn reset_metrics() {
        ORDERS_TOTAL.reset();
        ORDERS_FILLED.reset();
        ORDERS_CANCELLED.reset();
        ORDERS_REJECTED.reset();
        ACCOUNT_BALANCE.set(0.0);
        POSITION_VALUE.set(0.0);
        DAILY_PNL.set(0.0);
        TOTAL_PNL.set(0.0);
        MARGIN_RATIO.set(0.0);
        DRAWDOWN.set(0.0);
        VaR_95.set(0.0);
        VaR_99.set(0.0);
        ACTIVE_STRATEGIES.set(0.0);
        STRATEGY_SHARPE_RATIO.set(0.0);
        SYSTEM_CPU_USAGE.set(0.0);
        SYSTEM_MEMORY_USAGE.set(0.0);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}