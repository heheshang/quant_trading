use rust_decimal::prelude::*;
use rust_decimal::Decimal;

const RSI_SCALE: Decimal = Decimal::ONE_HUNDRED;

/// 简单移动平均(SMA)
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

    result
}

/// 指数移动平均(EMA)
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

    for i in period..data.len() {
        let ema_value =
            (data[i] - result[result.len() - 1]) * multiplier + result[result.len() - 1];
        result.push(ema_value);
    }

    result
}

/// 布林带
pub struct BollingerBands {
    pub upper: Vec<Decimal>,
    pub middle: Vec<Decimal>,
    pub lower: Vec<Decimal>,
}

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

/// 相对强弱指标(RSI)
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

    result
}

/// MACD指标
pub struct MACD {
    pub macd_line: Vec<Decimal>,
    pub signal_line: Vec<Decimal>,
    pub histogram: Vec<Decimal>,
}

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

    MACD {
        macd_line,
        signal_line,
        histogram,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

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
}
