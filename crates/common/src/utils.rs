use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::time::{SystemTime, UNIX_EPOCH};

/// 计算年化收益率（CAGR，复利年化）
pub fn calculate_annual_return(
    initial_capital: Decimal,
    final_capital: Decimal,
    days: i64,
) -> Decimal {
    if days <= 0 || initial_capital <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let years = days as f64 / 365.0;
    if years <= 0.0 {
        return Decimal::ZERO;
    }

    let initial = initial_capital.to_f64().unwrap_or(0.0);
    if initial <= 0.0 {
        return Decimal::ZERO;
    }
    let ratio = final_capital.to_f64().unwrap_or(0.0) / initial;

    // CAGR = (final/initial)^(1/years) - 1，而非算术年化 (total_return / years)
    let cagr = ratio.powf(1.0 / years) - 1.0;
    Decimal::from_f64_retain(cagr)
        .unwrap_or(Decimal::ZERO)
        .round_dp(8)
}

/// 计算每周期夏普比率（未年化）：(mean - rf) / std。
/// 如需年化夏普，请用 `calculate_annualized_sharpe_ratio`。
pub fn calculate_sharpe_ratio(returns: &[Decimal], risk_free_rate: Decimal) -> Decimal {
    if returns.is_empty() {
        return Decimal::ZERO;
    }

    let mean_return = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
    let excess_return = mean_return - risk_free_rate;

    let variance = returns
        .iter()
        .map(|r| {
            let diff = *r - mean_return;
            diff * diff
        })
        .sum::<Decimal>()
        / Decimal::from(returns.len());

    let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);

    if std_dev > Decimal::ZERO {
        excess_return / std_dev
    } else {
        Decimal::ZERO
    }
}

/// 计算年化夏普比率：每周期夏普 × √(periods_per_year)。
pub fn calculate_annualized_sharpe_ratio(
    returns: &[Decimal],
    risk_free_rate: Decimal,
    periods_per_year: f64,
) -> Decimal {
    let per_period = calculate_sharpe_ratio(returns, risk_free_rate);
    if periods_per_year <= 0.0 {
        return per_period;
    }
    let scale = Decimal::from_f64_retain(periods_per_year.sqrt()).unwrap_or(Decimal::ONE);
    per_period * scale
}

/// 计算最大回撤
pub fn calculate_max_drawdown(equity_curve: &[(DateTime<Utc>, Decimal)]) -> Decimal {
    if equity_curve.is_empty() {
        return Decimal::ZERO;
    }

    let mut max_value = equity_curve[0].1;
    let mut max_drawdown = Decimal::ZERO;

    for (_, value) in equity_curve.iter() {
        if *value > max_value {
            max_value = *value;
        }

        let drawdown = (max_value - *value) / max_value;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    max_drawdown
}

/// 计算胜率
pub fn calculate_win_rate(winning_trades: i32, total_trades: i32) -> Decimal {
    if total_trades == 0 {
        return Decimal::ZERO;
    }

    Decimal::from(winning_trades) / Decimal::from(total_trades)
}

/// 毫秒时间戳 → UTC 时间；非法值（超出 chrono 可表示范围）返回 `None`。
pub fn datetime_from_millis(ms: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp_millis(ms)
}

/// 毫秒时间戳 → UTC 时间；非法时回退当前时间（用于 WS 消息等不可信时间戳）。
pub fn datetime_from_millis_or_now(ms: i64) -> DateTime<Utc> {
    datetime_from_millis(ms).unwrap_or_else(Utc::now)
}

/// 向下取整到 step（lot size）；step 非正时返回原值。
pub fn floor_to_step(value: Decimal, step: Decimal) -> Decimal {
    if step <= Decimal::ZERO {
        return value;
    }
    (value / step).floor() * step
}

/// 四舍五入到最近 tick（tick size）；tick 非正时返回原值。
pub fn round_to_tick(value: Decimal, tick: Decimal) -> Decimal {
    if tick <= Decimal::ZERO {
        return value;
    }
    (value / tick).round() * tick
}

/// 当前 UNIX 毫秒时间戳。
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_annual_return() {
        // 1 年：20% 收益 → CAGR 20%
        let annual_return = calculate_annual_return(dec!(100000), dec!(120000), 365);
        assert_eq!(annual_return, dec!(0.2));
    }

    #[test]
    fn test_annual_return_compounds() {
        // 2 年：100000 → 144000（年化 20% 复利）
        let annual_return = calculate_annual_return(dec!(100000), dec!(144000), 730);
        // CAGR = (1.44)^(1/2) - 1 = 0.2
        assert_eq!(annual_return, dec!(0.2));
    }

    #[test]
    fn test_max_drawdown() {
        let equity_curve = vec![
            (Utc::now(), dec!(100000)),
            (Utc::now(), dec!(110000)),
            (Utc::now(), dec!(105000)),
            (Utc::now(), dec!(95000)),
            (Utc::now(), dec!(120000)),
        ];

        let max_dd = calculate_max_drawdown(&equity_curve);
        assert!(max_dd > Decimal::ZERO);
    }
}
