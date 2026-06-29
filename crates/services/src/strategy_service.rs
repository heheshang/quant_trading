use crate::error::{ServiceError, ServiceResult};
use crate::market_data_provider::MarketDataProvider;
use quant_common::types::{
    BacktestResult, ParameterSchema, ParamType, StrategyParams, StrategyStatus, StrategyType,
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
            ServiceError::NotInitialized("Strategy registry not initialized".into())
        })?;
        Ok(reg.list_types())
    }

    /// 检查注册中心是否包含指定类型
    pub fn has_strategy_type(&self, type_name: &str) -> ServiceResult<bool> {
        let reg = self.registry.as_ref().ok_or_else(|| {
            ServiceError::NotInitialized("Strategy registry not initialized".into())
        })?;
        Ok(reg.has_type(type_name))
    }

    /// Get the full metadata (including parameter schema) for a strategy type.
    pub fn get_strategy_type_info(
        &self,
        type_name: &str,
    ) -> ServiceResult<strategy_engine::registry::StrategyTypeInfo> {
        let reg = self.registry.as_ref().ok_or_else(|| {
            ServiceError::NotInitialized("Strategy registry not initialized".into())
        })?;
        reg.get_type_info(type_name).ok_or_else(|| {
            ServiceError::NotFound(format!("Unknown strategy type '{}'", type_name))
        })
    }

    /// Validate user-supplied params against the parameter schema for a strategy type.
    ///
    /// Checks:
    /// - All required schema params are present
    /// - Numeric params are within range (if range defined)
    /// - Select params are one of the allowed values
    /// - Type correctness (number, string, select)
    pub fn validate_strategy_params(
        &self,
        type_name: &str,
        params: &serde_json::Value,
    ) -> ServiceResult<()> {
        let info = self.get_strategy_type_info(type_name)?;
        Self::validate_strategy_params_with_info(&info, params)
    }

    /// Static version of validate_strategy_params that takes an already-fetched
    /// StrategyTypeInfo reference, avoiding redundant lookups.
    fn validate_strategy_params_with_info(
        info: &strategy_engine::registry::StrategyTypeInfo,
        params: &serde_json::Value,
    ) -> ServiceResult<()> {
        for schema in &info.parameters {
            let value = match params.get(&schema.name) {
                Some(v) => v,
                None => {
                    return Err(ServiceError::InvalidParameter(format!(
                        "Missing required parameter '{}': {}",
                        schema.name, schema.description
                    )));
                }
            };
            Self::validate_param_value(schema, value)?;
        }
        Ok(())
    }

    /// Validate a single parameter value against its schema.
    fn validate_param_value(
        schema: &ParameterSchema,
        value: &serde_json::Value,
    ) -> ServiceResult<()> {
        match &schema.param_type {
            ParamType::Number => {
                let n = value.as_f64().ok_or_else(|| {
                    ServiceError::InvalidParameter(format!(
                        "Parameter '{}' must be a number, got {:?}",
                        schema.name, value
                    ))
                })?;
                if let Some(range) = &schema.range {
                    if n < range.min || n > range.max {
                        return Err(ServiceError::InvalidParameter(format!(
                            "Parameter '{}' value {} is out of range [{}, {}]",
                            schema.name, n, range.min, range.max
                        )));
                    }
                }
            }
            ParamType::String => {
                if !value.is_string() {
                    return Err(ServiceError::InvalidParameter(format!(
                        "Parameter '{}' must be a string, got {:?}",
                        schema.name, value
                    )));
                }
            }
            ParamType::Select(options) => {
                let s = value.as_str().ok_or_else(|| {
                    ServiceError::InvalidParameter(format!(
                        "Parameter '{}' must be a string (one of {:?}), got {:?}",
                        schema.name, options, value
                    ))
                })?;
                if !options.iter().any(|o| o == s) {
                    return Err(ServiceError::InvalidParameter(format!(
                        "Parameter '{}' value '{}' is not one of the allowed options: {:?}",
                        schema.name, s, options
                    )));
                }
            }
        }
        Ok(())
    }

    /// Create a new strategy with auto-generated UUID v7 strategy_id.
    ///
    /// Validates params against the schema, then inserts into DB.
    /// Returns the newly generated strategy_id.
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip(self), fields(type_name, strategy_name))]
    pub async fn create_strategy(
        &self,
        type_name: &str,
        strategy_name: &str,
        params: serde_json::Value,
        enabled: bool,
        max_position: Decimal,
        max_daily_loss: Decimal,
        instance_label: Option<String>,
        description: Option<String>,
        tags: Vec<String>,
        symbols: Vec<String>,
        user_id: i64,
    ) -> ServiceResult<String> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        let type_info = self.get_strategy_type_info(type_name)?;

        let params = match params {
            serde_json::Value::Object(mut map) => {
                for p in &type_info.parameters {
                    if !map.contains_key(&p.name) && !p.default.is_null() {
                        map.insert(p.name.clone(), p.default.clone());
                    }
                }
                serde_json::Value::Object(map)
            }
            other => other,
        };

        Self::validate_strategy_params_with_info(&type_info, &params)?;

        let strategy_type = quant_common::types::StrategyType::from_type_name(type_name).ok_or_else(
            || {
                ServiceError::InvalidParameter(format!("Unknown strategy type '{}'", type_name))
            },
        )?;

        let now = chrono::Utc::now();
        let strategy_id = uuid::Uuid::now_v7().to_string();

        let strategy = StrategyParams {
            strategy_id: strategy_id.clone(),
            strategy_name: strategy_name.to_string(),
            strategy_type,
            params,
            enabled,
            max_position,
            max_daily_loss,
            status: StrategyStatus::Draft,
            description,
            tags,
            symbols,
            instance_label,
            created_at: now,
            updated_at: now,
            user_id,
        };

        if !strategy.is_valid() {
            return Err(ServiceError::InvalidParameter(
                "Strategy name must not be empty and max_position must be > 0".into(),
            ));
        }

        repo.insert(&strategy).await?;

        info!(
            strategy_id = %strategy_id,
            type_name = %type_name,
            strategy_name = %strategy_name,
            "Strategy created"
        );
        Ok(strategy_id)
    }

    // ── CRUD ───────────────────────────────────────────────────────────────

    /// List all strategies (unpaginated, no status filter).
    #[instrument(skip_all)]
    pub async fn get_strategies(&self) -> ServiceResult<Vec<StrategyParams>> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let (rows, _total) = repo.find_all(None, None, None, None, 10000, 0).await?;

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
        let (rows, _total) = repo.find_all(None, None, status_filter, None, page_size, offset).await?;

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

        // Fill default values from schema before upsert
        let mut strategy = strategy.clone();
        let type_name = format!("{:?}", strategy.strategy_type);
        if let Ok(type_info) = self.get_strategy_type_info(&type_name) {
            if let serde_json::Value::Object(ref mut map) = strategy.params {
                for p in &type_info.parameters {
                    if !map.contains_key(&p.name) && !p.default.is_null() {
                        map.insert(p.name.clone(), p.default.clone());
                    }
                }
            }
        }

        let existing = repo.find_by_id(&strategy.strategy_id).await?;

        if existing.is_some() {
            repo.update(&strategy).await?;
            info!(strategy_id = %strategy.strategy_id, "Strategy updated");
        } else {
            repo.insert(&strategy).await?;
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

        let updated = repo.update(strategy).await?;

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

        let deleted = repo.delete_by_id(strategy_id).await?;

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

        if let Some(mut params) = repo.find_by_id(strategy_id).await? {
            params.enabled = enabled;
            params.updated_at = chrono::Utc::now();
            repo.update(&params).await?;
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

        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| {
            error!("Strategy '{}' not found", strategy_id);
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let type_str = format!("{:?}", params.strategy_type);

        // 通过注册中心创建策略实例，回退到硬编码
        let strategy: Box<dyn Strategy> = match self.registry.as_ref() {
            Some(reg) if reg.has_type(&type_str) => reg.create(&type_str, params.clone()).await?,
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
        ?;

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

    /// Atomic compare-and-set of strategy status. Returns
    /// `ServiceError::ConcurrentModification` when the row's current status
    /// does not match `expected` (i.e. another writer won the race).
    async fn cas_status(
        &self,
        strategy_id: &str,
        target: StrategyStatus,
        expected: StrategyStatus,
    ) -> ServiceResult<()> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let updated = repo.update_status_if(strategy_id, target, expected, None).await?;
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
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Deployed;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_deploy().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;
        self.cas_status(strategy_id, target, current_status).await?;

        info!(strategy_id = %strategy_id, "Strategy deployed");
        Ok(target)
    }

    /// 启动策略（Deployed → Running）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn start_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Running;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_start().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;
        self.cas_status(strategy_id, target, current_status).await?;

        // Register with scheduler AFTER successful DB update
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
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Archived;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_stop().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;
        self.cas_status(strategy_id, target, current_status).await?;

        // Unregister from scheduler AFTER successful DB update
        if let Some(ref scheduler) = self.scheduler {
            match scheduler.stop_strategy(strategy_id).await {
                Ok(()) => {},
                Err(e) => {
                    warn!("Strategy not found in scheduler (already stopped?): {}: {}", strategy_id, e);
                }
            }
        }

        info!(strategy_id = %strategy_id, "Strategy stopped");
        Ok(target)
    }

    /// 暂停策略（Running → Paused）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn pause_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Paused;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_pause().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;
        self.cas_status(strategy_id, target, current_status).await?;

        // Unregister from scheduler AFTER successful DB update
        if let Some(ref scheduler) = self.scheduler {
            match scheduler.stop_strategy(strategy_id).await {
                Ok(()) => {},
                Err(e) => {
                    warn!("Strategy not found in scheduler (already paused?): {}: {}", strategy_id, e);
                }
            }
        }

        info!(strategy_id = %strategy_id, "Strategy paused");
        Ok(target)
    }

    /// 恢复策略（Paused → Running）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn resume_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Running;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_resume().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;
        self.cas_status(strategy_id, target, current_status).await?;

        // Re-register with scheduler AFTER successful DB update
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
        let params = repo.find_by_id(strategy_id).await?.ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;

        let target = StrategyStatus::Archived;
        let current_status = params.status;
        if !current_status.can_transition_to(target) {
            return Err(ServiceError::InvalidStatusTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", target),
            });
        }

        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        strategy.on_archive().await.map_err(|e| ServiceError::Strategy(e.to_string()))?;
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
    pub async fn get_running_strategies(&self) -> ServiceResult<Vec<quant_common::types::SchedulerTaskInfo>> {
        let scheduler = self.scheduler.as_ref().ok_or_else(|| {
            ServiceError::NotInitialized("Scheduler not initialized".into())
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
        repo.find_all(limit, offset).await.map_err(ServiceError::from)
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
            ?
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromPrimitive;
    use async_trait::async_trait;
    use mockall::mock;
    use quant_common::config::SchedulerConfig;
    use quant_repository::{RepoError, StrategyStats, StrategySummaryRow};

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
            user_id: 0,
            status: Default::default(),
            description: None,
            tags: vec![],
            symbols: vec![],
            instance_label: None,
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

    // ── Mock Strategy Repository ──────────────────────────────────────────────

    mock! {
        pub StrategyRepo {}

        #[async_trait]
        impl StRepo for StrategyRepo {
            #[mockall::concretize]
            async fn find_all(
                &self,
                search: Option<&str>,
                strategy_type: Option<StrategyType>,
                status: Option<StrategyStatus>,
                enabled: Option<bool>,
                limit: i64,
                offset: i64,
            ) -> Result<(Vec<StrategySummaryRow>, i64), RepoError>;

            #[mockall::concretize]
            async fn count(
                &self,
                search: Option<&str>,
                strategy_type: Option<StrategyType>,
                status: Option<StrategyStatus>,
                enabled: Option<bool>,
            ) -> Result<i64, RepoError>;

            #[mockall::concretize]
            async fn find_by_id(&self, strategy_id: &str) -> Result<Option<StrategyParams>, RepoError>;

            async fn insert(&self, params: &StrategyParams) -> Result<i32, RepoError>;

            async fn update(&self, params: &StrategyParams) -> Result<bool, RepoError>;

            #[mockall::concretize]
            async fn delete_by_id(&self, strategy_id: &str) -> Result<bool, RepoError>;

            #[mockall::concretize]
            async fn update_status(
                &self,
                strategy_id: &str,
                status: StrategyStatus,
                updated_by: Option<&str>,
            ) -> Result<bool, RepoError>;

            #[mockall::concretize]
            async fn update_status_if(
                &self,
                strategy_id: &str,
                new_status: StrategyStatus,
                expected_old_status: StrategyStatus,
                updated_by: Option<&str>,
            ) -> Result<bool, RepoError>;

            async fn stats(&self) -> Result<StrategyStats, RepoError>;
        }
    }

    // ── Convenience Helpers ───────────────────────────────────────────────────

    fn mock_strategy_params(status: StrategyStatus) -> StrategyParams {
        StrategyParams {
            strategy_id: "test_001".to_string(),
            strategy_name: "Test Strategy".to_string(),
            strategy_type: StrategyType::MeanReversion,
            params: serde_json::json!({}),
            enabled: true,
            max_position: Decimal::ZERO,
            max_daily_loss: Decimal::ZERO,
            status,
            description: None,
            tags: vec![],
            symbols: vec![],
            instance_label: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            user_id: 0,
        }
    }

    fn mock_summary_row(id: i32, strategy_id: &str) -> StrategySummaryRow {
        StrategySummaryRow {
            id,
            strategy_id: strategy_id.to_string(),
            strategy_name: "Test Strategy".to_string(),
            strategy_type: "MeanReversion".to_string(),
            params: serde_json::json!({}),
            enabled: true,
            status: "Draft".to_string(),
            max_position: Decimal::ZERO,
            max_daily_loss: Decimal::ZERO,
            description: None,
            instance_label: None,
            tags: serde_json::json!([]),
            symbols: serde_json::json!([]),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            user_id: Some(0),
        }
    }

    fn make_mock_service(repo: MockStrategyRepo, with_scheduler: bool) -> StrategyService {
        let strategy_repo: Arc<dyn StRepo> = Arc::new(repo);
        let scheduler = if with_scheduler {
            Some(Arc::new(StrategyScheduler::new(SchedulerConfig::default())))
        } else {
            None
        };
        StrategyService::new(None, None, None, Some(strategy_repo), scheduler)
    }

    // ── deploy_strategy ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_deploy_strategy_from_backtesting() {
        let mut mock_repo = MockStrategyRepo::new();

        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Backtesting))));
        mock_repo
            .expect_update_status_if()
            .returning(|_, _, _, _| Ok(true));

        let svc = make_mock_service(mock_repo, false);
        let result = svc.deploy_strategy("test_001").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StrategyStatus::Deployed);
    }

    // ── start_strategy ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_start_strategy_from_deployed() {
        let mut mock_repo = MockStrategyRepo::new();

        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Deployed))));
        mock_repo
            .expect_update_status_if()
            .returning(|_, _, _, _| Ok(true));

        let svc = make_mock_service(mock_repo, true);
        let result = svc.start_strategy("test_001").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StrategyStatus::Running);
    }

    // ── stop_strategy ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_stop_strategy_from_running() {
        let mut mock_repo = MockStrategyRepo::new();

        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Running))));
        mock_repo
            .expect_update_status_if()
            .returning(|_, _, _, _| Ok(true));

        let svc = make_mock_service(mock_repo, true);
        let result = svc.stop_strategy("test_001").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StrategyStatus::Archived);
    }

    // ── pause_strategy ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_pause_strategy_from_running() {
        let mut mock_repo = MockStrategyRepo::new();

        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Running))));
        mock_repo
            .expect_update_status_if()
            .returning(|_, _, _, _| Ok(true));

        let svc = make_mock_service(mock_repo, true);
        let result = svc.pause_strategy("test_001").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StrategyStatus::Paused);
    }

    // ── resume_strategy ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_resume_strategy_from_paused() {
        let mut mock_repo = MockStrategyRepo::new();

        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Paused))));
        mock_repo
            .expect_update_status_if()
            .returning(|_, _, _, _| Ok(true));

        let svc = make_mock_service(mock_repo, true);
        let result = svc.resume_strategy("test_001").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StrategyStatus::Running);
    }

    // ── archive_strategy ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_archive_strategy_from_running() {
        let mut mock_repo = MockStrategyRepo::new();

        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Running))));
        mock_repo
            .expect_update_status_if()
            .returning(|_, _, _, _| Ok(true));

        let svc = make_mock_service(mock_repo, false);
        let result = svc.archive_strategy("test_001").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StrategyStatus::Archived);
    }

    // ── Illegal Status Transitions ────────────────────────────────────────────

    #[tokio::test]
    async fn test_deploy_strategy_from_running_rejected() {
        let mut mock_repo = MockStrategyRepo::new();
        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Running))));
        let svc = make_mock_service(mock_repo, false);
        let result = svc.deploy_strategy("test_001").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::InvalidStatusTransition { from, to } => {
                assert_eq!(from, "Running");
                assert_eq!(to, "Deployed");
            }
            other => panic!("Expected InvalidStatusTransition, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_start_strategy_from_draft_rejected() {
        let mut mock_repo = MockStrategyRepo::new();
        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Draft))));
        let svc = make_mock_service(mock_repo, true);
        let result = svc.start_strategy("test_001").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::InvalidStatusTransition { from, to } => {
                assert_eq!(from, "Draft");
                assert_eq!(to, "Running");
            }
            other => panic!("Expected InvalidStatusTransition, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pause_strategy_from_deployed_rejected() {
        let mut mock_repo = MockStrategyRepo::new();
        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Deployed))));
        let svc = make_mock_service(mock_repo, true);
        let result = svc.pause_strategy("test_001").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::InvalidStatusTransition { from, to } => {
                assert_eq!(from, "Deployed");
                assert_eq!(to, "Paused");
            }
            other => panic!("Expected InvalidStatusTransition, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_resume_strategy_from_draft_rejected() {
        let mut mock_repo = MockStrategyRepo::new();
        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Draft))));
        let svc = make_mock_service(mock_repo, true);
        let result = svc.resume_strategy("test_001").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::InvalidStatusTransition { from, to } => {
                assert_eq!(from, "Draft");
                assert_eq!(to, "Running");
            }
            other => panic!("Expected InvalidStatusTransition, got: {:?}", other),
        }
    }

    // ── list_strategies pagination ────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_strategies_invalid_page_zero() {
        let mock_repo = MockStrategyRepo::new();
        let svc = make_mock_service(mock_repo, false);

        let result = svc.list_strategies(None, 0, 20).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::PaginationInvalid { .. } => {}
            other => panic!("Expected PaginationInvalid, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_list_strategies_invalid_page_size_too_large() {
        let mock_repo = MockStrategyRepo::new();
        let svc = make_mock_service(mock_repo, false);

        let result = svc.list_strategies(None, 1, 101).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::PaginationInvalid { .. } => {}
            other => panic!("Expected PaginationInvalid, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_list_strategies_valid_pagination() {
        let mut mock_repo = MockStrategyRepo::new();

        let rows = vec![
            mock_summary_row(1, "strat_001"),
            mock_summary_row(2, "strat_002"),
        ];

        mock_repo
            .expect_find_all()
            .returning(move |_, _, _, _, _, _| Ok((rows.clone(), 2)));

        let svc = make_mock_service(mock_repo, false);
        let result = svc.list_strategies(None, 1, 20).await;

        assert!(result.is_ok());
        let strategies = result.unwrap();
        assert_eq!(strategies.len(), 2);
    }

    #[tokio::test]
    async fn test_list_strategies_page_size_one() {
        let mut mock_repo = MockStrategyRepo::new();

        let rows = vec![mock_summary_row(1, "strat_001")];

        mock_repo
            .expect_find_all()
            .returning(move |_, _, _, _, _, _| Ok((rows.clone(), 1)));

        let svc = make_mock_service(mock_repo, false);
        let result = svc.list_strategies(None, 1, 1).await;

        assert!(result.is_ok());
        let strategies = result.unwrap();
        assert_eq!(strategies.len(), 1);
    }

    #[tokio::test]
    async fn test_list_strategies_page_size_max() {
        let mut mock_repo = MockStrategyRepo::new();

        let rows = (1..=3).map(|i| mock_summary_row(i, &format!("strat_{:03}", i))).collect::<Vec<_>>();

        mock_repo
            .expect_find_all()
            .returning(move |_, _, _, _, _, _| Ok((rows.clone(), 3)));

        let svc = make_mock_service(mock_repo, false);
        let result = svc.list_strategies(None, 1, 100).await;

        assert!(result.is_ok());
        let strategies = result.unwrap();
        assert_eq!(strategies.len(), 3);
    }

    // ── save_strategy ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_save_strategy_existing_calls_update() {
        let mut mock_repo = MockStrategyRepo::new();

        let existing = mock_strategy_params(StrategyStatus::Draft);
        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Draft))));
        mock_repo.expect_update().returning(|_| Ok(true));

        let svc = make_mock_service(mock_repo, false);
        let result = svc.save_strategy(&existing).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_001");
    }

    #[tokio::test]
    async fn test_save_strategy_new_calls_insert() {
        let mut mock_repo = MockStrategyRepo::new();

        let new_strategy = mock_strategy_params(StrategyStatus::Draft);
        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(|_| Ok(None));
        mock_repo.expect_insert().returning(|_| Ok(1));

        let svc = make_mock_service(mock_repo, false);
        let result = svc.save_strategy(&new_strategy).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_001");
    }

    // ── create_strategy with defaults ──────────────────────────────────────

    #[tokio::test]
    async fn test_create_strategy_defaults_filled() {
        let mut mock_repo = MockStrategyRepo::new();

        // Expect insert with params that have schema defaults filled
        mock_repo
            .expect_insert()
            .withf(|params: &StrategyParams| {
                params.params.get("lookback_period") == Some(&serde_json::json!(20))
                    && params.params.get("entry_threshold") == Some(&serde_json::json!(2.0))
                    && params.params.get("exit_threshold") == Some(&serde_json::json!(0.5))
            })
            .returning(|_| Ok(1));

        let mut svc = make_mock_service(mock_repo, false);
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let result = svc
            .create_strategy(
                "MeanReversion",
                "Test Strategy",
                serde_json::json!({}),
                true,
                Decimal::from(10000),
                Decimal::from(500),
                None,
                None,
                vec![],
                vec!["BTC/USDT".to_string()],
                1,
            )
            .await;

        assert!(result.is_ok());
    }

    // ── save_strategy update path with defaults ─────────────────────────────────

    #[tokio::test]
    async fn test_save_strategy_update_fills_defaults() {
        let mut mock_repo = MockStrategyRepo::new();

        // Existing strategy with empty params
        let existing = StrategyParams {
            strategy_id: "test_001".to_string(),
            strategy_name: "Test".to_string(),
            strategy_type: StrategyType::MeanReversion,
            params: serde_json::json!({}),
            enabled: true,
            max_position: Decimal::from(10000),
            max_daily_loss: Decimal::from(500),
            status: StrategyStatus::Draft,
            description: None,
            tags: vec![],
            symbols: vec![],
            instance_label: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            user_id: 0,
        };

        let existing_for_mock = existing.clone();
        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(move |_| Ok(Some(existing_for_mock.clone())));

        // The update should receive params with schema defaults filled
        mock_repo
            .expect_update()
            .withf(|params: &StrategyParams| {
                params.params.get("lookback_period") == Some(&serde_json::json!(20))
                    && params.params.get("entry_threshold") == Some(&serde_json::json!(2.0))
                    && params.params.get("exit_threshold") == Some(&serde_json::json!(0.5))
            })
            .returning(|_| Ok(true));

        let mut svc = make_mock_service(mock_repo, false);
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let result = svc.save_strategy(&existing).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_001");
    }

    #[tokio::test]
    async fn test_save_strategy_update_preserves_existing_params() {
        let mut mock_repo = MockStrategyRepo::new();

        // Existing strategy with some params already set
        let existing = StrategyParams {
            strategy_id: "test_001".to_string(),
            strategy_name: "Test".to_string(),
            strategy_type: StrategyType::MeanReversion,
            params: serde_json::json!({
                "lookback_period": 50,
                "entry_threshold": 3.0,
            }),
            enabled: true,
            max_position: Decimal::from(10000),
            max_daily_loss: Decimal::from(500),
            status: StrategyStatus::Draft,
            description: None,
            tags: vec![],
            symbols: vec![],
            instance_label: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            user_id: 0,
        };

        let existing_for_mock = existing.clone();
        mock_repo
            .expect_find_by_id()
            .withf(|s: &str| s == "test_001")
            .returning(move |_| Ok(Some(existing_for_mock.clone())));

        // Should keep user-provided values and only fill missing ones (exit_threshold)
        mock_repo
            .expect_update()
            .withf(|params: &StrategyParams| {
                params.params.get("lookback_period") == Some(&serde_json::json!(50))
                    && params.params.get("entry_threshold") == Some(&serde_json::json!(3.0))
                    && params.params.get("exit_threshold") == Some(&serde_json::json!(0.5))
            })
            .returning(|_| Ok(true));

        let mut svc = make_mock_service(mock_repo, false);
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let result = svc.save_strategy(&existing).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_001");
    }

    // ── get_strategy_type_info ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_strategy_type_info_returns_metadata() {
        let mut svc = make_service_no_db();
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let result = svc.get_strategy_type_info("MeanReversion");
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.type_name, "MeanReversion");
        assert!(!info.parameters.is_empty());

        // Verify specific schema fields
        let lookback = info.parameters.iter().find(|p| p.name == "lookback_period");
        assert!(lookback.is_some());
        assert_eq!(lookback.unwrap().default, serde_json::json!(20));
    }

    #[tokio::test]
    async fn test_get_strategy_type_info_unknown_type() {
        let mut svc = make_service_no_db();
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let result = svc.get_strategy_type_info("NonExistentType");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::NotFound(_)));
    }

    // ── validate_strategy_params ──────────────────────────────────────────

    #[tokio::test]
    async fn test_validate_params_rejects_out_of_range() {
        let mut svc = make_service_no_db();
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let params = serde_json::json!({
            "lookback_period": 999,
            "entry_threshold": 2.0,
            "exit_threshold": 0.5,
        });
        let result = svc.validate_strategy_params("MeanReversion", &params);
        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::InvalidParameter(msg) => {
                assert!(msg.contains("out of range"), "Expected range error, got: {}", msg);
            }
            other => panic!("Expected InvalidParameter, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validate_params_rejects_wrong_type() {
        let mut svc = make_service_no_db();
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let params = serde_json::json!({
            "lookback_period": "not-a-number",
            "entry_threshold": 2.0,
            "exit_threshold": 0.5,
        });
        let result = svc.validate_strategy_params("MeanReversion", &params);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_params_valid_params_pass() {
        let mut svc = make_service_no_db();
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let params = serde_json::json!({
            "lookback_period": 20,
            "entry_threshold": 2.0,
            "exit_threshold": 0.5,
        });
        let result = svc.validate_strategy_params("MeanReversion", &params);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_params_missing_required_field() {
        let mut svc = make_service_no_db();
        let registry = Arc::new(strategy_engine::registry::default_registry());
        svc.set_registry(registry);

        let params = serde_json::json!({
            "lookback_period": 20,
        });
        let result = svc.validate_strategy_params("MeanReversion", &params);
        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::InvalidParameter(msg) => {
                assert!(msg.contains("Missing required parameter"), "Expected missing param error, got: {}", msg);
            }
            other => panic!("Expected InvalidParameter, got: {:?}", other),
        }
    }

    // ── Concurrent Lifecycle (PR2: TOCTOU fix) ────────────────────────────────

    #[tokio::test]
    async fn test_update_status_if_returns_false_on_condition_mismatch() {
        // CAS returning false must surface ConcurrentModification (not InvalidStatusTransition)
        let mut mock_repo = MockStrategyRepo::new();
        mock_repo
            .expect_find_by_id()
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Backtesting))));
        mock_repo
            .expect_update_status_if()
            .withf(|_id, _new, expected, _by| *expected == StrategyStatus::Backtesting)
            .returning(|_, _, _, _| Ok(false));

        let svc = make_mock_service(mock_repo, false);
        let result = svc.deploy_strategy("test_001").await;

        assert!(matches!(
            result.unwrap_err(),
            ServiceError::ConcurrentModification { ref strategy_id, expected: StrategyStatus::Backtesting }
            if strategy_id == "test_001"
        ));
    }

    #[tokio::test]
    async fn test_deploy_strategy_concurrent_ac_a_simultaneous() {
        // AC-A (同时到达): tokio::join! two deploy requests; the shared mock
        // atomically grants "first writer wins" so one gets rows=1, the other rows=0.
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let wins = Arc::new(AtomicUsize::new(0));
        let wins_clone = wins.clone();
        let mut mock_repo = MockStrategyRepo::new();
        mock_repo
            .expect_find_by_id()
            .times(2)
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Backtesting))));
        mock_repo
            .expect_update_status_if()
            .times(2)
            .returning(move |_, _, _, _| Ok(wins_clone.fetch_add(1, Ordering::SeqCst) == 0));

        let svc = Arc::new(make_mock_service(mock_repo, false));
        let svc_a = svc.clone();
        let svc_b = svc.clone();
        let (res_a, res_b) = tokio::join!(
            async move { svc_a.deploy_strategy("test_001").await },
            async move { svc_b.deploy_strategy("test_001").await },
        );
        let results = [&res_a, &res_b];
        let oks = results.iter().filter(|r| r.is_ok()).count();
        let conflicts = results.iter().filter(|r| matches!(r, Err(ServiceError::ConcurrentModification { .. }))).count();
        assert_eq!(oks, 1, "results: {:?} / {:?}", res_a, res_b);
        assert_eq!(conflicts, 1, "results: {:?} / {:?}", res_a, res_b);
    }

    #[tokio::test]
    async fn test_deploy_strategy_concurrent_ac_b_sequential() {
        // AC-B (顺序到达): first request moved the row to Deployed; the second
        // reads Deployed, finds Deployed→Deployed is not valid, and surfaces
        // InvalidStatusTransition (state moved on, not a race loss).
        let mut mock_repo = MockStrategyRepo::new();
        mock_repo
            .expect_find_by_id()
            .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Deployed))));

        let svc = make_mock_service(mock_repo, false);
        let result = svc.deploy_strategy("test_001").await;

        assert!(matches!(
            result.unwrap_err(),
            ServiceError::InvalidStatusTransition { ref from, ref to }
            if from == "Deployed" && to == "Deployed"
        ));
    }
}
