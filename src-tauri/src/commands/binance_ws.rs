//! Binance market-data WebSocket commands.
//!
//! Manages the [`BinanceWebSocket`] lifecycle and forwards parsed
//! kline/depth/orderbook/ticker/trade messages to the UI via Tauri events.

use exchange_binance::types::BinanceEnvironment;
use exchange_binance::{websocket::BinanceWsMessage, BinanceWebSocket, UserDataStreamClient};
use rust_decimal::prelude::ToPrimitive;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

use crate::state::AppState;
use tracing::debug;

#[tauri::command]
pub async fn start_binance_market_data(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if state.binance_ws_state.running.load(Ordering::SeqCst) {
        return Err("Binance WebSocket already running".to_string());
    }

    let (environment, ws_url) = {
        let config = state.config.read().await;
        (
            BinanceEnvironment::parse(&config.binance.environment),
            config.binance.ws_url.clone(),
        )
    };
    let ws = BinanceWebSocket::new(environment, ws_url);

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
                    debug!(symbol = %k.symbol, interval = %k.interval, "Binance WS kline");
                    let _ = app_clone.emit("binance:kline", &k);
                }
                BinanceWsMessage::Depth(d) => {
                    let _ = app_clone.emit("binance:depth", &d);
                }
                BinanceWsMessage::OrderBook(d) => {
                    let _ = app_clone.emit("binance:orderbook", &d);
                }
                BinanceWsMessage::Ticker(t) => {
                    let _ = app_clone.emit("binance:ticker", &t);
                }
                BinanceWsMessage::Trade(t) => {
                    let _ = app_clone.emit("binance:trade", &t);
                }
                // 用户数据流走独立 WS 连接，这里忽略（不影响市场流）。
                BinanceWsMessage::AccountPosition(_) | BinanceWsMessage::OrderUpdate(_) => {}
                BinanceWsMessage::ConnectionStatus(s) => {
                    debug!("Binance WS status: {s}");
                    let _ = app_clone.emit("binance:status", serde_json::json!({ "status": s }));
                }
                BinanceWsMessage::Error(e) => {
                    debug!("Binance WS error: {e}");
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
pub async fn subscribe_binance_ticker(
    state: State<'_, AppState>,
    symbol: String,
) -> Result<(), String> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => ws
            .subscribe_ticker(&symbol)
            .await
            .map_err(|e| e.to_string()),
        None => Err("Binance WebSocket not started".to_string()),
    }
}

#[tauri::command]
pub async fn subscribe_binance_trades(
    state: State<'_, AppState>,
    symbol: String,
) -> Result<(), String> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => ws
            .subscribe_trades(&symbol)
            .await
            .map_err(|e| e.to_string()),
        None => Err("Binance WebSocket not started".to_string()),
    }
}

#[tauri::command]
pub async fn subscribe_binance_orderbook(
    state: State<'_, AppState>,
    symbol: String,
) -> Result<(), String> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => ws
            .subscribe_orderbook(&symbol)
            .await
            .map_err(|e| e.to_string()),
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
    state
        .binance_ws_state
        .running
        .store(false, Ordering::SeqCst);
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

/// 启动用户数据流（`@userDataStream`，REST 限流/封禁时的实时账户/订单补充源）。
///
/// 独立 WS 连接，避免与市场数据流争抢单一 receiver。流程：获取 listenKey →
/// 订阅 → 转发 `binance:account`/`binance:order` 事件 → 每 30 分钟 keepalive。
#[tauri::command]
pub async fn start_binance_user_data_stream(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if state.binance_ws_state.user_data_running.load(Ordering::SeqCst) {
        return Err("Binance user data stream already running".to_string());
    }

    // WebSocket-API 用户数据流（替换已弃用的 REST listenKey —— 410）。
    let (ws_api_url, api_key) = {
        let config = state.config.read().await;
        (
            config
                .binance
                .ws_api_url
                .clone()
                .unwrap_or_else(|| "wss://ws-api.binance.com/ws-api/v3".to_string()),
            config.binance.api_key.clone(),
        )
    };

    let client = UserDataStreamClient::new(ws_api_url, api_key);
    let listen_key = client
        .start()
        .await
        .map_err(|e| format!("Failed to start user data stream: {}", e))?;

    let mut rx = client.get_receiver().await;
    let app_clone = app.clone();
    let running = state.binance_ws_state.user_data_running.clone();
    running.store(true, Ordering::SeqCst);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                BinanceWsMessage::AccountPosition(p) => {
                    let _ = app_clone.emit("binance:account", &p);
                }
                BinanceWsMessage::OrderUpdate(o) => {
                    let _ = app_clone.emit("binance:order", &o);
                }
                BinanceWsMessage::Error(e) => {
                    debug!("Binance user data WS error: {e}");
                    let _ = app_clone.emit("binance:user_data_error", &e);
                }
                _ => {}
            }
        }
        running.store(false, Ordering::SeqCst);
    });

    *state.binance_ws_state.user_data_ws.write().await = Some(client);
    Ok(listen_key)
}

/// 停止用户数据流。
#[tauri::command]
pub async fn stop_binance_user_data_stream(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.binance_ws_state.user_data_ws.write().await;
    if let Some(ws) = guard.take() {
        ws.stop();
    }
    state
        .binance_ws_state
        .user_data_running
        .store(false, Ordering::SeqCst);
    Ok(())
}

/// 启动实盘订单状态监控（后台轮询 Binance + 同步 `live_trades` + 推事件）。
///
/// 每 5s 拉取开放订单，把状态/成交量同步进 `live_trades`（保留策略关联），
/// 有变化时推送 `binance:live_orders_updated`，前端自动刷新。这是 WS 用户流
/// 不可用（测试网 -1099）时的 REST 兜底，保证实盘单状态自动更新。
/// 启动资产曲线后台快照写入器。
///
/// 每 60s 拉取实盘余额 + 全市场价格，计算 USDT 总权益并写入 `account_snapshots`。
/// 独立于前端刷新运行，使资产曲线随时间持续增长。
pub fn start_equity_snapshot_writer(app_services: &quant_services::AppServices) {
    let binance = app_services.binance_service.clone();
    let account_service = app_services.account_service.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let (Ok(balances), Ok(prices)) = (binance.get_balance().await, binance.get_all_ticker_prices().await)
            else {
                continue;
            };
            let mut equity = rust_decimal::Decimal::ZERO;
            for b in balances {
                let asset = b.asset.as_str();
                let qty = b.free + b.locked;
                let price = if matches!(asset, "USDT" | "USDC" | "TUSD" | "BUSD" | "FDUSD" | "DAI") {
                    rust_decimal::Decimal::ONE
                } else {
                    prices
                        .get(&format!("{asset}USDT"))
                        .copied()
                        .unwrap_or(rust_decimal::Decimal::ZERO)
                };
                equity += qty * price;
            }
            let _ = account_service.record_equity_snapshot(equity).await;
        }
    });
}

/// 每 5s 用真实权益快照刷新监控指标 Gauge（余额/持仓市值/当日盈亏），
/// 使 Prometheus 端点与 Monitor「指标监控」一致（账户值来自后台快照写入器）。
pub fn start_monitor_metrics(app_services: &quant_services::AppServices) {
    let account_service = app_services.account_service.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            if let Ok(Some(equity)) = account_service.get_latest_equity("USDT").await {
                let eq = equity.to_f64().unwrap_or(0.0);
                monitor_layer::MetricsCollector::set_account_balance(eq);
            }
            if let Ok(pnl) = account_service.get_today_equity_pnl("USDT").await {
                monitor_layer::MetricsCollector::set_daily_pnl(pnl.to_f64().unwrap_or(0.0));
            }
        }
    });
}

pub fn start_live_order_monitor(app: AppHandle, app_services: &quant_services::AppServices) {
    let binance = app_services.binance_service.clone();
    let live_trades = app_services.live_trades.clone();
    let account_service = app_services.account_service.clone();
    tokio::spawn(async move {
        use quant_common::types::OrderStatus;
        use rust_decimal::Decimal;
        use std::collections::HashMap;
        // 上一轮仍开放的单（order_id → 域 symbol），用于检测终态回写。
        let mut last_open: HashMap<i64, String> = HashMap::new();
        // 最近一次记录的状态（order_id → (status, executed_qty)），变化才写库/发事件。
        let mut last_state: HashMap<i64, (String, Decimal)> = HashMap::new();
        let map_open_status = |s: &str| -> Option<OrderStatus> {
            match s {
                "PARTIALLY_FILLED" => Some(OrderStatus::PartiallyFilled),
                "NEW" => Some(OrderStatus::Submitted),
                _ => None,
            }
        };
        let map_terminal = |s: &str| -> Option<OrderStatus> {
            match s {
                "FILLED" => Some(OrderStatus::Filled),
                "CANCELED" => Some(OrderStatus::Cancelled),
                "REJECTED" => Some(OrderStatus::Rejected),
                "EXPIRED" => Some(OrderStatus::Expired),
                _ => None,
            }
        };
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let open = match binance.get_open_orders(None).await {
                Ok(o) => o,
                Err(_) => {
                    // 限流/网络：退避 10s 再试，避免高频打 Binance。
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    continue;
                }
            };
            let mut current: HashMap<i64, String> = HashMap::new();
            let mut changed = false;
            for o in &open {
                let sym = exchange_binance::from_binance_symbol(&o.symbol);
                current.insert(o.order_id, sym);
                // 状态未变 → 跳过写库/发事件（避免每 5s 无条件写 + 事件洪泛）。
                let key = (o.status.clone(), o.executed_qty);
                if last_state.get(&o.order_id) == Some(&key) {
                    continue;
                }
                let _ = live_trades
                    .update_status(o.order_id, &o.status, o.executed_qty)
                    .await;
                if let Some(status) = map_open_status(&o.status) {
                    let _ = account_service
                        .update_order_status(
                            o.order_id,
                            status,
                            o.executed_qty,
                            Decimal::ZERO,
                        )
                        .await;
                }
                last_state.insert(o.order_id, key);
                changed = true;
            }
            // 不再开放的单 → 确认终态并回写；get_order 失败则保留待下轮重试。
            let mut retry: HashMap<i64, String> = HashMap::new();
            for (order_id, sym) in last_open.iter() {
                if current.contains_key(order_id) {
                    continue;
                }
                match binance.get_order(sym, *order_id).await {
                    Ok(o) => {
                        if let Some(status) = map_terminal(&o.status) {
                            let _ = account_service
                                .update_order_status(
                                    *order_id,
                                    status,
                                    o.executed_qty,
                                    Decimal::ZERO,
                                )
                                .await;
                            let _ = live_trades
                                .update_status(*order_id, &o.status, o.executed_qty)
                                .await;
                            changed = true;
                        }
                        last_state.remove(order_id);
                    }
                    Err(_) => {
                        // 瞬时失败不丢状态：保留待重试。
                        retry.insert(*order_id, sym.clone());
                    }
                }
            }
            last_open = current;
            last_open.extend(retry);
            if changed {
                let _ = app.emit("binance:live_orders_updated", ());
            }
        }
    });
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
            audit_logger: Arc::new(security::AuditLogger::new(None)),
            pg_client: None,
            redis_cache: None,
            binance_client: Arc::new(RwLock::new(None)),
            order_manager: OrderManager::new(),
            app_services: None,
            binance_ws_state: BinanceWsState::new(),
            auth_session: Arc::new(RwLock::new(Some(crate::state::AuthedUser::admin()))),
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
    async fn subscribe_depth_requires_running_ws() {
        let state = make_test_state();
        let result = subscribe_binance_depth(state_guard(&state), "BTC-USDT".to_string()).await;
        assert_eq!(result.unwrap_err(), "Binance WebSocket not started");
    }

    #[tokio::test]
    async fn subscribe_ticker_requires_running_ws() {
        let state = make_test_state();
        let result = subscribe_binance_ticker(state_guard(&state), "BTC-USDT".to_string()).await;
        assert_eq!(result.unwrap_err(), "Binance WebSocket not started");
    }

    #[tokio::test]
    async fn subscribe_trades_requires_running_ws() {
        let state = make_test_state();
        let result = subscribe_binance_trades(state_guard(&state), "BTC-USDT".to_string()).await;
        assert_eq!(result.unwrap_err(), "Binance WebSocket not started");
    }

    #[tokio::test]
    async fn subscribe_orderbook_requires_running_ws() {
        let state = make_test_state();
        let result = subscribe_binance_orderbook(state_guard(&state), "BTC-USDT".to_string()).await;
        assert_eq!(result.unwrap_err(), "Binance WebSocket not started");
    }

    #[tokio::test]
    async fn stop_clears_ws_state() {
        let state = make_test_state();
        *state.binance_ws_state.ws.write().await = Some(exchange_binance::BinanceWebSocket::new(
            exchange_binance::types::BinanceEnvironment::Spot,
            None,
        ));
        state.binance_ws_state.running.store(true, Ordering::SeqCst);
        stop_binance_market_data(state_guard(&state)).await.unwrap();
        assert!(!state.binance_ws_state.running.load(Ordering::SeqCst));
        assert!(state.binance_ws_state.ws.read().await.is_none());
    }
}
