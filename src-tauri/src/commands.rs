use crate::state::AppState;
use chrono::Utc;
use exchange_okx::types::*;
use quant_common::config::AppConfig;
use quant_common::types::{
    Account, Alert, BacktestResult, MarketData, Order, Position, StrategyParams,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal_macros::dec;
use std::collections::HashMap;
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
                    level: if status.contains("failed") { "warn".to_string() } else { "info".to_string() },
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

    // Fallback: query DB via account_service for latest snapshot
    if let Some(ref services) = state.app_services {
        if let Ok(account) = services.account_service.get_account_info().await {
            // Return market data derived from account state
            return Ok(MarketData {
                symbol: symbol.clone(),
                timestamp: chrono::Utc::now(),
                open: account.total_assets,
                high: account.total_assets + account.daily_pnl,
                low: account.total_assets - account.daily_pnl,
                close: account.total_assets,
                volume: dec!(0),
                turnover: dec!(0),
                open_interest: None,
                bid_prices: vec![],
                bid_volumes: vec![],
                ask_prices: vec![],
                ask_volumes: vec![],
            });
        }
    }

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
            Err(e) => state.log_buffer.add_entry(quant_common::types::LogEntry {
                timestamp: chrono::Utc::now(),
                level: "warn".to_string(),
                message: format!("OKX data source unavailable for {}, falling back: {}", symbol, e),
                module: Some("commands".to_string()),
            }).await,
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

    let app_config = state.config.read().await;
    let trading_config = app_config.trading.clone();
    let risk_config = app_config.risk.clone();
    let enable_pre_trade = app_config.risk.enable_pre_trade_check;
    drop(app_config);

    if enable_pre_trade {
        let checker = risk_layer::PreTradeRiskChecker::new(risk_config);

        // Fetch account/positions from DB if available for the risk check
        if let Some(ref services) = state.app_services {
            if let (Ok(account), Ok(positions)) = (
                services.account_service.get_account_info().await,
                services.account_service.get_positions().await,
            ) {
                checker
                    .check_order(&order, &account, &positions)
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
        .map_err(|e| format!("Failed to submit order: {}", e))?
        .to_string();

    // Log order submission
    state
        .log_buffer
        .add_entry(quant_common::types::LogEntry {
            timestamp: Utc::now(),
            level: "info".to_string(),
            message: format!("Order {} submitted for symbol {}", order_id, order.symbol),
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

    // Fetch real market data from OKX data source for order execution
    let market_data = get_market_data_internal(&state, &order.symbol).await
        .unwrap_or_else(|_| {
            // Fallback: build a minimal MarketData from order price so execution can proceed
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
            }).await;
        }
    });

    Ok(order_id)
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

#[tauri::command]
pub async fn run_backtest(
    state: State<'_, AppState>,
    strategy_id: String,
    start_date: String,
    end_date: String,
    initial_capital: f64,
    commission_rate: f64,
    slippage: f64,
    symbols: Vec<String>,
) -> Result<BacktestResult, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    let start = chrono::DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", start_date))
        .map_err(|e| format!("Invalid start date: {}", e))?
        .with_timezone(&chrono::Utc);

    let end = chrono::DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", end_date))
        .map_err(|e| format!("Invalid end date: {}", e))?
        .with_timezone(&chrono::Utc);

    let init_cap = rust_decimal::Decimal::from_f64(initial_capital)
        .ok_or_else(|| format!("Invalid initial capital: {}", initial_capital))?;
    let comm_rate = rust_decimal::Decimal::from_f64(commission_rate)
        .ok_or_else(|| format!("Invalid commission rate: {}", commission_rate))?;
    let slip = rust_decimal::Decimal::from_f64(slippage)
        .ok_or_else(|| format!("Invalid slippage: {}", slippage))?;

    services
        .strategy_service
        .run_backtest(&strategy_id, start, end, init_cap, comm_rate, slip, &symbols)
        .await
        .map_err(|e| e.to_string())
}

/// 查询回测结果列表（分页）
#[tauri::command]
pub async fn get_backtest_results(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<quant_repository::BacktestResultSummaryRow>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    services
        .strategy_service
        .get_backtest_results(limit, offset)
        .await
        .map_err(|e| e.to_string())
}

/// 查询单个回测结果详情（含 equity_curve）
#[tauri::command]
pub async fn get_backtest_result(
    state: State<'_, AppState>,
    id: String,
) -> Result<BacktestResult, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    let id_num: i64 = id.parse().map_err(|e| format!("Invalid ID: {}", e))?;

    services
        .strategy_service
        .get_backtest_result(id_num)
        .await
        .map_err(|e| e.to_string())
}

/// 删除回测结果
#[tauri::command]
pub async fn delete_backtest_result(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    let id_num: i64 = id.parse().map_err(|e| format!("Invalid ID: {}", e))?;

    services
        .strategy_service
        .delete_backtest_result(id_num)
        .await
        .map_err(|e| e.to_string())
}

/// 获取实时指标数据
#[tauri::command]
pub async fn get_metrics() -> Result<HashMap<String, f64>, String> {
    let mut metrics = HashMap::new();

    metrics.insert(
        "orders_total".to_string(),
        monitor_layer::ORDERS_TOTAL.get(),
    );
    metrics.insert(
        "orders_filled".to_string(),
        monitor_layer::ORDERS_FILLED.get(),
    );
    metrics.insert(
        "orders_cancelled".to_string(),
        monitor_layer::ORDERS_CANCELLED.get(),
    );
    metrics.insert(
        "account_balance".to_string(),
        monitor_layer::ACCOUNT_BALANCE.get(),
    );
    metrics.insert(
        "position_value".to_string(),
        monitor_layer::POSITION_VALUE.get(),
    );
    metrics.insert("daily_pnl".to_string(), monitor_layer::DAILY_PNL.get());

    Ok(metrics)
}

/// 获取告警信息
#[tauri::command]
pub async fn get_alerts(state: State<'_, AppState>) -> Result<Vec<Alert>, String> {
    let alerts = state.alert_manager.get_alerts().await;
    Ok(alerts)
}

/// 确认告警
#[tauri::command]
pub async fn acknowledge_alert(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    alert_id: String,
) -> Result<bool, String> {
    let alert_id_num: i64 = alert_id.parse().map_err(|e| format!("Invalid alert ID: {}", e))?;

    let acknowledged = state.alert_manager.acknowledge_alert(alert_id_num).await;
    if acknowledged {
        let _ = app.emit("ws:alerts", serde_json::json!({
            "type": "acknowledged",
            "alert_id": alert_id_num,
        }));
    }
    Ok(acknowledged)
}

/// 获取日志信息
#[tauri::command]
pub async fn get_logs(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    level: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<quant_common::types::LogEntry>, String> {
    let logs = if let Some(level_filter) = level {
        state.log_buffer.get_entries_by_level(&level_filter).await
    } else {
        state.log_buffer.get_entries().await
    };

    let logs = if let Some(n) = limit {
        let n = n as usize;
        if logs.len() > n {
            logs[logs.len() - n..].to_vec()
        } else {
            logs
        }
    } else {
        logs
    };

    let _ = app.emit("ws:logs", &logs);
    Ok(logs)
}

/// 获取所有策略
#[tauri::command]
pub async fn get_strategies(state: State<'_, AppState>) -> Result<Vec<StrategyParams>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .strategy_service
        .get_strategies()
        .await
        .map_err(|e| e.to_string())
}

/// 创建或更新策略
#[tauri::command]
pub async fn save_strategy(
    state: State<'_, AppState>,
    strategy: StrategyParams,
) -> Result<String, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .strategy_service
        .save_strategy(&strategy)
        .await
        .map_err(|e| e.to_string())
}

/// 删除策略
#[tauri::command]
pub async fn delete_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<bool, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .strategy_service
        .delete_strategy(&strategy_id)
        .await
        .map_err(|e| e.to_string())
}

/// 启用/禁用策略
#[tauri::command]
pub async fn toggle_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
    enabled: bool,
) -> Result<bool, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .strategy_service
        .toggle_strategy(&strategy_id, enabled)
        .await
        .map_err(|e| e.to_string())
}

/// 部署策略
#[tauri::command]
pub async fn deploy_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<String, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let status = services
        .strategy_service
        .deploy_strategy(&strategy_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("{:?}", status))
}

/// 启动策略
#[tauri::command]
pub async fn start_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<String, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let status = services
        .strategy_service
        .start_strategy(&strategy_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("{:?}", status))
}

/// 停止策略
#[tauri::command]
pub async fn stop_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<String, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let status = services
        .strategy_service
        .stop_strategy(&strategy_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("{:?}", status))
}

/// 暂停策略
#[tauri::command]
pub async fn pause_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<String, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let status = services
        .strategy_service
        .pause_strategy(&strategy_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("{:?}", status))
}

/// 恢复策略
#[tauri::command]
pub async fn resume_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<String, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let status = services
        .strategy_service
        .resume_strategy(&strategy_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("{:?}", status))
}

/// 归档策略
#[tauri::command]
pub async fn archive_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<String, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let status = services
        .strategy_service
        .archive_strategy(&strategy_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("{:?}", status))
}

/// 获取风险指标
#[tauri::command]
pub async fn get_risk_metrics(state: State<'_, AppState>) -> Result<HashMap<String, f64>, String> {
    // Prefer RiskService (DB-backed) for real VaR computation with historical returns
    if let Some(ref services) = state.app_services {
        match services.risk_service.get_risk_metrics().await {
            Ok(metrics) => return Ok(metrics),
            Err(e) => {
                state
                    .log_buffer
                    .add_entry(quant_common::types::LogEntry {
                        timestamp: Utc::now(),
                        level: "warn".to_string(),
                        message: format!("RiskService unavailable, using config-only metrics: {}", e),
                        module: Some("risk".to_string()),
                    })
                    .await;
            }
        }
    }

    // Fallback: compute VaR with empty returns (returns 0.0 with internal warning)
    // and return config-based metrics
    use risk_layer::VaRCalculator;
    use rust_decimal::prelude::ToPrimitive;

    let mut metrics = HashMap::new();
    let config = state.config.read().await;
    let risk_config = &config.risk;

    let var_95 = VaRCalculator::historical_simulation(&[dec!(0.0)], 0.95);
    let var_99 = VaRCalculator::historical_simulation(&[dec!(0.0)], 0.99);

    metrics.insert("var_95".to_string(), var_95.to_f64().unwrap_or(0.0));
    metrics.insert("var_99".to_string(), var_99.to_f64().unwrap_or(0.0));
    metrics.insert("max_position_size".to_string(), risk_config.max_position_size);
    metrics.insert("max_daily_loss".to_string(), risk_config.max_daily_loss);
    metrics.insert("max_drawdown".to_string(), risk_config.max_drawdown);

    Ok(metrics)
}

/// 获取风险配置
#[tauri::command]
pub async fn get_risk_config(state: State<'_, AppState>) -> Result<quant_common::config::RiskConfig, String> {
    let config = state.config.read().await;
    Ok(config.risk.clone())
}

/// 更新风险配置
#[tauri::command]
pub async fn update_risk_config(
    state: State<'_, AppState>,
    config: quant_common::config::RiskConfig,
) -> Result<bool, String> {
    // Delegate to ConfigService for persistence
    match state.app_services.as_ref() {
        Some(services) => {
            let mut new_config = services.config_service.get_config().await;
            new_config.risk = config;
            let status = services.config_service.update_config(new_config).await;
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: if status.contains("failed") { "warn".to_string() } else { "info".to_string() },
                    message: status,
                    module: Some("config".to_string()),
                })
                .await;
            Ok(true)
        }
        None => {
            let mut app_config = state.config.write().await;
            app_config.risk = config;
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "warn".to_string(),
                    message: "Risk config updated in memory only (no persistence)".to_string(),
                    module: Some("config".to_string()),
                })
                .await;
            Ok(true)
        }
    }
}

/// 执行事前风控检查
#[tauri::command]
pub async fn pre_trade_check(
    state: State<'_, AppState>,
    order: Order,
    account: Account,
    positions: Vec<Position>,
) -> Result<bool, String> {
    use risk_layer::PreTradeRiskChecker;

    // Use the application's live risk configuration instead of hardcoded defaults
    let app_config = state.config.read().await;
    let risk_config = app_config.risk.clone();
    drop(app_config);

    let checker = PreTradeRiskChecker::new(risk_config);

    match checker.check_order(&order, &account, &positions) {
        Ok(_) => {
            // Log successful check
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "info".to_string(),
                    message: format!("Pre-trade check passed for order {}", order.order_id),
                    module: Some("risk".to_string()),
                })
                .await;
            Ok(true)
        }
        Err(e) => {
            let error_msg = format!("Pre-trade check failed: {}", e);

            // Create alert for failed check
            let alert = Alert {
                alert_id: 0,
                level: quant_common::types::AlertLevel::Warning,
                source: "Risk Management".to_string(),
                message: error_msg.clone(),
                timestamp: Utc::now(),
                acknowledged: false,
            };
            state.alert_manager.send_alert(alert).await;

            // Log the failure
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "warning".to_string(),
                    message: error_msg,
                    module: Some("risk".to_string()),
                })
                .await;

            Ok(false)
        }
    }
}

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
pub async fn verify_token(
    state: State<'_, AppState>,
    token: String,
) -> Result<bool, String> {
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
            let views: Vec<InstrumentView> = instruments.into_iter().map(InstrumentView::from).collect();
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
    let data_source = state.okx_data_source.read().await;

    match data_source.as_ref() {
        Some(source) => {
            use data_layer::market_data::DataSource;

            let start = std::time::Instant::now();
            monitor_layer::OKX_API_CALLS.inc();

            let result = source.get_realtime_data(&symbol).await;

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
        None => Err("OKX data source not initialized".to_string()),
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
    let data_source = state.okx_data_source.read().await;

    match data_source.as_ref() {
        Some(source) => {
            use chrono::DateTime;
            use data_layer::market_data::DataSource;

            let start_dt = DateTime::parse_from_rfc3339(&start)
                .map_err(|e| format!("Invalid start date: {}", e))?
                .with_timezone(&Utc);

            let end_dt = DateTime::parse_from_rfc3339(&end)
                .map_err(|e| format!("Invalid end date: {}", e))?
                .with_timezone(&Utc);

            monitor_layer::OKX_API_CALLS.inc();
            let start_time = std::time::Instant::now();

            let result = source.get_historical_data(&symbol, start_dt, end_dt).await;

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
        None => Err("OKX data source not initialized".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exchange_okx::ClientInterface;
    use exchange_okx::MockOkxClient;
    use monitor_layer::{AlertManager, LogBuffer};
    use quant_common::types::StrategyType;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn make_test_state() -> AppState {
        use crate::state::WsState;
        use trading_layer::OrderManager;

        let alert_manager = Arc::new(AlertManager::new(false, vec![]));
        let log_buffer = Arc::new(LogBuffer::new(1000));
        AppState {
            config: Arc::new(RwLock::new(AppConfig::default())),
            alert_manager,
            log_buffer,
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

    /// Create an AppState with an optional mock OKX client for testing.
    ///
    /// Pass `Some(mock)` to inject a mock client with pre-configured expectations,
    /// or `None` to simulate the "not initialized" state.
    fn create_mock_okx_state(mock_client: Option<MockOkxClient>) -> AppState {
        use crate::state::WsState;
        use trading_layer::OrderManager;

        let okx_client: Arc<RwLock<Option<Arc<RwLock<dyn ClientInterface + Send + Sync>>>>> =
            Arc::new(RwLock::new(mock_client.map(|mc| {
                let inner: Arc<RwLock<dyn ClientInterface + Send + Sync>> =
                    Arc::new(RwLock::new(mc));
                inner
            })));

        let alert_manager = Arc::new(AlertManager::new(false, vec![]));
        let log_buffer = Arc::new(LogBuffer::new(1000));
        AppState {
            config: Arc::new(RwLock::new(AppConfig::default())),
            alert_manager,
            log_buffer,
            pg_client: None,
            redis_cache: None,
            okx_client,
            okx_executor: Arc::new(RwLock::new(None)),
            okx_data_source: Arc::new(RwLock::new(None)),
            order_manager: OrderManager::new(),
            app_services: None,
            ws_state: WsState::new(),
        }
    }

    #[tokio::test]
    async fn test_get_market_data_without_okx_returns_error() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_market_data(state_guard, "BTC-USDT".to_string()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Market data unavailable for BTC-USDT"));
    }

    #[tokio::test]
    async fn test_get_account_info_without_db_returns_error() {
        let state = make_test_state();
        // SAFETY: State is a transparent wrapper around &T
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_account_info(state_guard).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Account service not initialized (no database connection)"
        );
    }

    #[tokio::test]
    async fn test_get_positions_without_db_returns_error() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_positions(state_guard).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Account service not initialized (no database connection)"
        );
    }

    #[tokio::test]
    async fn test_get_active_orders_returns_submitted() {
        let state = make_test_state();
        // Submit an order first so OrderManager has a submitted order
        let order = Order {
            order_id: 0,
            strategy_id: "test_strategy".to_string(),
            symbol: "600519.SH".to_string(),
            order_type: quant_common::types::OrderType::Limit,
            side: quant_common::types::OrderSide::Buy,
            price: Some(dec!(1685.00)),
            quantity: dec!(100),
            filled_quantity: dec!(0),
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: dec!(0),
            slippage: dec!(0),
        };
        state.order_manager.submit_order(order).await.unwrap();

        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_active_orders(state_guard).await;
        assert!(result.is_ok());
        let orders = result.unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(
            orders[0].status,
            quant_common::types::OrderStatus::Submitted
        );
    }

    #[tokio::test]
    async fn test_get_strategies_requires_services() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_strategies(state_guard).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Application services not initialized");
    }

    #[tokio::test]
    async fn test_save_strategy_requires_services() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let strategy = StrategyParams {
            strategy_id: "test_001".to_string(),
            strategy_name: "Test Strategy".to_string(),
            strategy_type: StrategyType::MeanReversion,
            params: serde_json::json!({}),
            enabled: true,
            max_position: dec!(100000),
            max_daily_loss: dec!(5000),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let result = save_strategy(state_guard, strategy).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Application services not initialized");
    }

    #[tokio::test]
    async fn test_delete_strategy_requires_services() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = delete_strategy(state_guard, "test_001".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Application services not initialized");
    }

    #[tokio::test]
    async fn test_toggle_strategy_requires_services() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = toggle_strategy(state_guard, "test_001".to_string(), false).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Application services not initialized");
    }

    #[tokio::test]
    async fn test_get_risk_metrics_contains_var() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_risk_metrics(state_guard).await;
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(metrics.contains_key("var_95"));
        assert!(metrics.contains_key("var_99"));
        assert!(metrics.contains_key("max_position_size"));
    }

    #[tokio::test]
    async fn test_get_risk_config_returns_defaults() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_risk_config(state_guard).await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.max_position_size, 0.2);
        assert_eq!(config.max_daily_loss, 0.05);
        assert!(config.enable_pre_trade_check);
    }

    #[tokio::test]
    async fn test_update_risk_config_returns_true() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let new_config = quant_common::config::RiskConfig {
            max_position_size: 0.3,
            max_daily_loss: 0.1,
            max_drawdown: 0.2,
            max_concentration: 0.2,
            enable_pre_trade_check: true,
            enable_real_time_monitor: true,
            var_confidence_level: 0.99,
        };
        let result = update_risk_config(state_guard, new_config).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_login_without_db_returns_error() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = login(state_guard, "admin".to_string(), "admin123".to_string()).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Authentication unavailable: no database connection"
        );
    }

    #[tokio::test]
    async fn test_verify_invalid_token_without_db_returns_false() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = verify_token(state_guard, "invalid.token.here".to_string()).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_verify_empty_token_without_db_returns_false() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = verify_token(state_guard, String::new()).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_get_user_profile_without_db_returns_error() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_user_profile(state_guard, None).await;
        assert!(result.is_err());
    }

    // ── OKX Commands ──

    #[tokio::test]
    async fn test_get_okx_balance_success() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_account_balance()
            .returning(|_| {
                Box::pin(async {
                    Ok(vec![OkxBalance {
                        ccy: "BTC".to_string(),
                        eq: "1.5".to_string(),
                        cash_bal: "1.0".to_string(),
                        avail_eq: "1.5".to_string(),
                        frozen_bal: "0".to_string(),
                    }])
                })
            });

        let state = create_mock_okx_state(Some(mock));
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_balance(state_guard, Some("BTC".to_string())).await;
        assert!(result.is_ok());
        let balances = result.unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].ccy, "BTC");
        assert!((balances[0].eq - 1.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_get_okx_balance_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_balance(state_guard, Some("BTC".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX client not initialized");
    }

    #[tokio::test]
    async fn test_get_okx_positions_success() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_positions()
            .returning(|_| {
                Box::pin(async {
                    Ok(vec![OkxPosition {
                        inst_id: "BTC-USDT".to_string(),
                        pos: "1".to_string(),
                        avail_pos: "1".to_string(),
                        avg_px: "45000.0".to_string(),
                        upl: "100.0".to_string(),
                        upl_ratio: "0.02".to_string(),
                    }])
                })
            });

        let state = create_mock_okx_state(Some(mock));
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_positions(state_guard, Some("BTC-USDT".to_string())).await;
        assert!(result.is_ok());
        let positions = result.unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].inst_id, "BTC-USDT");
        assert!((positions[0].pos - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_get_okx_positions_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_positions(state_guard, None).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX client not initialized");
    }

    #[tokio::test]
    async fn test_place_okx_order_success() {
        let mut mock = MockOkxClient::new();
        mock.expect_place_order()
            .returning(|_| {
                Box::pin(async {
                    Ok(OkxOrder {
                        ord_id: "123456789".to_string(),
                        cl_ord_id: "cl-123".to_string(),
                        inst_id: "BTC-USDT".to_string(),
                        side: "buy".to_string(),
                        ord_type: "market".to_string(),
                        px: "0".to_string(),
                        sz: "1".to_string(),
                        state: "live".to_string(),
                        avg_px: "0".to_string(),
                        acc_fill_sz: "0".to_string(),
                        u_time: "1597026383000".to_string(),
                    })
                })
            });

        let state = create_mock_okx_state(Some(mock));
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let request = OkxPlaceOrderRequest {
            inst_id: "BTC-USDT".to_string(),
            td_mode: "cash".to_string(),
            side: "buy".to_string(),
            ord_type: "market".to_string(),
            sz: "1".to_string(),
            px: None,
            cl_ord_id: None,
            tag: None,
            pos_side: None,
            ccy: None,
            px_usd: None,
            px_vol: None,
            reduce_only: None,
            tgt_ccy: None,
        };

        let result = place_okx_order(state_guard, request).await;
        assert!(result.is_ok());
        let order = result.unwrap();
        assert_eq!(order.ord_id, "123456789");
        assert_eq!(order.inst_id, "BTC-USDT");
        assert_eq!(order.state, "live");
    }

    #[tokio::test]
    async fn test_place_okx_order_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let request = OkxPlaceOrderRequest {
            inst_id: "BTC-USDT".to_string(),
            td_mode: "cash".to_string(),
            side: "buy".to_string(),
            ord_type: "market".to_string(),
            sz: "1".to_string(),
            px: None,
            cl_ord_id: None,
            tag: None,
            pos_side: None,
            ccy: None,
            px_usd: None,
            px_vol: None,
            reduce_only: None,
            tgt_ccy: None,
        };

        let result = place_okx_order(state_guard, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX client not initialized");
    }

    #[tokio::test]
    async fn test_cancel_okx_order_success() {
        let mut mock = MockOkxClient::new();
        mock.expect_cancel_order()
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let state = create_mock_okx_state(Some(mock));
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result =
            cancel_okx_order(state_guard, "BTC-USDT".to_string(), "123".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_cancel_okx_order_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result =
            cancel_okx_order(state_guard, "BTC-USDT".to_string(), "123".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX client not initialized");
    }

    #[tokio::test]
    async fn test_get_okx_candles_success() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_candles()
            .returning(|_, _, _| {
                Box::pin(async {
                    Ok(vec![OkxCandle {
                        ts: "1597026383000".to_string(),
                        open: "45000".to_string(),
                        high: "45500".to_string(),
                        low: "44900".to_string(),
                        close: "45200".to_string(),
                        vol: "100.0".to_string(),
                        vol_ccy: "4500000".to_string(),
                    }])
                })
            });

        let state = create_mock_okx_state(Some(mock));
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_candles(
            state_guard,
            "BTC-USDT".to_string(),
            Some("1H".to_string()),
            Some(10),
        )
        .await;
        assert!(result.is_ok());
        let candles = result.unwrap();
        assert_eq!(candles.len(), 1);
        assert!((candles[0].o - 45000.0).abs() < f64::EPSILON);
        assert!((candles[0].c - 45200.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_get_okx_candles_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result =
            get_okx_candles(state_guard, "BTC-USDT".to_string(), None, None).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX client not initialized");
    }

    #[tokio::test]
    async fn test_get_okx_candles_invalid_params() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_candles()
            .returning(|_, _, _| {
                Box::pin(async {
                    Err(quant_common::Error::Internal("Invalid instrument ID".to_string()))
                })
            });

        let state = create_mock_okx_state(Some(mock));
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_candles(
            state_guard,
            "INVALID".to_string(),
            Some("1H".to_string()),
            Some(5),
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to get OKX candles"));
    }

    #[tokio::test]
    async fn test_get_okx_instruments_success() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_instruments()
            .returning(|_| {
                Box::pin(async {
                    Ok(serde_json::json!([{
                        "instId": "BTC-USDT",
                        "instType": "SPOT",
                        "uly": "",
                        "baseCcy": "BTC",
                        "quoteCcy": "USDT",
                        "ctVal": "1",
                        "tickSz": "0.1",
                        "lotSz": "0.0001",
                        "minSz": "0.0001"
                    }]))
                })
            });

        let state = create_mock_okx_state(Some(mock));
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_instruments(state_guard, Some("SPOT".to_string())).await;
        assert!(result.is_ok());
        let instruments = result.unwrap();
        assert_eq!(instruments.len(), 1);
        assert_eq!(instruments[0].inst_id, "BTC-USDT");
        assert_eq!(instruments[0].inst_type, "SPOT");
    }

    #[tokio::test]
    async fn test_get_okx_instruments_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_instruments(state_guard, None).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX client not initialized");
    }

    #[tokio::test]
    async fn test_check_okx_status_connected() {
        let mock = MockOkxClient::new();
        let state = create_mock_okx_state(Some(mock));
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = check_okx_status(state_guard).await;
        assert!(result.is_ok());
        let status = result.unwrap();

        // Verify all 4 fields
        assert_eq!(status["connected"].as_bool(), Some(true));
        assert_eq!(status["enabled"].as_bool(), Some(false));
        assert_eq!(status["environment"].as_str(), Some("demo"));
        assert_eq!(status["has_credentials"].as_bool(), Some(false));
    }

    #[tokio::test]
    async fn test_check_okx_status_disconnected() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = check_okx_status(state_guard).await;
        assert!(result.is_ok());
        let status = result.unwrap();

        assert_eq!(status["connected"].as_bool(), Some(false));
        assert_eq!(status["enabled"].as_bool(), Some(false));
        assert_eq!(status["environment"].as_str(), Some("demo"));
        assert_eq!(status["has_credentials"].as_bool(), Some(false));
    }

    #[tokio::test]
    async fn test_get_okx_announcements_success() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_announcements()
            .returning(|| {
                Box::pin(async {
                    Ok(vec![exchange_okx::AnnouncementPage {
                        details: vec![exchange_okx::AnnouncementDetail {
                            ann_type: "listing".to_string(),
                            p_time: "1597026383086".to_string(),
                            title: "Test Announcement".to_string(),
                            url: "https://www.okx.com/support/test".to_string(),
                        }],
                        total_page: "1".to_string(),
                    }])
                })
            });

        let state = create_mock_okx_state(Some(mock));
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_announcements(state_guard).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value.is_array());
        assert_eq!(value[0]["details"][0]["title"], "Test Announcement");
    }

    #[tokio::test]
    async fn test_get_okx_announcements_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_announcements(state_guard).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX client not initialized");
    }

    #[tokio::test]
    async fn test_execute_okx_order_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let order = Order {
            order_id: 1,
            strategy_id: "test".to_string(),
            symbol: "BTC-USDT".to_string(),
            order_type: quant_common::types::OrderType::Market,
            side: quant_common::types::OrderSide::Buy,
            price: None,
            quantity: dec!(1),
            filled_quantity: dec!(0),
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: dec!(0),
            slippage: dec!(0),
        };

        let result = execute_okx_order(state_guard, order).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX executor not initialized");
    }

    #[tokio::test]
    async fn test_get_okx_realtime_data_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_realtime_data(state_guard, "BTC-USDT".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX data source not initialized");
    }

    #[tokio::test]
    async fn test_get_okx_historical_data_not_initialized() {
        let state = create_mock_okx_state(None);
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

        let result = get_okx_historical_data(
            state_guard,
            "BTC-USDT".to_string(),
            "2024-01-01T00:00:00Z".to_string(),
            "2024-01-02T00:00:00Z".to_string(),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "OKX data source not initialized");
    }
}
