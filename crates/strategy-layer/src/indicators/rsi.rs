//! 相对强弱指标 (Relative Strength Index, RSI)

use rust_decimal::Decimal;
use tracing::{info, instrument};

const RSI_SCALE: Decimal = Decimal::ONE_HUNDRED;

/// 相对强弱指标(RSI)
///
/// 采用 Wilder 平滑（SMMA，α=1/period），即经典 RSI 定义。
#[instrument(level = "debug", skip(data), fields(data_len = data.len(), period))]
pub fn rsi(data: &[Decimal], period: usize) -> Vec<Decimal> {
    if data.len() < period + 1 || period == 0 {
        return Vec::new();
    }

    let mut gains = Vec::with_capacity(data.len() - 1);
    let mut losses = Vec::with_capacity(data.len() - 1);

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

    // Wilder 平滑（与 ATR/ADX 一致）替代普通 SMA，符合经典 RSI 定义
    let avg_gains = super::ema_wilder(&gains, period);
    let avg_losses = super::ema_wilder(&losses, period);

    let mut result = Vec::with_capacity(avg_gains.len());
    for i in 0..avg_gains.len() {
        let avg_gain = avg_gains[i];
        let avg_loss = avg_losses[i];

        let rsi_value = if avg_loss == Decimal::ZERO {
            if avg_gain == Decimal::ZERO {
                // 价格完全不变 → 中性 50
                Decimal::from(50)
            } else {
                // 无亏损 → RSI = 100
                RSI_SCALE
            }
        } else {
            let rs = avg_gain / avg_loss;
            RSI_SCALE - (RSI_SCALE / (Decimal::ONE + rs))
        };
        result.push(rsi_value);
    }

    info!(result_len = result.len(), period, "RSI computed");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_rsi_all_gains_is_100() {
        // 单调上涨 → 无亏损 → RSI = 100
        let data: Vec<Decimal> = (1..=20).map(Decimal::from).collect();
        let result = rsi(&data, 14);
        assert!(!result.is_empty());
        for v in &result {
            assert_eq!(*v, Decimal::ONE_HUNDRED);
        }
    }

    #[test]
    fn test_rsi_flat_is_50() {
        // 价格不变 → 涨跌幅全 0 → RSI = 50（中性）
        let data = vec![dec!(100); 20];
        let result = rsi(&data, 14);
        assert!(!result.is_empty());
        for v in &result {
            assert_eq!(*v, Decimal::from(50));
        }
    }

    #[test]
    fn test_rsi_bounded() {
        let data: Vec<Decimal> = vec![
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
            dec!(115),
            dec!(117),
            dec!(116),
            dec!(118),
            dec!(120),
        ];
        let result = rsi(&data, 14);
        assert!(!result.is_empty());
        for v in &result {
            assert!(*v >= Decimal::ZERO && *v <= Decimal::ONE_HUNDRED);
        }
    }
}
