use chrono::{DateTime, Duration, Utc};
use quant_common::types::{Order, OrderSide, OrderType};
use quant_common::{Error, Result};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use tracing::{info, instrument};

/// TWAP算法参数
#[derive(Debug)]
pub struct TWAPParams {
    pub total_quantity: Decimal,
    pub duration_minutes: i64,
    pub num_slices: usize,
}

/// VWAP算法参数
#[derive(Debug)]
pub struct VWAPParams {
    pub total_quantity: Decimal,
    pub duration_minutes: i64,
}

/// 智能算法订单拆分器
pub struct AlgorithmicOrderSlicer;

impl AlgorithmicOrderSlicer {
    /// TWAP算法 - 时间加权平均价格
    /// 将大单均匀分割成小单，在时间上均匀分布
    #[instrument(skip(params), fields(symbol = %symbol, slices = params.num_slices))]
    pub fn twap(
        symbol: String,
        side: OrderSide,
        params: TWAPParams,
        start_time: DateTime<Utc>,
    ) -> Result<Vec<Order>> {
        if params.num_slices == 0 {
            return Err(Error::Validation(
                "Number of slices must be > 0".to_string(),
            ));
        }

        let slice_quantity = params.total_quantity / Decimal::from(params.num_slices);
        let total_secs = params.duration_minutes * 60;

        let mut orders = Vec::new();
        let n = params.num_slices;
        for i in 0..n {
            // 时间按比例分布（避免整数截断漂移），i=n 时正好铺满 duration。
            let offset_secs = if n > 1 {
                total_secs * i as i64 / n as i64
            } else {
                0
            };
            let t = start_time + Duration::seconds(offset_secs);
            // 末片吸收整除余量，保证总量精确等于 total_quantity。
            let quantity = if i == n - 1 {
                params.total_quantity - slice_quantity * Decimal::from(n as i64 - 1)
            } else {
                slice_quantity
            };
            let order = Order { order_id: 0,
                strategy_id: format!("TWAP_{}", i),
                symbol: symbol.clone(),
                order_type: OrderType::Market,
                side: side.clone(),
                price: None,
                quantity,
                filled_quantity: Decimal::ZERO,
                status: quant_common::types::OrderStatus::Pending,
                created_at: t,
                updated_at: t,
                commission: Decimal::ZERO,
                slippage: Decimal::ZERO, exchange: "algorithm".to_string(), };

            orders.push(order);
        }

        info!("TWAP sliced into {} orders", orders.len());
        Ok(orders)
    }

    /// VWAP算法 - 成交量加权平均价格
    /// 根据历史成交量分布来分配订单量
    #[instrument(skip(volume_profile), fields(symbol = %symbol, total_quantity = %params.total_quantity))]
    pub fn vwap(
        symbol: String,
        side: OrderSide,
        params: VWAPParams,
        volume_profile: Vec<(DateTime<Utc>, Decimal)>,
    ) -> Result<Vec<Order>> {
        if volume_profile.is_empty() {
            return Err(Error::Validation(
                "Volume profile cannot be empty".to_string(),
            ));
        }

        let total_volume: Decimal = volume_profile.iter().map(|(_, v)| v).sum();

        if total_volume == Decimal::ZERO {
            return Err(Error::Validation("Total volume cannot be zero".to_string()));
        }

        let mut orders = Vec::new();
        let mut placed = Decimal::ZERO;

        for (i, (timestamp, volume)) in volume_profile.iter().enumerate() {
            // 末片吸收整除余量，保证总量精确等于 total_quantity。
            let slice_quantity = if i == volume_profile.len() - 1 {
                params.total_quantity - placed
            } else {
                params.total_quantity * (*volume / total_volume)
            };
            placed += slice_quantity;

            let order = Order { order_id: 0,
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
                slippage: Decimal::ZERO, exchange: "algorithm".to_string(), };

            orders.push(order);
        }

        info!("VWAP sliced into {} orders", orders.len());
        Ok(orders)
    }

    /// 冰山订单 - 隐藏大单，分批显示
    #[instrument(skip_all, fields(symbol = %symbol, total = %total_quantity, display = %display_quantity))]
    pub fn iceberg(
        symbol: String,
        side: OrderSide,
        total_quantity: Decimal,
        display_quantity: Decimal,
        price: Option<Decimal>,
    ) -> Result<Vec<Order>> {
        if display_quantity >= total_quantity {
            return Err(Error::Validation(
                "Display quantity must be less than total quantity".to_string(),
            ));
        }

        let num_slices = (total_quantity / display_quantity).ceil();
        let mut orders = Vec::new();
        let mut remaining = total_quantity;

        for i in 0..num_slices.to_usize().unwrap_or(1) {
            let quantity = if remaining >= display_quantity {
                display_quantity
            } else {
                remaining
            };

            let order = Order { order_id: 0,
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
            slippage: Decimal::ZERO, exchange: "algorithm".to_string(), };

            orders.push(order);
            remaining -= quantity;

            if remaining == Decimal::ZERO {
                break;
            }
        }

        info!("Iceberg sliced into {} orders", orders.len());
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

        let orders =
            AlgorithmicOrderSlicer::twap("TEST".to_string(), OrderSide::Buy, params, Utc::now())
                .unwrap();

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
        )
        .unwrap();

        assert_eq!(orders.len(), 10);
        assert!(orders.iter().all(|o| o.quantity == dec!(100)));
    }
}
