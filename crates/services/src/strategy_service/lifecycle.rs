//! 策略回测与生命周期方法。

use crate::error::{ServiceError, ServiceResult};
use quant_common::types::{BacktestResult, StrategyParams, StrategyStatus, StrategyType};
use quant_repository::BacktestResultSummaryRow;
use rust_decimal::Decimal;
use std::time::Instant;
use strategy_engine::backtest::BacktestOptions;
use strategy_engine::{BacktestEngine, Strategy};
use tracing::{error, info, instrument, warn};

use super::StrategyService;

impl StrategyService {
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

        let symbol = Self::resolve_backtest_symbol(symbols, &params);
        if symbols.len() > 1 {
            warn!(
                strategy_id = %strategy_id,
                symbols = ?symbols,
                primary_symbol = %symbol,
                "Multi-symbol backtest currently uses the primary symbol for market data"
            );
        }

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
        let result = engine
            .run_with_options(&*strategy, market_data, BacktestOptions::default())
            .await
            .map_err(|e| {
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
        .await?;

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

    pub(crate) fn resolve_backtest_symbol(symbols: &[String], params: &StrategyParams) -> String {
        if let Some(symbol) = symbols.iter().find(|s| !s.trim().is_empty()) {
            return symbol.trim().to_string();
        }

        params
            .params
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("BTC-USDT")
            .to_string()
    }

    // ── Lifecycle ──────────────────────────────────────────────────────────

    /// Atomic compare-and-set of strategy status. Returns
    /// `ServiceError::ConcurrentModification` when the row's current status
    /// does not match `expected` (i.e. another writer won the race).
    async fn cas_status(
        &self,
        strategy_id: &str,
        target: StrategyStatus,
        expected: StrategyStatus,
    ) -> ServiceResult<()> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let updated = repo
            .update_status_if(strategy_id, target, expected, None)
            .await?;
        if !updated {
            return Err(ServiceError::ConcurrentModification {
                strategy_id: strategy_id.to_string(),
                expected,
            });
        }
        Ok(())
    }

    /// 部署策略（Draft → Deployed）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn deploy_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| {
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let target = StrategyStatus::Deployed;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy
            .on_deploy()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        self.cas_status(strategy_id, target, current_status).await?;

        info!(strategy_id = %strategy_id, "Strategy deployed");
        Ok(target)
    }

    /// 启动策略（Deployed → Running）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn start_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| {
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let target = StrategyStatus::Running;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy
            .on_start()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        // Fail-closed: refuse to mark the strategy Running when the scheduler is
        // not wired for live trading, so the UI never shows "Running" while the
        // strategy is actually idling.
        if let Some(ref scheduler) = self.scheduler {
            if !scheduler.trading_ready() {
                return Err(ServiceError::Scheduler(
                    "Scheduler not configured for live trading (missing pipeline or market data provider)"
                        .to_string(),
                ));
            }
        }
        self.cas_status(strategy_id, target, current_status).await?;

        // Register with scheduler AFTER successful DB update
        if let Some(ref scheduler) = self.scheduler {
            let interval_secs = scheduler.config().default_interval_secs;
            scheduler
                .start_strategy(
                    strategy_id.to_string(),
                    params.strategy_name.clone(),
                    strategy,
                    interval_secs,
                )
                .await
                .map_err(|e| {
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
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| {
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let target = StrategyStatus::Archived;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy
            .on_stop()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        self.cas_status(strategy_id, target, current_status).await?;

        // Unregister from scheduler AFTER successful DB update
        if let Some(ref scheduler) = self.scheduler {
            match scheduler.stop_strategy(strategy_id).await {
                Ok(()) => {}
                Err(e) => {
                    warn!(
                        "Strategy not found in scheduler (already stopped?): {}: {}",
                        strategy_id, e
                    );
                }
            }
        }

        info!(strategy_id = %strategy_id, "Strategy stopped");
        Ok(target)
    }

    /// 暂停策略（Running → Paused）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn pause_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| {
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let target = StrategyStatus::Paused;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy
            .on_pause()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        self.cas_status(strategy_id, target, current_status).await?;

        // Unregister from scheduler AFTER successful DB update
        if let Some(ref scheduler) = self.scheduler {
            match scheduler.stop_strategy(strategy_id).await {
                Ok(()) => {}
                Err(e) => {
                    warn!(
                        "Strategy not found in scheduler (already paused?): {}: {}",
                        strategy_id, e
                    );
                }
            }
        }

        info!(strategy_id = %strategy_id, "Strategy paused");
        Ok(target)
    }

    /// 恢复策略（Paused → Running）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn resume_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| {
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let target = StrategyStatus::Running;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy
            .on_resume()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        // Fail-closed: refuse to mark the strategy Running when the scheduler is
        // not wired for live trading (see `start_strategy`).
        if let Some(ref scheduler) = self.scheduler {
            if !scheduler.trading_ready() {
                return Err(ServiceError::Scheduler(
                    "Scheduler not configured for live trading (missing pipeline or market data provider)"
                        .to_string(),
                ));
            }
        }
        self.cas_status(strategy_id, target, current_status).await?;

        // Re-register with scheduler AFTER successful DB update
        if let Some(ref scheduler) = self.scheduler {
            let interval_secs = scheduler.config().default_interval_secs;
            scheduler
                .start_strategy(
                    strategy_id.to_string(),
                    params.strategy_name.clone(),
                    strategy,
                    interval_secs,
                )
                .await
                .map_err(|e| {
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
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| {
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let target = StrategyStatus::Archived;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy
            .on_archive()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        self.cas_status(strategy_id, target, current_status).await?;

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
            Some(reg) if reg.has_type(&type_str) => reg.create(&type_str, params.clone()).await?,
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
    pub async fn get_running_strategies(
        &self,
    ) -> ServiceResult<Vec<quant_common::types::SchedulerTaskInfo>> {
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or_else(|| ServiceError::NotInitialized("Scheduler not initialized".into()))?;
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
        repo.find_all(limit, offset)
            .await
            .map_err(ServiceError::from)
    }

    /// Query a single backtest result by ID (includes equity_curve).
    #[instrument(skip(self), fields(%id))]
    pub async fn get_backtest_result(&self, id: i64) -> ServiceResult<BacktestResult> {
        let repo = self
            .backtest_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        repo.find_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("Backtest result '{}' not found", id)))
    }

    /// Delete a backtest result by ID. Returns true if a row was deleted.
    #[instrument(skip(self), fields(%id))]
    pub async fn delete_backtest_result(&self, id: i64) -> ServiceResult<bool> {
        let repo = self
            .backtest_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let deleted = repo.delete_by_id(id).await?;
        if deleted {
            info!(%id, "Backtest result deleted");
        } else {
            warn!(%id, "Backtest result not found for deletion");
        }
        Ok(deleted)
    }
}
