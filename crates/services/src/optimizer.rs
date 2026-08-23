use crate::error::{ServiceError, ServiceResult};
use async_trait::async_trait;
use quant_common::config::ParamOptimizerConfig;
use quant_common::types::{BacktestResult, MarketData, StrategyParams};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use std::sync::Arc;
use strategy_engine::{run_backtest_multi, MultiStrategyResult, StrategyRegistry};
use tracing::{error, instrument};

/// 参数组合
#[derive(Debug, Clone, Serialize)]
pub struct ParameterCombo {
    pub label: String,
    pub params: serde_json::Value,
    pub result: Option<BacktestResult>,
}

/// 参数优化结果
#[derive(Debug, Clone, Serialize)]
pub struct OptimizationResult {
    pub config: ParamOptimizerConfig,
    pub combinations: Vec<ParameterCombo>,
    pub best: Option<ParameterCombo>,
    pub total_combinations: usize,
}

/// 优化算法（当前仅开放 GridSearch）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationAlgorithm {
    GridSearch,
    Bayesian,
    Genetic,
}

impl OptimizationAlgorithm {
    /// Parse an algorithm name. Unknown names yield `InvalidParameter`.
    pub fn parse(s: &str) -> ServiceResult<Self> {
        match s.to_ascii_lowercase().as_str() {
            "grid_search" | "gridsearch" | "grid" => Ok(Self::GridSearch),
            "bayesian" | "bayesian_optimization" | "bayes" => Ok(Self::Bayesian),
            "genetic" | "genetic_algorithm" | "ga" => Ok(Self::Genetic),
            _ => Err(ServiceError::InvalidParameter(format!(
                "Unknown optimization algorithm '{s}'"
            ))),
        }
    }
}

/// 优化指标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationMetric {
    SharpeRatio,
    AnnualReturn,
    MaxDrawdown,
}

impl OptimizationMetric {
    /// Parse a metric name. Unknown names yield `InvalidParameter`.
    pub fn parse(s: &str) -> ServiceResult<Self> {
        match s.to_ascii_lowercase().as_str() {
            "sharpe_ratio" | "sharpe" => Ok(Self::SharpeRatio),
            "annual_return" => Ok(Self::AnnualReturn),
            "max_drawdown" | "max_dd" => Ok(Self::MaxDrawdown),
            _ => Err(ServiceError::InvalidParameter(format!(
                "Unknown optimization metric '{s}'"
            ))),
        }
    }

    /// Extract the metric value from a backtest result.
    fn value(&self, r: &BacktestResult) -> Decimal {
        match self {
            Self::SharpeRatio => r.sharpe_ratio,
            Self::AnnualReturn => r.annual_return,
            Self::MaxDrawdown => r.max_drawdown,
        }
    }

    /// `true` when a larger value is better, `false` when a smaller value is better.
    fn higher_is_better(&self) -> bool {
        !matches!(self, Self::MaxDrawdown)
    }
}

/// 展开参数网格。
///
/// 接受两种形式：
/// - 对象：`{"rsi_period": [14, 7], "entry_threshold": [30, 25]}` → 笛卡尔积；
/// - 数组：`[{...}, {...}]` → 已是展开后的组合，原样返回。
pub fn expand_grid(grid: &serde_json::Value) -> ServiceResult<Vec<serde_json::Value>> {
    match grid {
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return Err(ServiceError::InvalidParameter(
                    "Parameter grid is empty".to_string(),
                ));
            }
            Ok(items.clone())
        }
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return Err(ServiceError::InvalidParameter(
                    "Parameter grid is empty".to_string(),
                ));
            }
            let mut keys = Vec::new();
            let mut value_lists: Vec<Vec<serde_json::Value>> = Vec::new();
            for (k, v) in map {
                let arr = v.as_array().ok_or_else(|| {
                    ServiceError::InvalidParameter(format!(
                        "Grid value for '{k}' must be an array of candidate values"
                    ))
                })?;
                if arr.is_empty() {
                    return Err(ServiceError::InvalidParameter(format!(
                        "Grid value for '{k}' is empty"
                    )));
                }
                keys.push(k.clone());
                value_lists.push(arr.clone());
            }

            let mut combos: Vec<serde_json::Map<String, serde_json::Value>> =
                vec![serde_json::Map::new()];
            for (k, values) in keys.iter().zip(value_lists.iter()) {
                let mut next = Vec::new();
                for combo in &combos {
                    for v in values {
                        let mut c = combo.clone();
                        c.insert(k.clone(), v.clone());
                        next.push(c);
                    }
                }
                combos = next;
            }
            Ok(combos.into_iter().map(serde_json::Value::Object).collect())
        }
        _ => Err(ServiceError::InvalidParameter(
            "Parameter grid must be an object of arrays or an array of objects".to_string(),
        )),
    }
}

#[async_trait]
pub trait SearchAlgorithm: Send + Sync {
    fn name(&self) -> &str;

    #[allow(clippy::too_many_arguments)]
    async fn search(
        &self,
        registry: Arc<StrategyRegistry>,
        strategy_type: &str,
        param_grid: Vec<serde_json::Value>,
        market_data: Vec<MarketData>,
        initial_capital: Decimal,
        commission_rate: Decimal,
        slippage: Decimal,
        metric: OptimizationMetric,
    ) -> ServiceResult<OptimizationResult>;
}

/// 按指标对结果排序（`None` 结果排到最后）。
fn sort_combinations(combinations: &mut [ParameterCombo], metric: OptimizationMetric) {
    combinations.sort_by(|a, b| match (&a.result, &b.result) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(ra), Some(rb)) => {
            let a_metric = metric.value(ra);
            let b_metric = metric.value(rb);
            if metric.higher_is_better() {
                b_metric.cmp(&a_metric)
            } else {
                a_metric.cmp(&b_metric)
            }
        }
    });
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
        metric: OptimizationMetric,
    ) -> ServiceResult<OptimizationResult> {
        let total = param_grid.len();
        if total == 0 {
            return Err(ServiceError::InvalidParameter(
                "Parameter grid is empty".to_string(),
            ));
        }

        let parsed_type = parse_strategy_type(strategy_type)?;
        let mut strategies = Vec::new();
        for (i, params) in param_grid.iter().enumerate() {
            let sp = StrategyParams::builder(
                format!("opt-{}-{}", strategy_type, i),
                format!("{}-opt-{}", strategy_type, i),
                parsed_type.clone(),
            )
            .params(params.clone())
            .max_position(initial_capital)
            .max_daily_loss(initial_capital * Decimal::from_f64(0.1).unwrap_or(Decimal::ZERO))
            .build();

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
                let params = param_grid
                    .get(idx)
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                ParameterCombo {
                    label,
                    params,
                    result: Some(result),
                }
            })
            .collect();

        sort_combinations(&mut combinations, metric);

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
        _param_grid: Vec<serde_json::Value>,
        _market_data: Vec<MarketData>,
        _initial_capital: Decimal,
        _commission_rate: Decimal,
        _slippage: Decimal,
        _metric: OptimizationMetric,
    ) -> ServiceResult<OptimizationResult> {
        Err(ServiceError::NotImplemented(
            "BayesianOptimization is not implemented; only GridSearch is available".to_string(),
        ))
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
        _param_grid: Vec<serde_json::Value>,
        _market_data: Vec<MarketData>,
        _initial_capital: Decimal,
        _commission_rate: Decimal,
        _slippage: Decimal,
        _metric: OptimizationMetric,
    ) -> ServiceResult<OptimizationResult> {
        Err(ServiceError::NotImplemented(
            "GeneticAlgorithm is not implemented; only GridSearch is available".to_string(),
        ))
    }
}

fn parse_strategy_type(s: &str) -> ServiceResult<quant_common::types::StrategyType> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| ServiceError::InvalidParameter(format!("Unknown strategy type '{s}': {e}")))
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
        Self {
            registry,
            config,
            algorithm,
        }
    }

    /// Set the search algorithm (defaults to GridSearch at construction).
    pub fn set_algorithm(&mut self, algorithm: Box<dyn SearchAlgorithm>) {
        self.algorithm = algorithm;
    }

    #[instrument(skip(self, market_data), fields(strategy_type = %strategy_type, combos = %param_grid.len()))]
    #[allow(clippy::too_many_arguments)]
    pub async fn optimize(
        &self,
        strategy_type: &str,
        param_grid: Vec<serde_json::Value>,
        market_data: Vec<MarketData>,
        initial_capital: Decimal,
        commission_rate: Decimal,
        slippage: Decimal,
        metric: OptimizationMetric,
    ) -> ServiceResult<OptimizationResult> {
        let total = param_grid.len();
        if total == 0 {
            return Err(ServiceError::InvalidParameter(
                "Parameter grid is empty".to_string(),
            ));
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
                metric,
            )
            .await
    }

    /// Run an optimization with a named algorithm.
    ///
    /// Only GridSearch is implemented; Bayesian / Genetic return a clear
    /// `NotImplemented` error (they are intentionally not wired).
    #[allow(clippy::too_many_arguments)]
    pub async fn optimize_with_algorithm(
        &self,
        algorithm: OptimizationAlgorithm,
        strategy_type: &str,
        param_grid: Vec<serde_json::Value>,
        market_data: Vec<MarketData>,
        initial_capital: Decimal,
        commission_rate: Decimal,
        slippage: Decimal,
        metric: OptimizationMetric,
    ) -> ServiceResult<OptimizationResult> {
        if algorithm != OptimizationAlgorithm::GridSearch {
            return Err(ServiceError::NotImplemented(format!(
                "{algorithm:?} is not implemented; only GridSearch is available"
            )));
        }
        self.optimize(
            strategy_type,
            param_grid,
            market_data,
            initial_capital,
            commission_rate,
            slippage,
            metric,
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
                OptimizationMetric::SharpeRatio,
            )
            .await;
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::InvalidParameter(_)
        ));
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
                OptimizationMetric::SharpeRatio,
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
                OptimizationMetric::SharpeRatio,
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
                OptimizationMetric::SharpeRatio,
            )
            .await;
        assert!(result_mr.is_ok());
    }

    #[tokio::test]
    async fn test_optimizer_sorts_by_metric_and_returns_top_n() {
        let optimizer = make_optimizer();
        let grid = vec![
            serde_json::json!({"rsi_period": 14, "entry_threshold": 30}),
            serde_json::json!({"rsi_period": 7, "entry_threshold": 25}),
            serde_json::json!({"rsi_period": 9, "entry_threshold": 20}),
        ];
        let result = optimizer
            .optimize(
                "MeanReversion",
                grid,
                sample_market_data(),
                rust_decimal::Decimal::new(10000, 0),
                rust_decimal::Decimal::ZERO,
                rust_decimal::Decimal::ZERO,
                OptimizationMetric::AnnualReturn,
            )
            .await
            .expect("optimize should succeed");
        let combo_count = result.combinations.len();
        assert_eq!(
            result.best.as_ref().map(|c| c.label.as_str()),
            result.combinations.first().map(|c| c.label.as_str())
        );

        // Sorted descending by annual_return.
        for pair in result.combinations.windows(2) {
            let a = pair[0].result.as_ref().map(|r| r.annual_return);
            let b = pair[1].result.as_ref().map(|r| r.annual_return);
            if let (Some(a), Some(b)) = (a, b) {
                assert!(
                    a >= b,
                    "combinations must be sorted by annual_return descending: {a} < {b}"
                );
            }
        }

        // top-N slice is the leading N entries.
        let top_n = 2.min(combo_count);
        let top = &result.combinations[..top_n];
        assert_eq!(top.len(), top_n);
        assert_eq!(top[0].label, result.combinations[0].label);
    }

    #[test]
    fn test_invalid_metric_returns_clear_error() {
        let err = OptimizationMetric::parse("invalid_metric").unwrap_err();
        assert!(matches!(err, ServiceError::InvalidParameter(_)));
    }

    #[test]
    fn test_metric_parse_known_values() {
        assert_eq!(
            OptimizationMetric::parse("sharpe_ratio").unwrap(),
            OptimizationMetric::SharpeRatio
        );
        assert_eq!(
            OptimizationMetric::parse("annual_return").unwrap(),
            OptimizationMetric::AnnualReturn
        );
        assert_eq!(
            OptimizationMetric::parse("max_drawdown").unwrap(),
            OptimizationMetric::MaxDrawdown
        );
    }

    #[tokio::test]
    async fn test_bayesian_returns_not_implemented() {
        let registry = Arc::new(default_registry());
        let config = ParamOptimizerConfig::default();
        let optimizer =
            ParamOptimizer::with_algorithm(registry, config, Box::new(BayesianOptimization));
        let grid = vec![serde_json::json!({"rsi_period": 14})];
        let err = optimizer
            .optimize(
                "MeanReversion",
                grid,
                sample_market_data(),
                rust_decimal::Decimal::new(10000, 0),
                rust_decimal::Decimal::ZERO,
                rust_decimal::Decimal::ZERO,
                OptimizationMetric::SharpeRatio,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::NotImplemented(_)));
    }

    #[tokio::test]
    async fn test_genetic_returns_not_implemented() {
        let registry = Arc::new(default_registry());
        let config = ParamOptimizerConfig::default();
        let optimizer =
            ParamOptimizer::with_algorithm(registry, config, Box::new(GeneticAlgorithm));
        let grid = vec![serde_json::json!({"rsi_period": 14})];
        let err = optimizer
            .optimize(
                "MeanReversion",
                grid,
                sample_market_data(),
                rust_decimal::Decimal::new(10000, 0),
                rust_decimal::Decimal::ZERO,
                rust_decimal::Decimal::ZERO,
                OptimizationMetric::AnnualReturn,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::NotImplemented(_)));
    }

    #[test]
    fn test_expand_grid_cartesian_product() {
        let grid = serde_json::json!({
            "rsi_period": [14, 7],
            "entry_threshold": [30, 25],
        });
        let combos = expand_grid(&grid).expect("grid should expand");
        assert_eq!(combos.len(), 4);
        assert!(combos
            .iter()
            .any(|c| c.pointer("/rsi_period") == Some(&serde_json::json!(14))));
        assert!(combos
            .iter()
            .any(|c| c.pointer("/rsi_period") == Some(&serde_json::json!(7))));
        assert!(combos
            .iter()
            .any(|c| c.pointer("/entry_threshold") == Some(&serde_json::json!(25))));
    }

    #[test]
    fn test_expand_grid_passthrough_array() {
        let grid = serde_json::json!([
            {"rsi_period": 14},
            {"rsi_period": 7},
        ]);
        let combos = expand_grid(&grid).expect("array grid should pass through");
        assert_eq!(combos.len(), 2);
    }

    #[test]
    fn test_expand_grid_empty_object_returns_error() {
        let err = expand_grid(&serde_json::json!({})).unwrap_err();
        assert!(matches!(err, ServiceError::InvalidParameter(_)));
    }

    #[test]
    fn test_expand_grid_non_array_value_returns_error() {
        let err = expand_grid(&serde_json::json!({"rsi_period": 14})).unwrap_err();
        assert!(matches!(err, ServiceError::InvalidParameter(_)));
    }

    #[test]
    fn test_sort_combinations_max_drawdown_ascending() {
        let mk = |label: &str, max_dd: i64| ParameterCombo {
            label: label.to_string(),
            params: serde_json::json!({}),
            result: Some(BacktestResult {
                id: None,
                strategy_id: "s".to_string(),
                start_date: chrono::Utc::now(),
                end_date: chrono::Utc::now(),
                initial_capital: Decimal::new(10000, 0),
                final_capital: Decimal::new(10000, 0),
                total_return: Decimal::ZERO,
                annual_return: Decimal::ZERO,
                sharpe_ratio: Decimal::ZERO,
                max_drawdown: Decimal::new(max_dd, 0),
                win_rate: Decimal::ZERO,
                profit_loss_ratio: Decimal::ZERO,
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                equity_curve: Vec::new(),
            }),
        };
        let mut combos = vec![mk("a", 30), mk("b", 10), mk("c", 20)];
        sort_combinations(&mut combos, OptimizationMetric::MaxDrawdown);
        assert_eq!(combos[0].label, "b", "lowest drawdown should rank first");
        assert_eq!(combos[1].label, "c");
        assert_eq!(combos[2].label, "a");
    }
}
