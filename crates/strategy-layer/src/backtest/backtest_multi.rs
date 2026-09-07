//! 多策略回测对比。

use crate::strategy::Strategy;
use quant_common::types::{BacktestResult, BacktestTrade, MarketData};
use quant_common::utils::{
    calculate_annual_return, calculate_annualized_sharpe_ratio, calculate_max_drawdown,
};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use std::collections::HashMap;

use super::{BacktestEngine, BacktestOptions, RealizedPnl};

/// 多策略回测结果对比
#[derive(Debug, Clone)]
pub struct MultiStrategyResult {
    pub results: Vec<(String, BacktestResult)>,
}

/// 对多个策略同时运行回测并返回对比结果。
///
/// 若 `market_data` 含多个标的（组合回测）：**按标的拆分**，每个标的用各自历史单独回测
/// （避免多标的 bar 交错污染指标），再把各标的的结果合并为一个组合结果。初始资本均分到每个标的。
pub async fn run_backtest_multi(
    strategies: Vec<(Box<dyn Strategy>, String)>,
    market_data: Vec<MarketData>,
    initial_capital: Decimal,
    commission_rate: Decimal,
    slippage: Decimal,
) -> Result<MultiStrategyResult> {
    // 按标的拆分（保持各标的 bar 顺序）。
    let mut by_symbol: HashMap<String, Vec<MarketData>> = HashMap::new();
    let mut symbol_order: Vec<String> = Vec::new();
    for d in market_data {
        if !by_symbol.contains_key(&d.symbol) {
            symbol_order.push(d.symbol.clone());
        }
        by_symbol.entry(d.symbol.clone()).or_default().push(d);
    }
    if symbol_order.is_empty() {
        return Err(Error::Validation(
            "Empty market data for multi-backtest".to_string(),
        ));
    }
    let num_symbols = symbol_order.len().max(1);
    let per_capital = if num_symbols > 1 {
        initial_capital / Decimal::from(num_symbols)
    } else {
        initial_capital
    };

    let mut results = Vec::new();
    for (strategy, label) in strategies {
        let mut symbol_results: Vec<BacktestResult> = Vec::new();
        let mut realized_pnl = RealizedPnl::default();
        for symbol in &symbol_order {
            let data = by_symbol.get(symbol).cloned().unwrap_or_default();
            let mut engine = BacktestEngine::new(per_capital, commission_rate, slippage);
            let result = engine
                .run_with_options(&*strategy, data, BacktestOptions::default())
                .await?;
            let symbol_pnl = engine.realized_pnl();
            realized_pnl.total_profit += symbol_pnl.total_profit;
            realized_pnl.total_loss += symbol_pnl.total_loss;
            symbol_results.push(result);
        }
        let result = aggregate_portfolio(symbol_results, initial_capital, realized_pnl)?;
        results.push((label, result));
    }

    Ok(MultiStrategyResult { results })
}

/// 一次聚合各标的回测结果为一个组合结果（避免链式 merge 重复计初资本）。
fn aggregate_portfolio(
    mut symbol_results: Vec<BacktestResult>,
    initial_capital: Decimal,
    realized_pnl: RealizedPnl,
) -> Result<BacktestResult> {
    if symbol_results.is_empty() {
        return Err(Error::Internal(
            "No per-symbol backtest results".to_string(),
        ));
    }
    // 单标的：结果即为全量（per_capital == initial_capital）。
    if symbol_results.len() == 1 {
        return Ok(symbol_results.remove(0));
    }

    // 在统一时间轴累加权益变化：首条前保留初始现金，缺 bar 和末条后延续上次权益。
    let mut changes = std::collections::BTreeMap::new();
    let mut equity = Decimal::ZERO;
    for r in &symbol_results {
        equity += r.initial_capital;
        let mut previous = r.initial_capital;
        for (timestamp, value) in &r.equity_curve {
            *changes.entry(*timestamp).or_insert(Decimal::ZERO) += *value - previous;
            previous = *value;
        }
    }
    let equity_curve: Vec<_> = changes
        .into_iter()
        .map(|(timestamp, change)| {
            equity += change;
            (timestamp, equity)
        })
        .collect();
    let winning = symbol_results.iter().map(|r| r.winning_trades).sum::<i32>();
    let losing = symbol_results.iter().map(|r| r.losing_trades).sum::<i32>();
    let total_trades = symbol_results.iter().map(|r| r.total_trades).sum::<i32>();
    let start_date = symbol_results
        .iter()
        .map(|r| r.start_date)
        .min()
        .unwrap_or_else(chrono::Utc::now);
    let end_date = symbol_results
        .iter()
        .map(|r| r.end_date)
        .max()
        .unwrap_or_else(chrono::Utc::now);
    // 每标的以 per_capital 起始，最终资本和 = initial_capital + 组合盈亏。
    let final_capital = symbol_results
        .iter()
        .map(|r| r.final_capital)
        .sum::<Decimal>();
    let strategy_id = symbol_results[0].strategy_id.clone();
    let strategy_name = symbol_results[0].strategy_name.clone();
    let id = symbol_results[0].id;

    let mut trades: Vec<BacktestTrade> =
        Vec::with_capacity(symbol_results.iter().map(|r| r.trades.len()).sum());
    for r in symbol_results {
        trades.extend(r.trades);
    }
    trades.sort_by_key(|trade| trade.date);

    let days = (end_date - start_date).num_days();
    let total_return = if initial_capital > Decimal::ZERO {
        (final_capital - initial_capital) / initial_capital
    } else {
        Decimal::ZERO
    };
    let annual_return = calculate_annual_return(initial_capital, final_capital, days);
    let periods_per_year = if equity_curve.len() >= 2 {
        let span = (equity_curve[1].0 - equity_curve[0].0).num_milliseconds();
        if span > 0 {
            (365.0 * 86_400_000.0) / span as f64
        } else {
            365.0
        }
    } else {
        365.0
    };
    let returns: Vec<Decimal> = equity_curve
        .windows(2)
        .map(|w| {
            let prev = w[0].1;
            if prev.is_zero() {
                Decimal::ZERO
            } else {
                (w[1].1 - prev) / prev
            }
        })
        .collect();
    let sharpe = calculate_annualized_sharpe_ratio(&returns, Decimal::ZERO, periods_per_year);
    let max_drawdown = calculate_max_drawdown(&equity_curve);
    let win_rate = if winning + losing > 0 {
        Decimal::from(winning) / Decimal::from(winning + losing)
    } else {
        Decimal::ZERO
    };
    let profit_loss_ratio = realized_pnl.profit_loss_ratio();

    Ok(BacktestResult {
        id,
        strategy_id,
        strategy_name,
        start_date,
        end_date,
        initial_capital,
        final_capital,
        total_return,
        annual_return,
        sharpe_ratio: sharpe.round_dp(6),
        max_drawdown,
        win_rate,
        profit_loss_ratio,
        total_trades,
        winning_trades: winning,
        losing_trades: losing,
        equity_curve,
        trades,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn mini_result(
        initial: Decimal,
        final_capital: Decimal,
        equity: Vec<(chrono::DateTime<Utc>, Decimal)>,
        winning: i32,
        losing: i32,
        total_trades: i32,
    ) -> BacktestResult {
        BacktestResult {
            id: None,
            strategy_id: "strat".into(),
            strategy_name: Some("Strat".into()),
            start_date: Utc::now(),
            end_date: Utc::now(),
            initial_capital: initial,
            final_capital,
            total_return: Decimal::ZERO,
            annual_return: Decimal::ZERO,
            sharpe_ratio: Decimal::ZERO,
            max_drawdown: Decimal::ZERO,
            win_rate: Decimal::ZERO,
            profit_loss_ratio: Decimal::ZERO,
            total_trades,
            winning_trades: winning,
            losing_trades: losing,
            equity_curve: equity,
            trades: vec![BacktestTrade {
                date: Utc::now(),
                symbol: "X".into(),
                r#type: "buy".into(),
                price: Decimal::ZERO,
                quantity: Decimal::ZERO,
                amount: Decimal::ZERO,
                commission: Decimal::ZERO,
            }],
        }
    }

    #[test]
    fn single_symbol_returns_result_as_is() {
        let mut r = mini_result(Decimal::from(10000), Decimal::from(11000), vec![], 1, 0, 1);
        r.profit_loss_ratio = Decimal::from(100);
        let agg =
            aggregate_portfolio(vec![r], Decimal::from(10000), RealizedPnl::default()).unwrap();
        assert_eq!(agg.final_capital, Decimal::from(11000));
        assert_eq!(agg.winning_trades, 1);
        assert_eq!(agg.trades.len(), 1);
        assert_eq!(agg.profit_loss_ratio, Decimal::from(100));
    }

    #[test]
    fn two_symbols_sums_equity_and_capital() {
        let t0 = Utc::now();
        let t1 = t0 + chrono::Duration::seconds(60);
        let a = mini_result(
            Decimal::from(5000),
            Decimal::from(5500),
            vec![(t0, Decimal::from(5000)), (t1, Decimal::from(5500))],
            1,
            0,
            1,
        );
        let b = mini_result(
            Decimal::from(5000),
            Decimal::from(4800),
            vec![(t0, Decimal::from(5000)), (t1, Decimal::from(4800))],
            0,
            1,
            1,
        );
        let agg =
            aggregate_portfolio(vec![a, b], Decimal::from(10000), RealizedPnl::default()).unwrap();
        // 最终资本 = 5500 + 4800 = 10300；t1 组合权益 = 5500 + 4800。
        assert_eq!(agg.final_capital, Decimal::from(10300));
        assert_eq!(agg.equity_curve.len(), 2);
        assert_eq!(agg.equity_curve[0].1, Decimal::from(10000));
        assert_eq!(agg.equity_curve[1].1, Decimal::from(10300));
        assert_eq!(agg.winning_trades, 1);
        assert_eq!(agg.losing_trades, 1);
        assert_eq!(agg.total_trades, 2);
        assert_eq!(agg.trades.len(), 2);
        // 全量初资本为基准的 total_return = (10300-10000)/10000。
        assert_eq!(agg.total_return, Decimal::new(300, 4));
    }

    #[test]
    fn staggered_curves_preserve_initial_cash_and_forward_fill_gaps() {
        let t0 = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let t1 = t0 + chrono::Duration::hours(1);
        let t2 = t1 + chrono::Duration::hours(1);
        let t3 = t2 + chrono::Duration::hours(1);
        let t4 = t3 + chrono::Duration::hours(1);
        let a = mini_result(
            Decimal::from(100),
            Decimal::from(120),
            vec![
                (t0, Decimal::from(100)),
                (t2, Decimal::from(110)),
                (t3, Decimal::from(120)),
            ],
            0,
            0,
            0,
        );
        let b = mini_result(
            Decimal::from(200),
            Decimal::from(220),
            vec![(t1, Decimal::from(200)), (t4, Decimal::from(220))],
            0,
            0,
            0,
        );
        let result =
            aggregate_portfolio(vec![a, b], Decimal::from(300), RealizedPnl::default()).unwrap();
        assert_eq!(
            result.equity_curve,
            vec![
                (t0, Decimal::from(300)),
                (t1, Decimal::from(300)),
                (t2, Decimal::from(310)),
                (t3, Decimal::from(320)),
                (t4, Decimal::from(340)),
            ]
        );
        assert_eq!(result.final_capital, Decimal::from(340));
        assert_eq!(result.max_drawdown, Decimal::ZERO);
    }

    #[test]
    fn empty_symbol_results_is_error() {
        assert!(aggregate_portfolio(vec![], Decimal::from(10000), RealizedPnl::default()).is_err());
    }
}
