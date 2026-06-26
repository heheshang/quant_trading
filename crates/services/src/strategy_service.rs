use crate::error::{ServiceError, ServiceResult};
use crate::market_data_provider::MarketDataProvider;
use quant_common::types::{
    BacktestResult, StrategyParams, StrategyStatus, StrategyType,
};
use quant_repository::{BacktestRepository, BacktestResultSummaryRow, PostgresClient};
use rust_decimal::Decimal;
use sqlx::Row;
use std::sync::Arc;
use std::time::Instant;
use strategy_engine::registry::StrategyRegistry;
use strategy_engine::{BacktestEngine, Strategy};
use tracing::{error, info, instrument, warn};

/// 策略服务 — 管理策略注册、生命周期、回测与调度
pub struct StrategyService {
    postgres: Option<Arc<PostgresClient>>,
    market_data_provider: Option<Arc<dyn MarketDataProvider>>,
    backtest_repo: Option<Arc<dyn BacktestRepository>>,
    registry: Option<Arc<StrategyRegistry>>,
}

impl StrategyService {
    /// 创建 StrategyService（registry 可选，注入后启用注册中心功能）
    pub fn new(
        postgres: Option<Arc<PostgresClient>>,
        market_data_provider: Option<Arc<dyn MarketDataProvider>>,
        backtest_repo: Option<Arc<dyn BacktestRepository>>,
    ) -> Self {
        Self {
            postgres,
            market_data_provider,
            backtest_repo,
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

    #[instrument(skip_all)]
    pub async fn get_strategies(&self) -> ServiceResult<Vec<StrategyParams>> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let rows = sqlx::query(
            r#"
            SELECT strategy_id, strategy_name, strategy_type, params,
                   enabled, max_position, max_daily_loss, created_at, updated_at
            FROM strategies
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(ServiceError::from)?;

        let strategies: Vec<StrategyParams> = rows
            .iter()
            .map(|row| {
                let ty_str: String = row.get("strategy_type");
                let strategy_type: StrategyType =
                    serde_json::from_value(serde_json::Value::String(ty_str))
                        .unwrap_or(StrategyType::MeanReversion);
                StrategyParams {
                    strategy_id: row.get("strategy_id"),
                    strategy_name: row.get("strategy_name"),
                    strategy_type,
                    params: row.get("params"),
                    enabled: row.get("enabled"),
                    max_position: row.get("max_position"),
                    max_daily_loss: row.get("max_daily_loss"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }
            })
            .collect();

        info!(count = strategies.len(), "Strategies retrieved");
        Ok(strategies)
    }

    #[instrument(skip(self, strategy), fields(strategy_id = %strategy.strategy_id))]
    pub async fn save_strategy(&self, strategy: &StrategyParams) -> ServiceResult<String> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let type_str = serde_json::to_value(&strategy.strategy_type)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "MeanReversion".to_string());

        sqlx::query(
            r#"
            INSERT INTO strategies (strategy_id, strategy_name, strategy_type, params,
                                    enabled, max_position, max_daily_loss, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (strategy_id) DO UPDATE SET
                strategy_name = EXCLUDED.strategy_name,
                strategy_type = EXCLUDED.strategy_type,
                params = EXCLUDED.params,
                enabled = EXCLUDED.enabled,
                max_position = EXCLUDED.max_position,
                max_daily_loss = EXCLUDED.max_daily_loss,
                updated_at = NOW()
            "#,
        )
        .bind(&strategy.strategy_id)
        .bind(&strategy.strategy_name)
        .bind(&type_str)
        .bind(&strategy.params)
        .bind(strategy.enabled)
        .bind(strategy.max_position)
        .bind(strategy.max_daily_loss)
        .bind(strategy.created_at)
        .bind(strategy.updated_at)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to save strategy {}: {}", strategy.strategy_id, e);
            ServiceError::from(e)
        })?;

        info!(strategy_id = %strategy.strategy_id, strategy_name = %strategy.strategy_name, "Strategy saved");
        Ok(strategy.strategy_id.clone())
    }

    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn delete_strategy(&self, strategy_id: &str) -> ServiceResult<bool> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let affected = sqlx::query("DELETE FROM strategies WHERE strategy_id = $1")
            .bind(strategy_id)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("Failed to delete strategy {}: {}", strategy_id, e);
                ServiceError::from(e)
            })?;

        let deleted = affected.rows_affected() > 0;
        if deleted {
            info!(strategy_id = %strategy_id, "Strategy deleted");
        } else {
            warn!(strategy_id = %strategy_id, "Strategy not found for deletion");
        }
        Ok(deleted)
    }

    #[instrument(skip(self), fields(strategy_id = %strategy_id, enabled))]
    pub async fn toggle_strategy(&self, strategy_id: &str, enabled: bool) -> ServiceResult<bool> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let affected = sqlx::query(
            "UPDATE strategies SET enabled = $1, updated_at = NOW() WHERE strategy_id = $2",
        )
        .bind(enabled)
        .bind(strategy_id)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to toggle strategy {}: {}", strategy_id, e);
            ServiceError::from(e)
        })?;

        let toggled = affected.rows_affected() > 0;
        info!(strategy_id = %strategy_id, enabled, "Strategy toggled");
        Ok(toggled)
    }

    // ── Backtest ────────────────────────────────────────────────────────────

    /// 从数据库读取策略参数并构建策略实例（通过注册中心）
    async fn build_strategy_from_db(
        &self,
        strategy_id: &str,
    ) -> ServiceResult<(String, Box<dyn Strategy>, StrategyParams)> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let row = sqlx::query(
            r#"
            SELECT strategy_type, params, strategy_name, max_position, max_daily_loss
            FROM strategies
            WHERE strategy_id = $1
            "#,
        )
        .bind(strategy_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch strategy {}: {}", strategy_id, e);
            ServiceError::from(e)
        })?
        .ok_or_else(|| {
            error!("Strategy '{}' not found", strategy_id);
            ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id))
        })?;

        let db_type: String = row.get("strategy_type");
        let db_params: serde_json::Value = row.get("params");
        let db_name: String = row.get("strategy_name");
        let db_max_pos: Decimal = row.get("max_position");
        let db_max_loss: Decimal = row.get("max_daily_loss");

        let strategy_type: StrategyType =
            serde_json::from_value(serde_json::Value::String(db_type.clone()))
                .unwrap_or(StrategyType::MeanReversion);

        let params = StrategyParams {
            strategy_id: strategy_id.to_string(),
            strategy_name: db_name,
            strategy_type: strategy_type.clone(),
            params: db_params,
            enabled: true,
            max_position: db_max_pos,
            max_daily_loss: db_max_loss,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // 通过注册中心创建策略实例，回退到硬编码
        let strategy: Box<dyn Strategy> = match self.registry.as_ref() {
            Some(reg) if reg.has_type(&db_type) => {
                reg.create(&db_type, params.clone()).await.map_err(|e| {
                    ServiceError::Strategy(format!("Failed to create strategy '{}': {}", db_type, e))
                })?
            }
            _ => {
                // 回退：仅支持 MeanReversion
                if strategy_type != StrategyType::MeanReversion {
                    return Err(ServiceError::Strategy(format!(
                        "Strategy type '{:?}' is not supported. Registry not initialized or type not registered.",
                        strategy_type
                    )));
                }
                let mut s = strategy_engine::strategy::MeanReversionStrategy::new();
                s.initialize(params.clone())
                    .await
                    .map_err(|e| ServiceError::Strategy(e.to_string()))?;
                Box::new(s)
            }
        };

        Ok((db_type, strategy, params))
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
        let (_db_type, mut strategy, _params) = self.build_strategy_from_db(strategy_id).await?;
        strategy
            .on_deploy()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        info!(strategy_id = %strategy_id, "Strategy deployed");
        // 注意：状态持久化由调用侧（Tauri command）写到 DB status 字段
        Ok(StrategyStatus::Deployed)
    }

    /// 启动策略（Deployed → Running）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn start_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let (_db_type, mut strategy, _params) = self.build_strategy_from_db(strategy_id).await?;
        strategy
            .on_start()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        info!(strategy_id = %strategy_id, "Strategy started");
        Ok(StrategyStatus::Running)
    }

    /// 停止策略（Running → Stopped）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn stop_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let (_db_type, mut strategy, _params) = self.build_strategy_from_db(strategy_id).await?;
        strategy
            .on_stop()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        info!(strategy_id = %strategy_id, "Strategy stopped");
        Ok(StrategyStatus::Archived)
    }

    /// 暂停策略（Running → Paused）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn pause_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let (_db_type, mut strategy, _params) = self.build_strategy_from_db(strategy_id).await?;
        strategy
            .on_pause()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        info!(strategy_id = %strategy_id, "Strategy paused");
        Ok(StrategyStatus::Paused)
    }

    /// 恢复策略（Paused → Running）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn resume_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let (_db_type, mut strategy, _params) = self.build_strategy_from_db(strategy_id).await?;
        strategy
            .on_resume()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        info!(strategy_id = %strategy_id, "Strategy resumed");
        Ok(StrategyStatus::Running)
    }

    /// 归档策略（任何状态 → Archived）
    #[instrument(skip(self), fields(strategy_id = %strategy_id))]
    pub async fn archive_strategy(&self, strategy_id: &str) -> ServiceResult<StrategyStatus> {
        let (_db_type, mut strategy, _params) = self.build_strategy_from_db(strategy_id).await?;
        strategy
            .on_archive()
            .await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        info!(strategy_id = %strategy_id, "Strategy archived");
        Ok(StrategyStatus::Archived)
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
        StrategyService::new(None, None, None)
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
