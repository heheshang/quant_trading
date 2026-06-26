//! Scheduler Task Handle — 调度任务句柄

/// 调度任务句柄，持有 Tokio JoinHandle 用于管理异步任务
pub struct SchedulerTaskHandle {
    /// 异步任务的 JoinHandle（None 表示任务已结束）
    pub join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SchedulerTaskHandle {
    /// 中断正在执行的任务
    pub fn abort(&mut self) {
        if let Some(jh) = self.join_handle.take() {
            jh.abort();
        }
    }
}
