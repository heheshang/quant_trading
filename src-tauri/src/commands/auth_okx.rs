use crate::state::AppState;
use chrono::Utc;
use exchange_okx::types::*;
use quant_common::types::{MarketData, Order};
use tauri::State;

/// 用户登录
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<String, String> {
    // Delegate to AuthService which verifies against database password hashes
    match state.app_services.as_ref() {
        Some(services) => services
            .auth_service
            .login(&username, &password)
            .await
            .map_err(|e| e.to_string()),
        None => {
            // Fallback only when no database — never hardcode credentials
            Err("Authentication unavailable: no database connection".to_string())
        }
    }
}

/// 验证 Token
#[tauri::command]
pub async fn verify_token(state: State<'_, AppState>, token: String) -> Result<bool, String> {
    match state.app_services.as_ref() {
        Some(services) => Ok(services.auth_service.verify_token(&token).await),
        None => {
            let config = state.config.read().await;
            let auth_service = security::AuthService::new(
                config.security.jwt_secret.clone(),
                config.security.token_expiry_hours as i64,
            );
            Ok(auth_service.verify_token(&token).is_ok())
        }
    }
}

/// 更新用户资料
#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    profile_data: serde_json::Value,
) -> Result<bool, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    // Use the username from the profile data, or default to "admin"
    let username = profile_data
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("admin");

    services
        .auth_service
        .update_profile(username, &profile_data)
        .await
        .map_err(|e| e.to_string())
}

/// 修改密码
#[tauri::command]
pub async fn change_password(
    state: State<'_, AppState>,
    current_password: String,
    new_password: String,
    username: Option<String>,
) -> Result<bool, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let username = username.unwrap_or_else(|| "admin".to_string());

    services
        .auth_service
        .change_password(&username, &current_password, &new_password)
        .await
        .map_err(|e| e.to_string())
}

/// 获取用户资料
#[tauri::command]
pub async fn get_user_profile(
    state: State<'_, AppState>,
    username: Option<String>,
) -> Result<serde_json::Value, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let username = username.unwrap_or_else(|| "admin".to_string());
    services
        .auth_service
        .get_user_profile(&username)
        .await
        .map_err(|e| e.to_string())
}

// ==================== OKX Integration Commands ====================

/// 获取 OKX 账户余额
#[tauri::command]
pub async fn get_okx_balance(
    state: State<'_, AppState>,
    ccy: Option<String>,
) -> Result<Vec<BalanceView>, String> {
    let okx_client_opt = state.okx_client.read().await;

    match okx_client_opt.as_ref() {
        Some(client_arc) => {
            let client = client_arc.read().await;
            let balances = client
                .get_account_balance(ccy.as_deref())
                .await
                .map_err(|e| format!("Failed to get OKX balance: {}", e))?;

            // Log the operation
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "info".to_string(),
                    message: format!(
                        "Retrieved OKX balance for {}",
                        ccy.unwrap_or_else(|| "all currencies".to_string())
                    ),
                    module: Some("okx".to_string()),
                })
                .await;

            let views: Vec<BalanceView> = balances.into_iter().map(BalanceView::from).collect();
            Ok(views)
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 获取 OKX 持仓
#[tauri::command]
pub async fn get_okx_positions(
    state: State<'_, AppState>,
    inst_id: Option<String>,
) -> Result<Vec<PositionView>, String> {
    let okx_client_opt = state.okx_client.read().await;

    match okx_client_opt.as_ref() {
        Some(client_arc) => {
            let client = client_arc.read().await;
            let positions = client
                .get_positions(inst_id.as_deref())
                .await
                .map_err(|e| format!("Failed to get OKX positions: {}", e))?;

            // Log the operation
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "info".to_string(),
                    message: format!(
                        "Retrieved OKX positions for {}",
                        inst_id.unwrap_or_else(|| "all instruments".to_string())
                    ),
                    module: Some("okx".to_string()),
                })
                .await;

            let views: Vec<PositionView> = positions.into_iter().map(PositionView::from).collect();
            Ok(views)
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 下单到 OKX
#[tauri::command]
pub async fn place_okx_order(
    state: State<'_, AppState>,
    request: OkxPlaceOrderRequest,
) -> Result<OrderView, String> {
    let okx_client_opt = state.okx_client.read().await;

    match okx_client_opt.as_ref() {
        Some(client_arc) => {
            let client = client_arc.read().await;
            let order = client
                .place_order(request.clone())
                .await
                .map_err(|e| format!("Failed to place OKX order: {}", e))?;

            // Log the operation
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "info".to_string(),
                    message: format!(
                        "Placed OKX order: {} {} {} @ {}",
                        request.side,
                        request.sz,
                        request.inst_id,
                        request.px.as_ref().unwrap_or(&"market".to_string())
                    ),
                    module: Some("okx".to_string()),
                })
                .await;

            // Increment metrics
            monitor_layer::MetricsCollector::inc_orders_total();

            Ok(OrderView::from(order))
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 撤销 OKX 订单
#[tauri::command]
pub async fn cancel_okx_order(
    state: State<'_, AppState>,
    inst_id: String,
    ord_id: String,
) -> Result<bool, String> {
    let okx_client_opt = state.okx_client.read().await;

    match okx_client_opt.as_ref() {
        Some(client_arc) => {
            let client = client_arc.read().await;
            client
                .cancel_order(&inst_id, &ord_id)
                .await
                .map_err(|e| format!("Failed to cancel OKX order: {}", e))?;

            // Log the operation
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "info".to_string(),
                    message: format!("Cancelled OKX order: {} on {}", ord_id, inst_id),
                    module: Some("okx".to_string()),
                })
                .await;

            // Increment metrics
            monitor_layer::MetricsCollector::inc_orders_cancelled();

            Ok(true)
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 获取 OKX K线数据
#[tauri::command]
pub async fn get_okx_candles(
    state: State<'_, AppState>,
    inst_id: String,
    bar: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<CandleView>, String> {
    let okx_client_opt = state.okx_client.read().await;

    match okx_client_opt.as_ref() {
        Some(client_arc) => {
            let client = client_arc.read().await;
            let bar = bar.as_deref().unwrap_or("1H");
            let candles = client
                .get_candles(&inst_id, bar, limit)
                .await
                .map_err(|e| format!("Failed to get OKX candles: {}", e))?;

            let views: Vec<CandleView> = candles.into_iter().map(CandleView::from).collect();
            Ok(views)
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 获取 OKX 交易对信息
#[tauri::command]
pub async fn get_okx_instruments(
    state: State<'_, AppState>,
    inst_type: Option<String>,
) -> Result<Vec<InstrumentView>, String> {
    let okx_client_opt = state.okx_client.read().await;

    match okx_client_opt.as_ref() {
        Some(client_arc) => {
            let client = client_arc.read().await;
            let inst_type = inst_type.as_deref().unwrap_or("SPOT");
            let instruments_json = client
                .get_instruments(inst_type)
                .await
                .map_err(|e| format!("Failed to get OKX instruments: {}", e))?;

            let instruments: Vec<OkxInstrument> = serde_json::from_value(instruments_json)
                .map_err(|e| format!("Failed to parse instruments: {}", e))?;
            let views: Vec<InstrumentView> =
                instruments.into_iter().map(InstrumentView::from).collect();
            Ok(views)
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 检查 OKX 连接状态
#[tauri::command]
pub async fn check_okx_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let okx_client = state.okx_client.read().await;
    let config = state.config.read().await;

    let status = serde_json::json!({
        "connected": okx_client.is_some(),
        "enabled": config.okx.enable,
        "environment": config.okx.environment,
        "has_credentials": !config.okx.api_key.is_empty(),
    });

    Ok(status)
}

/// 获取 OKX 公告
#[tauri::command]
pub async fn get_okx_announcements(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let okx_client_opt = state.okx_client.read().await;

    match okx_client_opt.as_ref() {
        Some(client_arc) => {
            let client = client_arc.read().await;
            let announcements = client
                .get_announcements()
                .await
                .map_err(|e| format!("Failed to get OKX announcements: {}", e))?;

            serde_json::to_value(announcements)
                .map_err(|e| format!("Failed to serialize announcements: {}", e))
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 使用 OKX 执行器下单
#[tauri::command]
pub async fn execute_okx_order(state: State<'_, AppState>, order: Order) -> Result<String, String> {
    let executor = state.okx_executor.read().await;

    match executor.as_ref() {
        Some(exec) => {
            let start = std::time::Instant::now();

            // Increment API calls metric
            monitor_layer::OKX_API_CALLS.inc();

            let result = exec.execute_order(&order).await;

            // Record latency
            let duration = start.elapsed().as_secs_f64();
            monitor_layer::OKX_API_LATENCY.observe(duration);

            match result {
                Ok(ord_id) => {
                    // Increment orders placed metric
                    monitor_layer::OKX_ORDERS_PLACED.inc();
                    monitor_layer::MetricsCollector::inc_orders_total();

                    // Log the operation
                    state
                        .log_buffer
                        .add_entry(quant_common::types::LogEntry {
                            timestamp: Utc::now(),
                            level: "info".to_string(),
                            message: format!(
                                "Executed order {} on OKX: {}",
                                order.order_id, ord_id
                            ),
                            module: Some("okx_executor".to_string()),
                        })
                        .await;

                    Ok(ord_id)
                }
                Err(e) => {
                    monitor_layer::OKX_API_ERRORS.inc();

                    // Log the error
                    state
                        .log_buffer
                        .add_entry(quant_common::types::LogEntry {
                            timestamp: Utc::now(),
                            level: "error".to_string(),
                            message: format!("Failed to execute order on OKX: {}", e),
                            module: Some("okx_executor".to_string()),
                        })
                        .await;

                    Err(format!("Failed to execute order: {}", e))
                }
            }
        }
        None => Err("OKX executor not initialized".to_string()),
    }
}

/// 使用 OKX 数据源获取实时数据
#[tauri::command]
pub async fn get_okx_realtime_data(
    state: State<'_, AppState>,
    symbol: String,
) -> Result<MarketData, String> {
    // Route through the services layer (market_service wraps the data source)
    // so the command never touches the data-layer directly (layering + DIP).
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| "Market service not initialized".to_string())?;

    let start = std::time::Instant::now();
    monitor_layer::OKX_API_CALLS.inc();

    let result = services.market_service.get_realtime_data(&symbol).await;

    let duration = start.elapsed().as_secs_f64();
    monitor_layer::OKX_API_LATENCY.observe(duration);

    match result {
        Ok(data) => Ok(data),
        Err(e) => {
            monitor_layer::OKX_API_ERRORS.inc();
            Err(format!("Failed to get realtime data: {}", e))
        }
    }
}

/// 使用 OKX 数据源获取历史数据
#[tauri::command]
pub async fn get_okx_historical_data(
    state: State<'_, AppState>,
    symbol: String,
    start: String,
    end: String,
) -> Result<Vec<MarketData>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| "Market service not initialized".to_string())?;

    use chrono::DateTime;

    let start_dt = DateTime::parse_from_rfc3339(&start)
        .map_err(|e| format!("Invalid start date: {}", e))?
        .with_timezone(&Utc);

    let end_dt = DateTime::parse_from_rfc3339(&end)
        .map_err(|e| format!("Invalid end date: {}", e))?
        .with_timezone(&Utc);

    monitor_layer::OKX_API_CALLS.inc();
    let start_time = std::time::Instant::now();

    let result = services
        .market_service
        .get_historical_data(&symbol, start_dt, end_dt)
        .await;

    let duration = start_time.elapsed().as_secs_f64();
    monitor_layer::OKX_API_LATENCY.observe(duration);

    match result {
        Ok(data) => Ok(data),
        Err(e) => {
            monitor_layer::OKX_API_ERRORS.inc();
            Err(format!("Failed to get historical data: {}", e))
        }
    }
}
