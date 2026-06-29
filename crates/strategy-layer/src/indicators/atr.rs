//! 平均真实波幅 (Average True Range, ATR)
//!
//! 共享给 ADX 模块使用，所以 `validate_ohlc_equal` / `compute_true_range` /
//! `ema_wilder` 都放在 `mod.rs` 中以避免 `atr.rs` 与 `adx.rs` 互相依赖。

use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use tracing::instrument;

use super::{compute_true_range, ema_wilder, validate_ohlc_equal, IndicatorError, IndicatorResult};

/// 平均真实波幅(ATR) - 波动率指标
#[instrument(level = "debug", skip(high, low, close), fields(data_len = high.len(), period))]
pub fn atr(
    high: &[Decimal],
    low: &[Decimal],
    close: &[Decimal],
    period: usize,
) -> IndicatorResult<Vec<Decimal>> {
    validate_ohlc_equal(high, low, close)?;

    if high.len() < period + 1 || period == 0 {
        return Err(IndicatorError::InsufficientData {
            required: period + 1,
            actual: high.len(),
        });
    }

    let tr_values = compute_true_range(high, low, close);
    Ok(ema_wilder(&tr_values, period))
}
