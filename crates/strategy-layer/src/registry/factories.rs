//! 内置策略工厂（MeanReversion / TrendFollowing）。

use async_trait::async_trait;
use quant_common::types::{ParameterSchema, StrategyParams};

use super::{FactoryError, StrategyFactory};
use crate::strategy::{
    MacdStrategy, MeanReversionStrategy, RsiStrategy, Strategy, TrendFollowingStrategy,
};

/// 均值回归策略的工厂实现
pub struct MeanReversionFactory;

#[async_trait]
impl StrategyFactory for MeanReversionFactory {
    async fn create(&self, params: StrategyParams) -> Result<Box<dyn Strategy>, FactoryError> {
        let mut strategy = MeanReversionStrategy::new();
        strategy
            .initialize(params)
            .await
            .map_err(|e| FactoryError::Initialize(e.to_string()))?;
        Ok(Box::new(strategy))
    }

    fn parameter_schema(&self) -> Vec<ParameterSchema> {
        vec![
            ParameterSchema {
                name: "lookback_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(20),
                range: Some(quant_common::types::ParamRange {
                    min: 5.0,
                    max: 100.0,
                    step: Some(1.0),
                }),
                description: "Lookback period for mean reversion calculation".into(),
            },
            ParameterSchema {
                name: "entry_threshold".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(2.0),
                range: Some(quant_common::types::ParamRange {
                    min: 0.5,
                    max: 5.0,
                    step: Some(0.1),
                }),
                description: "Entry threshold in standard deviations".into(),
            },
            ParameterSchema {
                name: "exit_threshold".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(0.5),
                range: Some(quant_common::types::ParamRange {
                    min: 0.1,
                    max: 3.0,
                    step: Some(0.1),
                }),
                description: "Exit threshold in standard deviations".into(),
            },
        ]
    }
}

/// 趋势跟踪策略的工厂实现
pub struct TrendFollowingFactory;

#[async_trait]
impl StrategyFactory for TrendFollowingFactory {
    async fn create(&self, params: StrategyParams) -> Result<Box<dyn Strategy>, FactoryError> {
        let mut strategy = TrendFollowingStrategy::new();
        strategy
            .initialize(params)
            .await
            .map_err(|e| FactoryError::Initialize(e.to_string()))?;
        Ok(Box::new(strategy))
    }

    fn parameter_schema(&self) -> Vec<ParameterSchema> {
        vec![
            ParameterSchema {
                name: "short_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(12),
                range: Some(quant_common::types::ParamRange {
                    min: 5.0,
                    max: 50.0,
                    step: Some(1.0),
                }),
                description: "Short EMA period for crossover detection".into(),
            },
            ParameterSchema {
                name: "long_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(26),
                range: Some(quant_common::types::ParamRange {
                    min: 20.0,
                    max: 200.0,
                    step: Some(1.0),
                }),
                description: "Long EMA period for crossover detection".into(),
            },
            ParameterSchema {
                name: "signal_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(9),
                range: Some(quant_common::types::ParamRange {
                    min: 3.0,
                    max: 30.0,
                    step: Some(1.0),
                }),
                description: "MACD signal line period for trend confirmation".into(),
            },
        ]
    }
}

/// MACD 交叉策略的工厂实现
pub struct MacdFactory;

#[async_trait]
impl StrategyFactory for MacdFactory {
    async fn create(&self, params: StrategyParams) -> Result<Box<dyn Strategy>, FactoryError> {
        let mut strategy = MacdStrategy::new();
        strategy
            .initialize(params)
            .await
            .map_err(|e| FactoryError::Initialize(e.to_string()))?;
        Ok(Box::new(strategy))
    }

    fn parameter_schema(&self) -> Vec<ParameterSchema> {
        vec![
            ParameterSchema {
                name: "fast_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(12),
                range: Some(quant_common::types::ParamRange {
                    min: 2.0,
                    max: 50.0,
                    step: Some(1.0),
                }),
                description: "Fast EMA period".into(),
            },
            ParameterSchema {
                name: "slow_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(26),
                range: Some(quant_common::types::ParamRange {
                    min: 5.0,
                    max: 100.0,
                    step: Some(1.0),
                }),
                description: "Slow EMA period".into(),
            },
            ParameterSchema {
                name: "signal_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(9),
                range: Some(quant_common::types::ParamRange {
                    min: 2.0,
                    max: 50.0,
                    step: Some(1.0),
                }),
                description: "Signal EMA period".into(),
            },
        ]
    }
}

/// RSI 反转策略的工厂实现
pub struct RsiFactory;

#[async_trait]
impl StrategyFactory for RsiFactory {
    async fn create(&self, params: StrategyParams) -> Result<Box<dyn Strategy>, FactoryError> {
        let mut strategy = RsiStrategy::new();
        strategy
            .initialize(params)
            .await
            .map_err(|e| FactoryError::Initialize(e.to_string()))?;
        Ok(Box::new(strategy))
    }

    fn parameter_schema(&self) -> Vec<ParameterSchema> {
        vec![
            ParameterSchema {
                name: "period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(14),
                range: Some(quant_common::types::ParamRange {
                    min: 2.0,
                    max: 100.0,
                    step: Some(1.0),
                }),
                description: "RSI lookback period".into(),
            },
            ParameterSchema {
                name: "oversold".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(30),
                range: Some(quant_common::types::ParamRange {
                    min: 10.0,
                    max: 45.0,
                    step: Some(1.0),
                }),
                description: "Oversold threshold (buy below)".into(),
            },
            ParameterSchema {
                name: "overbought".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(70),
                range: Some(quant_common::types::ParamRange {
                    min: 55.0,
                    max: 90.0,
                    step: Some(1.0),
                }),
                description: "Overbought threshold (sell above)".into(),
            },
        ]
    }
}
