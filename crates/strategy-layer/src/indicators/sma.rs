//! 简单移动平均 (Simple Moving Average, SMA)

use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use tracing::{info, instrument};

/// 简单移动平均(SMA)
#[instrument(level = "debug", skip(data), fields(data_len = data.len(), period))]
pub fn sma(data: &[Decimal], period: usize) -> Vec<Decimal> {
    let mut result = Vec::new();

    if data.len() < period || period == 0 {
        return result;
    }

    for i in period - 1..data.len() {
        let start_idx = i + 1 - period;
        let sum: Decimal = data[start_idx..=i].iter().sum();
        let avg = sum / Decimal::from(period);
        result.push(avg);
    }

    info!(result_len = result.len(), period, "SMA computed");
    result
}
