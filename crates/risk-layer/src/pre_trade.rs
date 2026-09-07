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
        if order.quantity <= Decimal::ZERO {
            return Err(Error::RiskControl(
                "Order quantity must be positive".to_string(),
            ));
        }

        let order_price = self.resolve_order_price(order, reference_price)?;

        // 1. 资金检查
        self.check_available_cash(order, account, order_price)?;

        // 2. 卖出持仓检查（避免裸卖空）
        self.check_sell_position_available(order, positions)?;

        // 3. 持仓限制检查
        self.check_position_limit(order, positions, account, order_price)?;

        // 4. 单日亏损限制
        self.check_daily_loss_limit(account)?;

        // 5. 集中度检查
        self.check_concentration_risk(order, positions, account, order_price)?;

        info!("Order passed pre-trade risk check: {}", order.order_id);
        Ok(())
    }

    fn resolve_order_price(
        &self,
        order: &Order,
        reference_price: Option<Decimal>,
    ) -> Result<Decimal> {
        let price = if matches!(order.order_type, quant_common::types::OrderType::Market) {
            reference_price.ok_or_else(|| {
                Error::RiskControl(
                    "Market orders require a reference price for risk checks".to_string(),
                )
            })?
        } else {
            order.price.ok_or_else(|| {
                Error::RiskControl("Order price is required for pre-trade risk checks".to_string())
            })?
        };

        if price <= Decimal::ZERO {
            return Err(Error::RiskControl(
                "Order risk-check price must be positive".to_string(),
            ));
        }

        Ok(price)
    }

    /// 检查卖出订单是否有足够的可用持仓（避免裸卖空）
    fn check_sell_position_available(&self, order: &Order, positions: &[Position]) -> Result<()> {
        if order.side != quant_common::types::OrderSide::Sell {
            return Ok(());
        }

        let available = positions
            .iter()
            .find(|p| p.symbol == order.symbol)
            .map(|p| p.available_quantity)
            .unwrap_or(Decimal::ZERO);

        if available < order.quantity {
            return Err(Error::RiskControl(format!(
                "Insufficient position to sell. Symbol: {}, Available: {}, Required: {}",
                order.symbol, available, order.quantity
            )));
        }

        Ok(())
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

    /// 检查持仓限制（max_position_size 为持仓市值占总资产的比例）
    fn check_position_limit(
        &self,
        order: &Order,
        positions: &[Position],
        account: &Account,
        order_price: Decimal,
    ) -> Result<()> {
        if account.total_assets <= Decimal::ZERO {
            return Err(Error::RiskControl(
                "Account total assets must be positive".to_string(),
            ));
        }

        let current_qty = positions
            .iter()
            .find(|p| p.symbol == order.symbol)
            .map(|p| p.quantity)
            .unwrap_or(Decimal::ZERO);

        let new_qty = match order.side {
            quant_common::types::OrderSide::Buy => current_qty + order.quantity,
            quant_common::types::OrderSide::Sell => {
                (current_qty - order.quantity).max(Decimal::ZERO)
            }
        };

        // 持仓限制按市值占比计算，而非直接比较持仓数量；卖出已被 clamp，无需 .abs()。
        let new_position_value = order_price * new_qty;
        let position_ratio = new_position_value / account.total_assets;
        let max_ratio =
            Decimal::from_f64_retain(self.config.max_position_size).unwrap_or(Decimal::ZERO);

        if position_ratio > max_ratio {
            warn!(
                "Position limit exceeded. Symbol: {}, Ratio: {}, Max: {}",
                order.symbol, position_ratio, max_ratio
            );
            return Err(Error::RiskControl(format!(
                "Position limit exceeded for symbol: {}",
                order.symbol
            )));
        }

        Ok(())
    }

    /// 检查当日亏损限制（max_daily_loss 为占总资产的比例）
    fn check_daily_loss_limit(&self, account: &Account) -> Result<()> {
        if account.total_assets <= Decimal::ZERO {
            return Err(Error::RiskControl(
                "Account total assets must be positive".to_string(),
            ));
        }

        let max_daily_loss_ratio =
            Decimal::from_f64_retain(self.config.max_daily_loss).unwrap_or(Decimal::ZERO);
        let max_daily_loss_amount = max_daily_loss_ratio * account.total_assets;

        if account.daily_pnl < -max_daily_loss_amount {
            warn!(
                "Daily loss limit exceeded. Current loss: {}, Max: {}",
                account.daily_pnl, max_daily_loss_amount
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
            quant_common::types::OrderSide::Sell => {
                (current_position_value - order_value).max(Decimal::ZERO)
            }
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
            exchange: "paper".to_string(),
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
            exchange: "paper".to_string(),
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

        for price in [None, Some(dec!(1))] {
            let mut order = order.clone();
            order.price = price;
            assert!(matches!(
                checker.check_order_with_reference_price(&order, &account, &[], None),
                Err(Error::RiskControl(_))
            ));
        }
    }

    fn risk_case() -> (PreTradeRiskChecker, Order, Account) {
        let checker = PreTradeRiskChecker::new(RiskConfig {
            max_position_size: 0.2,
            max_daily_loss: 0.05,
            max_drawdown: 0.15,
            max_concentration: 0.2,
            enable_pre_trade_check: true,
            enable_real_time_monitor: true,
            var_confidence_level: 0.95,
        });
        let order = Order {
            order_id: 0,
            strategy_id: "test".to_string(),
            symbol: "TEST".to_string(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: Some(dec!(100)),
            quantity: dec!(1),
            filled_quantity: Decimal::ZERO,
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: Decimal::ZERO,
            slippage: Decimal::ZERO,
            exchange: "paper".to_string(),
        };
        let account = Account {
            account_id: 0,
            total_assets: dec!(10000),
            available_cash: dec!(500),
            frozen_cash: Decimal::ZERO,
            market_value: Decimal::ZERO,
            total_pnl: Decimal::ZERO,
            daily_pnl: Decimal::ZERO,
            margin: Decimal::ZERO,
            margin_ratio: Decimal::ZERO,
            updated_at: Utc::now(),
        };
        (checker, order, account)
    }

    fn position(quantity: Decimal, available_quantity: Decimal) -> Position {
        Position {
            symbol: "TEST".to_string(),
            quantity,
            available_quantity,
            avg_price: dec!(100),
            market_value: quantity * dec!(100),
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn market_order_cash_check_ignores_attached_price() {
        let (checker, mut order, mut account) = risk_case();
        order.order_type = OrderType::Market;
        order.price = Some(dec!(1));
        order.quantity = dec!(10);

        // The supplied price would cost 10, but the reference price costs 1000.
        assert!(matches!(
            checker.check_order_with_reference_price(&order, &account, &[], Some(dec!(100))),
            Err(Error::RiskControl(_))
        ));
        account.available_cash = dec!(1000);
        assert!(checker
            .check_order_with_reference_price(&order, &account, &[], Some(dec!(100)))
            .is_ok());

        // Even an expensive attached price must not replace the reference price.
        order.price = Some(dec!(100000));
        assert!(checker
            .check_order_with_reference_price(&order, &account, &[], Some(dec!(100)))
            .is_ok());
    }

    #[test]
    fn nonpositive_quantities_are_rejected_for_buys_and_sells() {
        let (checker, mut order, account) = risk_case();
        let positions = [position(dec!(10), dec!(10))];
        for order_type in [OrderType::Market, OrderType::Limit] {
            order.order_type = order_type;
            for side in [OrderSide::Buy, OrderSide::Sell] {
                order.side = side;
                order.quantity = dec!(1);
                assert!(
                    checker
                        .check_order_with_reference_price(
                            &order,
                            &account,
                            &positions,
                            Some(dec!(100)),
                        )
                        .is_ok()
                );
                for quantity in [Decimal::ZERO, dec!(-1)] {
                    order.quantity = quantity;
                    assert!(matches!(
                        checker.check_order_with_reference_price(
                            &order,
                            &account,
                            &positions,
                            Some(dec!(100)),
                        ),
                        Err(Error::RiskControl(_))
                    ));
                }
            }
        }
    }

    #[test]
    fn limit_orders_require_positive_price_even_with_reference() {
        let (checker, mut order, account) = risk_case();
        assert!(checker.check_order(&order, &account, &[]).is_ok());
        for price in [None, Some(Decimal::ZERO), Some(dec!(-1))] {
            order.price = price;
            assert!(matches!(
                checker.check_order_with_reference_price(&order, &account, &[], Some(dec!(100))),
                Err(Error::RiskControl(_))
            ));
        }
    }

    #[test]
    fn market_orders_require_positive_reference_even_with_attached_price() {
        let (checker, mut order, account) = risk_case();
        order.order_type = OrderType::Market;
        for reference in [None, Some(Decimal::ZERO), Some(dec!(-1))] {
            assert!(matches!(
                checker.check_order_with_reference_price(&order, &account, &[], reference),
                Err(Error::RiskControl(_))
            ));
        }
    }

    #[test]
    fn sell_position_limit_includes_locked_quantity() {
        let (mut checker, mut order, account) = risk_case();
        // Isolate the position limit from the separate concentration policy.
        checker.config.max_concentration = 1.0;
        order.side = OrderSide::Sell;
        order.quantity = dec!(5);
        let positions = [position(dec!(30), dec!(10))];

        // 25 units remain, including 20 locked units: 25% exceeds the 20% cap.
        assert!(matches!(
            checker.check_order(&order, &account, &positions),
            Err(Error::RiskControl(_))
        ));
        order.quantity = dec!(10);
        assert!(checker.check_order(&order, &account, &positions).is_ok());
    }

    #[test]
    fn sell_orders_cannot_spend_locked_quantity() {
        let (mut checker, mut order, account) = risk_case();
        checker.config.max_position_size = 1.0;
        checker.config.max_concentration = 1.0;
        order.side = OrderSide::Sell;
        order.quantity = dec!(11);
        let positions = [position(dec!(30), dec!(10))];
        assert!(matches!(
            checker.check_order(&order, &account, &positions),
            Err(Error::RiskControl(_))
        ));
        order.quantity = dec!(10);
        assert!(checker.check_order(&order, &account, &positions).is_ok());
    }

    #[test]
    fn test_daily_loss_limit_uses_ratio_of_total_assets() {
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

        let account = Account {
            account_id: 0,
            total_assets: dec!(10000),
            available_cash: dec!(10000),
            frozen_cash: dec!(0),
            market_value: dec!(0),
            total_pnl: dec!(0),
            daily_pnl: dec!(-100), // 亏损 100（占总资产 1%），未达 5% 阈值
            margin: dec!(0),
            margin_ratio: dec!(0),
            updated_at: Utc::now(),
        };

        // -100 > -(0.05 * 10000) = -500 → 不触发
        assert!(checker.check_daily_loss_limit(&account).is_ok());

        // -600 < -500 → 触发
        let mut account = account;
        account.daily_pnl = dec!(-600);
        assert!(checker.check_daily_loss_limit(&account).is_err());
    }
}
