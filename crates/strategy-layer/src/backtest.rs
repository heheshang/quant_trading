use crate::strategy::Strategy;
use chrono::{DateTime, Utc};
use quant_common::types::{Account, BacktestResult, MarketData, Order, Position};
use quant_common::utils::{
    calculate_annual_return, calculate_max_drawdown, calculate_sharpe_ratio,
};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{error, info, instrument};

/// 回测引擎
pub struct BacktestEngine {
    initial_capital: Decimal,
    commission_rate: Decimal,
    slippage: Decimal,
    positions: HashMap<String, Position>,
    account: Account,
    equity_curve: Vec<(DateTime<Utc>, Decimal)>,
}

impl BacktestEngine {
    pub fn new(initial_capital: Decimal, commission_rate: Decimal, slippage: Decimal) -> Self {
        Self {
            initial_capital,
            commission_rate,
            slippage,
            positions: HashMap::new(),
            account: Account {
                account_id: uuid::Uuid::new_v4(),
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
        }
    }

    #[instrument(skip(self, strategy, market_data), fields(strategy = %strategy.name(), data_points = market_data.len()))]
    pub async fn run<S: Strategy>(
        &mut self,
        strategy: &S,
        market_data: Vec<MarketData>,
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
            data_by_time
                .entry(data.timestamp)
                .or_insert_with(Vec::new)
                .push(data);
        }

        let mut timestamps: Vec<DateTime<Utc>> = data_by_time.keys().cloned().collect();
        timestamps.sort();
        let processed_timestamps = timestamps.len();

        let mut total_trades = 0;
        let winning_trades = 0;
        let losing_trades = 0;

        // 回测主循环
        for timestamp in timestamps {
            let current_data = data_by_time.get(&timestamp).unwrap();

            // 生成交易信号
            let context = crate::strategy::StrategyContext {
                current_time: timestamp,
                positions: self.positions.values().cloned().collect(),
                market_data: current_data.clone(),
            };

            let orders = strategy.generate_signals(&context).await?;

            // 执行订单
            for order in orders {
                self.execute_order(order, current_data)?;
                total_trades += 1;
            }

            // 更新账户
            self.update_account(current_data, timestamp)?;

            // 记录权益曲线
            self.equity_curve
                .push((timestamp, self.account.total_assets));
        }

        info!(
            strategy = %strategy.name(),
            processed_timestamps,
            total_trades,
            "Backtest main loop complete"
        );

        // 计算回测结果
        let total_return =
            (self.account.total_assets - self.initial_capital) / self.initial_capital;
        let days = (end_date - start_date).num_days();
        let annual_return =
            calculate_annual_return(self.initial_capital, self.account.total_assets, days);

        // 计算每日收益率
        let mut daily_returns = Vec::new();
        for i in 1..self.equity_curve.len() {
            let prev_value = self.equity_curve[i - 1].1;
            let curr_value = self.equity_curve[i].1;
            if prev_value > Decimal::ZERO {
                let daily_return = (curr_value - prev_value) / prev_value;
                daily_returns.push(daily_return);
            }
        }

        let sharpe_ratio = calculate_sharpe_ratio(&daily_returns, Decimal::ZERO);
        let max_drawdown = calculate_max_drawdown(&self.equity_curve);

        // 计算胜率
        let win_rate = if total_trades > 0 {
            Decimal::from(winning_trades) / Decimal::from(total_trades)
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
            profit_loss_ratio: Decimal::ONE, // 简化处理
            total_trades,
            winning_trades,
            losing_trades,
            equity_curve: self.equity_curve.clone(),
        })
    }

    #[instrument(skip(self, market_data), fields(order_id = %order.order_id, symbol = %order.symbol))]
    fn execute_order(&mut self, order: Order, market_data: &[MarketData]) -> Result<()> {
        // 查找订单对应的市场数据
        let data = market_data
            .iter()
            .find(|d| d.symbol == order.symbol)
            .ok_or_else(|| {
                error!(symbol = %order.symbol, order_id = %order.order_id, "Market data not found for order execution");
                Error::NotFound("Market data not found".to_string())
            })?;

        let price = order.price.unwrap_or(data.close);
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

                    position.quantity += order.quantity;
                    position.available_quantity += order.quantity;
                    position.avg_price = price;
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
