use crate::state::AppState;
use data_layer::{
    AccountSnapshotRecord, FundingRateRecord, MarkPriceRecord, MarketDataRecord,
    PositionSnapshotRecord, TickerSnapshotRecord,
};
use quant_common::api::code;
use quant_common::api::{err_result, ok_result};
use tauri::State;

/// Parse an ISO-8601 RFC3339 timestamp string into an optional `DateTime<Utc>`.
fn parse_ts(value: Option<String>) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    match value {
        None => Ok(None),
        Some(v) => {
            let parsed = chrono::DateTime::parse_from_rfc3339(&v)
                .map_err(|e| format!("Invalid timestamp '{}': {}", v, e))?;
            Ok(Some(parsed.with_timezone(&chrono::Utc)))
        }
    }
}

/// 查询行情数据中可用的标的列表（下拉数据源，来自数据库）。
#[tauri::command]
pub async fn get_symbols(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<String>>> {
    use crate::commands::not_init_err;
    use quant_common::api::ok_result;
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let syms = services.market_service.list_symbols().await?;
    ok_result(syms)
}

/// 从数据库读取某标的/周期的最新 K 线（remote WS 导入后再由前端从 DB 读）。
#[tauri::command]
pub async fn get_klines(
    state: State<'_, AppState>,
    symbol: String,
    timeframe: String,
    limit: Option<i64>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<MarketDataRecord>>> {
    if let Err(e) = state.require_auth().await {
        return err_result(code::UNAUTHORIZED, e);
    }
    let Some(services) = state.app_services.as_ref() else {
        return err_result(code::NOT_INITIALIZED, "行情服务未初始化（无数据库连接）");
    };
    let data = services
        .market_service
        .get_klines_from_db(&symbol, &timeframe, limit.unwrap_or(100))
        .await?;
    ok_result(data)
}

/// 从数据库读取某标的最近 N 笔逐笔成交（remote WS 导入后前端从 DB 读）。
#[tauri::command]
pub async fn get_trades(
    state: State<'_, AppState>,
    symbol: String,
    limit: Option<i64>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<data_layer::StreamTradeRecord>>> {
    if let Err(e) = state.require_auth().await {
        return err_result(code::UNAUTHORIZED, e);
    }
    let Some(services) = state.app_services.as_ref() else {
        return err_result(code::NOT_INITIALIZED, "行情服务未初始化（无数据库连接）");
    };
    let data = services
        .market_service
        .get_trades_from_db(&symbol, limit.unwrap_or(100))
        .await?;
    ok_result(data)
}

/// 从数据库读取某标的最新订单簿快照（remote WS 导入后前端从 DB 读）。
#[tauri::command]
pub async fn get_orderbook(
    state: State<'_, AppState>,
    symbol: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Option<data_layer::OrderbookSnapshotRecord>>> {
    if let Err(e) = state.require_auth().await {
        return err_result(code::UNAUTHORIZED, e);
    }
    let Some(services) = state.app_services.as_ref() else {
        return err_result(code::NOT_INITIALIZED, "行情服务未初始化（无数据库连接）");
    };
    let data = services.market_service.get_orderbook_from_db(&symbol).await?;
    ok_result(data)
}

/// 从数据库读取逐资产余额（快照写入器每 60s 落库）。
#[tauri::command]
pub async fn get_balances(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<data_layer::BalanceRecord>>> {
    if let Err(e) = state.require_auth().await {
        return err_result(code::UNAUTHORIZED, e);
    }
    let Some(services) = state.app_services.as_ref() else {
        return err_result(code::NOT_INITIALIZED, "行情服务未初始化（无数据库连接）");
    };
    let data = services.market_service.get_balances_from_db().await?;
    ok_result(data)
}

/// 从数据库读取全标的最近价（快照写入器每 60s 落库）。
#[tauri::command]
pub async fn get_last_prices(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<data_layer::LastPriceRecord>>> {
    if let Err(e) = state.require_auth().await {
        return err_result(code::UNAUTHORIZED, e);
    }
    let Some(services) = state.app_services.as_ref() else {
        return err_result(code::NOT_INITIALIZED, "行情服务未初始化（无数据库连接）");
    };
    let data = services.market_service.get_last_prices_from_db().await?;
    ok_result(data)
}

/// 查询行情快照（按标的 + 可选时间范围）。
#[tauri::command]
pub async fn get_ticker_snapshots(
    state: State<'_, AppState>,
    inst_id: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<TickerSnapshotRecord>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::{code, ok_result, ApiFailure};
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let from = parse_ts(from).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let to = parse_ts(to).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let rows = services
        .market_service
        .get_ticker_snapshots(&inst_id, from, to, limit)
        .await?;
    ok_result(rows)
}

/// 记录当前账户权益快照（资产曲线的点）。
#[tauri::command]
pub async fn record_account_snapshot(
    state: State<'_, AppState>,
    eq: rust_decimal::Decimal,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<serde_json::Value>> {
    use crate::commands::not_init_err;
    use quant_common::api::ok_result;
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("应用服务未初始化"))?;
    services.account_service.record_equity_snapshot(eq).await?;
    ok_result(serde_json::Value::Null)
}

/// 查询账户快照（按币种 + 可选时间范围）。
#[tauri::command]
pub async fn get_account_snapshots(
    state: State<'_, AppState>,
    ccy: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<AccountSnapshotRecord>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::{code, ok_result, ApiFailure};
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let from = parse_ts(from).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let to = parse_ts(to).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let rows = services
        .market_service
        .get_account_snapshots(&ccy, from, to, limit)
        .await?;
    ok_result(rows)
}

/// 查询持仓快照（按标的 + 可选时间范围）。
#[tauri::command]
pub async fn get_position_snapshots(
    state: State<'_, AppState>,
    inst_id: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<PositionSnapshotRecord>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::{code, ok_result, ApiFailure};
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let from = parse_ts(from).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let to = parse_ts(to).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let rows = services
        .market_service
        .get_position_snapshots(&inst_id, from, to, limit)
        .await?;
    ok_result(rows)
}

/// 查询资金费率（按标的 + 可选时间范围）。
#[tauri::command]
pub async fn get_funding_rates(
    state: State<'_, AppState>,
    inst_id: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<FundingRateRecord>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::{code, ok_result, ApiFailure};
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let from = parse_ts(from).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let to = parse_ts(to).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let rows = services
        .market_service
        .get_funding_rates(&inst_id, from, to, limit)
        .await?;
    ok_result(rows)
}

/// 查询标记价格（按标的 + 可选时间范围）。
#[tauri::command]
pub async fn get_mark_prices(
    state: State<'_, AppState>,
    inst_id: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<MarkPriceRecord>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::{code, ok_result, ApiFailure};
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let from = parse_ts(from).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let to = parse_ts(to).map_err(|e| ApiFailure::new(code::INVALID_PARAM, e))?;
    let rows = services
        .market_service
        .get_mark_prices(&inst_id, from, to, limit)
        .await?;
    ok_result(rows)
}
