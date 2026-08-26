use crate::error::{ServiceError, ServiceResult};
use chrono::Utc;
use quant_common::types::{Account, Order, OrderStatus, Position};
use data_layer::PostgresClient;
use rust_decimal::Decimal;
use sqlx::Row;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// 订单状态计数（数据库累计，含历史）。
#[derive(Debug, Clone, Default)]
pub struct OrderCounts {
    pub total: i64,
    pub filled: i64,
    pub cancelled: i64,
    pub rejected: i64,
    pub open: i64,
}

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
    pub async fn get_active_orders(&self, exchange: Option<&str>) -> ServiceResult<Vec<Order>> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let rows = sqlx::query(
            r#"
            SELECT order_id, strategy_id, symbol, order_type, side, price,
                   quantity, filled_quantity, commission, slippage, status,
                   exchange, created_at, updated_at
            FROM orders
            WHERE status NOT IN ('Filled', 'Cancelled', 'Rejected', 'Expired')
              AND ($1::text IS NULL OR exchange = $1)
            ORDER BY created_at DESC
            "#,
        )
        .bind(exchange)
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
                        exchange: row.get("exchange"),
                    })
                })
                .collect::<ServiceResult<Vec<_>>>()?;

        info!(count = orders.len(), "Active orders retrieved");
        Ok(orders)
    }

    /// 最近订单（含已成交/撤单/拒绝），供「最近交易」等按时间倒序展示。
    /// 记录当前账户权益快照（供资产曲线展示）。
    pub async fn record_equity_snapshot(&self, eq: Decimal) -> ServiceResult<()> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();
        sqlx::query(
            "INSERT INTO account_snapshots (ccy, ts, eq) VALUES ('USDT', now(), $1)",
        )
        .bind(eq)
        .execute(pool)
        .await
        .map_err(ServiceError::Database)?;
        Ok(())
    }

    pub async fn get_recent_orders(&self, limit: u32, exchange: Option<&str>) -> ServiceResult<Vec<Order>> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let rows = sqlx::query(
            r#"
            SELECT order_id, strategy_id, symbol, order_type, side, price,
                   quantity, filled_quantity, commission, slippage, status,
                   exchange, created_at, updated_at
            FROM orders
            WHERE ($1::text IS NULL OR exchange = $1)
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(exchange)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch recent orders: {}", e);
            ServiceError::from(e)
        })?;

        let orders = Self::map_order_rows(&rows)?;
        info!(count = orders.len(), "Recent orders retrieved");
        Ok(orders)
    }

    /// 订单状态计数（数据库累计，含历史）。
    #[instrument(skip_all)]
    pub async fn get_order_counts(&self) -> ServiceResult<OrderCounts> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let row = sqlx::query(
            r#"
            SELECT
              COUNT(*) AS total,
              COUNT(*) FILTER (WHERE status = 'Filled') AS filled,
              COUNT(*) FILTER (WHERE status = 'Cancelled') AS cancelled,
              COUNT(*) FILTER (WHERE status = 'Rejected') AS rejected,
              COUNT(*) FILTER (WHERE status IN ('Pending','Submitted','PartiallyFilled')) AS open
            FROM orders
            "#,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!("Failed to count orders: {}", e);
            ServiceError::from(e)
        })?;

        let n = |i: usize| -> i64 { row.try_get::<i64, _>(i).unwrap_or(0) };
        Ok(OrderCounts {
            total: n(0),
            filled: n(1),
            cancelled: n(2),
            rejected: n(3),
            open: n(4),
        })
    }

    /// 最新账户权益（USDT 总权益，来自后台快照写入器，每 60s 记录）。
    #[instrument(skip_all)]
    pub async fn get_latest_equity(&self, ccy: &str) -> ServiceResult<Option<Decimal>> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();
        let row = sqlx::query(
            "SELECT eq FROM account_snapshots WHERE ccy = $1 ORDER BY ts DESC LIMIT 1",
        )
        .bind(ccy)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch latest equity: {}", e);
            ServiceError::from(e)
        })?;
        Ok(row.and_then(|r| r.try_get::<Option<Decimal>, _>("eq").ok().flatten()))
    }

    /// 当日权益差值（今日最新 − 今日起始），与 Dashboard「今日收益」一致。
    /// 使用 UTC 日界（与快照 timestamptz 对齐）。
    #[instrument(skip_all)]
    pub async fn get_today_equity_pnl(&self, ccy: &str) -> ServiceResult<Decimal> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        // 今日 0 点（UTC）后与之前各一条，用于差值。
        let rows = sqlx::query(
            r#"
            SELECT ts, eq
            FROM account_snapshots
            WHERE ccy = $1
              AND ts >= date_trunc('day', now()) - interval '1 day'
            ORDER BY ts ASC
            "#,
        )
        .bind(ccy)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch equity snapshots: {}", e);
            ServiceError::from(e)
        })?;

        let day_start = sqlx::query("SELECT date_trunc('day', now())")
            .fetch_one(pool)
            .await
            .map(|r| r.get::<chrono::DateTime<Utc>, _>("date_trunc"))
            .unwrap_or_else(|_| chrono::Utc::now());

        let mut today_eq: Vec<Decimal> = Vec::new();
        let mut before_eq: Option<Decimal> = None;
        for r in &rows {
            let ts: chrono::DateTime<Utc> = r.get("ts");
            let eq: Decimal = r.get("eq");
            if ts >= day_start {
                today_eq.push(eq);
            } else {
                before_eq = Some(eq);
            }
        }

        if today_eq.len() < 2 && before_eq.is_none() {
            return Ok(Decimal::ZERO);
        }
        let latest = today_eq.last().copied().unwrap_or(Decimal::ZERO);
        let baseline = before_eq.unwrap_or_else(|| today_eq.first().copied().unwrap_or(Decimal::ZERO));
        Ok(latest - baseline)
    }

    /// 把 `orders` 表行映射为 `Order`（共用逻辑，避免重复）。
    fn map_order_rows(rows: &[sqlx::postgres::PgRow]) -> ServiceResult<Vec<Order>> {
        rows.iter()
            .map(|row| -> ServiceResult<Order> {
                let status_str: String = row.get("status");
                let otype_str: String = row.get("order_type");
                let side_str: String = row.get("side");

                let status =
                    serde_json::from_value(serde_json::Value::String(status_str)).map_err(
                        |e| ServiceError::Deserialization {
                            field: "status",
                            source: e,
                        },
                    )?;
                let order_type =
                    serde_json::from_value(serde_json::Value::String(otype_str)).map_err(
                        |e| ServiceError::Deserialization {
                            field: "order_type",
                            source: e,
                        },
                    )?;
                let side = serde_json::from_value(serde_json::Value::String(side_str)).map_err(
                    |e| ServiceError::Deserialization {
                        field: "side",
                        source: e,
                    },
                )?;

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
                    exchange: row.get("exchange"),
                })
            })
            .collect()
    }

    pub async fn persist_order(
        &self,
        order: &Order,
        account_id: &i64,
        exchange: &str,
    ) -> ServiceResult<()> {
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
                                exchange, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
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
        .bind(exchange)
        .bind(order.created_at)
        .bind(order.updated_at)
        .execute(pool)
        .await
        .map_err(ServiceError::Database)?;

        Ok(())
    }

    /// Cancel an order persisted in the database (sets status to 'Cancelled').
    /// 订单执行完成回写终态（成交/部分成交等）；避免重启后 DB 停留在提交状态。
    #[instrument(skip(self), fields(order_id = %order_id, status = ?status))]
    pub async fn update_order_status(
        &self,
        order_id: i64,
        status: OrderStatus,
        filled_quantity: Decimal,
        commission: Decimal,
    ) -> ServiceResult<()> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();
        let status_str = serde_json::to_value(&status)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Submitted".to_string());
        sqlx::query(
            r#"
            UPDATE orders
            SET status = $1, filled_quantity = $2, commission = $3, updated_at = now()
            WHERE order_id = $4
              AND status NOT IN ('Filled', 'Cancelled', 'Rejected', 'Expired')
            "#,
        )
        .bind(&status_str)
        .bind(filled_quantity)
        .bind(commission)
        .bind(order_id)
        .execute(pool)
        .await
        .map_err(ServiceError::Database)?;
        info!(order_id = %order_id, status = %status_str, "Order status updated in DB");
        Ok(())
    }

    /// Cancel an order persisted in the database (sets status to 'Cancelled').
    ///
    /// Only transitions orders that are still active (not already filled,
    /// cancelled, rejected or expired). Returns `NotFound` when the order is
    /// absent or already in a terminal state.
    #[instrument(skip(self), fields(order_id = %order_id))]
    pub async fn cancel_order(&self, order_id: i64) -> ServiceResult<()> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let result = sqlx::query(
            r#"
            UPDATE orders
            SET status = 'Cancelled', updated_at = NOW()
            WHERE order_id = $1
              AND status NOT IN ('Filled', 'Cancelled', 'Rejected', 'Expired')
            "#,
        )
        .bind(order_id)
        .execute(pool)
        .await
        .map_err(ServiceError::Database)?;

        if result.rows_affected() == 0 {
            return Err(ServiceError::NotFound(format!(
                "Active order not found: {}",
                order_id
            )));
        }

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

    /// 解析当前账户 id（与 `get_account_info` 一致：取最近更新的账户）。
    #[instrument(skip_all)]
    async fn current_account_id(&self) -> ServiceResult<i64> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();
        let row = sqlx::query("SELECT account_id FROM accounts ORDER BY updated_at DESC LIMIT 1")
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                error!("Failed to resolve account: {}", e);
                ServiceError::from(e)
            })?
            .ok_or_else(|| {
                error!("No account found");
                ServiceError::NotFound("No account found".into())
            })?;
        Ok(row.get("account_id"))
    }

    /// 币安持仓同步写库。
    ///
    /// 以币安余额/价格为输入，`upsert` 到 `positions` 表（按 `account_id+symbol`）：
    /// - 自动补 `instruments`（币安持仓可能含未预置的币种，FK 需先落标）；
    /// - 已平仓（不在本次快照中）的 symbol 一并删除，保持库与币安一致。
    #[instrument(skip_all)]
    pub async fn upsert_positions(&self, positions: &[Position]) -> ServiceResult<()> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();
        let account_id = self.current_account_id().await?;
        let mut tx = pool.begin().await.map_err(ServiceError::Database)?;

        for p in positions {
            sqlx::query(
                "INSERT INTO instruments (symbol, exchange, instrument_type, tick_size)
                 VALUES ($1, 'BINANCE', 'Spot', 0.01)
                 ON CONFLICT (symbol) DO NOTHING",
            )
            .bind(&p.symbol)
            .execute(&mut *tx)
            .await
            .map_err(ServiceError::Database)?;

            sqlx::query(
                "INSERT INTO positions (account_id, symbol, quantity, available_quantity,
                        avg_price, market_value, unrealized_pnl, realized_pnl, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
                 ON CONFLICT (account_id, symbol) DO UPDATE SET
                        quantity = EXCLUDED.quantity,
                        available_quantity = EXCLUDED.available_quantity,
                        avg_price = EXCLUDED.avg_price,
                        market_value = EXCLUDED.market_value,
                        unrealized_pnl = EXCLUDED.unrealized_pnl,
                        realized_pnl = EXCLUDED.realized_pnl,
                        updated_at = now()",
            )
            .bind(account_id)
            .bind(&p.symbol)
            .bind(p.quantity)
            .bind(p.available_quantity)
            .bind(p.avg_price)
            .bind(p.market_value)
            .bind(p.unrealized_pnl)
            .bind(p.realized_pnl)
            .execute(&mut *tx)
            .await
            .map_err(ServiceError::Database)?;
        }

        // 清理该账户下已不在币安持仓中的 symbol，保持库与币安一致。
        // 仅当同步集非空时才清理：空集合（如瞬时空余额/接口异常）不清空，
        // 避免误删已有持仓（防御性，保持库与币安一致但不因一次空同步丢数据）。
        let symbols: Vec<String> = positions.iter().map(|p| p.symbol.clone()).collect();
        if !symbols.is_empty() {
            sqlx::query(
                "DELETE FROM positions
                 WHERE account_id = $1
                   AND NOT (symbol = ANY($2::text[]))",
            )
            .bind(account_id)
            .bind(&symbols)
            .execute(&mut *tx)
            .await
            .map_err(ServiceError::Database)?;
        }

        tx.commit().await.map_err(ServiceError::Database)?;
        Ok(())
    }

    /// 纸面持仓：由 `orders` 表已成交单净额推导（买 − 卖），供纸面卖单风控校验。
    ///
    /// `positions` 表是静态账本，纸面成交不会更新它；为避免裸卖空误判，纸面
    /// 持仓需从真实成交记录聚合。
    #[instrument(skip_all)]
    pub async fn get_paper_positions(&self) -> ServiceResult<Vec<Position>> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let rows = sqlx::query(
            r#"
            SELECT symbol,
                   SUM(CASE WHEN side = 'Buy' THEN filled_quantity ELSE -filled_quantity END) AS net_qty,
                   SUM(CASE WHEN side = 'Buy' THEN COALESCE(price, 0) * filled_quantity ELSE 0 END) AS buy_cost,
                   SUM(CASE WHEN side = 'Buy' THEN filled_quantity ELSE 0 END) AS buy_qty
            FROM orders
            WHERE status = 'Filled'
            GROUP BY symbol
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch paper positions: {}", e);
            ServiceError::from(e)
        })?;

        let mut positions = Vec::new();
        for r in &rows {
            let net: Decimal = r.get("net_qty");
            if net <= Decimal::ZERO {
                continue;
            }
            let buy_cost: Decimal = r.get("buy_cost");
            let buy_qty: Decimal = r.get("buy_qty");
            let avg = if buy_qty > Decimal::ZERO {
                buy_cost / buy_qty
            } else {
                Decimal::ZERO
            };
            positions.push(Position {
                symbol: r.get("symbol"),
                quantity: net,
                available_quantity: net,
                avg_price: avg,
                market_value: Decimal::ZERO,
                unrealized_pnl: Decimal::ZERO,
                realized_pnl: Decimal::ZERO,
                updated_at: chrono::Utc::now(),
            });
        }

        info!(count = positions.len(), "Paper positions derived");
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
        let result = svc.get_active_orders(None).await;
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
            exchange: "paper".to_string(),
        };
        let result = svc.persist_order(&order, &0, "paper").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_cancel_order_no_db() {
        let svc = AccountService::new(None);
        let result = svc.cancel_order(123).await;
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
