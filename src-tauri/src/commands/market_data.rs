use crate::state::AppState;
use data_layer::{
    AccountSnapshotRecord, FundingRateRecord, MarkPriceRecord, PositionSnapshotRecord,
    TickerSnapshotRecord,
};
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
pub async fn get_symbols(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .market_service
        .list_symbols()
        .await
        .map_err(|e| e.to_string())
}

/// 查询行情快照（按标的 + 可选时间范围）。
#[tauri::command]
pub async fn get_ticker_snapshots(
    state: State<'_, AppState>,
    inst_id: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<TickerSnapshotRecord>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let from = parse_ts(from)?;
    let to = parse_ts(to)?;
    services
        .market_service
        .get_ticker_snapshots(&inst_id, from, to, limit)
        .await
        .map_err(|e| e.to_string())
}

/// 记录当前账户权益快照（资产曲线的点）。
#[tauri::command]
pub async fn record_account_snapshot(
    state: State<'_, AppState>,
    eq: rust_decimal::Decimal,
) -> Result<(), String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .account_service
        .record_equity_snapshot(eq)
        .await
        .map_err(|e| e.to_string())
}

/// 查询账户快照（按币种 + 可选时间范围）。
#[tauri::command]
pub async fn get_account_snapshots(
    state: State<'_, AppState>,
    ccy: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<AccountSnapshotRecord>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let from = parse_ts(from)?;
    let to = parse_ts(to)?;
    services
        .market_service
        .get_account_snapshots(&ccy, from, to, limit)
        .await
        .map_err(|e| e.to_string())
}

/// 查询持仓快照（按标的 + 可选时间范围）。
#[tauri::command]
pub async fn get_position_snapshots(
    state: State<'_, AppState>,
    inst_id: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<PositionSnapshotRecord>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let from = parse_ts(from)?;
    let to = parse_ts(to)?;
    services
        .market_service
        .get_position_snapshots(&inst_id, from, to, limit)
        .await
        .map_err(|e| e.to_string())
}

/// 查询资金费率（按标的 + 可选时间范围）。
#[tauri::command]
pub async fn get_funding_rates(
    state: State<'_, AppState>,
    inst_id: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<FundingRateRecord>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let from = parse_ts(from)?;
    let to = parse_ts(to)?;
    services
        .market_service
        .get_funding_rates(&inst_id, from, to, limit)
        .await
        .map_err(|e| e.to_string())
}

/// 查询标记价格（按标的 + 可选时间范围）。
#[tauri::command]
pub async fn get_mark_prices(
    state: State<'_, AppState>,
    inst_id: String,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<MarkPriceRecord>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let from = parse_ts(from)?;
    let to = parse_ts(to)?;
    services
        .market_service
        .get_mark_prices(&inst_id, from, to, limit)
        .await
        .map_err(|e| e.to_string())
}
