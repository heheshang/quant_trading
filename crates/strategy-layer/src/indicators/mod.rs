//! 技术指标模块
//!
//! 拆分为 7 个子文件（按算法/数学相似度分组），`mod.rs` 仅承担：
//!   1. 子模块声明 + 公共 API `pub use`（保持外部 `crate::indicators::*` 访问语义）
//!   2. 共享的 `IndicatorError` / `IndicatorResult` 错误类型
//!   3. ATR/ADX 共享的 Wilder-EMA + 真实波幅 + OHLC 校验等私有辅助函数
//!   4. Stochastic / CCI / OBV 三项次要指标（保持向后兼容）
//!   5. 模块级单元测试

use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::fmt;
use tracing::instrument;

// ---------------------------------------------------------------------------
// Submodules
// ---------------------------------------------------------------------------

pub mod adx;
pub mod atr;
pub mod bollinger;
pub mod ema;
pub mod macd;
pub mod rsi;
pub mod sma;

// ---------------------------------------------------------------------------
// Public re-exports — preserve the flat `crate::indicators::*` API
// ---------------------------------------------------------------------------

pub use adx::adx;
pub use atr::atr;
pub use bollinger::{bollinger_bands, BollingerBands};
pub use ema::ema;
pub use macd::{macd, MACD};
pub use rsi::rsi;
pub use sma::sma;

// ---------------------------------------------------------------------------
// Indicator error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum IndicatorError {
    InsufficientData { required: usize, actual: usize },
    InputLengthMismatch { reason: String },
}

impl fmt::Display for IndicatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndicatorError::InsufficientData { required, actual } => {
                write!(
                    f,
                    "Insufficient data: required {} elements, got {}",
                    required, actual
                )
            }
            IndicatorError::InputLengthMismatch { reason } => {
                write!(f, "Input length mismatch: {}", reason)
            }
        }
    }
}

impl std::error::Error for IndicatorError {}

pub type IndicatorResult<T> = std::result::Result<T, IndicatorError>;

// ---------------------------------------------------------------------------
// Helpers shared between ATR and ADX
// ---------------------------------------------------------------------------

/// EMA with smoothing factor 1/period (Wilder-style for ADX/ATR)
pub(super) fn ema_wilder(data: &[Decimal], period: usize) -> Vec<Decimal> {
    if data.len() < period || period == 0 {
        return vec![];
    }

    let multiplier = Decimal::ONE / Decimal::from(period);
    let mut result = Vec::with_capacity(data.len());

    let sum: Decimal = data[0..period].iter().copied().sum();
    let first_ema = sum / Decimal::from(period);
    result.push(first_ema);

    for &val in &data[period..] {
        let val = (val - result[result.len() - 1]) * multiplier + result[result.len() - 1];
        result.push(val);
    }

    result
}

pub(super) fn validate_ohlc_equal(
    high: &[Decimal],
    low: &[Decimal],
    close: &[Decimal],
) -> IndicatorResult<()> {
    let len = high.len();
    if low.len() != len || close.len() != len {
        return Err(IndicatorError::InputLengthMismatch {
            reason: format!(
                "high.len()={}, low.len()={}, close.len()={}",
                len,
                low.len(),
                close.len()
            ),
        });
    }
    Ok(())
}

pub(super) fn compute_true_range(
    high: &[Decimal],
    low: &[Decimal],
    close: &[Decimal],
) -> Vec<Decimal> {
    let mut tr = Vec::with_capacity(high.len() - 1);
    for i in 1..high.len() {
        let hl = high[i] - low[i];
        let h_pc = (high[i] - close[i - 1]).abs();
        let l_pc = (low[i] - close[i - 1]).abs();
        tr.push(hl.max(h_pc).max(l_pc));
    }
    tr
}

// ---------------------------------------------------------------------------
// I-3: Stochastic Oscillator  (随机动量指标)
// ---------------------------------------------------------------------------

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
    let d_vals = sma::sma(&k_vals, d_period);

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

// ---------------------------------------------------------------------------
// I-4: CCI — Commodity Channel Index  (顺势指标)
// ---------------------------------------------------------------------------

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
    let tp_sma = sma::sma(&tp, period);
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

// ---------------------------------------------------------------------------
// I-5: OBV — On-Balance Volume  (能量潮指标)
// ---------------------------------------------------------------------------

#[instrument(level = "debug", skip(close, volume), fields(data_len = close.len()))]
pub fn obv(close: &[Decimal], volume: &[Decimal]) -> IndicatorResult<Vec<Decimal>> {
    if close.len() != volume.len() {
        return Err(IndicatorError::InputLengthMismatch {
            reason: format!(
                "close.len()={} != volume.len()={}",
                close.len(),
                volume.len()
            ),
        });
    }

    if close.is_empty() {
        return Ok(vec![]);
    }

    let mut result = Vec::with_capacity(close.len());
    result.push(Decimal::ZERO);

    for i in 1..close.len() {
        let prev = result[i - 1];
        let obv_val = if close[i] > close[i - 1] {
            prev + volume[i]
        } else if close[i] < close[i - 1] {
            prev - volume[i]
        } else {
            prev
        };
        result.push(obv_val);
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Tests — keep the original test names + add coverage for the new layout
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ---- existing tests ----

    #[test]
    fn test_sma() {
        let data = vec![dec!(100), dec!(102), dec!(101), dec!(103), dec!(105)];
        let sma_result = sma(&data, 3);

        assert_eq!(sma_result.len(), 3);
        assert!(sma_result[0] > Decimal::ZERO);
    }

    #[test]
    fn test_rsi() {
        let data = vec![
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
        ];

        let rsi_result = rsi(&data, 14);
        assert!(!rsi_result.is_empty());
    }

    // ---- I-1: ATR tests ----

    #[test]
    fn test_atr_basic() {
        let high = vec![
            dec!(101), dec!(103), dec!(102), dec!(104), dec!(106), dec!(105), dec!(107),
            dec!(109), dec!(108), dec!(110), dec!(112), dec!(111), dec!(113), dec!(115),
            dec!(114), dec!(116), dec!(118), dec!(117), dec!(119), dec!(120),
        ];
        let low = vec![
            dec!(99), dec!(100), dec!(99), dec!(101), dec!(103), dec!(102), dec!(104),
            dec!(106), dec!(105), dec!(107), dec!(109), dec!(108), dec!(110), dec!(112),
            dec!(111), dec!(113), dec!(115), dec!(114), dec!(116), dec!(117),
        ];
        let close = vec![
            dec!(100), dec!(102), dec!(101), dec!(103), dec!(105), dec!(104), dec!(106),
            dec!(108), dec!(107), dec!(109), dec!(111), dec!(110), dec!(112), dec!(114),
            dec!(113), dec!(115), dec!(117), dec!(116), dec!(118), dec!(119),
        ];

        let result = atr(&high, &low, &close, 14).unwrap();
        assert!(!result.is_empty());
        // 20 bars -> 19 TR values -> 19-14+1 = 6 ATR values
        assert_eq!(result.len(), high.len() - 14);
    }

    #[test]
    fn test_atr_insufficient_data() {
        let high = vec![dec!(101), dec!(102)];
        let low = vec![dec!(99), dec!(100)];
        let close = vec![dec!(100), dec!(101)];

        let result = atr(&high, &low, &close, 14);
        assert!(result.is_err());
        match result.unwrap_err() {
            IndicatorError::InsufficientData { .. } => {}
            e => panic!("Expected InsufficientData, got {:?}", e),
        }
    }

    #[test]
    fn test_atr_length_mismatch() {
        let high = vec![dec!(101), dec!(102)];
        let low = vec![dec!(99)];
        let close = vec![dec!(100), dec!(101)];

        let result = atr(&high, &low, &close, 14);
        assert!(result.is_err());
        match result.unwrap_err() {
            IndicatorError::InputLengthMismatch { .. } => {}
            e => panic!("Expected InputLengthMismatch, got {:?}", e),
        }
    }

    // ---- I-2: ADX tests ----

    #[test]
    fn test_adx_basic() {
        let high = vec![
            dec!(101), dec!(103), dec!(102), dec!(104), dec!(106), dec!(105), dec!(107),
            dec!(109), dec!(108), dec!(110), dec!(112), dec!(111), dec!(113), dec!(115),
            dec!(114), dec!(116), dec!(118), dec!(117), dec!(119), dec!(120), dec!(122),
            dec!(121), dec!(123), dec!(125), dec!(124), dec!(126), dec!(128), dec!(127),
            dec!(129), dec!(130),
        ];
        let low = vec![
            dec!(99), dec!(100), dec!(99), dec!(101), dec!(103), dec!(102), dec!(104),
            dec!(106), dec!(105), dec!(107), dec!(109), dec!(108), dec!(110), dec!(112),
            dec!(111), dec!(113), dec!(115), dec!(114), dec!(116), dec!(117), dec!(119),
            dec!(118), dec!(120), dec!(122), dec!(121), dec!(123), dec!(125), dec!(124),
            dec!(126), dec!(127),
        ];
        let close = vec![
            dec!(100), dec!(102), dec!(101), dec!(103), dec!(105), dec!(104), dec!(106),
            dec!(108), dec!(107), dec!(109), dec!(111), dec!(110), dec!(112), dec!(114),
            dec!(113), dec!(115), dec!(117), dec!(116), dec!(118), dec!(119), dec!(121),
            dec!(120), dec!(122), dec!(124), dec!(123), dec!(125), dec!(127), dec!(126),
            dec!(128), dec!(129),
        ];

        let result = adx(&high, &low, &close, 14).unwrap();
        assert!(!result.is_empty());
        for val in &result {
            assert!(*val >= Decimal::ZERO);
            assert!(*val <= Decimal::ONE_HUNDRED);
        }
    }

    #[test]
    fn test_adx_insufficient_data() {
        let high = vec![dec!(101), dec!(102)];
        let low = vec![dec!(99), dec!(100)];
        let close = vec![dec!(100), dec!(101)];

        let result = adx(&high, &low, &close, 14);
        assert!(result.is_err());
    }

    // ---- I-3: Stochastic tests ----

    #[test]
    fn test_stochastic_basic() {
        let high = vec![
            dec!(105), dec!(107), dec!(106), dec!(108), dec!(110), dec!(109), dec!(111),
            dec!(113), dec!(112), dec!(114), dec!(116), dec!(115), dec!(117), dec!(119),
            dec!(118), dec!(120),
        ];
        let low = vec![
            dec!(99), dec!(101), dec!(100), dec!(102), dec!(104), dec!(103), dec!(105),
            dec!(107), dec!(106), dec!(108), dec!(110), dec!(109), dec!(111), dec!(113),
            dec!(112), dec!(114),
        ];
        let close = vec![
            dec!(102), dec!(104), dec!(103), dec!(105), dec!(107), dec!(106), dec!(108),
            dec!(110), dec!(109), dec!(111), dec!(113), dec!(112), dec!(114), dec!(116),
            dec!(115), dec!(117),
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

    // ---- I-4: CCI tests ----

    #[test]
    fn test_cci_basic() {
        let high = vec![
            dec!(102), dec!(104), dec!(103), dec!(105), dec!(107), dec!(106), dec!(108),
            dec!(110), dec!(109), dec!(111), dec!(113), dec!(112), dec!(114), dec!(116),
            dec!(115), dec!(117), dec!(119), dec!(118), dec!(120), dec!(122), dec!(121),
            dec!(123), dec!(125), dec!(124), dec!(126),
        ];
        let low = vec![
            dec!(98), dec!(100), dec!(99), dec!(101), dec!(103), dec!(102), dec!(104),
            dec!(106), dec!(105), dec!(107), dec!(109), dec!(108), dec!(110), dec!(112),
            dec!(111), dec!(113), dec!(115), dec!(114), dec!(116), dec!(118), dec!(117),
            dec!(119), dec!(121), dec!(120), dec!(122),
        ];
        let close = vec![
            dec!(100), dec!(102), dec!(101), dec!(103), dec!(105), dec!(104), dec!(106),
            dec!(108), dec!(107), dec!(109), dec!(111), dec!(110), dec!(112), dec!(114),
            dec!(113), dec!(115), dec!(117), dec!(116), dec!(118), dec!(120), dec!(119),
            dec!(121), dec!(123), dec!(122), dec!(124),
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

    // ---- I-5: OBV tests ----

    #[test]
    fn test_obv_basic() {
        let close = vec![dec!(100), dec!(102), dec!(101), dec!(103), dec!(105)];
        let volume = vec![dec!(1000), dec!(1500), dec!(1200), dec!(1800), dec!(2000)];

        let result = obv(&close, &volume).unwrap();
        assert_eq!(result.len(), close.len());
        // OBV[0] is always 0
        assert_eq!(result[0], Decimal::ZERO);
        // close[1]=102 > close[0]=100 → OBV[1] = 0 + 1500 = 1500
        assert_eq!(result[1], dec!(1500));
    }

    #[test]
    fn test_obv_length_mismatch() {
        let close = vec![dec!(100), dec!(102)];
        let volume = vec![dec!(1000)];

        let result = obv(&close, &volume);
        assert!(result.is_err());
        match result.unwrap_err() {
            IndicatorError::InputLengthMismatch { .. } => {}
            e => panic!("Expected InputLengthMismatch, got {:?}", e),
        }
    }

    #[test]
    fn test_obv_flat_close() {
        let close = vec![dec!(100), dec!(100), dec!(100)];
        let volume = vec![dec!(1000), dec!(1500), dec!(1200)];

        let result = obv(&close, &volume).unwrap();
        // When close doesn't change, OBV stays same
        assert_eq!(result[1], result[0]);
        assert_eq!(result[2], result[1]);
    }

    #[test]
    fn test_obv_empty() {
        let result = obv(&[], &[]).unwrap();
        assert!(result.is_empty());
    }

    // ---- I-6: IndicatorError Display ----

    #[test]
    fn test_indicator_error_display() {
        let err = IndicatorError::InsufficientData {
            required: 14,
            actual: 5,
        };
        let msg = err.to_string();
        assert!(msg.contains("14"));
        assert!(msg.contains("5"));

        let err = IndicatorError::InputLengthMismatch {
            reason: "mismatch".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("mismatch"));
    }
}
