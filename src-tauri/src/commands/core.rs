use crate::state::AppState;
use chrono::Utc;
use quant_common::config::AppConfig;
use quant_common::types::{Account, MarketData, Order, Position};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.read().await;
    Ok(config.clone())
}

/// 更新系统配置
#[tauri::command]
pub async fn update_config(state: State<'_, AppState>, config: AppConfig) -> Result<bool, String> {
    // Delegate to ConfigService which updates both in-memory state and persistent file
    match state.app_services.as_ref() {
        Some(services) => {
            let status = services.config_service.update_config(config).await;
            // Log persistence status so users can see it in the UI log panel
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: if status.contains("failed") {
                        "warn".to_string()
                    } else {
                        "info".to_string()
                    },
                    message: status,
                    module: Some("config".to_string()),
                })
                .await;
            Ok(true)
        }
        None => {
            // Fallback: update in-memory only (no ConfigService without DB)
            {
                let mut app_config = state.config.write().await;
                *app_config = config;
            }
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "warn".to_string(),
                    message: "Config updated in memory only (no persistence path)".to_string(),
                    module: Some("config".to_string()),
                })
                .await;
            Ok(true)
        }
    }
}

#[tauri::command]
pub async fn get_market_data(
    state: State<'_, AppState>,
    symbol: String,
) -> Result<MarketData, String> {
    // Try OKX data source first (provides real OKX demo data)
    let data_source = state.okx_data_source.read().await;
    if let Some(source) = data_source.as_ref() {
        use data_layer::market_data::DataSource;
        match source.get_realtime_data(&symbol).await {
            Ok(data) => return Ok(data),
            Err(e) => {
                state
                    .log_buffer
                    .add_entry(quant_common::types::LogEntry {
                        timestamp: Utc::now(),
                        level: "warn".to_string(),
                        message: format!(
                            "OKX data source unavailable for {}, falling back: {}",
                            symbol, e
                        ),
                        module: Some("commands".to_string()),
                    })
                    .await;
            }
        }
    }
    drop(data_source);

    Err(format!(
        "Market data unavailable for {}: no data source connected",
        symbol
    ))
}

/// Internal helper: fetch real market data from OKX data source or DB.
async fn get_market_data_internal(
    state: &State<'_, AppState>,
    symbol: &str,
) -> Result<MarketData, String> {
    use data_layer::market_data::DataSource;
    let data_source = state.okx_data_source.read().await;
    if let Some(source) = data_source.as_ref() {
        match source.get_realtime_data(symbol).await {
            Ok(data) => return Ok(data),
            Err(e) => {
                state
                    .log_buffer
                    .add_entry(quant_common::types::LogEntry {
                        timestamp: chrono::Utc::now(),
                        level: "warn".to_string(),
                        message: format!(
                            "OKX data source unavailable for {}, falling back: {}",
                            symbol, e
                        ),
                        module: Some("commands".to_string()),
                    })
                    .await
            }
        }
    }
    drop(data_source);

    Err(format!("Market data unavailable for {}", symbol))
}

#[tauri::command]
pub async fn submit_order(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    order: Order,
) -> Result<String, String> {
    use rust_decimal_macros::dec;
    use std::sync::Arc;
    use trading_layer::ExecutionEngine;

    let order_manager = Arc::new(state.order_manager.clone());
    let mut order = order;

    let trading_config = state.config.read().await.trading.clone();
    let mut risk_config = state.config.read().await.risk.clone();
    let enable_pre_trade = risk_config.enable_pre_trade_check;

    if let Some(ref services) = state.app_services {
        if let Ok(db_risk_config) = services.risk_service.get_risk_config().await {
            risk_config = db_risk_config;
        }
    }

    let market_data = get_market_data_internal(&state, &order.symbol)
        .await
        .unwrap_or_else(|_| {
            let fallback_price = order.price.unwrap_or(dec!(100));
            MarketData {
                symbol: order.symbol.clone(),
                timestamp: chrono::Utc::now(),
                open: fallback_price,
                high: fallback_price,
                low: fallback_price,
                close: fallback_price,
                volume: dec!(0),
                turnover: dec!(0),
                open_interest: None,
                bid_prices: vec![],
                bid_volumes: vec![],
                ask_prices: vec![],
                ask_volumes: vec![],
            }
        });

    if enable_pre_trade {
        let checker = risk_layer::PreTradeRiskChecker::new(risk_config);

        // Fetch account/positions from DB if available for the risk check
        if let Some(ref services) = state.app_services {
            if let (Ok(account), Ok(positions)) = (
                services.account_service.get_account_info().await,
                services.account_service.get_positions().await,
            ) {
                checker
                    .check_order_with_reference_price(
                        &order,
                        &account,
                        &positions,
                        Some(market_data.close),
                    )
                    .map_err(|e| format!("Risk check failed: {}", e))?;
            }
        }
    }

    // Get OKX executor from shared state
    let okx_executor = state.okx_executor.read().await.clone();

    // Create execution engine with shared instances
    let execution_engine = ExecutionEngine::new(
        order_manager.clone(),
        trading_config,
        okx_executor.map(Arc::new),
    );

    // Submit order to shared order manager
    let order_id = order_manager
        .submit_order(order.clone())
        .await
        .map_err(|e| format!("Failed to submit order: {}", e))?;
    order.order_id = order_id;
    let order_id_str = order_id.to_string();

    // Log order submission
    state
        .log_buffer
        .add_entry(quant_common::types::LogEntry {
            timestamp: Utc::now(),
            level: "info".to_string(),
            message: format!(
                "Order {} submitted for symbol {}",
                order_id_str, order.symbol
            ),
            module: Some("trading".to_string()),
        })
        .await;

    // Persist order to PostgreSQL (graceful degradation if DB unavailable)
    if let Some(ref services) = state.app_services {
        match services.account_service.get_account_info().await {
            Ok(account) => {
                if let Err(e) = services
                    .account_service
                    .persist_order(&order, &account.account_id)
                    .await
                {
                    state
                        .log_buffer
                        .add_entry(quant_common::types::LogEntry {
                            timestamp: Utc::now(),
                            level: "warn".to_string(),
                            message: format!("Order persisted to DB failed: {}", e),
                            module: Some("commands".to_string()),
                        })
                        .await;
                }
            }
            Err(_) => {
                state
                    .log_buffer
                    .add_entry(quant_common::types::LogEntry {
                        timestamp: Utc::now(),
                        level: "warn".to_string(),
                        message: "Account not available for order persistence".to_string(),
                        module: Some("commands".to_string()),
                    })
                    .await;
            }
        }
    }

    let _ = app.emit(
        "order:submitted",
        serde_json::json!({
            "order_id": order_id,
            "symbol": order.symbol,
            "side": order.side,
            "order_type": order.order_type,
            "price": order.price,
            "quantity": order.quantity,
            "status": "Submitted",
            "timestamp": Utc::now().to_rfc3339(),
        }),
    );

    // Execute order asynchronously
    let log = state.log_buffer.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        if let Err(e) = execution_engine.execute_order(order, &market_data).await {
            log.add_entry(quant_common::types::LogEntry {
                timestamp: chrono::Utc::now(),
                level: "error".to_string(),
                message: format!("Order execution failed: {}", e),
                module: Some("trading".to_string()),
            })
            .await;
        }
    });

    Ok(order_id_str)
}

#[tauri::command]
pub async fn get_account_info(state: State<'_, AppState>) -> Result<Account, String> {
    match state.app_services.as_ref() {
        Some(services) => match services.account_service.get_account_info().await {
            Ok(account) => {
                monitor_layer::MetricsCollector::set_account_balance(
                    account.total_assets.to_f64().unwrap_or(0.0),
                );
                monitor_layer::MetricsCollector::set_position_value(
                    account.market_value.to_f64().unwrap_or(0.0),
                );
                monitor_layer::MetricsCollector::set_daily_pnl(
                    account.daily_pnl.to_f64().unwrap_or(0.0),
                );
                Ok(account)
            }
            Err(service_error) => {
                let msg = format!("Account info unavailable: {}", service_error);
                state
                    .log_buffer
                    .add_entry(quant_common::types::LogEntry {
                        timestamp: Utc::now(),
                        level: "error".to_string(),
                        message: msg.clone(),
                        module: Some("commands".to_string()),
                    })
                    .await;
                Err(msg)
            }
        },
        None => Err("Account service not initialized (no database connection)".to_string()),
    }
}

#[tauri::command]
pub async fn get_positions(state: State<'_, AppState>) -> Result<Vec<Position>, String> {
    match state.app_services.as_ref() {
        Some(services) => match services.account_service.get_positions().await {
            Ok(positions) => Ok(positions),
            Err(e) => Err(format!("Positions unavailable: {}", e)),
        },
        None => Err("Account service not initialized (no database connection)".to_string()),
    }
}

#[tauri::command]
pub async fn get_active_orders(state: State<'_, AppState>) -> Result<Vec<Order>, String> {
    // 优先从 OrderManager 内存获取真实活跃订单（Submitted / PartiallyFilled）
    let manager_orders = state.order_manager.get_active_orders().await;
    if !manager_orders.is_empty() {
        return Ok(manager_orders);
    }

    // 内存无活跃订单时，降级查询数据库
    if let Some(services) = state.app_services.as_ref() {
        if let Ok(orders) = services.account_service.get_active_orders().await {
            return Ok(orders);
        }
    }

    // 无任何数据源可用 → 返回空列表（不再返回硬编码 mock 假数据）
    Ok(Vec::new())
}
