//! Binance exchange commands.
//!
//! Thin Tauri adapters that delegate to `AppServices.binance_service`.
//! They never touch the Binance client or data layers directly (layering/DIP).

use crate::state::AppState;
use exchange_binance::types::{BinanceOrder, BinanceOrderType, BinancePlaceOrderRequest, BinanceSide};
use quant_common::types::{Order, OrderSide, OrderStatus, OrderType};
use risk_layer::pre_trade::PreTradeRiskChecker;
use rust_decimal::Decimal;
use tauri::State;

/// 把实盘下单请求映射为 App `Order`（用于前置风控校验）。
fn live_request_to_order(req: &BinancePlaceOrderRequest) -> Order {
    let side = match req.side {
        BinanceSide::Buy => OrderSide::Buy,
        BinanceSide::Sell => OrderSide::Sell,
    };
    let order_type = match req.order_type {
        BinanceOrderType::Market => OrderType::Market,
        BinanceOrderType::Limit => OrderType::Limit,
    };
    Order {
        order_id: 0,
        strategy_id: req.strategy_id.clone().unwrap_or_default(),
        symbol: exchange_binance::from_binance_symbol(&req.symbol),
        order_type,
        side,
        price: req.price,
        quantity: req.quantity,
        filled_quantity: Decimal::ZERO,
        status: OrderStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        commission: Decimal::ZERO,
        slippage: Decimal::ZERO,
        exchange: "live".to_string(),
    }
}

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

#[tauri::command]
pub async fn get_binance_balance(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<exchange_binance::types::BinanceBalance>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::{ok_result};
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = services.binance_service.get_balance().await?;
    ok_result(data)
}

/// 全市场价格（`/api/v3/ticker/price`，用于持仓实时价格/市值补全）。
#[tauri::command]
pub async fn get_binance_ticker_prices(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<std::collections::HashMap<String, rust_decimal::Decimal>>> {
    use crate::commands::not_init_err;
    use quant_common::api::ok_result;
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = services.binance_service.get_all_ticker_prices().await?;
    ok_result(data)
}

#[tauri::command]
pub async fn get_binance_candles(
    state: State<'_, AppState>,
    symbol: String,
    interval: String,
    limit: Option<u32>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<exchange_binance::types::BinanceKline>>> {
    use crate::commands::not_init_err;
    use quant_common::api::ok_result;
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = services.binance_service.get_candles(&symbol, &interval, limit).await?;
    ok_result(data)
}

#[tauri::command]
pub async fn get_binance_order_book(
    state: State<'_, AppState>,
    symbol: String,
    limit: Option<u32>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<exchange_binance::types::BinanceOrderBook>> {
    use crate::commands::not_init_err;
    use quant_common::api::ok_result;
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = services.binance_service.get_order_book(&symbol, limit).await?;
    ok_result(data)
}

#[tauri::command]
pub async fn place_binance_order(
    state: State<'_, AppState>,
    request: BinancePlaceOrderRequest,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<exchange_binance::types::BinanceOrder>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::{ok_result, ApiFailure};
    let Some(services) = state.app_services.as_ref() else {
        return Err(not_init_err("Binance 服务未初始化"));
    };
    // 实盘下单必须先鉴权（未登录/越权一律拒绝）。
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    // 前置风控（与纸面路径一致性）：现金/持仓/单日亏损/集中度。
    let app_order = live_request_to_order(&request);
    let risk_config = services
        .risk_service
        .get_risk_config()
        .await
        .map_err(|e| ApiFailure::new(quant_common::api::code::INTERNAL, format!("风控配置不可用（fail-closed）：{}", e)))?;
    if risk_config.enable_pre_trade_check {
        let checker = PreTradeRiskChecker::new(risk_config);
        let account = services
            .account_service
            .get_account_info()
            .await
            .map_err(|e| ApiFailure::new(quant_common::api::code::RISK_REJECTED, format!("风控失败：无法获取账户（fail-closed）：{}", e)))?;
        let positions = services
            .account_service
            .get_paper_positions()
            .await
            .map_err(|e| ApiFailure::new(quant_common::api::code::RISK_REJECTED, format!("风控失败：无法获取持仓（fail-closed）：{}", e)))?;
        let reference = services
            .market_service
            .get_realtime_data(&app_order.symbol)
            .await
            .map(|d| d.close)
            .ok();
        checker
            .check_order_with_reference_price(&app_order, &account, &positions, reference)
            .map_err(|e| ApiFailure::new(quant_common::api::code::RISK_REJECTED, format!("风控校验失败：{}", e)))?;
    }
    let order = services.binance_service.place_order(request.clone()).await?;
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
    ok_result(order)
}

/// 读取本地记录的 live 单成交记录（策略关联 + 成交价/量）。
#[tauri::command]
pub async fn get_live_trades(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<data_layer::LiveTrade>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = services.live_trades.list().await?;
    ok_result(data)
}

#[tauri::command]
pub async fn cancel_binance_order(
    state: State<'_, AppState>,
    symbol: String,
    order_id: i64,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::{ok_result};
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    services
        .binance_service
        .cancel_order(&symbol, order_id)
        .await?;
    ok_result(serde_json::Value::Null)
}

#[tauri::command]
pub async fn get_binance_positions(
    state: State<'_, AppState>,
    symbol: Option<String>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<exchange_binance::types::BinancePosition>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = services.binance_service.get_positions(symbol.as_deref()).await?;
    ok_result(data)
}

#[tauri::command]
pub async fn get_binance_orders(
    state: State<'_, AppState>,
    symbol: String,
    history: Option<bool>,
    limit: Option<u32>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<exchange_binance::types::BinanceOrder>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = if history.unwrap_or(false) {
        services.binance_service.get_all_orders(&symbol, limit).await?
    } else {
        let sym = if symbol.trim().is_empty() { None } else { Some(symbol.as_str()) };
        services.binance_service.get_open_orders(sym).await?
    };
    ok_result(data)
}

#[tauri::command]
pub async fn get_binance_order(
    state: State<'_, AppState>,
    symbol: String,
    order_id: i64,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<exchange_binance::types::BinanceOrder>> {
    use crate::commands::not_init_err;
    use quant_common::api::ok_result;
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = services.binance_service.get_order(&symbol, order_id).await?;
    ok_result(data)
}

#[tauri::command]
pub async fn get_binance_instruments(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    use crate::commands::not_init_err;
    use quant_common::api::ok_result;
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = services.binance_service.get_instruments().await?;
    ok_result(data)
}

#[tauri::command]
pub async fn check_binance_status(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    use crate::commands::not_init_err;
    use quant_common::api::ok_result;
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("Binance 服务未初始化"))?;
    let data = services.binance_service.check_status().await?;
    ok_result(data)
}
