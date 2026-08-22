use chrono::{DateTime, Utc};
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

/// 信号来源
#[derive(Debug, Clone, PartialEq)]
pub enum SignalSource {
    Strategy,
    Manual,
    Webhook,
    Scheduled,
}

/// 交易信号
#[derive(Debug, Clone)]
pub struct Signal {
    pub signal_type: SignalType,
    pub symbol: String,
    pub strength: f64, // 信号强度 0.0-1.0
    pub price: Option<Decimal>,
    pub quantity: Option<Decimal>,
    // 流水线/调度所需元数据
    pub id: String,
    pub strategy_id: String,
    pub source: SignalSource,
    pub generated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

impl Signal {
    fn generate_id() -> String {
        let u = Uuid::new_v4();
        format!("sig-{}", u)
    }
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
            order_id: 0,
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
            id: Signal::generate_id(),
            strategy_id: String::new(),
            source: SignalSource::Strategy,
            generated_at: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }

    #[instrument(fields(symbol = %symbol, macd = %macd, signal_line = %signal, price = %price))]
    pub fn from_macd(
        macd: Decimal,
        signal: Decimal,
        prev_histogram: Decimal,
        symbol: String,
        price: Decimal,
    ) -> Signal {
        let histogram = macd - signal;

        // 交叉事件判定：柱状图由负转正（金叉）或由正转负（死叉），
        // 而非“柱状图持续为正/负”的状态，避免在趋势中重复发信号。
        let signal_type = if prev_histogram <= Decimal::ZERO && histogram > Decimal::ZERO {
            SignalType::Buy // 金叉
        } else if prev_histogram >= Decimal::ZERO && histogram < Decimal::ZERO {
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
            id: Signal::generate_id(),
            strategy_id: String::new(),
            source: SignalSource::Strategy,
            generated_at: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_rsi_oversold_returns_buy() {
        let signal = SignalGenerator::from_rsi(
            Decimal::from(25), // RSI < 30 → oversold → Buy
            "BTC/USDT".to_string(),
            Decimal::from(100),
        );
        assert_eq!(signal.signal_type, SignalType::Buy);
        assert!(signal.strength > 0.0);
    }

    #[test]
    fn test_from_rsi_overbought_returns_sell() {
        let signal = SignalGenerator::from_rsi(
            Decimal::from(80), // RSI > 70 → overbought → Sell
            "BTC/USDT".to_string(),
            Decimal::from(100),
        );
        assert_eq!(signal.signal_type, SignalType::Sell);
        assert!(signal.strength > 0.0);
    }

    #[test]
    fn test_from_rsi_normal_returns_hold() {
        let signal = SignalGenerator::from_rsi(
            Decimal::from(50), // 30 <= RSI <= 70 → Hold
            "BTC/USDT".to_string(),
            Decimal::from(100),
        );
        assert_eq!(signal.signal_type, SignalType::Hold);
        assert_eq!(signal.strength, 0.0);
    }

    #[test]
    fn test_from_macd_golden_cross_returns_buy() {
        let signal = SignalGenerator::from_macd(
            Decimal::from(10), // MACD > Signal → 柱状图 5
            Decimal::from(5),
            Decimal::from(-1), // 上一柱状图为负 → 金叉
            "BTC/USDT".to_string(),
            Decimal::from(100),
        );
        assert_eq!(signal.signal_type, SignalType::Buy);
    }

    #[test]
    fn test_from_macd_death_cross_returns_sell() {
        let signal = SignalGenerator::from_macd(
            Decimal::from(5), // MACD < Signal → 柱状图 -5
            Decimal::from(10),
            Decimal::from(1), // 上一柱状图为正 → 死叉
            "BTC/USDT".to_string(),
            Decimal::from(100),
        );
        assert_eq!(signal.signal_type, SignalType::Sell);
    }

    #[test]
    fn test_from_macd_neutral_returns_hold() {
        let signal = SignalGenerator::from_macd(
            Decimal::from(10), // MACD == Signal → 柱状图 0
            Decimal::from(10),
            Decimal::from(1),
            "BTC/USDT".to_string(),
            Decimal::from(100),
        );
        assert_eq!(signal.signal_type, SignalType::Hold);
    }

    #[test]
    fn test_from_macd_positive_but_no_cross_returns_hold() {
        let signal = SignalGenerator::from_macd(
            Decimal::from(10), // 柱状图 5
            Decimal::from(5),
            Decimal::from(3), // 上一柱状图已为正 → 未发生交叉
            "BTC/USDT".to_string(),
            Decimal::from(100),
        );
        assert_eq!(signal.signal_type, SignalType::Hold);
    }

    #[test]
    fn test_signal_to_order_buy() {
        let signal = Signal {
            signal_type: SignalType::Buy,
            symbol: "BTC/USDT".to_string(),
            strength: 0.8,
            price: Some(Decimal::from(100)),
            quantity: Some(Decimal::from(10)),
            id: "sig-test-001".to_string(),
            strategy_id: "test_strategy".to_string(),
            source: SignalSource::Strategy,
            generated_at: Utc::now(),
            metadata: serde_json::json!({}),
        };
        let order = signal.to_order("test_strategy").unwrap();
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.symbol, "BTC/USDT");
        assert_eq!(order.price, Some(Decimal::from(100)));
        assert_eq!(order.quantity, Decimal::from(10));
    }

    #[test]
    fn test_signal_to_order_hold_returns_none() {
        let signal = Signal {
            signal_type: SignalType::Hold,
            symbol: "BTC/USDT".to_string(),
            strength: 0.0,
            price: None,
            quantity: None,
            id: "sig-test-002".to_string(),
            strategy_id: "test_strategy".to_string(),
            source: SignalSource::Strategy,
            generated_at: Utc::now(),
            metadata: serde_json::json!({}),
        };
        assert!(signal.to_order("test_strategy").is_none());
    }
}
