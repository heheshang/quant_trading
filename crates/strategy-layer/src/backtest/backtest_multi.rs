//! 多策略回测对比。

use crate::strategy::Strategy;
use quant_common::types::{BacktestResult, MarketData};
use quant_common::{Error, Result};
use rust_decimal::Decimal;

use super::{BacktestEngine, BacktestOptions};

/// 多策略回测结果对比
#[derive(Debug, Clone)]
pub struct MultiStrategyResult {
    pub results: Vec<(String, BacktestResult)>,
}

/// 对多个策略同时运行回测并返回对比结果
///
/// 每个策略使用相同的市场数据，并创建独立的 BacktestEngine，
/// 避免策略之间的状态干扰。
pub async fn run_backtest_multi(
    strategies: Vec<(Box<dyn Strategy>, String)>,
    market_data: Vec<MarketData>,
    initial_capital: Decimal,
    commission_rate: Decimal,
    slippage: Decimal,
) -> Result<MultiStrategyResult> {
    let mut handles = Vec::new();
    for (strategy, label) in strategies {
        let data = market_data.clone();
        handles.push(tokio::spawn(async move {
            let mut engine = BacktestEngine::new(initial_capital, commission_rate, slippage);
            let result = engine
                .run_with_options(strategy.as_ref(), data, BacktestOptions::default())
                .await;
            (label, result)
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok((label, Ok(result))) => {
                results.push((label, result));
            }
            Ok((label, Err(e))) => {
                return Err(Error::Internal(format!(
                    "Backtest failed for {}: {}",
                    label, e
                )));
            }
            Err(e) => {
                return Err(Error::Internal(format!("Task join error: {}", e)));
            }
        }
    }

    Ok(MultiStrategyResult { results })
}
