use rust_decimal::prelude::*;
use rust_decimal::Decimal;

/// VaR计算器 (Value at Risk)
pub struct VaRCalculator;

impl VaRCalculator {
    /// 历史模拟法计算VaR
    pub fn historical_simulation(returns: &[Decimal], confidence_level: f64) -> Decimal {
        if returns.is_empty() {
            return Decimal::ZERO;
        }

        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort();

        let index = ((1.0 - confidence_level) * returns.len() as f64) as usize;
        let index = index.min(returns.len() - 1);

        -sorted_returns[index]
    }

    /// 参数法计算VaR（假设正态分布）
    pub fn parametric(returns: &[Decimal], confidence_level: f64) -> Decimal {
        if returns.is_empty() {
            return Decimal::ZERO;
        }

        // 计算均值
        let mean: Decimal = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());

        // 计算标准差
        let variance: Decimal = returns
            .iter()
            .map(|r| (*r - mean) * (*r - mean))
            .sum::<Decimal>()
            / Decimal::from(returns.len());

        let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);

        // Z-score for different confidence levels
        let z_score = match confidence_level {
            x if x >= 0.99 => Decimal::from_f64_retain(2.33).unwrap(),
            x if x >= 0.95 => Decimal::from_f64_retain(1.65).unwrap(),
            _ => Decimal::from_f64_retain(1.28).unwrap(),
        };

        mean + z_score * std_dev
    }
}
