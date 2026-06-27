//! Scheduler Task Handle — 调度任务句柄

use std::sync::{atomic::AtomicU32, Arc, Mutex};

use chrono::{DateTime, Utc};

/// Runtime metadata for a scheduled task, updated by the task loop.
#[derive(Debug, Clone)]
pub struct SchedulerTaskMeta {
    /// Strategy display name.
    pub strategy_name: String,
    /// Polling interval in seconds.
    pub interval_secs: u64,
    /// Timestamp of the most recent successful signal generation.
    pub last_run_at: Option<DateTime<Utc>>,
    /// Consecutive error count (reset on success).
    pub error_count: u32,
}

/// 调度任务句柄，持有 Tokio JoinHandle 和运行时元数据。
pub struct SchedulerTaskHandle {
    /// 异步任务的 JoinHandle（None 表示任务已结束）
    pub join_handle: Option<tokio::task::JoinHandle<()>>,
    /// 运行时元数据（由任务循环更新）
    pub meta: Arc<Mutex<SchedulerTaskMeta>>,
    /// 错误计数原子值（用于任务循环中的无锁更新）
    pub error_counter: Arc<AtomicU32>,
}

impl SchedulerTaskHandle {
    /// Create a new handle with the given metadata and join handle.
    #[must_use]
    pub fn new(
        strategy_name: String,
        interval_secs: u64,
        join_handle: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            join_handle: Some(join_handle),
            meta: Arc::new(Mutex::new(SchedulerTaskMeta {
                strategy_name,
                interval_secs,
                last_run_at: None,
                error_count: 0,
            })),
            error_counter: Arc::new(AtomicU32::new(0)),
        }
    }

    /// 中断正在执行的任务
    pub fn abort(&mut self) {
        if let Some(jh) = self.join_handle.take() {
            jh.abort();
        }
    }
}
