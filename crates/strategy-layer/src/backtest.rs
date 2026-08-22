use crate::strategy::Strategy;
use chrono::{DateTime, Utc};
use quant_common::types::{Account, BacktestResult, MarketData, Order, Position};
use quant_common::utils::{
    calculate_annual_return, calculate_annualized_sharpe_ratio, calculate_max_drawdown,
};
use quant_common::{Error, Result};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};

/// Optional configuration for a backtest run.
#[derive(Clone, Default)]
pub struct BacktestOptions {
    pub cancellation_token: Option<CancellationToken>,
    pub timeout: Option<Duration>,
    #[allow(clippy::type_complexity)]
    pub progress: Option<Arc<dyn Fn(f64) + Send + Sync>>,
}

impl std::fmt::Debug for BacktestOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BacktestOptions")
            .field(
                "cancellation_token",
                &self.cancellation_token.as_ref().map(|_| "Some"),
            )
            .field("timeout", &self.timeout)
            .field("progress", &self.progress.as_ref().map(|_| "Some"))
            .finish()
    }
}

/// 回测引擎
pub struct BacktestEngine {
    initial_capital: Decimal,
    commission_rate: Decimal,
    slippage: Decimal,
    positions: HashMap<String, Position>,
    account: Account,
    equity_curve: Vec<(DateTime<Utc>, Decimal)>,
    winning_trades: i32,
    losing_trades: i32,
    total_profit: Decimal,
    total_loss: Decimal,
}

impl BacktestEngine {
    pub fn new(initial_capital: Decimal, commission_rate: Decimal, slippage: Decimal) -> Self {
        Self {
            initial_capital,
            commission_rate,
            slippage,
            positions: HashMap::new(),
            account: Account {
                account_id: 0,
                total_assets: initial_capital,
                available_cash: initial_capital,
                frozen_cash: Decimal::ZERO,
                market_value: Decimal::ZERO,
                total_pnl: Decimal::ZERO,
                daily_pnl: Decimal::ZERO,
                margin: Decimal::ZERO,
                margin_ratio: Decimal::ZERO,
                updated_at: Utc::now(),
            },
            equity_curve: Vec::new(),
            winning_trades: 0,
            losing_trades: 0,
            total_profit: Decimal::ZERO,
            total_loss: Decimal::ZERO,
        }
    }

    #[deprecated(note = "Use `run_with_options` instead")]
    #[instrument(skip(self, strategy, market_data), fields(strategy = %strategy.name(), data_points = market_data.len()))]
    pub async fn run(
        &mut self,
        strategy: &dyn Strategy,
        market_data: Vec<MarketData>,
    ) -> Result<BacktestResult> {
        self.run_with_options(strategy, market_data, BacktestOptions::default())
            .await
    }

    #[instrument(skip(self, strategy, market_data, options), fields(strategy = %strategy.name(), data_points = market_data.len()))]
    pub async fn run_with_options(
        &mut self,
        strategy: &dyn Strategy,
        market_data: Vec<MarketData>,
        options: BacktestOptions,
    ) -> Result<BacktestResult> {
        info!(
            strategy = %strategy.name(),
            data_points = market_data.len(),
            initial_capital = %self.initial_capital,
            "Starting backtest"
        );
        let start_date = market_data
            .first()
            .ok_or_else(|| {
                error!("Empty market data provided for backtest");
                Error::Validation("Empty market data".to_string())
            })?
            .timestamp;

        let end_date = market_data
            .last()
            .ok_or_else(|| {
                error!("Empty market data provided for backtest");
                Error::Validation("Empty market data".to_string())
            })?
            .timestamp;

        // 按时间戳分组数据
        let mut data_by_time: HashMap<DateTime<Utc>, Vec<MarketData>> = HashMap::new();
        for data in market_data {
            data_by_time.entry(data.timestamp).or_default().push(data);
        }

        let mut timestamps: Vec<DateTime<Utc>> = data_by_time.keys().cloned().collect();
        timestamps.sort();

        // 估算周期频率（用于年化夏普）：由前两根K线间隔推算每年周期数。
        // 默认按日频 365 处理（加密 7×24 市场）。
        let periods_per_year = if timestamps.len() >= 2 {
            let interval_secs = (timestamps[1] - timestamps[0]).num_seconds();
            if interval_secs > 0 {
                (365.0 * 86400.0) / interval_secs as f64
            } else {
                365.0
            }
        } else {
            365.0
        };

        let mut total_trades = 0;
        self.winning_trades = 0;
        self.losing_trades = 0;
        self.total_profit = Decimal::ZERO;
        self.total_loss = Decimal::ZERO;

        let deadline = options.timeout.map(|d| std::time::Instant::now() + d);

        // 待执行的挂单（上一根 K 线生成的信号，留待下一根 K 线成交）
        let mut pending_orders: Vec<Order> = Vec::new();

        let total_timestamps = timestamps.len();
        for (i, timestamp) in timestamps.into_iter().enumerate() {
            if let Some(dl) = deadline {
                if std::time::Instant::now() > dl {
                    return Err(Error::Internal("Backtest timed out".to_string()));
                }
            }

            if let Some(ref token) = options.cancellation_token {
                if token.is_cancelled() {
                    return Err(Error::Internal("Backtest cancelled".to_string()));
                }
            }

            tokio::task::yield_now().await;

            let current_data = data_by_time.get(&timestamp).ok_or_else(|| {
                error!(%timestamp, "Timestamp not found in data_by_time map");
                Error::Internal("Timestamp not found in market data".to_string())
            })?;

            // 执行上一根 K 线产生的挂单：以本根 K 线开盘价成交，避免前视偏差。
            // 实践：信号用 t 收盘价生成，成交发生在 t+1 开盘价（而非 t 收盘价）。
            let carried = std::mem::take(&mut pending_orders);
            for order in carried {
                let fill_price = current_data
                    .iter()
                    .find(|d| d.symbol == order.symbol)
                    .map(|d| d.open)
                    .unwrap_or(Decimal::ZERO);
                if fill_price > Decimal::ZERO {
                    self.execute_order_at_price(order, fill_price)?;
                    total_trades += 1;
                }
            }

            // 生成交易信号（用本根 K 线收盘价），挂单留待下一根 K 线执行
            let context = crate::strategy::StrategyContext {
                current_time: timestamp,
                positions: self.positions.values().cloned().collect(),
                market_data: current_data.clone(),
            };

            pending_orders = strategy.generate_signals(&context).await?;

            // 更新账户
            self.update_account(current_data, timestamp)?;

            // 记录权益曲线
            self.equity_curve
                .push((timestamp, self.account.total_assets));

            if let Some(ref progress) = options.progress {
                progress((i + 1) as f64 / total_timestamps as f64);
            }
        }

        info!(
            strategy = %strategy.name(),
            processed_timestamps = total_timestamps,
            total_trades,
            "Backtest main loop complete"
        );

        // 计算回测结果
        let total_return =
            (self.account.total_assets - self.initial_capital) / self.initial_capital;
        let days = (end_date - start_date).num_days();
        let annual_return =
            calculate_annual_return(self.initial_capital, self.account.total_assets, days);

        // 计算每周期收益率（权益曲线相邻两点的相对变化；周期频率由数据决定）
        let mut period_returns = Vec::new();
        for i in 1..self.equity_curve.len() {
            let prev_value = self.equity_curve[i - 1].1;
            let curr_value = self.equity_curve[i].1;
            if prev_value > Decimal::ZERO {
                let period_return = (curr_value - prev_value) / prev_value;
                period_returns.push(period_return);
            }
        }

        let sharpe_ratio =
            calculate_annualized_sharpe_ratio(&period_returns, Decimal::ZERO, periods_per_year);
        let max_drawdown = calculate_max_drawdown(&self.equity_curve);

        // 计算胜率
        let win_rate = if total_trades > 0 {
            Decimal::from(self.winning_trades) / Decimal::from(total_trades)
        } else {
            Decimal::ZERO
        };

        info!(
            strategy = %strategy.name(),
            total_return = %total_return,
            annual_return = %annual_return,
            sharpe_ratio = %sharpe_ratio,
            max_drawdown = %max_drawdown,
            win_rate = %win_rate,
            total_trades,
            "Backtest complete"
        );

        Ok(BacktestResult {
            id: None,
            strategy_id: strategy.name().to_string(),
            start_date,
            end_date,
            initial_capital: self.initial_capital,
            final_capital: self.account.total_assets,
            total_return,
            annual_return,
            sharpe_ratio,
            max_drawdown,
            win_rate,
            profit_loss_ratio: if self.total_loss > Decimal::ZERO {
                self.total_profit / self.total_loss
            } else if self.total_profit > Decimal::ZERO {
                Decimal::from_f64(100.0).unwrap_or(Decimal::ONE)
            } else {
                Decimal::ZERO
            },
            total_trades,
            winning_trades: self.winning_trades,
            losing_trades: self.losing_trades,
            equity_curve: self.equity_curve.clone(),
        })
    }

    /// 以市场价（或订单限价）执行订单。当前回测主循环已改用 `execute_order_at_price`
    /// （下一根K线开盘价成交，避免前视偏差），本方法保留给单元测试直接构造成交场景。
    #[allow(dead_code)]
    #[instrument(skip(self, market_data), fields(order_id = %order.order_id, symbol = %order.symbol))]
    fn execute_order(&mut self, order: Order, market_data: &[MarketData]) -> Result<()> {
        // 查找订单对应的市场数据（仅用于取参考价格）
        let data = market_data
            .iter()
            .find(|d| d.symbol == order.symbol)
            .ok_or_else(|| {
                error!(symbol = %order.symbol, order_id = %order.order_id, "Market data not found for order execution");
                Error::NotFound("Market data not found".to_string())
            })?;

        let price = order.price.unwrap_or(data.close);
        self.execute_order_at_price(order, price)
    }

    /// 以指定成交价执行订单（回测中用下一根 K 线开盘价成交，避免前视偏差）
    fn execute_order_at_price(&mut self, order: Order, price: Decimal) -> Result<()> {
        let total_cost = price * order.quantity;
        let commission = total_cost * self.commission_rate;
        let slippage_cost = total_cost * self.slippage;
        let total_expense = total_cost + commission + slippage_cost;

        match order.side {
            quant_common::types::OrderSide::Buy => {
                if self.account.available_cash >= total_expense {
                    self.account.available_cash -= total_expense;

                    // 更新持仓
                    let position =
                        self.positions
                            .entry(order.symbol.clone())
                            .or_insert_with(|| Position {
                                symbol: order.symbol.clone(),
                                quantity: Decimal::ZERO,
                                available_quantity: Decimal::ZERO,
                                avg_price: Decimal::ZERO,
                                market_value: Decimal::ZERO,
                                unrealized_pnl: Decimal::ZERO,
                                realized_pnl: Decimal::ZERO,
                                updated_at: Utc::now(),
                            });

                    let old_qty = position.quantity;
                    position.quantity += order.quantity;
                    position.available_quantity += order.quantity;
                    position.avg_price =
                        (old_qty * position.avg_price + order.quantity * price) / position.quantity;
                }
            }
            quant_common::types::OrderSide::Sell => {
                if let Some(position) = self.positions.get_mut(&order.symbol) {
                    if position.available_quantity >= order.quantity {
                        position.quantity -= order.quantity;
                        position.available_quantity -= order.quantity;

                        let proceeds = total_cost - commission - slippage_cost;
                        self.account.available_cash += proceeds;

                        // 计算实现盈亏
                        let pnl = (price - position.avg_price) * order.quantity;
                        position.realized_pnl += pnl;

                        // 统计胜/负交易并累加盈亏金额
                        if pnl > Decimal::ZERO {
                            self.winning_trades += 1;
                            self.total_profit += pnl;
                        } else if pnl < Decimal::ZERO {
                            self.losing_trades += 1;
                            self.total_loss += pnl.abs();
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[instrument(skip(self, market_data), fields(positions = self.positions.len()))]
    fn update_account(
        &mut self,
        market_data: &[MarketData],
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let mut total_market_value = Decimal::ZERO;
        let mut total_unrealized_pnl = Decimal::ZERO;

        for (symbol, position) in self.positions.iter_mut() {
            if let Some(data) = market_data.iter().find(|d| d.symbol == *symbol) {
                position.market_value = data.close * position.quantity;
                position.unrealized_pnl = (data.close - position.avg_price) * position.quantity;

                total_market_value += position.market_value;
                total_unrealized_pnl += position.unrealized_pnl;

                position.updated_at = timestamp;
            }
        }

        self.account.market_value = total_market_value;
        self.account.total_assets = self.account.available_cash + total_market_value;
        self.account.total_pnl = total_unrealized_pnl
            + self
                .positions
                .values()
                .map(|p| p.realized_pnl)
                .sum::<Decimal>();
        self.account.updated_at = timestamp;

        Ok(())
    }
}

/// 多策略回测结果对比
#[derive(Debug, Clone)]
pub struct MultiStrategyResult {
    pub results: Vec<(String, BacktestResult)>,
}

/// 对多个策略同时运行回测并返回对比结果
///
/// 每个策略使用相同的市场数据，并创建独立的 BacktestEngine，
/// 避免策略之间的状态干扰。
pub async fn run_backtest_multi(
    strategies: Vec<(Box<dyn Strategy>, String)>,
    market_data: Vec<MarketData>,
    initial_capital: Decimal,
    commission_rate: Decimal,
    slippage: Decimal,
) -> Result<MultiStrategyResult> {
    let mut handles = Vec::new();
    for (strategy, label) in strategies {
        let data = market_data.clone();
        handles.push(tokio::spawn(async move {
            let mut engine = BacktestEngine::new(initial_capital, commission_rate, slippage);
            let result = engine
                .run_with_options(strategy.as_ref(), data, BacktestOptions::default())
                .await;
            (label, result)
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok((label, Ok(result))) => {
                results.push((label, result));
            }
            Ok((label, Err(e))) => {
                return Err(Error::Internal(format!(
                    "Backtest failed for {}: {}",
                    label, e
                )));
            }
            Err(e) => {
                return Err(Error::Internal(format!("Task join error: {}", e)));
            }
        }
    }

    Ok(MultiStrategyResult { results })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::MeanReversionStrategy;
    use rust_decimal::Decimal;

    fn make_market_data(timestamp: DateTime<Utc>, close: Decimal, symbol: &str) -> MarketData {
        MarketData {
            timestamp,
            symbol: symbol.to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: Decimal::from(1000),
            turnover: Decimal::ZERO,
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        }
    }

    fn make_order(
        symbol: &str,
        side: quant_common::types::OrderSide,
        price: Decimal,
        quantity: Decimal,
    ) -> Order {
        Order {
            order_id: 0,
            strategy_id: "test".to_string(),
            symbol: symbol.to_string(),
            order_type: quant_common::types::OrderType::Limit,
            side,
            price: Some(price),
            quantity,
            filled_quantity: Decimal::ZERO,
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: Decimal::ZERO,
            slippage: Decimal::ZERO,
        }
    }

    #[tokio::test]
    async fn test_empty_market_data_returns_error() {
        let mut engine = BacktestEngine::new(
            Decimal::from(10000),
            Decimal::from_f64(0.001).unwrap(),
            Decimal::from_f64(0.0001).unwrap(),
        );
        let strategy = MeanReversionStrategy::new();
        let result = engine
            .run_with_options(&strategy, vec![], BacktestOptions::default())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_single_buy_and_sell_produces_profit_loss_ratio() {
        let now = Utc::now();
        let data = vec![
            make_market_data(now, Decimal::from(100), "BTC/USDT"),
            make_market_data(
                now + chrono::Duration::hours(1),
                Decimal::from(100),
                "BTC/USDT",
            ),
        ];

        let mut engine = BacktestEngine::new(Decimal::from(10000), Decimal::ZERO, Decimal::ZERO);

        // Buy at 100, quantity = 10
        engine
            .execute_order(
                make_order(
                    "BTC/USDT",
                    quant_common::types::OrderSide::Buy,
                    Decimal::from(100),
                    Decimal::from(10),
                ),
                &data,
            )
            .unwrap();

        // Sell at 110, profit = (110 - 100) * 10 = 100
        engine
            .execute_order(
                make_order(
                    "BTC/USDT",
                    quant_common::types::OrderSide::Sell,
                    Decimal::from(110),
                    Decimal::from(10),
                ),
                &data,
            )
            .unwrap();

        assert_eq!(engine.winning_trades, 1);
        assert_eq!(engine.total_profit, Decimal::from(100));
        assert_eq!(engine.total_loss, Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_multiple_buys_weighted_avg_price() {
        let now = Utc::now();
        let data = vec![
            make_market_data(now, Decimal::from(100), "BTC/USDT"),
            make_market_data(
                now + chrono::Duration::hours(1),
                Decimal::from(200),
                "BTC/USDT",
            ),
        ];

        let mut engine = BacktestEngine::new(Decimal::from(10000), Decimal::ZERO, Decimal::ZERO);

        // First buy: 10 units at 100
        engine
            .execute_order(
                make_order(
                    "BTC/USDT",
                    quant_common::types::OrderSide::Buy,
                    Decimal::from(100),
                    Decimal::from(10),
                ),
                &data,
            )
            .unwrap();

        // Second buy: 10 units at 200
        engine
            .execute_order(
                make_order(
                    "BTC/USDT",
                    quant_common::types::OrderSide::Buy,
                    Decimal::from(200),
                    Decimal::from(10),
                ),
                &data,
            )
            .unwrap();

        // Weighted avg = (10*100 + 10*200) / 20 = 150
        let btc_pos = engine.positions.get("BTC/USDT").unwrap();
        assert_eq!(btc_pos.avg_price, Decimal::from(150));
        assert_eq!(btc_pos.quantity, Decimal::from(20));
    }

    #[tokio::test]
    async fn test_profit_loss_ratio_calculation() {
        let now = Utc::now();
        let data = vec![
            make_market_data(now, Decimal::from(100), "BTC/USDT"),
            make_market_data(
                now + chrono::Duration::hours(1),
                Decimal::from(100),
                "BTC/USDT",
            ),
        ];

        let mut engine = BacktestEngine::new(Decimal::from(10000), Decimal::ZERO, Decimal::ZERO);

        // Buy at 100
        engine
            .execute_order(
                make_order(
                    "BTC/USDT",
                    quant_common::types::OrderSide::Buy,
                    Decimal::from(100),
                    Decimal::from(10),
                ),
                &data,
            )
            .unwrap();

        // Win trade: sell at 150, profit = 500
        engine
            .execute_order(
                make_order(
                    "BTC/USDT",
                    quant_common::types::OrderSide::Sell,
                    Decimal::from(150),
                    Decimal::from(5),
                ),
                &data,
            )
            .unwrap();

        // Loss trade: sell at 50, loss = 250
        engine
            .execute_order(
                make_order(
                    "BTC/USDT",
                    quant_common::types::OrderSide::Sell,
                    Decimal::from(50),
                    Decimal::from(5),
                ),
                &data,
            )
            .unwrap();

        // total_profit = 250, total_loss = 250
        // profit_loss_ratio = 250 / 250 = 1.0
        assert_eq!(engine.total_profit, Decimal::from(250));
        assert_eq!(engine.total_loss, Decimal::from(250));
    }
}
