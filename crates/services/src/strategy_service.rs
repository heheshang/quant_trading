use crate::error::{ServiceError, ServiceResult};
use crate::market_data_provider::MarketDataProvider;
use quant_common::types::{
    BacktestResult, StrategyParams, StrategyStatus, StrategyType,
};
use quant_repository::{BacktestRepository, BacktestResultSummaryRow, PostgresClient};
use quant_repository::StrategyRepository as StRepo;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::Instant;
use strategy_engine::registry::StrategyRegistry;
use strategy_engine::scheduler::StrategyScheduler;
use strategy_engine::{BacktestEngine, Strategy};
use tracing::{error, info, instrument, warn};

/// 策略服务 — 管理策略注册、生命周期、回测与调度
pub struct StrategyService {
    #[allow(dead_code)]
    postgres: Option<Arc<PostgresClient>>,
    market_data_provider: Option<Arc<dyn MarketDataProvider>>,
    backtest_repo: Option<Arc<dyn BacktestRepository>>,
    strategy_repo: Option<Arc<dyn StRepo>>,
    scheduler: Option<Arc<StrategyScheduler>>,
    registry: Option<Arc<StrategyRegistry>>,
}

impl StrategyService {
    /// 创建 StrategyService（registry 可选，注入后启用注册中心功能）
    pub fn new(
        postgres: Option<Arc<PostgresClient>>,
        market_data_provider: Option<Arc<dyn MarketDataProvider>>,
        backtest_repo: Option<Arc<dyn BacktestRepository>>,
        strategy_repo: Option<Arc<dyn StRepo>>,
        scheduler: Option<Arc<StrategyScheduler>>,
    ) -> Self {
        Self {
            postgres,
            market_data_provider,
            backtest_repo,
            strategy_repo,
            scheduler,
            registry: None,
        }
    }

    /// 设置策略注册中心（可在创建后注入）
    pub fn set_registry(&mut self, registry: Arc<StrategyRegistry>) {
        self.registry = Some(registry);
    }

    /// 获取策略类型元数据列表（来自注册中心）
    pub fn list_strategy_types(&self) -> ServiceResult<Vec<strategy_engine::registry::StrategyTypeInfo>> {
        let reg = self.registry.as_ref().ok_or_else(|| {
            ServiceError::Other("Strategy registry not initialized".into())
        })?;
        Ok(reg.list_types())
    }

    /// 检查注册中心是否包含指定类型
    pub fn has_strategy_type(&self, type_name: &str) -> ServiceResult<bool> {
        let reg = self.registry.as_ref().ok_or_else(|| {
            ServiceError::Other("Strategy registry not initialized".into())
        })?;
        Ok(reg.has_type(type_name))
    }

    // ── CRUD ───────────────────────────────────────────────────────────────

    /// List all strategies (unpaginated, no status filter).
    #[instrument(skip_all)]
    pub async fn get_strategies(&self) -> ServiceResult<Vec<StrategyParams>> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let (rows, _total) = repo.find_all(None, None, None, None, 10000, 0).await.map_err(|e| {
            error!("Failed to query strategies: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        let strategies: Vec<StrategyParams> = rows.iter().filter_map(|row| {
            match row.to_domain() {
                Ok(p) => Some(p),
                Err(e) => {
                    error!("Failed to convert strategy row {}: {}", row.strategy_id, e);
                    None
                }
            }
        }).collect();
        info!(count = strategies.len(), "Strategies retrieved");
        Ok(strategies)
    }

    /// List strategies with status filter and pagination.
    #[instrument(skip(self), fields(page, page_size))]
    pub async fn list_strategies(
        &self,
        status_filter: Option<StrategyStatus>,
        page: i64,
        page_size: i64,
    ) -> ServiceResult<Vec<StrategyParams>> {
        if !(1..=100).contains(&page_size) || page < 1 {
            return Err(ServiceError::PaginationInvalid {
                reason: format!("Page must be >= 1, page_size must be 1-100 (got page={}, page_size={})", page, page_size)
            });
        }
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let offset = (page - 1) * page_size;
        let (rows, _total) = repo.find_all(None, None, status_filter, None, page_size, offset).await.map_err(|e| {
            error!("Failed to query strategies: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        let strategies: Vec<StrategyParams> = rows.iter().filter_map(|row| {
            match row.to_domain() {
                Ok(p) => Some(p),
                Err(e) => {
                    error!("Failed to convert strategy row {}: {}", row.strategy_id, e);
                    None
                }
            }
        }).collect();
        info!(count = strategies.len(), "Strategies listed");
        Ok(strategies)
    }

    #[instrument(skip(self, strategy), fields(strategy_id = %strategy.strategy_id))]
    pub async fn save_strategy(&self, strategy: &StrategyParams) -> ServiceResult<String> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        let existing = repo.find_by_id(&strategy.strategy_id).await.map_err(|e| {
            error!("Failed to check strategy {}: {}", strategy.strategy_id, e);
            ServiceError::Other(e.to_string())
        })?;

        if existing.is_some() {
            repo.update(strategy).await.map_err(|e| {
                error!("Failed to update strategy {}: {}", strategy.strategy_id, e);
                ServiceError::Other(e.to_string())
            })?;
            info!(strategy_id = %strategy.strategy_id, "Strategy updated");
        } else {
            repo.insert(strategy).await.map_err(|e| {
                error!("Failed to insert strategy {}: {}", strategy.strategy_id, e);
                ServiceError::Other(e.to_string())
            })?;
            info!(strategy_id = %strategy.strategy_id, "Strategy inserted");
        }

        Ok(strategy.strategy_id.clone())
    }

    /// Update an existing strategy. Returns the strategy_id.
    #[instrument(skip(self, strategy), fields(strategy_id = %strategy.strategy_id))]
    pub async fn update_strategy(&self, strategy: &StrategyParams) -> ServiceResult<String> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        let updated = repo.update(strategy).await.map_err(|e| {
            error!("Failed to update strategy {}: {}", strategy.strategy_id, e);
            ServiceError::Other(e.to_string())
        })?;

        if !updated {
            return Err(ServiceError::NotFound(format!(
                "Strategy '{}' not found",
                strategy.strategy_id
            )));
        }

        info!(strategy_id = %strategy.strategy_id, "Strategy updated");
        Ok(strategy.strategy_id.clone())
    }

    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn delete_strategy(&self, strategy_id: &str) -> ServiceResult<bool> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        let deleted = repo.delete_by_id(strategy_id).await.map_err(|e| {
            error!("Failed to delete strategy {}: {}", strategy_id, e);
            ServiceError::Other(e.to_string())
        })?;

        if deleted {
            info!(strategy_id = %strategy_id, "Strategy deleted");
        } else {
            warn!(strategy_id = %strategy_id, "Strategy not found for deletion");
        }
        Ok(deleted)
    }

    #[instrument(skip(self), fields(strategy_id = %strategy_id, enabled))]
    pub async fn toggle_strategy(&self, strategy_id: &str, enabled: bool) -> ServiceResult<bool> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        if let Some(mut params) = repo.find_by_id(strategy_id).await.map_err(|e| {
            error!("Failed to fetch strategy for toggle {}: {}", strategy_id, e);
            ServiceError::Other(e.to_string())
        })? {
            params.enabled = enabled;
            params.updated_at = chrono::Utc::now();
            repo.update(&params).await.map_err(|e| {
                error!("Failed to toggle strategy {}: {}", strategy_id, e);
                ServiceError::Other(e.to_string())
            })?;
            info!(strategy_id = %strategy_id, enabled, "Strategy toggled");
            Ok(true)
        } else {
            warn!(strategy_id = %strategy_id, "Strategy not found for toggle");
            Ok(false)
        }
    }

    // ── Backtest ────────────────────────────────────────────────────────────

    /// 从数据库读取策略参数并构建策略实例（通过注册中心）
    async fn build_strategy_from_db(
        &self,
        strategy_id: &str,
    ) -> ServiceResult<(String, Box<dyn Strategy>, StrategyParams)> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        let params = repo.find_by_id(strategy_id).await.map_err(|e| {
            error!("Failed to fetch strategy {}: {}", strategy_id, e);
            ServiceError::Other(e.to_string())
        })?.ok_or_else(|| {
            error!("Strategy '{}' not found", strategy_id);
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let type_str = format!("{:?}", params.strategy_type);

        // 通过注册中心创建策略实例，回退到硬编码
        let strategy: Box<dyn Strategy> = match self.registry.as_ref() {
            Some(reg) if reg.has_type(&type_str) => {
                reg.create(&type_str, params.clone()).await.map_err(|e| {
                    ServiceError::Strategy(format!("Failed to create strategy '{}': {}", type_str, e))
                })?
            }
            _ => {
                if params.strategy_type != StrategyType::MeanReversion {
                    return Err(ServiceError::Strategy(format!(
                        "Strategy type '{:?}' is not supported. Registry not initialized or type not registered.",
                        params.strategy_type
                    )));
                }
                let mut s = strategy_engine::strategy::MeanReversionStrategy::new();
                s.initialize(params.clone())
                    .await
                    .map_err(|e| ServiceError::Strategy(e.to_string()))?;
                Box::new(s)
            }
        };

        Ok((type_str, strategy, params))
    }

    #[instrument(skip(self), fields(strategy_id = %strategy_id, initial_capital, symbols = ?symbols))]
    pub async fn run_backtest(
        &self,
        strategy_id: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        initial_capital: Decimal,
        commission_rate: Decimal,
        slippage: Decimal,
        symbols: &[String],
    ) -> ServiceResult<BacktestResult> {
        let start_time = Instant::now();

        let (db_type, strategy, params) = self.build_strategy_from_db(strategy_id).await?;

        let symbol = params
            .params
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("BTC-USDT")
            .to_string();

        let market_data = {
            let provider = self.market_data_provider.as_ref().ok_or_else(|| {
                error!("Market data provider not initialized for backtest");
                ServiceError::DataSource("Market data provider not initialized".to_string())
            })?;
            provider
                .get_historical_data(&symbol, start, end)
                .await
                .map_err(|e| {
                    error!("Failed to fetch market data for {}: {}", symbol, e);
                    ServiceError::DataSource(e)
                })?
        };

        if market_data.is_empty() {
            error!(symbol = %symbol, "No market data for backtest date range");
            return Err(ServiceError::Other(
                "No market data returned for the specified date range".into(),
            ));
        }

        let mut engine = BacktestEngine::new(initial_capital, commission_rate, slippage);
        let result = engine.run(&*strategy, market_data).await.map_err(|e| {
            error!("Backtest execution failed: {}", e);
            ServiceError::Backtest(e.to_string())
        })?;

        let backtest_result = BacktestResult {
            id: None,
            strategy_id: strategy_id.to_string(),
            ..result
        };

        // Persist via Repository
        let repo = self
            .backtest_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        repo.insert(
            &backtest_result,
            &params.strategy_name,
            symbols,
            commission_rate,
            slippage,
        )
        .await
        .map_err(|e| {
            error!("Failed to persist backtest result: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        let duration_ms = start_time.elapsed().as_millis();
        info!(
            strategy_id = %backtest_result.strategy_id,
            type = %db_type,
            total_return = %backtest_result.total_return,
            sharpe_ratio = %backtest_result.sharpe_ratio,
            max_drawdown = %backtest_result.max_drawdown,
            total_trades = backtest_result.total_trades,
            duration_ms,
            "Backtest completed"
        );
        Ok(backtest_result)
    }

    // ── Lifecycle ──────────────────────────────────────────────────────────

    /// 部署策略（Draft → Deployed）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn deploy_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let mut params = repo.find_by_id(strategy_id).await.map_err(|e| {
            error!("Failed to fetch strategy {}: {}", strategy_id, e);
            ServiceError::Other(e.to_string())
        })?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Deployed;
        params.transition_to(target).map_err(|_| {
            ServiceError::InvalidStatusTransition {
                from: format!("{:?}", params.status),
                to: format!("{:?}", target),
            }
        })?;

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_deploy().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;

        params.status = target;
        params.updated_at = chrono::Utc::now();
        repo.update(&params).await.map_err(|e| {
            error!("Failed to persist strategy status: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        info!(strategy_id = %strategy_id, "Strategy deployed");
        Ok(target)
    }

    /// 启动策略（Deployed → Running）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn start_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let mut params = repo.find_by_id(strategy_id).await.map_err(|e| {
            error!("Failed to fetch strategy {}: {}", strategy_id, e);
            ServiceError::Other(e.to_string())
        })?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Running;
        params.transition_to(target).map_err(|_| {
            ServiceError::InvalidStatusTransition {
                from: format!("{:?}", params.status),
                to: format!("{:?}", target),
            }
        })?;

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_start().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;

        params.status = target;
        params.updated_at = chrono::Utc::now();
        repo.update(&params).await.map_err(|e| {
            error!("Failed to persist strategy status: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        // Register with scheduler for periodic signal generation
        if let Some(ref scheduler) = self.scheduler {
            let interval_secs = scheduler.config().default_interval_secs;
            scheduler.start_strategy(
                strategy_id.to_string(),
                params.strategy_name.clone(),
                strategy,
                interval_secs,
            ).await.map_err(|e| {
                error!("Failed to start scheduler for {}: {}", strategy_id, e);
                ServiceError::Scheduler(e.to_string())
            })?;
        }

        info!(strategy_id = %strategy_id, "Strategy started");
        Ok(target)
    }

    /// 停止策略（Running → Archived）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn stop_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let mut params = repo.find_by_id(strategy_id).await.map_err(|e| {
            error!("Failed to fetch strategy {}: {}", strategy_id, e);
            ServiceError::Other(e.to_string())
        })?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Archived;
        params.transition_to(target).map_err(|_| {
            ServiceError::InvalidStatusTransition {
                from: format!("{:?}", params.status),
                to: format!("{:?}", target),
            }
        })?;

        // Unregister from scheduler first (before status change to avoid race)
        if let Some(ref scheduler) = self.scheduler {
            scheduler.stop_strategy(strategy_id).await.map_err(|e| {
                error!("Failed to stop scheduler for {}: {}", strategy_id, e);
                ServiceError::Scheduler(e.to_string())
            })?;
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_stop().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;

        params.status = target;
        params.updated_at = chrono::Utc::now();
        repo.update(&params).await.map_err(|e| {
            error!("Failed to persist strategy status: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        info!(strategy_id = %strategy_id, "Strategy stopped");
        Ok(target)
    }

    /// 暂停策略（Running → Paused）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn pause_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let mut params = repo.find_by_id(strategy_id).await.map_err(|e| {
            error!("Failed to fetch strategy {}: {}", strategy_id, e);
            ServiceError::Other(e.to_string())
        })?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Paused;
        params.transition_to(target).map_err(|_| {
            ServiceError::InvalidStatusTransition {
                from: format!("{:?}", params.status),
                to: format!("{:?}", target),
            }
        })?;

        // Unregister from scheduler
        if let Some(ref scheduler) = self.scheduler {
            scheduler.stop_strategy(strategy_id).await.map_err(|e| {
                error!("Failed to pause scheduler for {}: {}", strategy_id, e);
                ServiceError::Scheduler(e.to_string())
            })?;
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_pause().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;

        params.status = target;
        params.updated_at = chrono::Utc::now();
        repo.update(&params).await.map_err(|e| {
            error!("Failed to persist strategy status: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        info!(strategy_id = %strategy_id, "Strategy paused");
        Ok(target)
    }

    /// 恢复策略（Paused → Running）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn resume_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let mut params = repo.find_by_id(strategy_id).await.map_err(|e| {
            error!("Failed to fetch strategy {}: {}", strategy_id, e);
            ServiceError::Other(e.to_string())
        })?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Running;
        params.transition_to(target).map_err(|_| {
            ServiceError::InvalidStatusTransition {
                from: format!("{:?}", params.status),
                to: format!("{:?}", target),
            }
        })?;

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_resume().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;

        params.status = target;
        params.updated_at = chrono::Utc::now();
        repo.update(&params).await.map_err(|e| {
            error!("Failed to persist strategy status: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        // Re-register with scheduler
        if let Some(ref scheduler) = self.scheduler {
            let interval_secs = scheduler.config().default_interval_secs;
            scheduler.start_strategy(
                strategy_id.to_string(),
                params.strategy_name.clone(),
                strategy,
                interval_secs,
            ).await.map_err(|e| {
                error!("Failed to start scheduler for {}: {}", strategy_id, e);
                ServiceError::Scheduler(e.to_string())
            })?;
        }

        info!(strategy_id = %strategy_id, "Strategy resumed");
        Ok(target)
    }

    /// 归档策略（任何状态 → Archived）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn archive_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let mut params = repo.find_by_id(strategy_id).await.map_err(|e| {
            error!("Failed to fetch strategy {}: {}", strategy_id, e);
            ServiceError::Other(e.to_string())
        })?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Archived;
        params.transition_to(target).map_err(|_| {
            ServiceError::InvalidStatusTransition {
                from: format!("{:?}", params.status),
                to: format!("{:?}", target),
            }
        })?;

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_archive().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;

        params.status = target;
        params.updated_at = chrono::Utc::now();
        repo.update(&params).await.map_err(|e| {
            error!("Failed to persist strategy status: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        info!(strategy_id = %strategy_id, "Strategy archived");
        Ok(target)
    }

    /// Build strategy instance from params via registry or fallback.
    async fn build_strategy_from_params(
        &self,
        params: &StrategyParams,
    ) -> ServiceResult<(String, Box<dyn Strategy>)> {
        let type_str = format!("{:?}", params.strategy_type);
        let strategy: Box<dyn Strategy> = match self.registry.as_ref() {
            Some(reg) if reg.has_type(&type_str) => {
                reg.create(&type_str, params.clone()).await.map_err(|e| {
                    ServiceError::Strategy(format!("Failed to create strategy '{}': {}", type_str, e))
                })?
            }
            _ => {
                if params.strategy_type != StrategyType::MeanReversion {
                    return Err(ServiceError::Strategy(format!(
                        "Strategy type '{:?}' is not supported",
                        params.strategy_type
                    )));
                }
                let mut s = strategy_engine::strategy::MeanReversionStrategy::new();
                s.initialize(params.clone())
                    .await
                    .map_err(|e| ServiceError::Strategy(e.to_string()))?;
                Box::new(s)
            }
        };
        Ok((type_str, strategy))
    }

    // ── Scheduler Queries ──────────────────────────────────────────────────

    /// Return the list of currently running strategies from the scheduler.
    pub async fn get_running_strategies(&self) -> ServiceResult<Vec<quant_common::types::SchedulerTaskInfo>> {
        let scheduler = self.scheduler.as_ref().ok_or_else(|| {
            ServiceError::Other("Scheduler not initialized".into())
        })?;
        Ok(scheduler.list_running().await)
    }

    // ── Backtest Results Queries ──────────────────────────────────────────

    /// Query backtest results with pagination (sorted by created_at DESC).
    #[instrument(skip(self), fields(limit, offset))]
    pub async fn get_backtest_results(
        &self,
        limit: i64,
        offset: i64,
    ) -> ServiceResult<Vec<BacktestResultSummaryRow>> {
        let repo = self
            .backtest_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        repo.find_all(limit, offset).await.map_err(|e| {
            error!("Failed to query backtest results: {}", e);
            ServiceError::Other(e.to_string())
        })
    }

    /// Query a single backtest result by ID (includes equity_curve).
    #[instrument(skip(self), fields(%id))]
    pub async fn get_backtest_result(&self, id: i64) -> ServiceResult<BacktestResult> {
        let repo = self
            .backtest_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        repo.find_by_id(id)
            .await
            .map_err(|e| {
                error!("Failed to query backtest result {}: {}", id, e);
                ServiceError::Other(e.to_string())
            })?
            .ok_or_else(|| ServiceError::NotFound(format!("Backtest result '{}' not found", id)))
    }

    /// Delete a backtest result by ID. Returns true if a row was deleted.
    #[instrument(skip(self), fields(%id))]
    pub async fn delete_backtest_result(&self, id: i64) -> ServiceResult<bool> {
        let repo = self
            .backtest_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let deleted = repo.delete_by_id(id).await.map_err(|e| {
            error!("Failed to delete backtest result {}: {}", id, e);
            ServiceError::Other(e.to_string())
        })?;
        if deleted {
            info!(%id, "Backtest result deleted");
        } else {
            warn!(%id, "Backtest result not found for deletion");
        }
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromPrimitive;

    fn make_service_no_db() -> StrategyService {
        StrategyService::new(None, None, None, None, None)
    }

    #[tokio::test]
    async fn get_strategies_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc.get_strategies().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn save_strategy_no_db_returns_error() {
        let svc = make_service_no_db();
        let strategy = StrategyParams {
            strategy_id: "test_001".to_string(),
            strategy_name: "Test".to_string(),
            strategy_type: StrategyType::MeanReversion,
            params: serde_json::json!({}),
            enabled: true,
            max_position: Decimal::ZERO,
            max_daily_loss: Decimal::ZERO,
            status: Default::default(),
            description: None,
            tags: vec![],
            symbols: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let result = svc.save_strategy(&strategy).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn delete_strategy_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc.delete_strategy("test_001").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn toggle_strategy_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc.toggle_strategy("test_001", true).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn run_backtest_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc
            .run_backtest(
                "test_001",
                chrono::Utc::now() - chrono::Duration::days(7),
                chrono::Utc::now(),
                Decimal::from(100000),
                Decimal::from_f64(0.001).unwrap(),
                Decimal::from_f64(0.0005).unwrap(),
                &["BTC-USDT".to_string()],
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn get_backtest_results_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc.get_backtest_results(20, 0).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn get_backtest_result_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc.get_backtest_result(1).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn delete_backtest_result_no_db_returns_error() {
        let svc = make_service_no_db();
        let result = svc.delete_backtest_result(1).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[test]
    fn new_with_none_creates_service_with_no_deps() {
        let svc = make_service_no_db();
        assert!(svc.postgres.is_none());
        assert!(svc.market_data_provider.is_none());
        assert!(svc.backtest_repo.is_none());
    }

    // ── Registry ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_strategy_types_no_registry_returns_error() {
        let svc = make_service_no_db();
        let result = svc.list_strategy_types();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_strategy_types_with_registry() {
        let mut svc = make_service_no_db();
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let result = svc.list_strategy_types().unwrap();
        assert!(!result.is_empty());
        assert_eq!(result[0].type_name, "MeanReversion");
    }

    #[tokio::test]
    async fn lifecycle_methods_no_db_returns_error() {
        let svc = make_service_no_db();
        assert!(svc.deploy_strategy("test_001").await.is_err());
        assert!(svc.start_strategy("test_001").await.is_err());
        assert!(svc.stop_strategy("test_001").await.is_err());
        assert!(svc.pause_strategy("test_001").await.is_err());
        assert!(svc.resume_strategy("test_001").await.is_err());
        assert!(svc.archive_strategy("test_001").await.is_err());
    }
}
