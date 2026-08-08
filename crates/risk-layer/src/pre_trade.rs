use quant_common::config::RiskConfig;
use quant_common::types::{Account, Order, Position};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use tracing::{info, instrument, warn};

const DEFAULT_MAX_CONCENTRATION: f64 = 0.2;

/// 事前风控检查器
pub struct PreTradeRiskChecker {
    config: RiskConfig,
}

impl PreTradeRiskChecker {
    pub fn new(config: RiskConfig) -> Self {
        Self { config }
    }

    /// 检查订单是否通过风控
    #[instrument(skip(self), fields(risk_check = "pre_trade"))]
    pub fn check_order(
        &self,
        order: &Order,
        account: &Account,
        positions: &[Position],
    ) -> Result<()> {
        self.check_order_with_reference_price(order, account, positions, None)
    }

    #[instrument(skip(self), fields(risk_check = "pre_trade"))]
    pub fn check_order_with_reference_price(
        &self,
        order: &Order,
        account: &Account,
        positions: &[Position],
        reference_price: Option<Decimal>,
    ) -> Result<()> {
        let order_price = self.resolve_order_price(order, reference_price)?;

        // 1. 资金检查
        self.check_available_cash(order, account, order_price)?;

        // 2. 持仓限制检查
        self.check_position_limit(order, positions)?;

        // 3. 单日亏损限制
        self.check_daily_loss_limit(account)?;

        // 4. 集中度检查
        self.check_concentration_risk(order, positions, account, order_price)?;

        info!("Order passed pre-trade risk check: {}", order.order_id);
        Ok(())
    }

    fn resolve_order_price(
        &self,
        order: &Order,
        reference_price: Option<Decimal>,
    ) -> Result<Decimal> {
        if let Some(price) = order.price {
            return Ok(price);
        }

        if matches!(order.order_type, quant_common::types::OrderType::Market) {
            if let Some(price) = reference_price {
                return Ok(price);
            }
            return Err(Error::RiskControl(
                "Market orders require a reference price for risk checks".to_string(),
            ));
        }

        Err(Error::RiskControl(
            "Order price is required for pre-trade risk checks".to_string(),
        ))
    }

    /// 检查可用资金
    fn check_available_cash(
        &self,
        order: &Order,
        account: &Account,
        order_price: Decimal,
    ) -> Result<()> {
        let required_cash = match order.side {
            quant_common::types::OrderSide::Buy => order_price * order.quantity,
            quant_common::types::OrderSide::Sell => Decimal::ZERO,
        };

        if account.available_cash < required_cash {
            warn!(
                "Insufficient cash. Required: {}, Available: {}",
                required_cash, account.available_cash
            );
            return Err(Error::RiskControl(format!(
                "Insufficient cash. Required: {}, Available: {}",
                required_cash, account.available_cash
            )));
        }

        Ok(())
    }

    /// 检查持仓限制
    fn check_position_limit(&self, order: &Order, positions: &[Position]) -> Result<()> {
        let current_position = positions
            .iter()
            .find(|p| p.symbol == order.symbol)
            .map(|p| p.quantity)
            .unwrap_or(Decimal::ZERO);

        let new_position = match order.side {
            quant_common::types::OrderSide::Buy => current_position + order.quantity,
            quant_common::types::OrderSide::Sell => current_position - order.quantity,
        };

        let max_position =
            Decimal::from_f64_retain(self.config.max_position_size).unwrap_or(Decimal::ZERO);

        if new_position.abs() > max_position {
            warn!(
                "Position limit exceeded. Symbol: {}, New position: {}, Max: {}",
                order.symbol, new_position, max_position
            );
            return Err(Error::RiskControl(format!(
                "Position limit exceeded for symbol: {}",
                order.symbol
            )));
        }

        Ok(())
    }

    /// 检查当日亏损限制
    fn check_daily_loss_limit(&self, account: &Account) -> Result<()> {
        let max_daily_loss =
            Decimal::from_f64_retain(self.config.max_daily_loss).unwrap_or(Decimal::ZERO);

        if account.daily_pnl < -max_daily_loss {
            warn!(
                "Daily loss limit exceeded. Current loss: {}, Max: {}",
                account.daily_pnl, max_daily_loss
            );
            return Err(Error::RiskControl("Daily loss limit exceeded".to_string()));
        }

        Ok(())
    }

    /// 检查集中度风险
    fn check_concentration_risk(
        &self,
        order: &Order,
        positions: &[Position],
        account: &Account,
        order_price: Decimal,
    ) -> Result<()> {
        let order_value = order_price * order.quantity;

        let current_position_value = positions
            .iter()
            .find(|p| p.symbol == order.symbol)
            .map(|p| p.market_value)
            .unwrap_or(Decimal::ZERO);

        let new_position_value = match order.side {
            quant_common::types::OrderSide::Buy => current_position_value + order_value,
            quant_common::types::OrderSide::Sell => current_position_value - order_value,
        };

        if account.total_assets <= Decimal::ZERO {
            return Err(Error::RiskControl(
                "Account total assets must be positive".to_string(),
            ));
        }

        let concentration_ratio = new_position_value / account.total_assets;
        let max_concentration = Decimal::from_f64_retain(self.config.max_concentration)
            .unwrap_or(Decimal::from_f64_retain(DEFAULT_MAX_CONCENTRATION).unwrap());

        if concentration_ratio > max_concentration {
            warn!(
                "Concentration risk too high. Symbol: {}, Ratio: {}, Max: {}",
                order.symbol, concentration_ratio, max_concentration
            );
            return Err(Error::RiskControl(format!(
                "Concentration risk too high for symbol: {}",
                order.symbol
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use quant_common::types::{OrderSide, OrderType};
    use rust_decimal_macros::dec;
    #[test]
    fn test_cash_check() {
        let config = RiskConfig {
            max_position_size: 0.2,
            max_daily_loss: 0.05,
            max_drawdown: 0.15,
            max_concentration: 0.2,
            enable_pre_trade_check: true,
            enable_real_time_monitor: true,
            var_confidence_level: 0.95,
        };

        let checker = PreTradeRiskChecker::new(config);

        let order = Order {
            order_id: 0,
            strategy_id: "test".to_string(),
            symbol: "TEST".to_string(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: Some(dec!(100)),
            quantity: dec!(10),
            filled_quantity: dec!(0),
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: dec!(0),
            slippage: dec!(0),
        };

        let account = Account {
            account_id: 0,
            total_assets: dec!(10000),
            available_cash: dec!(500), // Insufficient
            frozen_cash: dec!(0),
            market_value: dec!(0),
            total_pnl: dec!(0),
            daily_pnl: dec!(0),
            margin: dec!(0),
            margin_ratio: dec!(0),
            updated_at: Utc::now(),
        };

        let result = checker.check_available_cash(&order, &account, dec!(100));
        assert!(result.is_err());
    }

    #[test]
    fn test_market_order_without_reference_price_is_rejected() {
        let config = RiskConfig {
            max_position_size: 0.2,
            max_daily_loss: 0.05,
            max_drawdown: 0.15,
            max_concentration: 0.2,
            enable_pre_trade_check: true,
            enable_real_time_monitor: true,
            var_confidence_level: 0.95,
        };

        let checker = PreTradeRiskChecker::new(config);
        let order = Order {
            order_id: 0,
            strategy_id: "test".to_string(),
            symbol: "TEST".to_string(),
            order_type: OrderType::Market,
            side: OrderSide::Buy,
            price: None,
            quantity: dec!(10),
            filled_quantity: dec!(0),
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: dec!(0),
            slippage: dec!(0),
        };
        let account = Account {
            account_id: 0,
            total_assets: dec!(10000),
            available_cash: dec!(100000),
            frozen_cash: dec!(0),
            market_value: dec!(0),
            total_pnl: dec!(0),
            daily_pnl: dec!(0),
            margin: dec!(0),
            margin_ratio: dec!(0),
            updated_at: Utc::now(),
        };

        let result = checker.check_order_with_reference_price(&order, &account, &[], None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reference price"));
    }
}
