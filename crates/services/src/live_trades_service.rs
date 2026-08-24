//! Service for persisting live (Binance) order fills/metadata.
//!
//! Records each Binance order placed through the app (strategy link + fill
//! price/qty) so the UI can show strategy info and compute real P&L locally,
//! avoiding per-asset Binance re-queries (rate-limit avoidance).
use crate::error::{ServiceError, ServiceResult};
use data_layer::{LiveTrade, LiveTradesRepository};
use rust_decimal::Decimal;
use std::sync::Arc;

#[derive(Clone)]
pub struct LiveTradesService {
    repo: Option<Arc<LiveTradesRepository>>,
}

impl LiveTradesService {
    pub fn new(repo: Option<Arc<LiveTradesRepository>>) -> Self {
        Self { repo }
    }

    fn repo_or_err(&self, what: &str) -> ServiceResult<Arc<LiveTradesRepository>> {
        self.repo.clone().ok_or_else(|| {
            ServiceError::Other(format!("live_trades store not available (no database): {}", what))
        })
    }

    /// Upsert a live trade record.
    pub async fn record(
        &self,
        order_id: i64,
        symbol: &str,
        strategy_id: Option<&str>,
        side: &str,
        price: Decimal,
        quantity: Decimal,
        filled_quantity: Decimal,
        status: &str,
    ) -> ServiceResult<()> {
        let repo = self.repo_or_err("record")?;
        repo.upsert(
            order_id,
            symbol,
            strategy_id.unwrap_or(""),
            side,
            price,
            quantity,
            filled_quantity,
            status,
        )
        .await
        .map_err(|e| ServiceError::Other(e.to_string()))
    }

    /// All live trades, newest first.
    pub async fn list(&self) -> ServiceResult<Vec<LiveTrade>> {
        let repo = self.repo_or_err("list")?;
        repo.list_all().await.map_err(|e| ServiceError::Other(e.to_string()))
    }

    /// Update only status / filled quantity (preserve strategy link).
    pub async fn update_status(
        &self,
        order_id: i64,
        status: &str,
        filled_quantity: Decimal,
    ) -> ServiceResult<()> {
        let repo = self.repo_or_err("update_status")?;
        repo.update_status(order_id, status, filled_quantity)
            .await
            .map_err(|e| ServiceError::Other(e.to_string()))
    }
}
