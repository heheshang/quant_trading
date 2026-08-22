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

    if slow_ema.is_empty() {
        return MACD {
            macd_line: Vec::new(),
            signal_line: Vec::new(),
            histogram: Vec::new(),
        };
    }

    // 快/慢 EMA 数组长度不同（fast 比 slow 多 slow-fast 个前置点），
    // 必须对齐到同一时间轴后再相减，否则会引入 (slow-fast) 根 K 线的错位。
    // fast_ema[i] 对应时间 (fast-1)+i，slow_ema[i] 对应时间 (slow-1)+i；
    // 同一时刻相减需取 fast_ema[i + (slow-fast)] - slow_ema[i]。
    let offset = slow_period.saturating_sub(fast_period);
    let mut macd_line = Vec::with_capacity(slow_ema.len());
    for (i, slow) in slow_ema.iter().enumerate() {
        macd_line.push(fast_ema[i + offset] - slow);
    }

    let signal_line = ema(&macd_line, signal_period);

    // histogram 同样需对齐：signal_line 比 macd_line 少 (signal-1) 个前置点。
    let macd_len = macd_line.len();
    let signal_offset = macd_len.saturating_sub(signal_line.len());
    let mut histogram = Vec::with_capacity(signal_line.len());
    for (i, s) in signal_line.iter().enumerate() {
        histogram.push(macd_line[i + signal_offset] - s);
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_macd_aligns_fast_and_slow_ema() {
        // 60 个递增价格点，确保 slow_ema 有足够数据计算信号线
        let data: Vec<Decimal> = (1..=60).map(Decimal::from).collect();
        let fast_period = 12;
        let slow_period = 26;
        let signal_period = 9;

        let fast_ema = ema(&data, fast_period);
        let slow_ema = ema(&data, slow_period);
        let offset = slow_period - fast_period;

        let result = macd(&data, fast_period, slow_period, signal_period);

        // macd_line 长度 = slow_ema 长度（对齐后）
        assert_eq!(result.macd_line.len(), slow_ema.len());
        // 首个 MACD 值 = 同一时刻的 fast_ema - slow_ema（错位修复验证）
        assert_eq!(result.macd_line[0], fast_ema[offset] - slow_ema[0]);
        // histogram 长度 = macd_line.len() - signal_period + 1
        assert_eq!(
            result.histogram.len(),
            result.macd_line.len() - signal_period + 1
        );
        // signal_line 长度 = histogram 长度
        assert_eq!(result.signal_line.len(), result.histogram.len());
    }

    #[test]
    fn test_macd_insufficient_data_returns_empty() {
        let data: Vec<Decimal> = (1..=20).map(Decimal::from).collect();
        let result = macd(&data, 12, 26, 9);
        // 数据不足 slow_period=26 时，slow_ema 为空，整个 MACD 应为空
        assert!(result.macd_line.is_empty());
        assert!(result.signal_line.is_empty());
        assert!(result.histogram.is_empty());
    }

    #[test]
    fn test_macd_constant_data() {
        let data = vec![dec!(100); 40];
        let result = macd(&data, 12, 26, 9);
        // 价格不变时 EMA 不变，MACD 全为 0
        for v in &result.macd_line {
            assert_eq!(*v, Decimal::ZERO);
        }
        for v in &result.histogram {
            assert_eq!(*v, Decimal::ZERO);
        }
    }
}
