use exchange_okx::types::OkxEnvironment;
use exchange_okx::websocket::{OkxWebSocket, WsMessage};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::broadcast;
use tracing::warn;

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

    let ws = OkxWebSocket::new(OkxEnvironment::Demo);

    // Subscribe to broadcast BEFORE starting the WS, so no messages are lost
    let mut receiver = ws.subscribe();

    for symbol in &symbols {
        ws.subscribe_ticker(symbol)
            .await
            .map_err(|e| format!("Failed to subscribe ticker {}: {}", symbol, e))?;
        ws.subscribe_trades(symbol)
            .await
            .map_err(|e| format!("Failed to subscribe trades {}: {}", symbol, e))?;
    }

    ws.start()
        .await
        .map_err(|e| format!("Failed to start WebSocket: {}", e))?;

    let app_clone = app.clone();
    let running = state.ws_state.running.clone();
    running.store(true, Ordering::SeqCst);

    tokio::spawn(async move {
        loop {
            let msg = match receiver.recv().await {
                Ok(msg) => Some(msg),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Broadcast receiver lagged by {} messages", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => None,
            };

            match msg {
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
pub async fn stop_market_data(state: State<'_, AppState>) -> Result<(), String> {
    *state.ws_state.ws.write().await = None;
    state.ws_state.running.store(false, Ordering::SeqCst);
    Ok(())
}
