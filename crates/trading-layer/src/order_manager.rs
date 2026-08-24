use chrono::Utc;
use quant_common::types::{Order, OrderStatus};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};

/// 订单管理器
#[derive(Clone)]
pub struct OrderManager {
    orders: Arc<RwLock<HashMap<i64, Order>>>,
}

impl Default for OrderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderManager {
    pub fn new() -> Self {
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 提交订单
    #[instrument(skip(self), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side))]
    pub async fn submit_order(&self, mut order: Order) -> Result<i64> {
        // 验证订单
        self.validate_order(&order)?;

        let mut orders = self.orders.write().await;
        let order_id = Self::allocate_order_id(&orders, order.order_id);
        order.order_id = order_id;
        order.status = OrderStatus::Submitted;

        orders.insert(order_id, order);

        info!("Order submitted: {}", order_id);
        Ok(order_id)
    }

    fn allocate_order_id(orders: &HashMap<i64, Order>, requested_order_id: i64) -> i64 {
        if requested_order_id > 0 && !orders.contains_key(&requested_order_id) {
            return requested_order_id;
        }

        let mut candidate = Utc::now().timestamp_nanos_opt().unwrap_or_default();
        let mut attempt = 0;
        while candidate <= 0 || orders.contains_key(&candidate) {
            attempt += 1;
            candidate = Utc::now().timestamp_nanos_opt().unwrap_or_default() + attempt;
        }

        candidate
    }

    /// 更新订单状态
    #[instrument(skip(self), fields(order_id = %order_id, status = ?status))]
    pub async fn update_order_status(&self, order_id: i64, status: OrderStatus) -> Result<()> {
        let mut orders = self.orders.write().await;

        if let Some(order) = orders.get_mut(&order_id) {
            order.status = status.clone();
            order.updated_at = chrono::Utc::now();
            info!("Order {} updated to status: {:?}", order_id, status);
            Ok(())
        } else {
            warn!("Order not found: {}", order_id);
            Err(Error::NotFound(format!("Order not found: {}", order_id)))
        }
    }

    /// 获取订单
    #[instrument(skip(self), fields(order_id = %order_id))]
    pub async fn get_order(&self, order_id: i64) -> Result<Order> {
        let orders = self.orders.read().await;
        match orders.get(&order_id) {
            Some(order) => Ok(order.clone()),
            None => {
                warn!("Order not found: {}", order_id);
                Err(Error::NotFound(format!("Order not found: {}", order_id)))
            }
        }
    }

    /// 获取所有活跃订单
    pub async fn get_active_orders(&self) -> Vec<Order> {
        let orders = self.orders.read().await;
        orders
            .values()
            .filter(|o| {
                matches!(
                    o.status,
                    OrderStatus::Submitted | OrderStatus::PartiallyFilled
                )
            })
            .cloned()
            .collect()
    }

    /// 撤销订单
    #[instrument(skip(self), fields(order_id = %order_id))]
    pub async fn cancel_order(&self, order_id: i64) -> Result<()> {
        info!("Cancelling order: {}", order_id);
        self.update_order_status(order_id, OrderStatus::Cancelled)
            .await
    }

    /// 批量撤销订单
    pub async fn cancel_all_orders(&self, strategy_id: Option<String>) -> Result<usize> {
        let mut orders = self.orders.write().await;
        let mut cancelled_count = 0;

        for order in orders.values_mut() {
            if let Some(ref sid) = strategy_id {
                if &order.strategy_id != sid {
                    continue;
                }
            }

            if matches!(
                order.status,
                OrderStatus::Submitted | OrderStatus::PartiallyFilled
            ) {
                order.status = OrderStatus::Cancelled;
                order.updated_at = chrono::Utc::now();
                cancelled_count += 1;
            }
        }

        info!("Cancelled {} orders", cancelled_count);
        Ok(cancelled_count)
    }

    /// 验证订单
    fn validate_order(&self, order: &Order) -> Result<()> {
        if order.quantity <= Decimal::ZERO {
            return Err(Error::Validation(
                "Order quantity must be positive".to_string(),
            ));
        }

        if let Some(price) = order.price {
            if price <= Decimal::ZERO {
                return Err(Error::Validation(
                    "Order price must be positive".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::types::{OrderSide, OrderType};
    use rust_decimal_macros::dec;
    #[tokio::test]
    async fn test_order_manager() {
        let manager = OrderManager::new();

        let order = Order { order_id: 0,
        strategy_id: "test_strategy".to_string(),
        symbol: "TEST".to_string(),
        order_type: OrderType::Limit,
        side: OrderSide::Buy,
        price: Some(dec!(100)),
        quantity: dec!(10),
        filled_quantity: dec!(0),
        status: OrderStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        commission: dec!(0),
        slippage: dec!(0), exchange: "paper".to_string(), };

        let order_id = manager.submit_order(order).await.unwrap();
        let retrieved_order = manager.get_order(order_id).await.unwrap();

        assert!(order_id > 0);
        assert_eq!(retrieved_order.status, OrderStatus::Submitted);
        assert_eq!(retrieved_order.order_id, order_id);
    }
}
