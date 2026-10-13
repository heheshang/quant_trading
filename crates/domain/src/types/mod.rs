//! 领域类型
//!
//! 按业务域拆分为子模块

pub mod backtest;
pub mod market;
pub mod risk;
pub mod strategy;

// 重新导出公共类型
pub use backtest::{BacktestResult, BacktestTrade};
pub use market::{
    Account, Exchange, Instrument, InstrumentType, MarketData, Order, OrderSide, OrderStatus,
    OrderType, Position,
};
pub use risk::{Alert, AlertLevel, RiskMetrics};
pub use strategy::{
    StrategyParams, StrategyParamsBuilder, StrategyType, StrategyError,
    StrategyStatus, allowed_transitions, SchedulerTaskInfo, StatusTransition, StrategyGuard,
};
