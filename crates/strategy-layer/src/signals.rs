use chrono::Utc;
use quant_common::types::{Order, OrderSide, OrderType};
use rust_decimal::Decimal;
use tracing::{info, instrument};
use uuid::Uuid;

/// 交易信号类型
#[derive(Debug, Clone, PartialEq)]
pub enum SignalType {
    Buy,
    Sell,
    Hold,
}

/// 交易信号
#[derive(Debug, Clone)]
pub struct Signal {
    pub signal_type: SignalType,
    pub symbol: String,
    pub strength: f64, // 信号强度 0.0-1.0
    pub price: Option<Decimal>,
    pub quantity: Option<Decimal>,
}

impl Signal {
    /// 将信号转换为订单
    pub fn to_order(&self, strategy_id: &str) -> Option<Order> {
        let (side, order_type) = match self.signal_type {
            SignalType::Buy => (OrderSide::Buy, OrderType::Limit),
            SignalType::Sell => (OrderSide::Sell, OrderType::Limit),
            SignalType::Hold => return None,
        };

        Some(Order {
            order_id: Uuid::new_v4(),
            strategy_id: strategy_id.to_string(),
            symbol: self.symbol.clone(),
            order_type,
            side,
            price: self.price,
            quantity: self.quantity.unwrap_or(Decimal::ZERO),
            filled_quantity: Decimal::ZERO,
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: Decimal::ZERO,
            slippage: Decimal::ZERO,
        })
    }
}

/// 信号生成器
pub struct SignalGenerator;

impl SignalGenerator {
    #[instrument(fields(symbol = %symbol, rsi = %rsi_value, price = %price))]
    pub fn from_rsi(rsi_value: Decimal, symbol: String, price: Decimal) -> Signal {
        let signal_type = if rsi_value < Decimal::from(30) {
            SignalType::Buy // 超卖
        } else if rsi_value > Decimal::from(70) {
            SignalType::Sell // 超买
        } else {
            SignalType::Hold
        };

        let strength = if signal_type != SignalType::Hold {
            if rsi_value < Decimal::from(30) {
                ((Decimal::from(30) - rsi_value) / Decimal::from(30))
                    .to_string()
                    .parse()
                    .unwrap_or(0.5)
            } else {
                ((rsi_value - Decimal::from(70)) / Decimal::from(30))
                    .to_string()
                    .parse()
                    .unwrap_or(0.5)
            }
        } else {
            0.0
        };

        info!(
            symbol = %symbol,
            rsi = %rsi_value,
            signal = ?signal_type,
            strength,
            "RSI signal generated"
        );

        Signal {
            signal_type,
            symbol,
            strength,
            price: Some(price),
            quantity: None,
        }
    }

    #[instrument(fields(symbol = %symbol, macd = %macd, signal_line = %signal, price = %price))]
    pub fn from_macd(macd: Decimal, signal: Decimal, symbol: String, price: Decimal) -> Signal {
        let histogram = macd - signal;

        let signal_type = if histogram > Decimal::ZERO {
            SignalType::Buy // 金叉
        } else if histogram < Decimal::ZERO {
            SignalType::Sell // 死叉
        } else {
            SignalType::Hold
        };

        let strength: f64 = (histogram.abs() / price).to_string().parse().unwrap_or(0.5);
        let strength = strength.min(1.0);

        info!(
            symbol = %symbol,
            macd = %macd,
            signal_line = %signal,
            histogram = %histogram,
            signal = ?signal_type,
            strength,
            "MACD signal generated"
        );

        Signal {
            signal_type,
            symbol,
            strength,
            price: Some(price),
            quantity: None,
        }
    }
}
