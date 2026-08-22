//! Binance market-data WebSocket commands.
//!
//! Mirrors `ws_commands.rs`: manages the [`BinanceWebSocket`] lifecycle and
//! forwards parsed kline/depth messages to the UI via Tauri events.

use exchange_binance::types::BinanceEnvironment;
use exchange_binance::{websocket::BinanceWsMessage, BinanceWebSocket};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

use crate::state::AppState;

#[tauri::command]
pub async fn start_binance_market_data(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if state.binance_ws_state.running.load(Ordering::SeqCst) {
        return Err("Binance WebSocket already running".to_string());
    }

    let environment = {
        let config = state.config.read().await;
        BinanceEnvironment::parse(&config.binance.environment)
    };
    let ws = BinanceWebSocket::new(environment);

    let _ = app.emit(
        "binance:status",
        serde_json::json!({ "status": "connecting" }),
    );

    ws.start()
        .await
        .map_err(|e| format!("Failed to start Binance WebSocket: {}", e))?;

    let mut rx = ws.get_receiver().await;
    let app_clone = app.clone();
    let running = state.binance_ws_state.running.clone();
    running.store(true, Ordering::SeqCst);

    let _ = app_clone.emit(
        "binance:status",
        serde_json::json!({ "status": "connected" }),
    );

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                BinanceWsMessage::Kline(k) => {
                    let _ = app_clone.emit("binance:kline", &k);
                }
                BinanceWsMessage::Depth(d) => {
                    let _ = app_clone.emit("binance:depth", &d);
                }
                BinanceWsMessage::ConnectionStatus(s) => {
                    let _ = app_clone.emit("binance:status", serde_json::json!({ "status": s }));
                }
                BinanceWsMessage::Error(e) => {
                    let _ = app_clone.emit("binance:error", &e);
                }
            }
        }
        let _ = app_clone.emit(
            "binance:status",
            serde_json::json!({ "status": "disconnected" }),
        );
        running.store(false, Ordering::SeqCst);
    });

    *state.binance_ws_state.ws.write().await = Some(ws);
    Ok(())
}

#[tauri::command]
pub async fn subscribe_binance_candle(
    state: State<'_, AppState>,
    symbol: String,
    interval: String,
) -> Result<(), String> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => ws
            .subscribe_candle(&symbol, &interval)
            .await
            .map_err(|e| e.to_string()),
        None => Err("Binance WebSocket not started".to_string()),
    }
}

#[tauri::command]
pub async fn subscribe_binance_depth(
    state: State<'_, AppState>,
    symbol: String,
) -> Result<(), String> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => ws.subscribe_depth(&symbol).await.map_err(|e| e.to_string()),
        None => Err("Binance WebSocket not started".to_string()),
    }
}

#[tauri::command]
pub async fn stop_binance_market_data(state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut guard = state.binance_ws_state.ws.write().await;
        if let Some(ws) = guard.as_ref() {
            ws.stop();
        }
        *guard = None;
    }
    state.binance_ws_state.running.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn get_binance_subscriptions(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => Ok(ws.subscriptions().await),
        None => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BinanceWsState;
    use monitor_layer::{AlertManager, LogBuffer};
    use quant_common::config::AppConfig;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use trading_layer::OrderManager;

    fn make_test_state() -> AppState {
        AppState {
            config: Arc::new(RwLock::new(AppConfig::default())),
            alert_manager: Arc::new(AlertManager::new(false, vec![])),
            log_buffer: Arc::new(LogBuffer::new(1000)),
            pg_client: None,
            redis_cache: None,
            okx_client: Arc::new(RwLock::new(None)),
            okx_executor: Arc::new(RwLock::new(None)),
            okx_data_source: Arc::new(RwLock::new(None)),
            binance_client: Arc::new(RwLock::new(None)),
            order_manager: OrderManager::new(),
            app_services: None,
            ws_state: crate::state::WsState::new(),
            binance_ws_state: BinanceWsState::new(),
        }
    }

    fn state_guard<'a>(state: &'a AppState) -> State<'a, AppState> {
        // SAFETY: tauri::State is a transparent wrapper around &T.
        unsafe { std::mem::transmute::<&AppState, State<'_, AppState>>(state) }
    }

    #[tokio::test]
    async fn subscribe_candle_requires_running_ws() {
        let state = make_test_state();
        let result = subscribe_binance_candle(
            state_guard(&state),
            "BTC-USDT".to_string(),
            "1h".to_string(),
        )
        .await;
        assert_eq!(result.unwrap_err(), "Binance WebSocket not started");
    }

    #[tokio::test]
    async fn stop_clears_ws_state() {
        let state = make_test_state();
        *state.binance_ws_state.ws.write().await =
            Some(exchange_binance::BinanceWebSocket::new(
                exchange_binance::types::BinanceEnvironment::Spot,
            ));
        state.binance_ws_state.running.store(true, Ordering::SeqCst);
        stop_binance_market_data(state_guard(&state)).await.unwrap();
        assert!(!state.binance_ws_state.running.load(Ordering::SeqCst));
        assert!(state.binance_ws_state.ws.read().await.is_none());
    }
}
