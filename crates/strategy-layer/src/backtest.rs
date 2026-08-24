use crate::strategy::Strategy;
use chrono::{DateTime, Utc};
use quant_common::types::{Account, BacktestResult, BacktestTrade, MarketData, Order, Position};
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
    trades: Vec<BacktestTrade>,
    fill_time: DateTime<Utc>,
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
            trades: Vec::new(),
            fill_time: Utc::now(),
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
        // Rolling window of bars seen so far, passed to the strategy so
        // lookback-based indicators (SMA/σ) have history to work with.
        let mut history: Vec<MarketData> = Vec::new();

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
                self.fill_time = timestamp;
                let fill_price = current_data
                    .iter()
                    .find(|d| d.symbol == order.symbol)
                    .map(|d| d.open)
                    .unwrap_or(Decimal::ZERO);
                if fill_price > Decimal::ZERO {
                    if self.execute_order_at_price(order, fill_price)? {
                        total_trades += 1;
                    }
                }
            }

            // 生成交易信号（用本根 K 线收盘价），挂单留待下一根 K 线执行
            history.extend(current_data.iter().cloned());
            let context = crate::strategy::StrategyContext {
                current_time: timestamp,
                positions: self.positions.values().cloned().collect(),
                market_data: history.clone(),
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
            strategy_name: None,
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
            trades: self.trades.clone(),
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
        let _ = self.execute_order_at_price(order, price)?;
        Ok(())
    }

    /// 以指定成交价执行订单（回测中用下一根 K 线开盘价成交，避免前视偏差）
    fn execute_order_at_price(&mut self, order: Order, price: Decimal) -> Result<bool> {
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
                    self.trades.push(BacktestTrade {
                        date: self.fill_time,
                        symbol: order.symbol.clone(),
                        r#type: "BUY".to_string(),
                        price,
                        quantity: order.quantity,
                        amount: total_cost,
                        commission,
                    });
                    return Ok(true);
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

                        self.trades.push(BacktestTrade {
                            date: self.fill_time,
                            symbol: order.symbol.clone(),
                            r#type: "SELL".to_string(),
                            price,
                            quantity: order.quantity,
                            amount: total_cost,
                            commission,
                        });
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
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

mod backtest_multi;
pub use backtest_multi::{run_backtest_multi, MultiStrategyResult};

#[cfg(test)]
mod tests;
