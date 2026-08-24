//! Binance exchange commands.
//!
//! Thin Tauri adapters that delegate to `AppServices.binance_service`.
//! They never touch the Binance client or data layers directly (layering/DIP).

use crate::state::AppState;
use exchange_binance::types::{BinanceOrder, BinancePlaceOrderRequest};
use quant_common::types::{Order, OrderSide, OrderStatus, OrderType};
use rust_decimal::Decimal;
use tauri::State;

/// 把 Binance 实盘单映射为 App `Order`（域格式 symbol / PascalCase 状态）。
fn binance_order_to_app_order(o: &BinanceOrder, strategy_id: Option<&str>) -> Order {
    let side = if o.side == "SELL" { OrderSide::Sell } else { OrderSide::Buy };
    let order_type = if o.order_type == "MARKET" { OrderType::Market } else { OrderType::Limit };
    let status = match o.status.as_str() {
        "NEW" => OrderStatus::Submitted,
        "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
        "FILLED" => OrderStatus::Filled,
        "CANCELED" => OrderStatus::Cancelled,
        "REJECTED" => OrderStatus::Rejected,
        "EXPIRED" => OrderStatus::Expired,
        _ => OrderStatus::Pending,
    };
    let from_ms = |ms: i64| chrono::DateTime::from_timestamp_millis(ms).unwrap_or_else(chrono::Utc::now);
    Order {
        order_id: o.order_id,
        strategy_id: strategy_id.unwrap_or("").to_string(),
        symbol: exchange_binance::from_binance_symbol(&o.symbol),
        order_type,
        side,
        price: Some(o.price),
        quantity: o.orig_qty,
        filled_quantity: o.executed_qty,
        status,
        created_at: from_ms(o.time),
        updated_at: from_ms(o.update_time),
        commission: Decimal::ZERO,
        slippage: Decimal::ZERO,
        exchange: "live".to_string(),
    }
}

/// 把实盘单镜像写入 `orders` 表（活跃单统一从 DB 读）。
async fn mirror_live_order(
    services: &quant_services::AppServices,
    o: &BinanceOrder,
    strategy_id: Option<&str>,
) {
    if let Ok(account) = services.account_service.get_account_info().await {
        let app_order = binance_order_to_app_order(o, strategy_id);
        let _ = services
            .account_service
            .persist_order(&app_order, &account.account_id, "live")
            .await;
    }
}

fn services<'a>(state: &'a State<'_, AppState>) -> Result<&'a quant_services::AppServices, String> {
    state
        .app_services
        .as_ref()
        .ok_or_else(|| "Binance service not initialized (no exchange client)".to_string())
}

#[tauri::command]
pub async fn get_binance_balance(
    state: State<'_, AppState>,
) -> Result<Vec<exchange_binance::types::BinanceBalance>, String> {
    services(&state)?
        .binance_service
        .get_balance()
        .await
        .map_err(|e| e.to_string())
}

/// 全市场价格（`/api/v3/ticker/price`，用于持仓实时价格/市值补全）。
#[tauri::command]
pub async fn get_binance_ticker_prices(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, rust_decimal::Decimal>, String> {
    services(&state)?
        .binance_service
        .get_all_ticker_prices()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_binance_candles(
    state: State<'_, AppState>,
    symbol: String,
    interval: String,
    limit: Option<u32>,
) -> Result<Vec<exchange_binance::types::BinanceKline>, String> {
    services(&state)?
        .binance_service
        .get_candles(&symbol, &interval, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_binance_order_book(
    state: State<'_, AppState>,
    symbol: String,
    limit: Option<u32>,
) -> Result<exchange_binance::types::BinanceOrderBook, String> {
    services(&state)?
        .binance_service
        .get_order_book(&symbol, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn place_binance_order(
    state: State<'_, AppState>,
    request: BinancePlaceOrderRequest,
) -> Result<exchange_binance::types::BinanceOrder, String> {
    let services = services(&state)?;
    let order = services
        .binance_service
        .place_order(request.clone())
        .await
        .map_err(|e| e.to_string())?;
    // 记录 live 单（策略关联 + 成交价/量），供策略显示与真实盈亏计算。
    let _ = services
        .live_trades
        .record(
            order.order_id,
            &exchange_binance::from_binance_symbol(&order.symbol),
            request.strategy_id.as_deref(),
            &order.side,
            order.price,
            order.orig_qty,
            order.executed_qty,
            &order.status,
        )
        .await;
    // 镜像写入 orders 表（活跃单统一从 DB 读）。
    mirror_live_order(services, &order, request.strategy_id.as_deref()).await;
    Ok(order)
}

/// 读取本地记录的 live 单成交记录（策略关联 + 成交价/量）。
#[tauri::command]
pub async fn get_live_trades(
    state: State<'_, AppState>,
) -> Result<Vec<data_layer::LiveTrade>, String> {
    services(&state)?
        .live_trades
        .list()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_binance_order(
    state: State<'_, AppState>,
    symbol: String,
    order_id: i64,
) -> Result<(), String> {
    services(&state)?
        .binance_service
        .cancel_order(&symbol, order_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_binance_positions(
    state: State<'_, AppState>,
    symbol: Option<String>,
) -> Result<Vec<exchange_binance::types::BinancePosition>, String> {
    services(&state)?
        .binance_service
        .get_positions(symbol.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_binance_orders(
    state: State<'_, AppState>,
    symbol: String,
    history: Option<bool>,
    limit: Option<u32>,
) -> Result<Vec<exchange_binance::types::BinanceOrder>, String> {
    if history.unwrap_or(false) {
        services(&state)?
            .binance_service
            .get_all_orders(&symbol, limit)
            .await
            .map_err(|e| e.to_string())
    } else {
        let sym = if symbol.trim().is_empty() {
            None
        } else {
            Some(symbol.as_str())
        };
        services(&state)?
            .binance_service
            .get_open_orders(sym)
            .await
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn get_binance_order(
    state: State<'_, AppState>,
    symbol: String,
    order_id: i64,
) -> Result<exchange_binance::types::BinanceOrder, String> {
    services(&state)?
        .binance_service
        .get_order(&symbol, order_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_binance_instruments(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    services(&state)?
        .binance_service
        .get_instruments()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_binance_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    services(&state)?
        .binance_service
        .check_status()
        .await
        .map_err(|e| e.to_string())
}
