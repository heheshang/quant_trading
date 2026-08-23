//! Binance exchange commands.
//!
//! Thin Tauri adapters that delegate to `AppServices.binance_service`.
//! They never touch the Binance client or data layers directly (layering/DIP).

use crate::state::AppState;
use exchange_binance::types::BinancePlaceOrderRequest;
use tauri::State;

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
    services(&state)?
        .binance_service
        .place_order(request)
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
        services(&state)?
            .binance_service
            .get_open_orders(Some(&symbol))
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
