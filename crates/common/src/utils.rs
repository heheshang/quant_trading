use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

/// 计算年化收益率
pub fn calculate_annual_return(
    initial_capital: Decimal,
    final_capital: Decimal,
    days: i64,
) -> Decimal {
    if days <= 0 {
        return Decimal::ZERO;
    }
    
    let total_return = (final_capital - initial_capital) / initial_capital;
    let years = Decimal::from(days) / Decimal::from(365);
    
    if years > Decimal::ZERO {
        total_return / years
    } else {
        Decimal::ZERO
    }
}

/// 计算夏普比率
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_annual_return() {
        let initial = dec!(100000);
        let final_value = dec!(120000);
        let days = 365;
        
        let annual_return = calculate_annual_return(initial, final_value, days);
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
