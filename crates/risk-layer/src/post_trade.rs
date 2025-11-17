use quant_common::types::{Account, BacktestResult};
use rust_decimal::Decimal;

/// 事后交易分析器
pub struct PostTradeAnalyzer;

impl PostTradeAnalyzer {
    /// 归因分析
    pub fn attribution_analysis(result: &BacktestResult) -> AttributionResult {
        AttributionResult {
            market_return: Decimal::ZERO,  // 简化
            alpha: result.total_return,     // 超额收益
            strategy_specific: result.total_return,
        }
    }

    /// 压力测试
    pub fn stress_test(account: &Account, scenarios: Vec<StressScenario>) -> Vec<StressTestResult> {
        scenarios
            .into_iter()
            .map(|scenario| {
                let impact = account.total_assets * scenario.shock;
                StressTestResult {
                    scenario_name: scenario.name,
                    expected_loss: impact,
                    survival: impact.abs() < account.total_assets,
                }
            })
            .collect()
    }
}

pub struct AttributionResult {
    pub market_return: Decimal,
    pub alpha: Decimal,
    pub strategy_specific: Decimal,
}

pub struct StressScenario {
    pub name: String,
    pub shock: Decimal,  // 价格冲击百分比
}

pub struct StressTestResult {
    pub scenario_name: String,
    pub expected_loss: Decimal,
    pub survival: bool,
}
