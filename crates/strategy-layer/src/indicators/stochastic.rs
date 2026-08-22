//! 随机动量指标 (Stochastic Oscillator)

use rust_decimal::Decimal;
use tracing::instrument;

use super::sma::sma;
use super::{validate_ohlc_equal, IndicatorError, IndicatorResult};

/// 随机动量指标：返回 (%K, %D)，%D 为 %K 的 SMA。
#[instrument(level = "debug", skip(high, low, close), fields(data_len = high.len(), k_period, d_period))]
pub fn stochastic(
    high: &[Decimal],
    low: &[Decimal],
    close: &[Decimal],
    k_period: usize,
    d_period: usize,
) -> IndicatorResult<(Vec<Decimal>, Vec<Decimal>)> {
    validate_ohlc_equal(high, low, close)?;

    if close.len() < k_period || k_period == 0 {
        return Err(IndicatorError::InsufficientData {
            required: k_period,
            actual: close.len(),
        });
    }

    // %K values
    let mut k_vals = Vec::with_capacity(close.len() - k_period + 1);
    for i in (k_period - 1)..close.len() {
        let start = i + 1 - k_period;
        let highest = high[start..=i]
            .iter()
            .copied()
            .max()
            .unwrap_or(Decimal::ZERO);
        let lowest = low[start..=i]
            .iter()
            .copied()
            .min()
            .unwrap_or(Decimal::ZERO);
        let range = highest - lowest;
        let k = if range != Decimal::ZERO {
            (close[i] - lowest) / range * Decimal::ONE_HUNDRED
        } else {
            Decimal::ZERO
        };
        k_vals.push(k);
    }

    // %D = SMA of %K
    let d_vals = sma(&k_vals, d_period);

    // Truncate %K to match %D length
    let len = k_vals.len();
    let d_len = d_vals.len();
    let truncated_k: Vec<Decimal> = k_vals[len.saturating_sub(d_len)..].to_vec();

    // Both slices should be the same length now
    let final_k = if d_len > 0 && truncated_k.len() == d_len {
        truncated_k
    } else {
        k_vals
    };

    Ok((final_k, d_vals))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_stochastic_basic() {
        let high = vec![
            dec!(105),
            dec!(107),
            dec!(106),
            dec!(108),
            dec!(110),
            dec!(109),
            dec!(111),
            dec!(113),
            dec!(112),
            dec!(114),
            dec!(116),
            dec!(115),
            dec!(117),
            dec!(119),
            dec!(118),
            dec!(120),
        ];
        let low = vec![
            dec!(99),
            dec!(101),
            dec!(100),
            dec!(102),
            dec!(104),
            dec!(103),
            dec!(105),
            dec!(107),
            dec!(106),
            dec!(108),
            dec!(110),
            dec!(109),
            dec!(111),
            dec!(113),
            dec!(112),
            dec!(114),
        ];
        let close = vec![
            dec!(102),
            dec!(104),
            dec!(103),
            dec!(105),
            dec!(107),
            dec!(106),
            dec!(108),
            dec!(110),
            dec!(109),
            dec!(111),
            dec!(113),
            dec!(112),
            dec!(114),
            dec!(116),
            dec!(115),
            dec!(117),
        ];

        let result = stochastic(&high, &low, &close, 14, 3).unwrap();
        let (k_vals, d_vals) = result;
        assert_eq!(k_vals.len(), d_vals.len());
        assert!(!k_vals.is_empty());
        for val in k_vals.iter().chain(d_vals.iter()) {
            assert!(*val >= Decimal::ZERO);
            assert!(*val <= Decimal::ONE_HUNDRED);
        }
    }

    #[test]
    fn test_stochastic_insufficient_data() {
        let high = vec![dec!(101)];
        let low = vec![dec!(99)];
        let close = vec![dec!(100)];

        let result = stochastic(&high, &low, &close, 14, 3);
        assert!(result.is_err());
    }
}
