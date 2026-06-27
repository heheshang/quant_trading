use exchange_okx::types::OkxPlaceOrderRequest;
use exchange_okx::{Client as OkxClient, ClientInterface};
use quant_common::types::{Order, OrderSide, OrderStatus, OrderType};
use quant_common::{Error, Result};
use rust_decimal::prelude::ToPrimitive;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument};

/// OKX 订单执行器
#[derive(Clone)]
pub struct OkxExecutor {
    client: Arc<RwLock<OkxClient>>,
}

impl OkxExecutor {
    /// 创建新的 OKX 执行器
    pub fn new(client: Arc<RwLock<OkxClient>>) -> Self {
        Self { client }
    }

    /// 执行订单到 OKX
    #[instrument(skip(self), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side))]
    pub async fn execute_order(&self, order: &Order) -> Result<String> {
        let client = self.client.read().await;

        // 将内部订单类型转换为 OKX 订单请求
        let okx_request = self.convert_order_to_okx(order)?;

        info!(
            "Placing order on OKX: {} {} {} @ {:?}",
            okx_request.side, okx_request.sz, okx_request.inst_id, okx_request.px
        );

        // 提交订单到 OKX
        let okx_order = client.place_order(okx_request).await?;

        info!("OKX order placed successfully: {}", okx_order.ord_id);

        Ok(okx_order.ord_id)
    }

    /// 取消 OKX 订单
    #[instrument(skip(self), fields(inst_id = %inst_id, ord_id = %ord_id))]
    pub async fn cancel_order(&self, inst_id: &str, ord_id: &str) -> Result<()> {
        let client = self.client.read().await;
        client.cancel_order(inst_id, ord_id).await?;
        info!("Cancelled OKX order: {} on {}", ord_id, inst_id);
        Ok(())
    }

    /// 将内部订单转换为 OKX 订单请求
    fn convert_order_to_okx(&self, order: &Order) -> Result<OkxPlaceOrderRequest> {
        // 转换订单方向
        let side = match order.side {
            OrderSide::Buy => "buy".to_string(),
            OrderSide::Sell => "sell".to_string(),
        };

        // 转换订单类型
        let ord_type = match order.order_type {
            OrderType::Market => "market".to_string(),
            OrderType::Limit => "limit".to_string(),
            OrderType::StopLoss | OrderType::StopLimit => {
                return Err(Error::Validation(format!(
                    "Stop order type {:?} not supported by OKX executor",
                    order.order_type
                )));
            }
            OrderType::TWAP | OrderType::VWAP | OrderType::Iceberg => {
                return Err(Error::Validation(format!(
                    "Algorithmic order type {:?} not supported by OKX executor",
                    order.order_type
                )));
            }
        };

        // 转换价格
        let px = order.price.map(|p| {
            p.to_f64()
                .map(|f| f.to_string())
                .unwrap_or_else(|| "0".to_string())
        });

        // 转换数量
        let sz = order
            .quantity
            .to_f64()
            .map(|f| f.to_string())
            .ok_or_else(|| Error::Validation("Invalid order quantity".to_string()))?;

        Ok(OkxPlaceOrderRequest {
            inst_id: order.symbol.clone(),
            td_mode: "cash".to_string(), // 现货交易模式
            side,
            ord_type,
            sz,
            px,
            cl_ord_id: Some(order.order_id.to_string()),
            tag: Some("quant_trading".to_string()),
            pos_side: None,
            ccy: None,
            px_usd: None,
            px_vol: None,
            reduce_only: None,
            tgt_ccy: None,
        })
    }

    /// 查询订单状态（可用于订单跟踪）
    #[instrument(skip(self), fields(inst_id = %_inst_id, ord_id = %_ord_id))]
    pub async fn get_order_status(&self, _inst_id: &str, _ord_id: &str) -> Result<OrderStatus> {
        // This would require implementing get_order in the OKX client
        // For now, return a placeholder
        Ok(OrderStatus::Submitted)
    }
}
