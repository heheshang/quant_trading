//! 调度引擎错误类型。

/// 调度引擎错误
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("Strategy is already running: {0}")]
    AlreadyRunning(String),

    #[error("Strategy not found in scheduler: {0}")]
    NotFound(String),

    #[error("Scheduler error: {0}")]
    Other(String),
}
