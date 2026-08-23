//! Exchange-agnostic live-execution abstraction.
//!
//! [`crate::binance_executor::BinanceExecutor`] implements [`LiveExchange`], so
//! the execution layer can be driven by the live venue through a single trait
//! object (`Arc<dyn LiveExchange>`).

use quant_common::types::{Order, OrderStatus};
use quant_common::Result;
use rust_decimal::Decimal;

/// Standardized live-order fill details, normalized across exchanges.
///
/// Unifies the exchange-specific order response (Binance `BinanceOrder`) into a
/// single shape the execution strategies consume.
#[derive(Debug, Clone)]
pub struct OrderDetails {
    /// Volume-weighted average fill price; `Zero` when nothing filled.
    pub avg_price: Decimal,
    /// Quantity actually filled.
    pub filled_quantity: Decimal,
    /// Normalized order status.
    pub status: OrderStatus,
    /// Exchange-reported commission. `None` means "unavailable", in which case
    /// the caller estimates from configuration. Binance responses carry no fee
    /// field, so `BinanceExecutor` surfaces `Some(0)`.
    pub fee: Option<Decimal>,
}

/// Exchange-agnostic live-execution seam.
///
/// Implementors translate an app [`Order`] into the venue's order request and
/// normalize the resulting details.
#[async_trait::async_trait]
pub trait LiveExchange: Send + Sync {
    /// Submit `order` to the venue and return the venue's order id as a string.
    async fn execute_order(&self, order: &Order) -> Result<String>;
    /// Cancel an order by its venue order id.
    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<()>;
    /// Fetch normalized fill details for an order.
    async fn get_order_details(&self, symbol: &str, order_id: &str) -> Result<OrderDetails>;
}
