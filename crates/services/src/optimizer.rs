use crate::error::{ServiceError, ServiceResult};
use async_trait::async_trait;
use quant_common::config::ParamOptimizerConfig;
use quant_common::types::{BacktestResult, MarketData, StrategyParams};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use std::sync::Arc;
use strategy_engine::{run_backtest_multi, MultiStrategyResult, StrategyRegistry};
use tracing::{error, instrument};

/// 参数组合
#[derive(Debug, Clone)]
pub struct ParameterCombo {
    pub label: String,
    pub params: serde_json::Value,
    pub result: Option<BacktestResult>,
}

/// 参数优化结果
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub config: ParamOptimizerConfig,
    pub combinations: Vec<ParameterCombo>,
    pub best: Option<ParameterCombo>,
    pub total_combinations: usize,
}

#[async_trait]
pub trait SearchAlgorithm: Send + Sync {
    fn name(&self) -> &str;

    async fn search(
        &self,
        registry: Arc<StrategyRegistry>,
        strategy_type: &str,
        param_grid: Vec<serde_json::Value>,
        market_data: Vec<MarketData>,
        initial_capital: Decimal,
        commission_rate: Decimal,
        slippage: Decimal,
    ) -> ServiceResult<OptimizationResult>;
}

pub struct GridSearch;

#[async_trait]
impl SearchAlgorithm for GridSearch {
    fn name(&self) -> &str {
        "grid_search"
    }

    async fn search(
        &self,
        registry: Arc<StrategyRegistry>,
        strategy_type: &str,
        param_grid: Vec<serde_json::Value>,
        market_data: Vec<MarketData>,
        initial_capital: Decimal,
        commission_rate: Decimal,
        slippage: Decimal,
    ) -> ServiceResult<OptimizationResult> {
        let total = param_grid.len();
        if total == 0 {
            return Err(ServiceError::InvalidParameter("Parameter grid is empty".to_string()));
        }

        let parsed_type = parse_strategy_type(strategy_type)?;
        let mut strategies = Vec::new();
        for (i, params) in param_grid.iter().enumerate() {
            let sp = StrategyParams {
                strategy_id: format!("opt-{}-{}", strategy_type, i),
                strategy_name: format!("{}-opt-{}", strategy_type, i),
                strategy_type: parsed_type.clone(),
                params: params.clone(),
                enabled: true,
                max_position: initial_capital,
                max_daily_loss: initial_capital * Decimal::from_f64(0.1).unwrap_or(Decimal::ZERO),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                status: Default::default(),
                description: None,
                tags: vec![],
                symbols: vec![],
                instance_label: None,
                user_id: 0,
                version: 0,
            };

            let strategy = match registry.create(strategy_type, sp.clone()).await {
                Ok(s) => s,
                Err(e) => {
                    error!(combo = %i, error = %e, "Failed to create strategy");
                    continue;
                }
            };

            strategies.push((strategy, format!("combo-{}", i)));
        }

        let multi_result: MultiStrategyResult = run_backtest_multi(
            strategies,
            market_data,
            initial_capital,
            commission_rate,
            slippage,
        )
        .await
        .map_err(|e| ServiceError::Other(format!("Multi-backtest failed: {}", e)))?;

        let mut combinations: Vec<ParameterCombo> = multi_result
            .results
            .into_iter()
            .map(|(label, result)| {
                let idx: usize = label
                    .strip_prefix("combo-")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let params = param_grid.get(idx).cloned().unwrap_or(serde_json::json!({}));
                ParameterCombo {
                    label,
                    params,
                    result: Some(result),
                }
            })
            .collect();

        combinations.sort_by(|a, b| {
            let a_sharpe = a.result.as_ref().map(|r| r.sharpe_ratio).unwrap_or(Decimal::ZERO);
            let b_sharpe = b.result.as_ref().map(|r| r.sharpe_ratio).unwrap_or(Decimal::ZERO);
            b_sharpe.cmp(&a_sharpe)
        });

        let best = combinations.first().cloned();

        Ok(OptimizationResult {
            config: ParamOptimizerConfig::default(),
            combinations,
            best,
            total_combinations: total,
        })
    }
}

pub struct BayesianOptimization;

#[async_trait]
impl SearchAlgorithm for BayesianOptimization {
    fn name(&self) -> &str {
        "bayesian_optimization"
    }

    async fn search(
        &self,
        _registry: Arc<StrategyRegistry>,
        _strategy_type: &str,
        param_grid: Vec<serde_json::Value>,
        _market_data: Vec<MarketData>,
        _initial_capital: Decimal,
        _commission_rate: Decimal,
        _slippage: Decimal,
    ) -> ServiceResult<OptimizationResult> {
        let total = param_grid.len();
        if total == 0 {
            return Err(ServiceError::InvalidParameter("Parameter grid is empty".to_string()));
        }

        // Baseline stub: pick a random sample and return empty results.
        // Full Bayesian Optimization implementation is future work.
        Ok(OptimizationResult {
            config: ParamOptimizerConfig::default(),
            combinations: Vec::new(),
            best: None,
            total_combinations: total,
        })
    }
}

pub struct GeneticAlgorithm;

#[async_trait]
impl SearchAlgorithm for GeneticAlgorithm {
    fn name(&self) -> &str {
        "genetic_algorithm"
    }

    async fn search(
        &self,
        _registry: Arc<StrategyRegistry>,
        _strategy_type: &str,
        param_grid: Vec<serde_json::Value>,
        _market_data: Vec<MarketData>,
        _initial_capital: Decimal,
        _commission_rate: Decimal,
        _slippage: Decimal,
    ) -> ServiceResult<OptimizationResult> {
        let total = param_grid.len();
        if total == 0 {
            return Err(ServiceError::InvalidParameter("Parameter grid is empty".to_string()));
        }

        // Stub: full Genetic Algorithm implementation is future work.
        Ok(OptimizationResult {
            config: ParamOptimizerConfig::default(),
            combinations: Vec::new(),
            best: None,
            total_combinations: total,
        })
    }
}

fn parse_strategy_type(s: &str) -> ServiceResult<quant_common::types::StrategyType> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|e| {
        ServiceError::InvalidParameter(format!("Unknown strategy type '{s}': {e}"))
    })
}

/// 参数优化器
pub struct ParamOptimizer {
    registry: Arc<StrategyRegistry>,
    config: ParamOptimizerConfig,
    pub algorithm: Box<dyn SearchAlgorithm>,
}

impl ParamOptimizer {
    pub fn new(registry: Arc<StrategyRegistry>, config: ParamOptimizerConfig) -> Self {
        Self {
            registry: registry.clone(),
            config: config.clone(),
            algorithm: Box::new(GridSearch),
        }
    }

    pub fn with_algorithm(
        registry: Arc<StrategyRegistry>,
        config: ParamOptimizerConfig,
        algorithm: Box<dyn SearchAlgorithm>,
    ) -> Self {
        Self { registry, config, algorithm }
    }

    #[instrument(skip(self, market_data), fields(strategy_type = %strategy_type, combos = %param_grid.len()))]
    pub async fn optimize(
        &self,
        strategy_type: &str,
        param_grid: Vec<serde_json::Value>,
        market_data: Vec<MarketData>,
        initial_capital: Decimal,
        commission_rate: Decimal,
        slippage: Decimal,
    ) -> ServiceResult<OptimizationResult> {
        let total = param_grid.len();
        if total == 0 {
            return Err(ServiceError::InvalidParameter("Parameter grid is empty".to_string()));
        }

        let max_iter = self.config.max_iterations as usize;
        if total > max_iter {
            return Err(ServiceError::InvalidParameter(format!(
                "Parameter grid has {} combinations, max is {}",
                total, max_iter
            )));
        }

        self.algorithm
            .search(
                self.registry.clone(),
                strategy_type,
                param_grid,
                market_data,
                initial_capital,
                commission_rate,
                slippage,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strategy_engine::default_registry;

    fn make_optimizer() -> ParamOptimizer {
        let registry = Arc::new(default_registry());
        let config = ParamOptimizerConfig {
            enabled: true,
            max_iterations: 10,
            timeout_secs: 60,
            parallel_jobs: 4,
        };
        ParamOptimizer::new(registry, config)
    }

    fn sample_market_data() -> Vec<MarketData> {
        let now = chrono::Utc::now();
        (0..5)
            .map(|i| MarketData {
                timestamp: now + chrono::Duration::hours(i),
                symbol: "BTC/USDT".to_string(),
                open: rust_decimal::Decimal::new(100 + i * 10, 0),
                high: rust_decimal::Decimal::new(100 + i * 10 + 5, 0),
                low: rust_decimal::Decimal::new(100 + i * 10 - 5, 0),
                close: rust_decimal::Decimal::new(100 + i * 10, 0),
                volume: rust_decimal::Decimal::new(1000, 0),
                turnover: rust_decimal::Decimal::ZERO,
                open_interest: None,
                bid_prices: vec![],
                bid_volumes: vec![],
                ask_prices: vec![],
                ask_volumes: vec![],
            })
            .collect()
    }

    #[tokio::test]
    async fn test_optimizer_empty_grid_returns_error() {
        let optimizer = make_optimizer();
        let result = optimizer
            .optimize(
                "mean_reversion",
                vec![],
                sample_market_data(),
                rust_decimal::Decimal::new(10000, 0),
                rust_decimal::Decimal::ZERO,
                rust_decimal::Decimal::ZERO,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_optimizer_creates_combinations() {
        let optimizer = make_optimizer();
        let grid = vec![
            serde_json::json!({"rsi_period": 14, "entry_threshold": 30}),
            serde_json::json!({"rsi_period": 7, "entry_threshold": 25}),
        ];
        let result = optimizer
            .optimize(
                "MeanReversion",
                grid,
                sample_market_data(),
                rust_decimal::Decimal::new(10000, 0),
                rust_decimal::Decimal::ZERO,
                rust_decimal::Decimal::ZERO,
            )
            .await;
        if let Err(ref e) = result {
            eprintln!("Optimizer error: {:?}", e);
        }
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let opt_result = result.unwrap();
        assert_eq!(opt_result.total_combinations, 2);
        assert!(opt_result.best.is_some());
    }

    /// P0-3: optimizer must dispatch by the `strategy_type: &str` parameter
    /// rather than hardcoding `StrategyType::MeanReversion`. Register a mock
    /// `TrendFollowing` factory and verify the optimizer routes through it.
    #[tokio::test]
    async fn test_optimizer_dispatches_by_dynamic_strategy_type() {
        use strategy_engine::registry::{FactoryError, MeanReversionFactory, StrategyFactory};
        use strategy_engine::strategy::{MeanReversionStrategy, Strategy};

        struct TrendFollowingMockFactory;

        #[async_trait::async_trait]
        impl StrategyFactory for TrendFollowingMockFactory {
            async fn create(
                &self,
                _params: quant_common::types::StrategyParams,
            ) -> Result<Box<dyn Strategy>, FactoryError> {
                let s = MeanReversionStrategy::new();
                Ok(Box::new(s))
            }

            fn parameter_schema(&self) -> Vec<quant_common::types::ParameterSchema> {
                Vec::new()
            }
        }

        let mut registry = strategy_engine::registry::StrategyRegistry::new();
        registry.register(
            "TrendFollowing",
            Box::new(TrendFollowingMockFactory),
            "趋势跟随",
            "mock trend-following factory",
        );
        registry.register(
            "MeanReversion",
            Box::new(MeanReversionFactory),
            "均值回归",
            "mr",
        );

        let config = ParamOptimizerConfig {
            enabled: true,
            max_iterations: 10,
            timeout_secs: 60,
            parallel_jobs: 4,
        };
        let optimizer = ParamOptimizer::new(Arc::new(registry), config);

        let grid = vec![serde_json::json!({"fast": 10, "slow": 30})];
        let result = optimizer
            .optimize(
                "TrendFollowing",
                grid,
                sample_market_data(),
                rust_decimal::Decimal::new(10000, 0),
                rust_decimal::Decimal::ZERO,
                rust_decimal::Decimal::ZERO,
            )
            .await;
        if let Err(ref e) = result {
            eprintln!("Optimizer error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "optimizer must dispatch to TrendFollowing factory, got: {:?}",
            result.err()
        );
        let opt_result = result.unwrap();
        assert_eq!(opt_result.total_combinations, 1);
        assert!(opt_result.best.is_some());

        let result_mr = optimizer
            .optimize(
                "MeanReversion",
                vec![serde_json::json!({"lookback_period": 10})],
                sample_market_data(),
                rust_decimal::Decimal::new(10000, 0),
                rust_decimal::Decimal::ZERO,
                rust_decimal::Decimal::ZERO,
            )
            .await;
        assert!(result_mr.is_ok());
    }
}
