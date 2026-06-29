//! 平均趋向指数 (Average Directional Index, ADX)
//!
//! 趋势强度指标。与 `atr` 共享底层辅助函数（Wilder EMA、真实波幅、OHLC 校验）。

use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use tracing::instrument;

use super::{compute_true_range, ema_wilder, validate_ohlc_equal, IndicatorError, IndicatorResult};

/// 平均趋向指数(ADX) - 趋势强度指标
#[instrument(level = "debug", skip(high, low, close), fields(data_len = high.len(), period))]
pub fn adx(
    high: &[Decimal],
    low: &[Decimal],
    close: &[Decimal],
    period: usize,
) -> IndicatorResult<Vec<Decimal>> {
    validate_ohlc_equal(high, low, close)?;

    if high.len() < 2 * period + 1 || period == 0 {
        return Err(IndicatorError::InsufficientData {
            required: 2 * period + 1,
            actual: high.len(),
        });
    }

    let tr_values = compute_true_range(high, low, close);

    let mut plus_dm = Vec::with_capacity(high.len() - 1);
    let mut minus_dm = Vec::with_capacity(high.len() - 1);
    for i in 1..high.len() {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        if up_move > down_move && up_move > Decimal::ZERO {
            plus_dm.push(up_move);
            minus_dm.push(Decimal::ZERO);
        } else if down_move > up_move && down_move > Decimal::ZERO {
            plus_dm.push(Decimal::ZERO);
            minus_dm.push(down_move);
        } else {
            plus_dm.push(Decimal::ZERO);
            minus_dm.push(Decimal::ZERO);
        }
    }

    // Wilder-EMA of TR, +DM, -DM
    let atr_vals = ema_wilder(&tr_values, period);
    let plus_dm_smooth = ema_wilder(&plus_dm, period);
    let minus_dm_smooth = ema_wilder(&minus_dm, period);

    // Calculate DX for each aligned point
    let mut dx_values = Vec::with_capacity(atr_vals.len());
    for i in 0..atr_vals.len() {
        let plus_di = if atr_vals[i] != Decimal::ZERO {
            plus_dm_smooth[i] / atr_vals[i] * Decimal::ONE_HUNDRED
        } else {
            Decimal::ZERO
        };
        let minus_di = if atr_vals[i] != Decimal::ZERO {
            minus_dm_smooth[i] / atr_vals[i] * Decimal::ONE_HUNDRED
        } else {
            Decimal::ZERO
        };
        let di_sum = plus_di + minus_di;
        let di_diff = (plus_di - minus_di).abs();
        let dx = if di_sum != Decimal::ZERO {
            di_diff / di_sum * Decimal::ONE_HUNDRED
        } else {
            Decimal::ZERO
        };
        dx_values.push(dx);
    }

    // ADX = Wilder-EMA of DX
    Ok(ema_wilder(&dx_values, period))
}
