use exchange_okx::types::OkxEnvironment;
use exchange_okx::websocket::{OkxWebSocket, WsMessage};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

use crate::state::AppState;

#[tauri::command]
pub async fn start_market_data(
    app: AppHandle,
    state: State<'_, AppState>,
    symbols: Vec<String>,
) -> Result<(), String> {
    if state.ws_state.running.load(Ordering::SeqCst) {
        return Err("WebSocket already running".to_string());
    }

    let environment = {
        let config = state.config.read().await;
        if config.okx.environment == "live" {
            OkxEnvironment::Live
        } else {
            OkxEnvironment::Demo
        }
    };
    let ws = OkxWebSocket::new(environment);

    // Subscribe BEFORE starting the WS, so subscriptions register first
    for symbol in &symbols {
        ws.subscribe_ticker(symbol)
            .await
            .map_err(|e| format!("Failed to subscribe ticker {}: {}", symbol, e))?;
        ws.subscribe_trades(symbol)
            .await
            .map_err(|e| format!("Failed to subscribe trades {}: {}", symbol, e))?;
    }

    let _ = app.emit(
        "ws:connection_status",
        serde_json::json!({ "status": "connecting" }),
    );

    ws.start()
        .await
        .map_err(|e| format!("Failed to start WebSocket: {}", e))?;

    // Get a separate receiver so the background task reads messages independently
    let mut rx = ws.get_receiver().await;

    let app_clone = app.clone();
    let running = state.ws_state.running.clone();
    running.store(true, Ordering::SeqCst);

    let _ = app_clone.emit(
        "ws:connection_status",
        serde_json::json!({ "status": "connected" }),
    );

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Some(WsMessage::Ticker(data)) => {
                    let _ = app_clone.emit("ws:ticker", &data);
                }
                Some(WsMessage::Trades(data)) => {
                    let _ = app_clone.emit("ws:trades", &data);
                }
                Some(WsMessage::OrderBook(data)) => {
                    let _ = app_clone.emit("ws:orderbook", &data);
                }
                Some(WsMessage::Candle(data)) => {
                    let _ = app_clone.emit("ws:candle", &data);
                }
                Some(WsMessage::Error(e)) if e != "ping" => {
                    let _ = app_clone.emit("ws:error", &e);
                }
                Some(WsMessage::Account(data)) => {
                    let _ = app_clone.emit("ws:account", &data);
                }
                Some(WsMessage::Orders(data)) => {
                    let _ = app_clone.emit("ws:orders", &data);
                }
                Some(WsMessage::Positions(data)) => {
                    let _ = app_clone.emit("ws:positions", &data);
                }
                None => break,
                _ => {}
            }
        }
        let _ = app_clone.emit(
            "ws:connection_status",
            serde_json::json!({ "status": "disconnected" }),
        );
        running.store(false, Ordering::SeqCst);
    });

    *state.ws_state.ws.write().await = Some(ws);

    Ok(())
}

#[tauri::command]
pub async fn subscribe_market_data(
    state: State<'_, AppState>,
    channel: String,
    symbol: String,
) -> Result<(), String> {
    let guard = state.ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => match channel.as_str() {
            "ticker" => ws
                .subscribe_ticker(&symbol)
                .await
                .map_err(|e| e.to_string()),
            "trades" => ws
                .subscribe_trades(&symbol)
                .await
                .map_err(|e| e.to_string()),
            ch if ch.starts_with("candle") => {
                let bar = ch.trim_start_matches("candle");
                if bar.is_empty() {
                    return Err("Invalid candle channel: missing bar".to_string());
                }
                ws.subscribe_candle(&symbol, bar)
                    .await
                    .map_err(|e| e.to_string())
            }
            "orderbook" => ws
                .subscribe_order_book(&symbol, "5")
                .await
                .map_err(|e| e.to_string()),
            _ => Err(format!("Unknown channel: {}", channel)),
        },
        None => Err("WebSocket not started".to_string()),
    }
}

#[tauri::command]
pub async fn unsubscribe_market_data(
    state: State<'_, AppState>,
    channel: String,
    symbol: String,
) -> Result<(), String> {
    let guard = state.ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => match channel.as_str() {
            "ticker" => ws
                .unsubscribe_public("tickers", &symbol)
                .await
                .map_err(|e| e.to_string()),
            "trades" => ws
                .unsubscribe_public("trades", &symbol)
                .await
                .map_err(|e| e.to_string()),
            ch if ch.starts_with("candle") => {
                let bar = ch.trim_start_matches("candle");
                ws.unsubscribe_public(&format!("candle{}", bar), &symbol)
                    .await
                    .map_err(|e| e.to_string())
            }
            "orderbook" => ws
                .unsubscribe_public("books5", &symbol)
                .await
                .map_err(|e| e.to_string()),
            _ => Err(format!("Unknown channel: {}", channel)),
        },
        None => Err("WebSocket not started".to_string()),
    }
}

#[tauri::command]
pub async fn stop_market_data(state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut guard = state.ws_state.ws.write().await;
        if let Some(ws) = guard.as_ref() {
            ws.stop();
        }
        *guard = None;
    }
    state.ws_state.running.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn get_subscriptions(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let guard = state.ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => {
            let subs = ws.subscriptions().await;
            let mut result = Vec::with_capacity(subs.len());
            for sub in &subs {
                result.push(format!("{}:{}", sub.channel, sub.inst_id));
            }
            Ok(result)
        }
        None => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::WsState;
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
            order_manager: OrderManager::new(),
            app_services: None,
            ws_state: WsState::new(),
        }
    }

    fn state_guard<'a>(state: &'a AppState) -> State<'a, AppState> {
        // SAFETY: tauri::State is a transparent wrapper around &T.
        unsafe { std::mem::transmute::<&AppState, State<'_, AppState>>(state) }
    }

    #[tokio::test]
    async fn test_unsubscribe_market_data_not_started() {
        let state = make_test_state();
        let result = unsubscribe_market_data(
            state_guard(&state),
            "ticker".to_string(),
            "BTC-USDT".to_string(),
        )
        .await;
        assert_eq!(result.unwrap_err(), "WebSocket not started");
    }

    #[tokio::test]
    async fn test_unsubscribe_market_data_removes_subscription() {
        let state = make_test_state();
        *state.ws_state.ws.write().await = Some(OkxWebSocket::new(OkxEnvironment::Demo));

        subscribe_market_data(
            state_guard(&state),
            "ticker".to_string(),
            "BTC-USDT".to_string(),
        )
        .await
        .unwrap();

        unsubscribe_market_data(
            state_guard(&state),
            "ticker".to_string(),
            "BTC-USDT".to_string(),
        )
        .await
        .unwrap();

        let subscriptions = get_subscriptions(state_guard(&state)).await.unwrap();
        assert!(subscriptions.is_empty());
    }

    #[tokio::test]
    async fn test_subscribe_candle_requires_bar() {
        let state = make_test_state();
        *state.ws_state.ws.write().await = Some(OkxWebSocket::new(OkxEnvironment::Demo));

        let result = subscribe_market_data(
            state_guard(&state),
            "candle".to_string(),
            "BTC-USDT".to_string(),
        )
        .await;
        assert!(result.unwrap_err().contains("missing bar"));
    }

    #[tokio::test]
    async fn test_stop_market_data_clears_state() {
        let state = make_test_state();
        *state.ws_state.ws.write().await = Some(OkxWebSocket::new(OkxEnvironment::Demo));
        state.ws_state.running.store(true, Ordering::SeqCst);

        stop_market_data(state_guard(&state)).await.unwrap();

        assert!(!state.ws_state.running.load(Ordering::SeqCst));
        assert!(state.ws_state.ws.read().await.is_none());
    }
}
