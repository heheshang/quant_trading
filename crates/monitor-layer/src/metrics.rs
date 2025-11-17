use prometheus::{Counter, Gauge, Histogram, HistogramOpts, Registry};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    
    // 订单指标
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
    
    // 延迟指标
    pub static ref ORDER_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new("order_latency_seconds", "Order execution latency")
    ).unwrap();
    
    // 账户指标
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
}

pub struct MetricsCollector;

impl MetricsCollector {
    pub fn init() {
        REGISTRY.register(Box::new(ORDERS_TOTAL.clone())).unwrap();
        REGISTRY.register(Box::new(ORDERS_FILLED.clone())).unwrap();
        REGISTRY.register(Box::new(ORDERS_CANCELLED.clone())).unwrap();
        REGISTRY.register(Box::new(ORDER_LATENCY.clone())).unwrap();
        REGISTRY.register(Box::new(ACCOUNT_BALANCE.clone())).unwrap();
        REGISTRY.register(Box::new(POSITION_VALUE.clone())).unwrap();
        REGISTRY.register(Box::new(DAILY_PNL.clone())).unwrap();
    }
}
