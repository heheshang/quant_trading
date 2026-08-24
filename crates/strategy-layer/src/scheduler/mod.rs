//! Strategy Scheduler — 策略调度执行引擎
//!
//! 为每个运行中的策略创建独立的 Tokio 异步任务，按配置间隔定时执行
//! 策略的信号生成逻辑。支持并发控制、熔断保护和优雅关闭。

mod circuit_breaker;
mod error;
mod task;

pub use error::SchedulerError;

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
    pipeline: std::sync::RwLock<Option<Arc<PipelineExecutor>>>,
    /// 市场数据提供者
    market_data_provider: std::sync::RwLock<Option<Arc<dyn MarketDataProvider>>>,
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
            pipeline: std::sync::RwLock::new(None),
            market_data_provider: std::sync::RwLock::new(None),
        }
    }

    pub fn set_pipeline(&self, pipeline: Arc<PipelineExecutor>) {
        if let Ok(mut guard) = self.pipeline.write() {
            *guard = Some(pipeline);
        }
    }

    /// 设置市场数据提供者
    pub fn set_market_data_provider(&self, provider: Arc<dyn MarketDataProvider>) {
        if let Ok(mut guard) = self.market_data_provider.write() {
            *guard = Some(provider);
        }
    }

    /// 判断调度器是否已具备真实交易能力（调度器启用 + 流水线 + 行情源均已注入）。
    ///
    /// 用于在启动策略前 fail-closed：若未配置，启动应拒绝以消除
    /// 「状态显示 Running 却空转」的误导。
    ///
    /// 安全语义：仅当 `SchedulerConfig::enabled` 为真（调度器显式启用）且
    /// 流水线与行情提供者都已注入时才允许进入 Running；否则一律拒绝。
    #[must_use]
    pub fn trading_ready(&self) -> bool {
        if !self.config.enabled {
            return false;
        }
        let pipeline_ready = self.pipeline.read().map(|g| g.is_some()).unwrap_or(false);
        let provider_ready = self
            .market_data_provider
            .read()
            .map(|g| g.is_some())
            .unwrap_or(false);
        pipeline_ready && provider_ready
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

        // fail-closed: 缺少流水线或行情源时拒绝启动，避免「Running 却空转」。
        if !self.trading_ready() {
            return Err(SchedulerError::NotConfigured(
                "signal pipeline and/or market data provider are not wired; refusing to start an idling strategy"
                    .to_string(),
            ));
        }

        let shutdown_rx = self.shutdown_tx.subscribe();
        let tasks = Arc::clone(&self.tasks);
        let cbs = Arc::clone(&self.circuit_breakers);
        let pipeline = self.pipeline.read().ok().and_then(|g| (*g).clone());
        let market_data_provider = self
            .market_data_provider
            .read()
            .ok()
            .and_then(|g| (*g).clone());
        let strategy_symbols = strategy.params().symbols.clone();

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

                        // 逐标的生成信号：每个标的用各自 24h 历史，避免多标的交错污染指标；
                        // 每个标的独立执行流水线（多标的策略可对每个 symbol 各下一单）。
                        let now = chrono::Utc::now();
                        let mut any_signal = false;
                        for symbol in &strategy_symbols {
                            let mut market_data = Vec::new();
                            if let Some(ref provider) = market_data_provider {
                                match provider.get_historical_data(
                                    symbol,
                                    now - chrono::Duration::hours(24),
                                    now,
                                    "1H",
                                ).await {
                                    Ok(data) => market_data = data,
                                    Err(e) => {
                                        warn!(strategy_id = %task_sid, symbol = %symbol, error = %e, "Failed to fetch market data");
                                        continue;
                                    }
                                }
                            }
                            if market_data.is_empty() {
                                continue;
                            }
                            let context = crate::strategy::StrategyContext {
                                current_time: now,
                                positions: Vec::new(),
                                market_data,
                            };
                            match strategy.generate_signals(&context).await {
                                Ok(signals) => {
                                    if !signals.is_empty() {
                                        any_signal = true;
                                    }
                                    info!(
                                        strategy_id = %task_sid,
                                        symbol = %symbol,
                                        signal_count = signals.len(),
                                        "Signals generated"
                                    );
                                    // 通过流水线处理订单
                                    if let Some(ref pipeline) = pipeline {
                                        for order in signals {
                                            if let Err(e) = pipeline.execute(order).await {
                                                error!(
                                                    strategy_id = %task_sid,
                                                    symbol = %symbol,
                                                    error = %e,
                                                    "Pipeline step failed"
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        strategy_id = %task_sid,
                                        symbol = %symbol,
                                        error = %e,
                                        "Signal generation failed"
                                    );
                                }
                            }
                        }
                        if any_signal {
                            // 重置熔断器（至少一个标的成功执行）+ 更新成功元数据。
                            {
                                let mut cbs = cbs.write().await;
                                if let Some(cb) = cbs.get_mut(&task_sid) {
                                    cb.reset();
                                }
                            }
                            {
                                let mut m = task_meta.lock().unwrap();
                                m.last_run_at = Some(chrono::Utc::now());
                                m.error_count = 0;
                            }
                            task_error_counter.store(0, Ordering::Release);
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
                SchedulerTaskHandle::new(strategy_name, interval_secs, handle),
            );
        }

        Ok(())
    }

    /// 停止一个策略的定时执行
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn stop_strategy(&self, strategy_id: &str) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.write().await;
        let handle = tasks
            .remove(strategy_id)
            .ok_or_else(|| SchedulerError::NotFound(strategy_id.to_string()))?;

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
                    error_count: handle
                        .error_counter
                        .load(std::sync::atomic::Ordering::Acquire),
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
// SchedulerError 移至 scheduler/error.rs，此处 re-export 保持 API 不变。

#[cfg(test)]
mod tests;
