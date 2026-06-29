//! 指数移动平均 (Exponential Moving Average, EMA)

use rust_decimal::Decimal;
use tracing::{info, instrument};

use super::sma::sma;

/// 指数移动平均(EMA)
#[instrument(level = "debug", skip(data), fields(data_len = data.len(), period))]
pub fn ema(data: &[Decimal], period: usize) -> Vec<Decimal> {
    let mut result = Vec::new();

    if data.is_empty() || period == 0 {
        return result;
    }

    let multiplier = Decimal::from(2) / Decimal::from(period + 1);

    // 首个EMA值使用SMA
    let sma_values = sma(data, period);
    if sma_values.is_empty() {
        return result;
    }

    result.push(sma_values[0]);

    for &val in &data[period..] {
        let ema_value = (val - result[result.len() - 1]) * multiplier + result[result.len() - 1];
        result.push(ema_value);
    }

    info!(result_len = result.len(), period, "EMA computed");
    result
}
