use crate::error::{ServiceError, ServiceResult};
use crate::market_data_provider::MarketDataProvider;
use quant_common::types::{
    ParamType, ParameterSchema, StrategyParams, StrategyStatus, StrategyType,
};
use quant_repository::StrategyRepository as StRepo;
use quant_repository::{BacktestRepository, PostgresClient};
use rust_decimal::Decimal;
use std::sync::Arc;
use strategy_engine::registry::StrategyRegistry;
use strategy_engine::scheduler::StrategyScheduler;
use strategy_engine::Strategy;
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
    pub fn list_strategy_types(
        &self,
    ) -> ServiceResult<Vec<strategy_engine::registry::StrategyTypeInfo>> {
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
        reg.get_type_info(type_name)
            .ok_or_else(|| ServiceError::NotFound(format!("Unknown strategy type '{}'", type_name)))
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

        let strategy_type = quant_common::types::StrategyType::from_type_name(type_name)
            .ok_or_else(|| {
                ServiceError::InvalidParameter(format!("Unknown strategy type '{}'", type_name))
            })?;

        let strategy_id = uuid::Uuid::now_v7().to_string();

        let strategy = StrategyParams::builder(
            strategy_id.clone(),
            strategy_name.to_string(),
            strategy_type,
        )
        .params(params)
        .enabled(enabled)
        .max_position(max_position)
        .max_daily_loss(max_daily_loss)
        .description(description)
        .tags(tags)
        .symbols(symbols)
        .instance_label(instance_label)
        .user_id(user_id)
        .build();

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

        let strategies: Vec<StrategyParams> = rows
            .iter()
            .filter_map(|row| match row.to_domain() {
                Ok(p) => Some(p),
                Err(e) => {
                    error!("Failed to convert strategy row {}: {}", row.strategy_id, e);
                    None
                }
            })
            .collect();
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
                reason: format!(
                    "Page must be >= 1, page_size must be 1-100 (got page={}, page_size={})",
                    page, page_size
                ),
            });
        }
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let offset = (page - 1) * page_size;
        let (rows, _total) = repo
            .find_all(None, None, status_filter, None, page_size, offset)
            .await?;

        let strategies: Vec<StrategyParams> = rows
            .iter()
            .filter_map(|row| match row.to_domain() {
                Ok(p) => Some(p),
                Err(e) => {
                    error!("Failed to convert strategy row {}: {}", row.strategy_id, e);
                    None
                }
            })
            .collect();
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

    /// Update an existing strategy. Uses optimistic locking via `update_with_version`.
    /// Returns the strategy_id on success, or `ServiceError::Conflict` on version mismatch.
    #[instrument(skip(self, strategy), fields(strategy_id = %strategy.strategy_id))]
    pub async fn update_strategy(&self, strategy: &StrategyParams) -> ServiceResult<String> {
        let repo = self
            .strategy_repo
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;

        let current = repo
            .find_by_id(&strategy.strategy_id)
            .await?
            .ok_or_else(|| {
                ServiceError::NotFound(format!("Strategy '{}' not found", strategy.strategy_id))
            })?;

        let expected_version = current.version;
        let updated = repo
            .update_with_version(&strategy.strategy_id, strategy, expected_version)
            .await?;

        if !updated {
            return Err(ServiceError::Conflict(format!(
                "Strategy '{}' was modified by another session (version {})",
                strategy.strategy_id, expected_version
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
}

mod lifecycle;

#[cfg(test)]
mod tests;
