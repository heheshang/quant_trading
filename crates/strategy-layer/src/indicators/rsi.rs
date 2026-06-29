//! 相对强弱指标 (Relative Strength Index, RSI)

use rust_decimal::Decimal;
use tracing::{info, instrument};

const RSI_SCALE: Decimal = Decimal::ONE_HUNDRED;

/// 相对强弱指标(RSI)
#[instrument(level = "debug", skip(data), fields(data_len = data.len(), period))]
pub fn rsi(data: &[Decimal], period: usize) -> Vec<Decimal> {
    let mut result = Vec::new();

    if data.len() < period + 1 {
        return result;
    }

    let mut gains = Vec::new();
    let mut losses = Vec::new();

    for i in 1..data.len() {
        let change = data[i] - data[i - 1];
        if change > Decimal::ZERO {
            gains.push(change);
            losses.push(Decimal::ZERO);
        } else {
            gains.push(Decimal::ZERO);
            losses.push(change.abs());
        }
    }

    for i in period - 1..gains.len() {
        let start_idx = i + 1 - period;
        let avg_gain: Decimal =
            gains[start_idx..=i].iter().sum::<Decimal>() / Decimal::from(period);
        let avg_loss: Decimal =
            losses[start_idx..=i].iter().sum::<Decimal>() / Decimal::from(period);

        let rs = if avg_loss != Decimal::ZERO {
            avg_gain / avg_loss
        } else {
            RSI_SCALE
        };

        let rsi_value = RSI_SCALE - (RSI_SCALE / (Decimal::ONE + rs));
        result.push(rsi_value);
    }

    info!(result_len = result.len(), period, "RSI computed");
    result
}
