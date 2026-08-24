//! Binance market-data WebSocket commands.
//!
//! Manages the [`BinanceWebSocket`] lifecycle and forwards parsed
//! kline/depth/orderbook/ticker/trade messages to the UI via Tauri events.

use chrono::{DateTime, Utc};
use data_layer::{
    LiveTrade, MarketDataRepository, NewMarketDataRecord, NewTickerSnapshot,
};
use exchange_binance::types::{BinanceBalance, BinanceEnvironment};
use exchange_binance::{
    websocket::BinanceWsMessage, BinanceWebSocket, UserDataStreamClient,
};
use quant_common::types::Position;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

use crate::state::AppState;
use quant_common::api::{ok_result, ApiFailure};
use tracing::{debug, info, warn};

/// 后台导入管线的消息：WS 数据 → DB。
enum MarketImport {
    Kline(NewMarketDataRecord),
    Ticker(NewTickerSnapshot),
    Trade(data_layer::NewStreamTrade),
    OrderBook(data_layer::NewOrderbookSnapshot),
}

/// 把 `@kline` WS 消息转换为 `market_data` 行（domain symbol 即 instrument_id）。
fn kline_to_record(k: &exchange_binance::websocket::BinanceWsKline) -> NewMarketDataRecord {
    NewMarketDataRecord {
        instrument_id: k.symbol.clone(),
        timeframe: k.interval.clone(),
        timestamp: DateTime::from_timestamp_millis(k.open_time).unwrap_or_else(Utc::now),
        open: k.open,
        high: k.high,
        low: k.low,
        close: k.close,
        volume: k.volume,
    }
}

/// 把 `@ticker` WS 消息转换为 `ticker_snapshots` 行。
///
/// `ts` 对齐到分钟（每分钟每标的至多一行），避免高频 ticker 流无限膨胀。
fn ticker_to_record(t: &exchange_binance::websocket::BinanceWsTicker) -> NewTickerSnapshot {
    let ts = DateTime::from_timestamp_millis(t.event_time)
        .map(floor_minute)
        .unwrap_or_else(Utc::now);
    NewTickerSnapshot {
        instrument_id: t.symbol.clone(),
        ts,
        last_px: Some(t.last_price),
        open_24h: Some(t.open),
        high_24h: Some(t.high),
        low_24h: Some(t.low),
        vol_24h: Some(t.volume),
        vol_ccy_24h: Some(t.quote_volume),
        change_24h: Some(t.price_change),
    }
}

/// 截断到分钟整点（秒/纳秒清零）。
fn floor_minute(dt: DateTime<Utc>) -> DateTime<Utc> {
    let secs = dt.timestamp();
    DateTime::from_timestamp(secs - (secs % 60), 0).unwrap_or(dt)
}

/// 把 `@trade` WS 消息转换为 `stream_trades` 行。
fn trade_to_record(t: &exchange_binance::websocket::BinanceWsTrade) -> data_layer::NewStreamTrade {
    data_layer::NewStreamTrade {
        symbol: t.symbol.clone(),
        price: t.price,
        quantity: t.quantity,
        trade_time: DateTime::from_timestamp_millis(t.trade_time).unwrap_or_else(Utc::now),
        is_buyer_maker: t.is_buyer_maker,
    }
}

/// 把 `@depth`/`@orderbook` WS 消息转换为 `orderbook_snapshots` 行（JSON 字符串）。
fn depth_to_record(d: &exchange_binance::websocket::BinanceWsDepth) -> data_layer::NewOrderbookSnapshot {
    data_layer::NewOrderbookSnapshot {
        symbol: d.symbol.clone(),
        bids: serde_json::to_string(&d.bids).unwrap_or_else(|_| "[]".to_string()),
        asks: serde_json::to_string(&d.asks).unwrap_or_else(|_| "[]".to_string()),
    }
}

/// 后台导入写入任务：串行消费消息并 upsert 到 DB，避免阻塞 WS 收流。
async fn market_import_writer(
    repo: Arc<MarketDataRepository>,
    mut rx: mpsc::Receiver<MarketImport>,
) {
    info!("market import writer started (repo attached)");
    let mut total: u64 = 0;
    let mut klines: u64 = 0;
    let mut tickers: u64 = 0;
    let mut trades: u64 = 0;
    let mut books: u64 = 0;
    while let Some(msg) = rx.recv().await {
        total += 1;
        match msg {
            MarketImport::Kline(k) => {
                klines += 1;
                if let Err(e) = repo.upsert_kline(&k).await {
                    warn!(error = %e, symbol = %k.instrument_id, "kline import failed");
                }
            }
            MarketImport::Ticker(t) => {
                tickers += 1;
                if let Err(e) = repo.upsert_ticker_snapshot(&t).await {
                    warn!(error = %e, symbol = %t.instrument_id, "ticker import failed");
                }
            }
            MarketImport::Trade(t) => {
                trades += 1;
                if let Err(e) = repo.insert_stream_trade(&t).await {
                    warn!(error = %e, symbol = %t.symbol, "trade import failed");
                }
            }
            MarketImport::OrderBook(d) => {
                books += 1;
                if let Err(e) = repo.upsert_orderbook_snapshot(&d).await {
                    warn!(error = %e, symbol = %d.symbol, "orderbook import failed");
                }
            }
        }
        if total % 200 == 0 {
            info!(
                total, klines, tickers, trades, books,
                "market import progress (last 200 msg window)"
            );
        }
    }
    info!(total, klines, tickers, trades, books, "market import writer stopped");
}
#[tauri::command]
pub async fn start_binance_market_data(
    app: AppHandle,
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    // 原子抢锁：仅一个调用能置位 running，避免 TOCTOU 启动重复连接。
    if state
        .binance_ws_state
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(ApiFailure::new(quant_common::api::code::CONFLICT, "Binance WebSocket already running".to_string()));
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
        .map_err(|e| {
            // 启动失败：释放锁，允许重试。
            state.binance_ws_state.running.store(false, Ordering::SeqCst);
            ApiFailure::new(quant_common::api::code::BINANCE_API, format!("Failed to start Binance WebSocket: {}", e))
        })?;

    let mut rx = ws.get_receiver().await;
    let app_clone = app.clone();
    let running = state.binance_ws_state.running.clone();

    // 导入管线：remote WS 数据 → DB（K线/ticker）。无 repo（未连接 DB）时跳过。
    let market_repo = state.app_services.as_ref().and_then(|s| s.market_data.clone());
    let (import_tx, import_rx) = mpsc::channel::<MarketImport>(256);
    if let Some(repo) = market_repo.clone() {
        tokio::spawn(market_import_writer(repo, import_rx));
    }
    let import_enabled = market_repo.is_some();
    if import_enabled {
        info!("binance market WS import pipeline enabled (repo attached, channel 256)");
    } else {
        warn!("binance market WS import pipeline DISABLED (no market_data repo / DB not connected)");
    }

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
                    if import_enabled {
                        let _ = import_tx.send(MarketImport::Kline(kline_to_record(&k))).await;
                    }
                }
                BinanceWsMessage::Depth(d) => {
                    let _ = app_clone.emit("binance:depth", &d);
                    if import_enabled {
                        let _ = import_tx.send(MarketImport::OrderBook(depth_to_record(&d))).await;
                    }
                }
                BinanceWsMessage::OrderBook(d) => {
                    let _ = app_clone.emit("binance:orderbook", &d);
                    if import_enabled {
                        let _ = import_tx.send(MarketImport::OrderBook(depth_to_record(&d))).await;
                    }
                }
                BinanceWsMessage::Ticker(t) => {
                    let _ = app_clone.emit("binance:ticker", &t);
                    if import_enabled {
                        let _ = import_tx.send(MarketImport::Ticker(ticker_to_record(&t))).await;
                    }
                }
                BinanceWsMessage::Trade(t) => {
                    let _ = app_clone.emit("binance:trade", &t);
                    if import_enabled {
                        let _ = import_tx.send(MarketImport::Trade(trade_to_record(&t))).await;
                    }
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
    ok_result(serde_json::Value::Null)
}

#[tauri::command]
pub async fn subscribe_binance_candle(
    state: State<'_, AppState>,
    symbol: String,
    interval: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => {
            ws
                .subscribe_candle(&symbol, &interval).await
                .map_err(|e| ApiFailure::new(quant_common::api::code::BINANCE_API, e.to_string()))?;
            ok_result(serde_json::Value::Null)
        },

        None => Err(ApiFailure::new(quant_common::api::code::NOT_INITIALIZED, "Binance WebSocket not started".to_string())),
    }
}

#[tauri::command]
pub async fn subscribe_binance_depth(
    state: State<'_, AppState>,
    symbol: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => {
            ws.subscribe_depth(&symbol).await
                .map_err(|e| ApiFailure::new(quant_common::api::code::BINANCE_API, e.to_string()))?;
            ok_result(serde_json::Value::Null)
        }
        None => Err(ApiFailure::new(quant_common::api::code::NOT_INITIALIZED, "Binance WebSocket not started".to_string())),
    }
}

#[tauri::command]
pub async fn subscribe_binance_ticker(
    state: State<'_, AppState>,
    symbol: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => {
            ws
                .subscribe_ticker(&symbol).await
                .map_err(|e| ApiFailure::new(quant_common::api::code::BINANCE_API, e.to_string()))?;
            ok_result(serde_json::Value::Null)
        },

        None => Err(ApiFailure::new(quant_common::api::code::NOT_INITIALIZED, "Binance WebSocket not started".to_string())),
    }
}

#[tauri::command]
pub async fn subscribe_binance_trades(
    state: State<'_, AppState>,
    symbol: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => {
            ws
                .subscribe_trades(&symbol).await
                .map_err(|e| ApiFailure::new(quant_common::api::code::BINANCE_API, e.to_string()))?;
            ok_result(serde_json::Value::Null)
        },

        None => Err(ApiFailure::new(quant_common::api::code::NOT_INITIALIZED, "Binance WebSocket not started".to_string())),
    }
}

#[tauri::command]
pub async fn subscribe_binance_orderbook(
    state: State<'_, AppState>,
    symbol: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => {
            ws
                .subscribe_orderbook(&symbol).await
                .map_err(|e| ApiFailure::new(quant_common::api::code::BINANCE_API, e.to_string()))?;
            ok_result(serde_json::Value::Null)
        },

        None => Err(ApiFailure::new(quant_common::api::code::NOT_INITIALIZED, "Binance WebSocket not started".to_string())),
    }
}

#[tauri::command]
pub async fn stop_binance_market_data(state: State<'_, AppState>) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
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
    ok_result(serde_json::Value::Null)
}

#[tauri::command]
pub async fn get_binance_subscriptions(state: State<'_, AppState>) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<String>>> {
    let guard = state.binance_ws_state.ws.read().await;
    match guard.as_ref() {
        Some(ws) => ok_result(ws.subscriptions().await),
        None => ok_result(vec![]),
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
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    if state
        .binance_ws_state
        .user_data_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(ApiFailure::new(quant_common::api::code::CONFLICT, "Binance user data stream already running".to_string()));
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
        .map_err(|e| ApiFailure::new(quant_common::api::code::BINANCE_API, format!("Failed to start user data stream: {}", e)))?;

    let mut rx = client.get_receiver().await;
    let app_clone = app.clone();
    let running = state.binance_ws_state.user_data_running.clone();
    let market_repo = state.app_services.as_ref().and_then(|s| s.market_data.clone());
    running.store(true, Ordering::SeqCst);
    info!("binance user data stream started (listen_key obtained)");
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                BinanceWsMessage::AccountPosition(p) => {
                    let _ = app_clone.emit("binance:account", &p);
                    // 用户数据流实时账户余额 → 落库 balances（与 REST 快照互补，更实时）。
                    if let Some(repo) = &market_repo {
                        let new_balances: Vec<data_layer::NewBalance> = p
                            .balances
                            .iter()
                            .map(|b| data_layer::NewBalance {
                                asset: b.asset.clone(),
                                free: b.free,
                                locked: b.locked,
                            })
                            .collect();
                        if !new_balances.is_empty() {
                            match repo.upsert_balances(&new_balances).await {
                                Ok(_) => info!(assets = new_balances.len(), "[user-stream] balances persisted"),
                                Err(e) => warn!(error = %e, "[user-stream] balances import failed"),
                            }
                        }
                    }
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
    ok_result(listen_key)
}

/// 停止用户数据流。
#[tauri::command]
pub async fn stop_binance_user_data_stream(state: State<'_, AppState>) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    let mut guard = state.binance_ws_state.user_data_ws.write().await;
    if let Some(ws) = guard.take() {
        ws.stop();
    }
    state
        .binance_ws_state
        .user_data_running
        .store(false, Ordering::SeqCst);
    ok_result(serde_json::Value::Null)
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
    let live_trades = app_services.live_trades.clone();
    let market_repo = app_services.market_data.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let (Ok(balances), Ok(prices)) = (binance.get_balance().await, binance.get_all_ticker_prices().await)
            else {
                continue;
            };
            let mut equity = rust_decimal::Decimal::ZERO;
            for b in &balances {
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

            // 逐资产余额 + 全标的最近价落库（供余额表/纸面定价/取价，前端不再直连币安 REST）。
            if let Some(repo) = &market_repo {
                let new_balances: Vec<data_layer::NewBalance> = balances
                    .iter()
                    .map(|b| data_layer::NewBalance {
                        asset: b.asset.clone(),
                        free: b.free,
                        locked: b.locked,
                    })
                    .collect();
                match repo.upsert_balances(&new_balances).await {
                    Ok(n) => info!(assets = new_balances.len(), rows = n, "[snapshot] balances persisted"),
                    Err(e) => warn!(error = %e, "[snapshot] balances import failed"),
                }
                let mut lp_ok = 0u64;
                for (sym, price) in &prices {
                    match repo
                        .upsert_last_price(&data_layer::NewLastPrice {
                            symbol: sym.clone(),
                            price: *price,
                        })
                        .await
                    {
                        Ok(_) => lp_ok += 1,
                        Err(e) => warn!(error = %e, symbol = %sym, "[snapshot] last_price import failed"),
                    }
                }
                info!(prices = prices.len(), wrote = lp_ok, "[snapshot] last_prices persisted");
            } else {
                warn!("[snapshot] market_data repo unavailable; balances/last_prices NOT imported");
            }

            // 币安持仓同步写库（positions 表），前端「持仓信息」从 DB 读取。
            let fills = live_trades.list().await.unwrap_or_default();
            let positions = build_positions(&balances, &prices, &fills);
            match account_service.upsert_positions(&positions).await {
                Ok(_) => info!(count = positions.len(), "[snapshot] positions persisted"),
                Err(e) => warn!(error = %e, "[snapshot] positions upsert failed"),
            }
        }
    });
}

/// 从币安余额 + 全市场价格 + 本地 live_trades 成交构造持仓快照。
///
/// - 稳定币余额视为现金，不算持仓；
/// - `avg_price` 取该 symbol 的 FILLED 买单加权均价，无成交则取现价（浮盈 0）；
/// - 未在行情表中命中的资产以现价 0 记录（不抛错，避免单资产阻塞整批同步）。
fn build_positions(
    balances: &[BinanceBalance],
    prices: &HashMap<String, Decimal>,
    fills: &[LiveTrade],
) -> Vec<Position> {
    // 每域名 symbol（如 BTC-USDT）的 FILLED 买单成本聚合：累计成本/数量。
    let mut cost: HashMap<String, (Decimal, Decimal)> = HashMap::new();
    for t in fills {
        if !t.status.eq_ignore_ascii_case("FILLED") {
            continue;
        }
        let e = cost
            .entry(t.symbol.clone())
            .or_insert((Decimal::ZERO, Decimal::ZERO));
        if t.side.eq_ignore_ascii_case("BUY") {
            e.0 += t.price * t.filled_quantity;
            e.1 += t.filled_quantity;
        }
    }

    let now = Utc::now();
    let mut positions = Vec::new();
    for b in balances {
        let asset = b.asset.as_str();
        let qty = b.free + b.locked;
        if qty <= Decimal::ZERO || is_stablecoin(asset) {
            continue;
        }
        let domain = format!("{asset}-USDT");
        let price = prices.get(&format!("{asset}USDT")).copied().unwrap_or(Decimal::ZERO);
        let (buy_cost, buy_qty) = cost
            .get(&domain)
            .copied()
            .unwrap_or((Decimal::ZERO, Decimal::ZERO));
        // 市价单可能以 price=0 入账，成本不可信时回落现价（避免浮盈=全市值误导）。
        let avg = if buy_qty > Decimal::ZERO && buy_cost > Decimal::ZERO {
            buy_cost / buy_qty
        } else {
            price
        };
        positions.push(Position {
            symbol: domain,
            quantity: qty,
            available_quantity: b.free,
            avg_price: avg,
            market_value: qty * price,
            unrealized_pnl: (price - avg) * qty,
            realized_pnl: Decimal::ZERO,
            updated_at: now,
        });
    }
    positions
}

/// 稳定币（视为现金，不算持仓）。
fn is_stablecoin(asset: &str) -> bool {
    matches!(asset, "USDT" | "USDC" | "TUSD" | "BUSD" | "FDUSD" | "DAI")
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
        let err = result.unwrap_err();
        assert_eq!(err.code, quant_common::api::code::NOT_INITIALIZED);
        assert_eq!(err.message, "Binance WebSocket not started");
    }

    #[tokio::test]
    async fn subscribe_depth_requires_running_ws() {
        let state = make_test_state();
        let result = subscribe_binance_depth(state_guard(&state), "BTC-USDT".to_string()).await;
        let err = result.unwrap_err();
        assert_eq!(err.code, quant_common::api::code::NOT_INITIALIZED);
        assert_eq!(err.message, "Binance WebSocket not started");
    }

    #[tokio::test]
    async fn subscribe_ticker_requires_running_ws() {
        let state = make_test_state();
        let result = subscribe_binance_ticker(state_guard(&state), "BTC-USDT".to_string()).await;
        let err = result.unwrap_err();
        assert_eq!(err.code, quant_common::api::code::NOT_INITIALIZED);
        assert_eq!(err.message, "Binance WebSocket not started");
    }

    #[tokio::test]
    async fn subscribe_trades_requires_running_ws() {
        let state = make_test_state();
        let result = subscribe_binance_trades(state_guard(&state), "BTC-USDT".to_string()).await;
        let err = result.unwrap_err();
        assert_eq!(err.code, quant_common::api::code::NOT_INITIALIZED);
        assert_eq!(err.message, "Binance WebSocket not started");
    }

    #[tokio::test]
    async fn subscribe_orderbook_requires_running_ws() {
        let state = make_test_state();
        let result = subscribe_binance_orderbook(state_guard(&state), "BTC-USDT".to_string()).await;
        let err = result.unwrap_err();
        assert_eq!(err.code, quant_common::api::code::NOT_INITIALIZED);
        assert_eq!(err.message, "Binance WebSocket not started");
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
