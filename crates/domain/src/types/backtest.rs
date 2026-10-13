//! 回测结果模块

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 回测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub id: Option<i64>,
    pub strategy_id: String,
    #[serde(default)]
    pub strategy_name: Option<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub final_capital: Decimal,
    pub total_return: Decimal,
    pub annual_return: Decimal,
    pub sharpe_ratio: Decimal,
    pub max_drawdown: Decimal,
    pub win_rate: Decimal,
    pub profit_loss_ratio: Decimal,
    pub total_trades: i32,
    pub winning_trades: i32,
    pub losing_trades: i32,
    pub equity_curve: Vec<(DateTime<Utc>, Decimal)>,
    #[serde(default)]
    pub trades: Vec<BacktestTrade>,
}

/// 回测交易记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestTrade {
    pub date: DateTime<Utc>,
    pub symbol: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub commission: Decimal,
}

impl BacktestResult {
    /// Profit/Loss ratio (revenue / cost).
    pub fn profit_factor(&self) -> Decimal {
        let losses = self.total_trades - self.winning_trades;
        if losses == 0 {
            return if self.winning_trades > 0 {
                Decimal::MAX
            } else {
                Decimal::ZERO
            };
        }
        Decimal::from(self.winning_trades) / Decimal::from(losses)
    }

    /// Net profit in absolute terms.
    pub fn net_profit(&self) -> Decimal {
        self.final_capital - self.initial_capital
    }

    /// Duration of the backtest in days.
    pub fn duration_days(&self) -> i64 {
        (self.end_date - self.start_date).num_days()
    }
}
