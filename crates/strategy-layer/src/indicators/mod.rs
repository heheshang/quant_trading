//! 技术指标模块
//!
//! 每个指标一个子文件（按算法/数学相似度分组），`mod.rs` 仅承担：
//!   1. 子模块声明 + 公共 API `pub use`（保持外部 `crate::indicators::*` 访问语义）
//!   2. 共享的 `IndicatorError` / `IndicatorResult` 错误类型
//!   3. ATR/ADX 共享的 Wilder-EMA + 真实波幅 + OHLC 校验等私有辅助函数
//!   4. 模块级单元测试

use rust_decimal::Decimal;
use std::fmt;

// ---------------------------------------------------------------------------
// Submodules
// ---------------------------------------------------------------------------

pub mod adx;
pub mod atr;
pub mod bollinger;
pub mod cci;
pub mod ema;
pub mod macd;
pub mod obv;
pub mod rsi;
pub mod sma;
pub mod stochastic;

// ---------------------------------------------------------------------------
// Public re-exports — preserve the flat `crate::indicators::*` API
// ---------------------------------------------------------------------------

pub use adx::adx;
pub use atr::atr;
pub use bollinger::{bollinger_bands, BollingerBands};
pub use cci::cci;
pub use ema::ema;
pub use macd::{macd, MACD};
pub use obv::obv;
pub use rsi::rsi;
pub use sma::sma;
pub use stochastic::stochastic;

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
            dec!(120),
        ];
        let low = vec![
            dec!(99),
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
            dec!(117),
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
            dec!(119),
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
            dec!(120),
            dec!(122),
            dec!(121),
            dec!(123),
            dec!(125),
            dec!(124),
            dec!(126),
            dec!(128),
            dec!(127),
            dec!(129),
            dec!(130),
        ];
        let low = vec![
            dec!(99),
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
            dec!(127),
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
            dec!(119),
            dec!(121),
            dec!(120),
            dec!(122),
            dec!(124),
            dec!(123),
            dec!(125),
            dec!(127),
            dec!(126),
            dec!(128),
            dec!(129),
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
