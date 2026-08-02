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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bollinger_bands_empty_data() {
        let bands = bollinger_bands(&[], 20, dec!(2));
        assert!(bands.upper.is_empty());
        assert!(bands.middle.is_empty());
        assert!(bands.lower.is_empty());
    }

    #[test]
    fn test_bollinger_bands_insufficient_data() {
        let data: Vec<Decimal> = (1..=5).map(Decimal::from).collect();
        let bands = bollinger_bands(&data, 20, dec!(2));
        assert!(bands.upper.is_empty());
        assert!(bands.middle.is_empty());
        assert!(bands.lower.is_empty());
    }

    #[test]
    fn test_bollinger_bands_normal_data() {
        let data: Vec<Decimal> = (1..=30).map(Decimal::from).collect();
        let bands = bollinger_bands(&data, 20, dec!(2));
        assert_eq!(bands.middle.len(), 11);
        assert_eq!(bands.upper.len(), 11);
        assert_eq!(bands.lower.len(), 11);
        let last = bands.middle.last().unwrap();
        assert!(last > &dec!(20) && last < &dec!(21));
        assert!(bands.upper.last().unwrap() > bands.middle.last().unwrap());
        assert!(bands.lower.last().unwrap() < bands.middle.last().unwrap());
    }

    #[test]
    fn test_bollinger_bands_constant_data_zero_bandwidth() {
        let data: Vec<Decimal> = vec![dec!(100); 25];
        let bands = bollinger_bands(&data, 20, dec!(2));
        assert_eq!(bands.upper.len(), 6);
        assert_eq!(bands.middle.len(), 6);
        assert_eq!(bands.lower.len(), 6);
        assert_eq!(bands.upper[0], dec!(100));
        assert_eq!(bands.middle[0], dec!(100));
        assert_eq!(bands.lower[0], dec!(100));
    }

    #[test]
    fn test_bollinger_bands_upper_breakout() {
        let mut data: Vec<Decimal> = vec![dec!(100); 19];
        data.push(dec!(200));
        let bands = bollinger_bands(&data, 20, dec!(2));
        assert_eq!(bands.middle.len(), 1);
        let last_close = data.last().unwrap();
        let upper = bands.upper.last().unwrap();
        assert!(
            last_close > upper,
            "last close {} should exceed upper band {}",
            last_close,
            upper
        );
    }

    #[test]
    fn test_bollinger_bands_lower_breakout() {
        let mut data: Vec<Decimal> = vec![dec!(100); 19];
        data.push(dec!(50));
        let bands = bollinger_bands(&data, 20, dec!(2));
        assert_eq!(bands.middle.len(), 1);
        let last_close = data.last().unwrap();
        let lower = bands.lower.last().unwrap();
        assert!(
            last_close < lower,
            "last close {} should be below lower band {}",
            last_close,
            lower
        );
    }

    #[test]
    fn test_bollinger_bands_period_zero() {
        let data: Vec<Decimal> = (1..=30).map(Decimal::from).collect();
        let bands = bollinger_bands(&data, 0, dec!(2));
        assert!(bands.upper.is_empty());
        assert!(bands.middle.is_empty());
        assert!(bands.lower.is_empty());
    }
}
