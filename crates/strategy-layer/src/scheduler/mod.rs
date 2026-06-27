//! Strategy Scheduler — 策略调度执行引擎
//!
//! 为每个运行中的策略创建独立的 Tokio 异步任务，按配置间隔定时执行
//! 策略的信号生成逻辑。支持并发控制、熔断保护和优雅关闭。

mod circuit_breaker;
mod task;

pub use circuit_breaker::CircuitBreaker;
pub use task::SchedulerTaskHandle;

use quant_common::config::SchedulerConfig;
use quant_common::types::SchedulerTaskInfo;
use quant_common::MarketDataProvider;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, instrument, warn};

use crate::pipeline::PipelineExecutor;
use crate::strategy::Strategy;

/// 策略调度引擎
pub struct StrategyScheduler {
    /// 策略ID → 任务句柄
    tasks: Arc<RwLock<HashMap<String, SchedulerTaskHandle>>>,
    /// 全局关闭信号发送端
    shutdown_tx: broadcast::Sender<()>,
    /// 熔断器集合
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    /// 调度配置
    config: SchedulerConfig,
    /// 信号流水线执行器
    pipeline: Option<Arc<PipelineExecutor>>,
    /// 市场数据提供者
    market_data_provider: Option<Arc<dyn MarketDataProvider>>, 
}

impl StrategyScheduler {
    /// 创建新的调度引擎
    #[must_use]
    pub fn new(config: SchedulerConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            config,
            pipeline: None,
            market_data_provider: None,
        }
    }

    /// 设置信号流水线执行器
    pub fn set_pipeline(&mut self, pipeline: Arc<PipelineExecutor>) {
        self.pipeline = Some(pipeline);
    }

    /// 设置市场数据提供者
    pub fn set_market_data_provider(&mut self, provider: Arc<dyn MarketDataProvider>) {
        self.market_data_provider = Some(provider);
    }

    /// 获取调度配置
    #[must_use]
    pub fn config(&self) -> &SchedulerConfig {
        &self.config
    }

    /// 启动一个策略的定时执行
    ///
    /// 为指定策略创建一个独立的 Tokio 任务，按 `interval_secs` 频率
    /// 执行 `generate_signals` 并通过流水线处理订单。
    ///
    /// # Errors
    ///
    /// 如果策略已运行则返回 `SchedulerAlreadyRunning`。
    pub async fn start_strategy(
        &self,
        strategy_id: String,
        strategy_name: String,
        strategy: Box<dyn Strategy>,
        interval_secs: u64,
    ) -> Result<(), SchedulerError> {
        let sid = strategy_id;

        // 检查是否已在运行
        {
            let tasks = self.tasks.read().await;
            if tasks.contains_key(&sid) {
                return Err(SchedulerError::AlreadyRunning(sid));
            }
        }

        let shutdown_rx = self.shutdown_tx.subscribe();
        let tasks = Arc::clone(&self.tasks);
        let cbs = Arc::clone(&self.circuit_breakers);
        let pipeline = self.pipeline.clone();
        let market_data_provider = self.market_data_provider.clone();

        // Shared metadata updated by the task loop.
        let meta = Arc::new(std::sync::Mutex::new(task::SchedulerTaskMeta {
            strategy_name: strategy_name.clone(),
            interval_secs,
            last_run_at: None,
            error_count: 0,
        }));
        let error_counter = Arc::new(std::sync::atomic::AtomicU32::new(0));

        // 初始化熔断器
        {
            let mut cbs = self.circuit_breakers.write().await;
            cbs.insert(
                sid.clone(),
                CircuitBreaker::new(
                    self.config.circuit_breaker_threshold,
                    self.config.circuit_breaker_window_secs,
                ),
            );
        }

        let task_sid = sid.clone();
        let interval = tokio::time::Duration::from_secs(interval_secs);
        let task_meta = Arc::clone(&meta);
        let task_error_counter = Arc::clone(&error_counter);

        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            info!(strategy_id = %task_sid, interval_secs, "Scheduler task started");

            let mut shutdown_rx = shutdown_rx;
            let mut interval_timer = tokio::time::interval(interval);
            // 跳过第一个 tick，避免创建时立即执行
            interval_timer.tick().await;

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!(strategy_id = %task_sid, "Scheduler task received shutdown signal");
                        break;
                    }
                    _ = interval_timer.tick() => {
                        // 检查熔断器
                        let is_tripped = {
                            let cbs = cbs.read().await;
                            cbs.get(&task_sid)
                                .is_some_and(|cb| cb.is_tripped())
                        };

                        if is_tripped {
                            warn!(strategy_id = %task_sid, "Circuit breaker tripped, skipping execution");
                            continue;
                        }

                        // 执行信号生成
                        info!(strategy_id = %task_sid, "Executing scheduled signal generation");

                        // 构造上下文，获取真实市场数据
                        let mut market_data = Vec::new();
                        if let Some(ref provider) = market_data_provider {
                            // 获取当前策略的符号（从策略参数中获取）
                            // 注意：这里需要从策略中获取符号信息，暂时使用空实现
                            // TODO: 从策略中获取 symbols
                            let symbols: Vec<String> = vec![];
                            for symbol in symbols {
                                match provider.get_historical_data(
                                    &symbol,
                                    chrono::Utc::now() - chrono::Duration::hours(24),
                                    chrono::Utc::now(),
                                ).await {
                                    Ok(data) => market_data.extend(data),
                                    Err(e) => {
                                        warn!(strategy_id = %task_sid, symbol = %symbol, error = %e, "Failed to fetch market data");
                                    }
                                }
                            }
                        }

                        let context = crate::strategy::StrategyContext {
                            current_time: chrono::Utc::now(),
                            positions: Vec::new(),
                            market_data,
                        };

                        match strategy.generate_signals(&context).await {
                            Ok(signals) => {
                                info!(
                                    strategy_id = %task_sid,
                                    signal_count = signals.len(),
                                    "Signals generated"
                                );

                                // 通过流水线处理订单
                                if let Some(ref pipeline) = pipeline {
                                    for order in signals {
                                        if let Err(e) = pipeline.execute(order).await {
                                            error!(
                                                strategy_id = %task_sid,
                                                error = %e,
                                                "Pipeline step failed"
                                            );
                                        }
                                    }
                                }

                                // 重置熔断器（成功执行）
                                {
                                    let mut cbs = cbs.write().await;
                                    if let Some(cb) = cbs.get_mut(&task_sid) {
                                        cb.reset();
                                    }
                                }

                                // Update shared metadata on success.
                                {
                                    let mut m = task_meta.lock().unwrap();
                                    m.last_run_at = Some(chrono::Utc::now());
                                    m.error_count = 0;
                                }
                                task_error_counter.store(0, Ordering::Release);
                            }
                            Err(e) => {
                                error!(
                                    strategy_id = %task_sid,
                                    error = %e,
                                    "Signal generation failed"
                                );

                                // 记录错误到熔断器
                                let should_pause = {
                                    let mut cbs = cbs.write().await;
                                    cbs.get_mut(&task_sid)
                                        .is_some_and(|cb| cb.record_error())
                                };

                                // Update error count in shared metadata.
                                let err_count = task_error_counter.fetch_add(1, Ordering::AcqRel) + 1;
                                {
                                    let mut m = task_meta.lock().unwrap();
                                    m.error_count = err_count;
                                }

                                if should_pause {
                                    error!(
                                        strategy_id = %task_sid,
                                        "Circuit breaker threshold reached, strategy will be auto-paused"
                                    );
                                    // 通知外部：熔断触发（通过日志和返回状态让调度器处理）
                                }
                            }
                        }
                    }
                }
            }

            // 清理
            {
                let mut tasks = tasks.write().await;
                tasks.remove(&task_sid);
            }
            {
                let mut cbs = cbs.write().await;
                cbs.remove(&task_sid);
            }
            info!(strategy_id = %task_sid, "Scheduler task finished");
        });

        // 注册任务句柄
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(
                sid,
                SchedulerTaskHandle::new(
                    strategy_name,
                    interval_secs,
                    handle,
                ),
            );
        }

        Ok(())
    }

    /// 停止一个策略的定时执行
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn stop_strategy(&self, strategy_id: &str) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.write().await;
        let handle = tasks.remove(strategy_id).ok_or_else(|| {
            SchedulerError::NotFound(strategy_id.to_string())
        })?;

        // 中止任务
        if let Some(jh) = handle.join_handle {
            jh.abort();
            info!(strategy_id = %strategy_id, "Scheduler task aborted");
        }

        // 清理熔断器
        {
            let mut cbs = self.circuit_breakers.write().await;
            cbs.remove(strategy_id);
        }

        Ok(())
    }

    /// 获取熔断器状态
    #[must_use]
    pub async fn circuit_breaker_status(&self, strategy_id: &str) -> Option<CircuitBreaker> {
        let cbs = self.circuit_breakers.read().await;
        cbs.get(strategy_id).cloned()
    }

    /// 获取熔断器状态（所有策略）
    #[must_use]
    pub async fn all_circuit_breaker_status(&self) -> HashMap<String, CircuitBreaker> {
        let cbs = self.circuit_breakers.read().await;
        cbs.clone()
    }

    /// 列出所有正在运行的策略
    #[must_use]
    pub async fn list_running(&self) -> Vec<SchedulerTaskInfo> {
        let tasks = self.tasks.read().await;
        tasks
            .iter()
            .map(|(id, handle)| {
                let m = handle.meta.lock().unwrap();
                SchedulerTaskInfo {
                    strategy_id: id.clone(),
                    strategy_name: m.strategy_name.clone(),
                    status: quant_common::types::StrategyStatus::Running,
                    interval_secs: m.interval_secs,
                    last_run_at: m.last_run_at,
                    error_count: handle.error_counter.load(std::sync::atomic::Ordering::Acquire),
                }
            })
            .collect()
    }

    /// 获取当前运行中的策略数量
    #[must_use]
    pub async fn running_count(&self) -> usize {
        self.tasks.read().await.len()
    }

    /// 关闭所有正在运行的任务
    #[instrument(skip(self))]
    pub async fn shutdown_all(&self) {
        info!("Shutting down all scheduler tasks");
        // 发送关闭信号
        let _ = self.shutdown_tx.send(());
        // 等待所有任务完成
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let mut tasks = self.tasks.write().await;
        tasks.clear();
        let mut cbs = self.circuit_breakers.write().await;
        cbs.clear();
        info!("All scheduler tasks shut down");
    }
}

// ─── SchedulerError ─────────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::MeanReversionStrategy;
    use quant_common::types::StrategyParams;
    use quant_common::types::StrategyType;

    fn make_scheduler() -> StrategyScheduler {
        StrategyScheduler::new(SchedulerConfig::default())
    }

    fn make_dummy_strategy() -> Box<dyn Strategy> {
        let mut s = MeanReversionStrategy::new();
        let params = StrategyParams {
            strategy_id: "test_scheduler".to_string(),
            strategy_name: "Scheduler Test".to_string(),
            strategy_type: StrategyType::MeanReversion,
            params: serde_json::json!({}),
            enabled: true,
            max_position: rust_decimal::Decimal::new(100000, 0),
            max_daily_loss: rust_decimal::Decimal::new(5000, 0),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            status: quant_common::types::StrategyStatus::Draft,
            description: None,
            tags: vec![],
            symbols: vec![],
        };
        // 忽略 initialize 错误
        let _ = s.initialize(params);
        Box::new(s)
    }

    #[tokio::test]
    async fn test_scheduler_starts_empty() {
        let scheduler = make_scheduler();
        assert_eq!(scheduler.running_count().await, 0);
        assert!(scheduler.list_running().await.is_empty());
    }

    #[tokio::test]
    async fn test_start_and_stop_strategy() {
        let scheduler = make_scheduler();
        scheduler
            .start_strategy("test_001".to_string(), "Test Strategy".to_string(), make_dummy_strategy(), 3600)
            .await
            .unwrap();
        assert_eq!(scheduler.running_count().await, 1);

        scheduler.stop_strategy("test_001").await.unwrap();
        assert_eq!(scheduler.running_count().await, 0);
    }

    #[tokio::test]
    async fn test_start_twice_returns_error() {
        let scheduler = make_scheduler();
        scheduler
            .start_strategy("dup".to_string(), "Dup Strategy".to_string(), make_dummy_strategy(), 3600)
            .await
            .unwrap();

        let result = scheduler
            .start_strategy("dup".to_string(), "Dup Strategy".to_string(), make_dummy_strategy(), 3600)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SchedulerError::AlreadyRunning(_)));
    }

    #[tokio::test]
    async fn test_stop_nonexistent_returns_error() {
        let scheduler = make_scheduler();
        let result = scheduler.stop_strategy("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SchedulerError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_shutdown_all() {
        let scheduler = make_scheduler();
        scheduler
            .start_strategy("s1".to_string(), "Strategy 1".to_string(), make_dummy_strategy(), 3600)
            .await
            .unwrap();
        scheduler
            .start_strategy("s2".to_string(), "Strategy 2".to_string(), make_dummy_strategy(), 3600)
            .await
            .unwrap();
        assert_eq!(scheduler.running_count().await, 2);

        scheduler.shutdown_all().await;
        assert_eq!(scheduler.running_count().await, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_initialized() {
        let scheduler = make_scheduler();
        scheduler
            .start_strategy("cb_test".to_string(), "CB Test".to_string(), make_dummy_strategy(), 3600)
            .await
            .unwrap();

        let status = scheduler.circuit_breaker_status("cb_test").await;
        assert!(status.is_some());
        assert!(!status.unwrap().is_tripped());
    }
}
