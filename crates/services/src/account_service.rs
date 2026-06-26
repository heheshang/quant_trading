use crate::error::{ServiceError, ServiceResult};
use quant_common::types::{Account, Order, Position};
use quant_repository::PostgresClient;
use sqlx::Row;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// Account and position query service.
pub struct AccountService {
    postgres: Option<Arc<PostgresClient>>,
}

impl AccountService {
    pub fn new(postgres: Option<Arc<PostgresClient>>) -> Self {
        Self { postgres }
    }

    #[instrument(skip_all)]
    pub async fn get_account_info(&self) -> ServiceResult<Account> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let row = sqlx::query(
            r#"
            SELECT account_id, total_assets, available_cash, frozen_cash,
                   market_value, total_pnl, daily_pnl, margin, margin_ratio, updated_at
            FROM accounts
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch account info: {}", e);
            ServiceError::from(e)
        })?
        .ok_or_else(|| {
            error!("No account found");
            ServiceError::NotFound("No account found".into())
        })?;

        info!(account_id = %row.get::<i64, _>("account_id"), "Account info retrieved");
        Ok(Account {
            account_id: row.get("account_id"),
            total_assets: row.get("total_assets"),
            available_cash: row.get("available_cash"),
            frozen_cash: row.get("frozen_cash"),
            market_value: row.get("market_value"),
            total_pnl: row.get("total_pnl"),
            daily_pnl: row.get("daily_pnl"),
            margin: row.get("margin"),
            margin_ratio: row.get("margin_ratio"),
            updated_at: row.get("updated_at"),
        })
    }

    #[instrument(skip_all)]
    pub async fn get_active_orders(&self) -> ServiceResult<Vec<Order>> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let rows = sqlx::query(
            r#"
            SELECT order_id, strategy_id, symbol, order_type, side, price,
                   quantity, filled_quantity, commission, slippage, status,
                   created_at, updated_at
            FROM orders
            WHERE status NOT IN ('Filled', 'Cancelled', 'Rejected')
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch active orders: {}", e);
            ServiceError::from(e)
        })?;

        let orders =
            rows.iter()
                .map(|row| -> ServiceResult<Order> {
                    let status_str: String = row.get("status");
                    let otype_str: String = row.get("order_type");
                    let side_str: String = row.get("side");

                    let status = serde_json::from_value(serde_json::Value::String(status_str))
                        .map_err(|e| ServiceError::Deserialization {
                            field: "status",
                            source: e,
                        })?;
                    let order_type = serde_json::from_value(serde_json::Value::String(otype_str))
                        .map_err(|e| ServiceError::Deserialization {
                        field: "order_type",
                        source: e,
                    })?;
                    let side = serde_json::from_value(serde_json::Value::String(side_str))
                        .map_err(|e| ServiceError::Deserialization {
                            field: "side",
                            source: e,
                        })?;

                    Ok(Order {
                        order_id: row.get("order_id"),
                        strategy_id: row.get("strategy_id"),
                        symbol: row.get("symbol"),
                        order_type,
                        side,
                        price: row.get("price"),
                        quantity: row.get("quantity"),
                        filled_quantity: row.get("filled_quantity"),
                        commission: row.get("commission"),
                        slippage: row.get("slippage"),
                        status,
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                    })
                })
                .collect::<ServiceResult<Vec<_>>>()?;

        info!(count = orders.len(), "Active orders retrieved");
        Ok(orders)
    }

    pub async fn persist_order(&self, order: &Order, account_id: &i64) -> ServiceResult<()> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let order_type_str = serde_json::to_value(&order.order_type)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Limit".to_string());
        let side_str = serde_json::to_value(&order.side)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Buy".to_string());
        let status_str = serde_json::to_value(&order.status)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Pending".to_string());

        sqlx::query(
            r#"
            INSERT INTO orders (order_id, account_id, strategy_id, symbol, order_type, side,
                                price, quantity, filled_quantity, commission, slippage, status,
                                created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (order_id) DO NOTHING
            "#,
        )
        .bind(order.order_id)
        .bind(account_id)
        .bind(&order.strategy_id)
        .bind(&order.symbol)
        .bind(&order_type_str)
        .bind(&side_str)
        .bind(order.price)
        .bind(order.quantity)
        .bind(order.filled_quantity)
        .bind(order.commission)
        .bind(order.slippage)
        .bind(&status_str)
        .bind(order.created_at)
        .bind(order.updated_at)
        .execute(pool)
        .await
        .map_err(ServiceError::Database)?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn get_positions(&self) -> ServiceResult<Vec<Position>> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let rows = sqlx::query(
            r#"
            SELECT symbol, quantity, available_quantity, avg_price,
                   market_value, unrealized_pnl, realized_pnl, updated_at
            FROM positions
            ORDER BY market_value DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch positions: {}", e);
            ServiceError::from(e)
        })?;

        let positions: Vec<Position> = rows
            .iter()
            .map(|row| Position {
                symbol: row.get("symbol"),
                quantity: row.get("quantity"),
                available_quantity: row.get("available_quantity"),
                avg_price: row.get("avg_price"),
                market_value: row.get("market_value"),
                unrealized_pnl: row.get("unrealized_pnl"),
                realized_pnl: row.get("realized_pnl"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        info!(count = positions.len(), "Positions retrieved");
        Ok(positions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::types::{OrderSide, OrderStatus, OrderType};

    #[tokio::test]
    async fn test_get_account_info_no_db() {
        let svc = AccountService::new(None);
        let result = svc.get_account_info().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_get_active_orders_no_db() {
        let svc = AccountService::new(None);
        let result = svc.get_active_orders().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_persist_order_no_db() {
        let svc = AccountService::new(None);
        let order = Order {
            order_id: 0,
            strategy_id: "strat_1".into(),
            symbol: "BTC-USDT".into(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: None,
            quantity: rust_decimal::Decimal::new(1, 0),
            filled_quantity: rust_decimal::Decimal::ZERO,
            status: OrderStatus::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            commission: rust_decimal::Decimal::ZERO,
            slippage: rust_decimal::Decimal::ZERO,
        };
        let result = svc.persist_order(&order, &0).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_get_positions_no_db() {
        let svc = AccountService::new(None);
        let result = svc.get_positions().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[test]
    fn test_deserialize_invalid_status_returns_error() {
        // Verify that deserializing an unknown status string produces an error,
        // NOT a silent fallback to a default value.
        let result = serde_json::from_value::<OrderStatus>(serde_json::Value::String(
            "InvalidStatus".into(),
        ));
        assert!(
            result.is_err(),
            "Unknown status should return Err, not fallback"
        );

        let result =
            serde_json::from_value::<OrderType>(serde_json::Value::String("UnknownType".into()));
        assert!(
            result.is_err(),
            "Unknown order type should return Err, not fallback"
        );

        let result = serde_json::from_value::<OrderSide>(serde_json::Value::String(
            "NeitherBuyNorSell".into(),
        ));
        assert!(
            result.is_err(),
            "Unknown side should return Err, not fallback"
        );
    }
}
