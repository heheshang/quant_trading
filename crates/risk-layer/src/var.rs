use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use tracing::{info, instrument, warn};

/// VaR计算器 (Value at Risk)
pub struct VaRCalculator;

impl VaRCalculator {
    /// 历史模拟法计算VaR
    #[instrument(skip(returns), fields(risk_check = "var_historical"))]
    pub fn historical_simulation(returns: &[Decimal], confidence_level: f64) -> Decimal {
        if returns.is_empty() {
            warn!("Historical VaR called with empty returns");
            return Decimal::ZERO;
        }

        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort();

        let index = ((1.0 - confidence_level) * returns.len() as f64) as usize;
        let index = index.min(returns.len() - 1);

        let var = -sorted_returns[index];
        info!(
            "Historical VaR computed: confidence={}, result={}",
            confidence_level, var
        );
        var
    }

    /// 参数法计算VaR（假设正态分布）
    #[instrument(skip(returns), fields(risk_check = "var_parametric"))]
    pub fn parametric(returns: &[Decimal], confidence_level: f64) -> Decimal {
        if returns.is_empty() {
            warn!("Parametric VaR called with empty returns");
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

        // VaR（损失，正数）= z·σ − μ。原实现 mean + z·σ 计算的是上尾（收益）分位数，
        // 方向相反。此处返回潜在损失的正值。
        let var = z_score * std_dev - mean;
        info!(
            "Parametric VaR computed: confidence={}, result={}",
            confidence_level, var
        );
        var
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_parametric_var_is_positive_loss() {
        // 均值为 0、标准差 0.1 的收益序列
        let returns = vec![dec!(0.1), dec!(-0.1), dec!(0.1), dec!(-0.1)];
        let var = VaRCalculator::parametric(&returns, 0.95);
        // VaR（损失）= z*σ - μ = 1.65 * 0.1 - 0 = 0.165（正数）
        assert!(var > Decimal::ZERO);
        assert!((var - dec!(0.165)).abs() < dec!(0.001));
    }

    #[test]
    fn test_parametric_var_increases_with_negative_mean() {
        // 负均值会增加 VaR（更差的收益分布 → 更大的潜在损失）
        let negative = vec![dec!(0.08), dec!(-0.12), dec!(0.08), dec!(-0.12)];
        let neutral = vec![dec!(0.1), dec!(-0.1), dec!(0.1), dec!(-0.1)];
        let var_negative = VaRCalculator::parametric(&negative, 0.95);
        let var_neutral = VaRCalculator::parametric(&neutral, 0.95);
        assert!(var_negative > var_neutral);
    }
}
