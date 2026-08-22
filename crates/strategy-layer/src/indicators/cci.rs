//! 顺势指标 (Commodity Channel Index, CCI)

use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use tracing::instrument;

use super::sma::sma;
use super::{validate_ohlc_equal, IndicatorError, IndicatorResult};

/// 顺势指标 (CCI)
#[instrument(level = "debug", skip(high, low, close), fields(data_len = high.len(), period))]
pub fn cci(
    high: &[Decimal],
    low: &[Decimal],
    close: &[Decimal],
    period: usize,
) -> IndicatorResult<Vec<Decimal>> {
    validate_ohlc_equal(high, low, close)?;

    if high.len() < period || period == 0 {
        return Err(IndicatorError::InsufficientData {
            required: period,
            actual: high.len(),
        });
    }

    // Typical Price
    let three = Decimal::from(3);
    let tp: Vec<Decimal> = high
        .iter()
        .zip(low.iter())
        .zip(close.iter())
        .map(|((&h, &l), &c)| (h + l + c) / three)
        .collect();

    // SMA of TP
    let tp_sma = sma(&tp, period);
    if tp_sma.is_empty() {
        return Ok(vec![]);
    }

    let cci_mult = Decimal::from_str("0.015").unwrap_or(Decimal::new(15, 3));

    let mut result = Vec::with_capacity(tp_sma.len());
    for i in 0..tp_sma.len() {
        let start_idx = i;
        let end_idx = i + period;
        // Mean deviation over the window
        let sum_abs: Decimal = tp[start_idx..end_idx]
            .iter()
            .map(|&x| (x - tp_sma[i]).abs())
            .sum();
        let mean_dev = sum_abs / Decimal::from(period);
        let cci_val = if mean_dev != Decimal::ZERO {
            (tp[i + period - 1] - tp_sma[i]) / (cci_mult * mean_dev)
        } else {
            Decimal::ZERO
        };
        result.push(cci_val);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_cci_basic() {
        let high = vec![
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
            dec!(119),
            dec!(118),
            dec!(120),
            dec!(122),
            dec!(121),
            dec!(123),
            dec!(125),
            dec!(124),
            dec!(126),
        ];
        let low = vec![
            dec!(98),
            dec!(100),
            dec!(99),
            dec!(101),
            dec!(103),
            dec!(102),
            dec!(104),
            dec!(106),
            dec!(105),
            dec!(107),
            dec!(109),
            dec!(108),
            dec!(110),
            dec!(112),
            dec!(111),
            dec!(113),
            dec!(115),
            dec!(114),
            dec!(116),
            dec!(118),
            dec!(117),
            dec!(119),
            dec!(121),
            dec!(120),
            dec!(122),
        ];
        let close = vec![
            dec!(100),
            dec!(102),
            dec!(101),
            dec!(103),
            dec!(105),
            dec!(104),
            dec!(106),
            dec!(108),
            dec!(107),
            dec!(109),
            dec!(111),
            dec!(110),
            dec!(112),
            dec!(114),
            dec!(113),
            dec!(115),
            dec!(117),
            dec!(116),
            dec!(118),
            dec!(120),
            dec!(119),
            dec!(121),
            dec!(123),
            dec!(122),
            dec!(124),
        ];

        let result = cci(&high, &low, &close, 20).unwrap();
        assert!(!result.is_empty());
        assert_eq!(result.len(), high.len() - 20 + 1);
    }

    #[test]
    fn test_cci_insufficient_data() {
        let high = vec![dec!(101)];
        let low = vec![dec!(99)];
        let close = vec![dec!(100)];

        let result = cci(&high, &low, &close, 20);
        assert!(result.is_err());
    }
}
