pub mod backtest;
pub mod indicators;
pub mod pipeline;
pub mod registry;
pub mod scheduler;
pub mod signals;
pub mod strategy;

pub use backtest::{BacktestEngine, MultiStrategyResult, run_backtest_multi};
pub use pipeline::{PipelineError, PipelineExecutor, PipelineStep};
pub use registry::{default_registry, MeanReversionFactory, StrategyFactory, StrategyRegistry};
pub use scheduler::{SchedulerError, StrategyScheduler};
pub use signals::{Signal, SignalGenerator, SignalSource, SignalType};
pub use strategy::{Strategy, StrategyContext};
