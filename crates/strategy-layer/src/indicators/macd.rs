//! MACD 指标 (Moving Average Convergence Divergence)

use rust_decimal::Decimal;
use tracing::{info, instrument};

use super::ema::ema;

/// MACD指标
pub struct MACD {
    pub macd_line: Vec<Decimal>,
    pub signal_line: Vec<Decimal>,
    pub histogram: Vec<Decimal>,
}

#[instrument(level = "debug", skip(data), fields(data_len = data.len(), fast_period, slow_period, signal_period))]
pub fn macd(
    data: &[Decimal],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> MACD {
    let fast_ema = ema(data, fast_period);
    let slow_ema = ema(data, slow_period);

    let mut macd_line = Vec::new();
    let min_len = fast_ema.len().min(slow_ema.len());

    for i in 0..min_len {
        macd_line.push(fast_ema[i] - slow_ema[i]);
    }

    let signal_line = ema(&macd_line, signal_period);

    let mut histogram = Vec::new();
    let min_len = macd_line.len().min(signal_line.len());

    for i in 0..min_len {
        histogram.push(macd_line[i] - signal_line[i]);
    }

    info!(
        macd_len = macd_line.len(),
        signal_len = signal_line.len(),
        histogram_len = histogram.len(),
        "MACD computed"
    );
    MACD {
        macd_line,
        signal_line,
        histogram,
    }
}
