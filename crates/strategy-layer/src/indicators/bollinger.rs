//! 布林带 (Bollinger Bands)

use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use tracing::instrument;

use super::sma::sma;

/// 布林带
pub struct BollingerBands {
    pub upper: Vec<Decimal>,
    pub middle: Vec<Decimal>,
    pub lower: Vec<Decimal>,
}

#[instrument(level = "debug", skip(data), fields(data_len = data.len(), period))]
pub fn bollinger_bands(
    data: &[Decimal],
    period: usize,
    std_dev_multiplier: Decimal,
) -> BollingerBands {
    let middle = sma(data, period);
    let mut upper = Vec::new();
    let mut lower = Vec::new();

    for (i, &ma) in middle.iter().enumerate() {
        let start_idx = i;
        let end_idx = i + period;

        if end_idx <= data.len() {
            let slice = &data[start_idx..end_idx];
            let variance: Decimal =
                slice.iter().map(|&x| (x - ma) * (x - ma)).sum::<Decimal>() / Decimal::from(period);

            let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);

            upper.push(ma + std_dev * std_dev_multiplier);
            lower.push(ma - std_dev * std_dev_multiplier);
        }
    }

    BollingerBands {
        upper,
        middle,
        lower,
    }
}
