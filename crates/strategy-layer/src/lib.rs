pub mod backtest;
pub mod indicators;
pub mod pipeline;
pub mod registry;
pub mod scheduler;
pub mod signals;
pub mod strategy;
pub mod traits;

pub use backtest::{run_backtest_multi, BacktestEngine, MultiStrategyResult};
pub use pipeline::{PipelineError, PipelineExecutor, PipelineStep};
pub use registry::{
    default_registry, MeanReversionFactory, StrategyFactory, StrategyRegistry,
    TrendFollowingFactory,
};
pub use scheduler::{SchedulerError, StrategyScheduler};
pub use signals::{Signal, SignalGenerator, SignalSource, SignalType};
pub use strategy::{Strategy, StrategyContext};
pub use traits::{OrderExecError, OrderExecutor, RiskCheckError, RiskChecker};
