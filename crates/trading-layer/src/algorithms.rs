use quant_common::{Error, Result};
use quant_common::types::{Order, OrderType, OrderSide};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, Duration};

/// TWAP算法参数
pub struct TWAPParams {
    pub total_quantity: Decimal,
    pub duration_minutes: i64,
    pub num_slices: usize,
}

/// VWAP算法参数
pub struct VWAPParams {
    pub total_quantity: Decimal,
    pub duration_minutes: i64,
}

/// 智能算法订单拆分器
pub struct AlgorithmicOrderSlicer;

impl AlgorithmicOrderSlicer {
    /// TWAP算法 - 时间加权平均价格
    /// 将大单均匀分割成小单，在时间上均匀分布
    pub fn twap(
        symbol: String,
        side: OrderSide,
        params: TWAPParams,
        start_time: DateTime<Utc>,
    ) -> Result<Vec<Order>> {
        if params.num_slices == 0 {
            return Err(Error::Validation("Number of slices must be > 0".to_string()));
        }

        let slice_quantity = params.total_quantity / Decimal::from(params.num_slices);
        let time_interval = params.duration_minutes / params.num_slices as i64;

        let mut orders = Vec::new();
        let mut current_time = start_time;

        for i in 0..params.num_slices {
            let order = Order {
                order_id: uuid::Uuid::new_v4(),
                strategy_id: format!("TWAP_{}", i),
                symbol: symbol.clone(),
                order_type: OrderType::Market,
                side: side.clone(),
                price: None,
                quantity: slice_quantity,
                filled_quantity: Decimal::ZERO,
                status: quant_common::types::OrderStatus::Pending,
                created_at: current_time,
                updated_at: current_time,
                commission: Decimal::ZERO,
                slippage: Decimal::ZERO,
            };

            orders.push(order);
            current_time = current_time + Duration::minutes(time_interval);
        }

        Ok(orders)
    }

    /// VWAP算法 - 成交量加权平均价格
    /// 根据历史成交量分布来分配订单量
    pub fn vwap(
        symbol: String,
        side: OrderSide,
        params: VWAPParams,
        volume_profile: Vec<(DateTime<Utc>, Decimal)>,
    ) -> Result<Vec<Order>> {
        if volume_profile.is_empty() {
            return Err(Error::Validation("Volume profile cannot be empty".to_string()));
        }

        let total_volume: Decimal = volume_profile.iter().map(|(_, v)| v).sum();
        
        if total_volume == Decimal::ZERO {
            return Err(Error::Validation("Total volume cannot be zero".to_string()));
        }

        let mut orders = Vec::new();

        for (i, (timestamp, volume)) in volume_profile.iter().enumerate() {
            let volume_ratio = *volume / total_volume;
            let slice_quantity = params.total_quantity * volume_ratio;

            let order = Order {
                order_id: uuid::Uuid::new_v4(),
                strategy_id: format!("VWAP_{}", i),
                symbol: symbol.clone(),
                order_type: OrderType::Market,
                side: side.clone(),
                price: None,
                quantity: slice_quantity,
                filled_quantity: Decimal::ZERO,
                status: quant_common::types::OrderStatus::Pending,
                created_at: *timestamp,
                updated_at: *timestamp,
                commission: Decimal::ZERO,
                slippage: Decimal::ZERO,
            };

            orders.push(order);
        }

        Ok(orders)
    }

    /// 冰山订单 - 隐藏大单，分批显示
    pub fn iceberg(
        symbol: String,
        side: OrderSide,
        total_quantity: Decimal,
        display_quantity: Decimal,
        price: Option<Decimal>,
    ) -> Result<Vec<Order>> {
        if display_quantity >= total_quantity {
            return Err(Error::Validation(
                "Display quantity must be less than total quantity".to_string()
            ));
        }

        let num_slices = (total_quantity / display_quantity).ceil();
        let mut orders = Vec::new();
        let mut remaining = total_quantity;

        for i in 0..num_slices.to_string().parse::<usize>().unwrap_or(1) {
            let quantity = if remaining >= display_quantity {
                display_quantity
            } else {
                remaining
            };

            let order = Order {
                order_id: uuid::Uuid::new_v4(),
                strategy_id: format!("ICEBERG_{}", i),
                symbol: symbol.clone(),
                order_type: OrderType::Limit,
                side: side.clone(),
                price,
                quantity,
                filled_quantity: Decimal::ZERO,
                status: quant_common::types::OrderStatus::Pending,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                commission: Decimal::ZERO,
                slippage: Decimal::ZERO,
            };

            orders.push(order);
            remaining -= quantity;

            if remaining == Decimal::ZERO {
                break;
            }
        }

        Ok(orders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_twap() {
        let params = TWAPParams {
            total_quantity: dec!(1000),
            duration_minutes: 60,
            num_slices: 10,
        };

        let orders = AlgorithmicOrderSlicer::twap(
            "TEST".to_string(),
            OrderSide::Buy,
            params,
            Utc::now(),
        ).unwrap();

        assert_eq!(orders.len(), 10);
        assert_eq!(orders[0].quantity, dec!(100));
    }

    #[test]
    fn test_iceberg() {
        let orders = AlgorithmicOrderSlicer::iceberg(
            "TEST".to_string(),
            OrderSide::Buy,
            dec!(1000),
            dec!(100),
            Some(dec!(50)),
        ).unwrap();

        assert_eq!(orders.len(), 10);
        assert!(orders.iter().all(|o| o.quantity == dec!(100)));
    }
}
