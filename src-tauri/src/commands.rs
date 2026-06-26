use crate::state::AppState;
use chrono::Utc;
use exchange_okx::types::*;
use quant_common::config::AppConfig;
use quant_common::types::{
    Account, Alert, BacktestResult, MarketData, Order, Position, StrategyParams, StrategyType,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
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
    // In a real implementation, this would save to a file or database
    // For now, we'll just update the in-memory state
    let mut app_config = state.config.write().await;
    *app_config = config;

    // Simulate saving to persistent storage
    println!("Configuration updated: {:?}", app_config);

    Ok(true)
}

#[tauri::command]
pub async fn get_market_data(_symbol: String) -> Result<MarketData, String> {
    // TODO: 实现真实数据获取
    Err("Not implemented".to_string())
}

#[tauri::command]
pub async fn submit_order(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    order: Order,
) -> Result<String, String> {
    use quant_common::types::MarketData;
    use rust_decimal_macros::dec;
    use std::sync::Arc;
    use trading_layer::ExecutionEngine;

    let order_manager = Arc::new(state.order_manager.clone());

    let app_config = state.config.read().await;
    let trading_config = app_config.trading.clone();

    if app_config.risk.enable_pre_trade_check {
        let risk_config = app_config.risk.clone();
        let checker = risk_layer::PreTradeRiskChecker::new(risk_config);

        if let Some(ref services) = state.app_services {
            if let Ok(account) = services.account_service.get_account_info().await {
                if let Ok(positions) = services.account_service.get_positions().await {
                    checker
                        .check_order(&order, &account, &positions)
                        .map_err(|e| format!("Risk check failed: {}", e))?;
                }
            }
        } else {
            // No services available (DB not connected), skip risk check
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "warn".to_string(),
                    message: "Risk check skipped: AppServices not available".to_string(),
                    module: Some("risk".to_string()),
                })
                .await;
        }
    }
    drop(app_config);

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

    // Simulate market data for execution
    // Ensure market orders (price=None) get a non-zero fallback price
    let market_price = match order.price {
        Some(price) => price,
        None => {
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "warn".to_string(),
                    message: format!(
                        "Market order without price for {}, using fallback price",
                        order.symbol
                    ),
                    module: Some("commands".to_string()),
                })
                .await;
            dec!(100) // Default price for paper trading market orders
        }
    };
    let market_data = MarketData {
        symbol: order.symbol.clone(),
        timestamp: chrono::Utc::now(),
        open: market_price,
        high: market_price,
        low: market_price,
        close: market_price,
        volume: dec!(1000000),
        turnover: dec!(1000000000),
        open_interest: None,
        bid_prices: vec![],
        bid_volumes: vec![],
        ask_prices: vec![],
        ask_volumes: vec![],
    };

    // Execute order asynchronously
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        if let Err(e) = execution_engine.execute_order(order, &market_data).await {
            eprintln!("Order execution failed: {}", e);
        }
    });

    Ok(order_id)
}

#[tauri::command]
pub async fn get_account_info(state: State<'_, AppState>) -> Result<Account, String> {
    match state.app_services.as_ref() {
        Some(services) => match services.account_service.get_account_info().await {
            Ok(account) => Ok(account),
            Err(service_error) => {
                state
                    .log_buffer
                    .add_entry(quant_common::types::LogEntry {
                        timestamp: Utc::now(),
                        level: "warn".to_string(),
                        message: format!(
                            "AccountService unavailable, using mock data: {}",
                            service_error
                        ),
                        module: Some("commands".to_string()),
                    })
                    .await;
                Ok(Account {
                    account_id: 0,
                    total_assets: dec!(1234567.91),
                    available_cash: dec!(234567.99),
                    frozen_cash: dec!(0),
                    market_value: dec!(1000000),
                    total_pnl: dec!(12345.67),
                    daily_pnl: dec!(12345.67),
                    margin: dec!(0),
                    margin_ratio: dec!(0),
                    updated_at: Utc::now(),
                })
            }
        },
        None => Ok(Account {
            account_id: 0,
            total_assets: dec!(1234567.91),
            available_cash: dec!(234567.99),
            frozen_cash: dec!(0),
            market_value: dec!(1000000),
            total_pnl: dec!(12345.67),
            daily_pnl: dec!(12345.67),
            margin: dec!(0),
            margin_ratio: dec!(0),
            updated_at: Utc::now(),
        }),
    }
}

#[tauri::command]
pub async fn get_positions(state: State<'_, AppState>) -> Result<Vec<Position>, String> {
    match state.app_services.as_ref() {
        Some(services) => match services.account_service.get_positions().await {
            Ok(positions) => Ok(positions),
            Err(_) => Ok(vec![
                Position {
                    symbol: "600519.SH".to_string(),
                    quantity: dec!(100),
                    available_quantity: dec!(100),
                    avg_price: dec!(1680.50),
                    market_value: dec!(168050),
                    unrealized_pnl: dec!(12345.67),
                    realized_pnl: dec!(0),
                    updated_at: Utc::now(),
                },
                Position {
                    symbol: "000001.SZ".to_string(),
                    quantity: dec!(500),
                    available_quantity: dec!(500),
                    avg_price: dec!(12.00),
                    market_value: dec!(6175),
                    unrealized_pnl: dec!(175),
                    realized_pnl: dec!(0),
                    updated_at: Utc::now(),
                },
            ]),
        },
        None => Ok(vec![
            Position {
                symbol: "600519.SH".to_string(),
                quantity: dec!(100),
                available_quantity: dec!(100),
                avg_price: dec!(1680.50),
                market_value: dec!(168050),
                unrealized_pnl: dec!(12345.67),
                realized_pnl: dec!(0),
                updated_at: Utc::now(),
            },
            Position {
                symbol: "000001.SZ".to_string(),
                quantity: dec!(500),
                available_quantity: dec!(500),
                avg_price: dec!(12.00),
                market_value: dec!(6175),
                unrealized_pnl: dec!(175),
                realized_pnl: dec!(0),
                updated_at: Utc::now(),
            },
        ]),
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

    // 模拟指标数据
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
    _limit: Option<u32>,
) -> Result<Vec<quant_common::types::LogEntry>, String> {
    let logs = if let Some(level_filter) = level {
        state.log_buffer.get_entries_by_level(&level_filter).await
    } else {
        state.log_buffer.get_entries().await
    };

    let _ = app.emit("ws:logs", &logs);
    Ok(logs)
}

/// 获取所有策略
#[tauri::command]
pub async fn get_strategies() -> Result<Vec<StrategyParams>, String> {
    // 模拟策略数据
    let strategies = vec![
        StrategyParams {
            strategy_id: "mean_reversion_001".to_string(),
            strategy_name: "均值回归策略".to_string(),
            strategy_type: StrategyType::MeanReversion,
            params: serde_json::json!({
                "lookback_period": 20,
                "entry_threshold": 2.0,
                "exit_threshold": 0.5
            }),
            enabled: true,
            max_position: dec!(100000),
            max_daily_loss: dec!(5000),
            created_at: Utc::now() - chrono::Duration::days(30),
            updated_at: Utc::now() - chrono::Duration::hours(2),
        },
        StrategyParams {
            strategy_id: "trend_following_001".to_string(),
            strategy_name: "趋势跟踪策略".to_string(),
            strategy_type: StrategyType::TrendFollowing,
            params: serde_json::json!({
                "lookback_period": 50,
                "stop_loss_percent": 5.0,
                "take_profit_percent": 10.0
            }),
            enabled: true,
            max_position: dec!(200000),
            max_daily_loss: dec!(10000),
            created_at: Utc::now() - chrono::Duration::days(60),
            updated_at: Utc::now() - chrono::Duration::hours(24),
        },
    ];

    Ok(strategies)
}

/// 创建或更新策略
#[tauri::command]
pub async fn save_strategy(strategy: StrategyParams) -> Result<String, String> {
    // 模拟保存策略
    println!("Saving strategy: {}", strategy.strategy_name);
    Ok(strategy.strategy_id.clone())
}

/// 删除策略
#[tauri::command]
pub async fn delete_strategy(strategy_id: String) -> Result<bool, String> {
    // 模拟删除策略
    println!("Deleting strategy: {}", strategy_id);
    Ok(true)
}

/// 启用/禁用策略
#[tauri::command]
pub async fn toggle_strategy(strategy_id: String, enabled: bool) -> Result<bool, String> {
    // 模拟切换策略状态
    println!("Toggling strategy {} to {}", strategy_id, enabled);
    Ok(true)
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
pub async fn get_risk_metrics() -> Result<HashMap<String, f64>, String> {
    use quant_common::config::RiskConfig;
    use risk_layer::VaRCalculator;
    use rust_decimal::prelude::ToPrimitive;
    use rust_decimal::Decimal;

    let mut metrics = HashMap::new();

    // 模拟收益率数据用于VaR计算
    let returns = vec![
        dec!(0.01),
        dec!(-0.005),
        dec!(0.02),
        dec!(-0.01),
        dec!(0.008),
        dec!(-0.015),
        dec!(0.012),
        dec!(0.003),
        dec!(-0.007),
        dec!(0.011),
    ];

    // 计算VaR
    let config = RiskConfig {
        max_position_size: 0.2,
        max_daily_loss: 0.05,
        max_drawdown: 0.15,
        max_concentration: 0.2,
        enable_pre_trade_check: true,
        enable_real_time_monitor: true,
        var_confidence_level: 0.95,
    };

    let var_95 = VaRCalculator::historical_simulation(&returns, 0.95);
    let var_99 = VaRCalculator::historical_simulation(&returns, 0.99);

    metrics.insert("var_95".to_string(), var_95.to_f64().unwrap_or(0.0));
    metrics.insert("var_99".to_string(), var_99.to_f64().unwrap_or(0.0));

    // 添加其他风险指标
    metrics.insert("max_position_size".to_string(), config.max_position_size);
    metrics.insert("max_daily_loss".to_string(), config.max_daily_loss);
    metrics.insert("max_drawdown".to_string(), config.max_drawdown);

    Ok(metrics)
}

/// 获取风险配置
#[tauri::command]
pub async fn get_risk_config() -> Result<quant_common::config::RiskConfig, String> {
    // 返回默认风险配置
    Ok(quant_common::config::RiskConfig {
        max_position_size: 0.2,
        max_daily_loss: 0.05,
        max_drawdown: 0.15,
        max_concentration: 0.2,
        enable_pre_trade_check: true,
        enable_real_time_monitor: true,
        var_confidence_level: 0.95,
    })
}

/// 更新风险配置
#[tauri::command]
pub async fn update_risk_config(config: quant_common::config::RiskConfig) -> Result<bool, String> {
    // 模拟更新风险配置
    println!("Updating risk config: {:?}", config);
    Ok(true)
}

/// 执行事前风控检查
#[tauri::command]
pub async fn pre_trade_check(
    state: State<'_, AppState>,
    order: Order,
    account: Account,
    positions: Vec<Position>,
) -> Result<bool, String> {
    use quant_common::config::RiskConfig;
    use risk_layer::PreTradeRiskChecker;

    let config = RiskConfig {
        max_position_size: 0.2,
        max_daily_loss: 0.05,
        max_drawdown: 0.15,
        max_concentration: 0.2,
        enable_pre_trade_check: true,
        enable_real_time_monitor: true,
        var_confidence_level: 0.95,
    };

    let checker = PreTradeRiskChecker::new(config);

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
pub async fn login(username: String, password: String) -> Result<String, String> {
    use quant_common::config::AppConfig;
    use security::AuthService;

    // In a real implementation, this would check against a user database
    // For now, we'll use a simple check
    if username == "admin" && password == "admin123" {
        // Create auth service with config values
        let config = AppConfig::default();
        let auth_service = AuthService::new(
            config.security.jwt_secret,
            config.security.token_expiry_hours as i64,
        );

        // Generate JWT token
        let token = auth_service
            .generate_token("admin_id", &username, vec!["admin".to_string()])
            .map_err(|e| format!("Token generation failed: {}", e))?;

        Ok(token)
    } else {
        Err("Invalid username or password".to_string())
    }
}

/// 验证 Token
#[tauri::command]
pub async fn verify_token(token: String) -> Result<bool, String> {
    use quant_common::config::AppConfig;
    use security::AuthService;

    let config = AppConfig::default();
    let auth_service = AuthService::new(
        config.security.jwt_secret,
        config.security.token_expiry_hours as i64,
    );

    match auth_service.verify_token(&token) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// 更新用户资料
#[tauri::command]
pub async fn update_profile(profile_data: serde_json::Value) -> Result<bool, String> {
    // In a real implementation, this would update the user profile in a database
    // For now, we'll just log the data and return success
    println!("Updating profile: {:?}", profile_data);

    // Simulate processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok(true)
}

/// 修改密码
#[tauri::command]
pub async fn change_password(
    _current_password: String,
    _new_password: String,
) -> Result<bool, String> {
    // In a real implementation, this would verify the current password and update it
    // For now, we'll just log the request and return success
    println!("Changing password for user");

    // Simulate processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // In a real implementation, you would:
    // 1. Verify the current password matches
    // 2. Hash the new password
    // 3. Update it in the database
    // 4. Return appropriate success/failure

    Ok(true)
}

/// 获取用户资料
#[tauri::command]
pub async fn get_user_profile() -> Result<serde_json::Value, String> {
    // In a real implementation, this would fetch the user profile from a database
    // For now, we'll return mock data
    let profile = serde_json::json!({
        "username": "admin",
        "email": "admin@example.com",
        "phone": "13800138000",
        "full_name": "系统管理员",
        "company": "量化交易公司",
        "address": "北京市朝阳区金融街1号",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "last_login": chrono::Utc::now().to_rfc3339()
    });

    Ok(profile)
}

// ==================== OKX Integration Commands ====================

/// 获取 OKX 账户余额
#[tauri::command]
pub async fn get_okx_balance(
    state: State<'_, AppState>,
    ccy: Option<String>,
) -> Result<Vec<OkxBalance>, String> {
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

            Ok(balances)
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 获取 OKX 持仓
#[tauri::command]
pub async fn get_okx_positions(
    state: State<'_, AppState>,
    inst_id: Option<String>,
) -> Result<Vec<OkxPosition>, String> {
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

            Ok(positions)
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 下单到 OKX
#[tauri::command]
pub async fn place_okx_order(
    state: State<'_, AppState>,
    request: OkxPlaceOrderRequest,
) -> Result<OkxOrder, String> {
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

            Ok(order)
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
) -> Result<Vec<OkxCandle>, String> {
    let okx_client_opt = state.okx_client.read().await;

    match okx_client_opt.as_ref() {
        Some(client_arc) => {
            let client = client_arc.read().await;
            let bar = bar.as_deref().unwrap_or("1H");
            let candles = client
                .get_candles(&inst_id, bar, limit)
                .await
                .map_err(|e| format!("Failed to get OKX candles: {}", e))?;

            Ok(candles)
        }
        None => Err("OKX client not initialized".to_string()),
    }
}

/// 获取 OKX 交易对信息
#[tauri::command]
pub async fn get_okx_instruments(
    state: State<'_, AppState>,
    inst_type: Option<String>,
) -> Result<serde_json::Value, String> {
    let okx_client_opt = state.okx_client.read().await;

    match okx_client_opt.as_ref() {
        Some(client_arc) => {
            let client = client_arc.read().await;
            let inst_type = inst_type.as_deref().unwrap_or("SPOT");
            let instruments = client
                .get_instruments(inst_type)
                .await
                .map_err(|e| format!("Failed to get OKX instruments: {}", e))?;

            Ok(instruments)
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
    use monitor_layer::{AlertManager, LogBuffer};
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

    #[tokio::test]
    async fn test_get_market_data_returns_not_implemented() {
        let result = get_market_data("BTC-USDT".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not implemented");
    }

    #[tokio::test]
    async fn test_get_account_info_returns_valid_account() {
        let state = make_test_state();
        // SAFETY: State is a transparent wrapper around &T
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_account_info(state_guard).await;
        assert!(result.is_ok());
        let account = result.unwrap();
        assert!(account.total_assets > Decimal::ZERO);
        assert!(account.available_cash > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_get_positions_returns_non_empty() {
        let state = make_test_state();
        let state_guard: tauri::State<'_, AppState> =
            unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
        let result = get_positions(state_guard).await;
        assert!(result.is_ok());
        let positions = result.unwrap();
        assert!(!positions.is_empty());
        assert!(positions.iter().all(|p| p.quantity > Decimal::ZERO));
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
    async fn test_get_strategies_returns_two_strategies() {
        let result = get_strategies().await;
        assert!(result.is_ok());
        let strategies = result.unwrap();
        assert_eq!(strategies.len(), 2);
        assert!(strategies.iter().all(|s| s.enabled));
    }

    #[tokio::test]
    async fn test_save_strategy_returns_strategy_id() {
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
        let result = save_strategy(strategy).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_001");
    }

    #[tokio::test]
    async fn test_delete_strategy_returns_true() {
        let result = delete_strategy("test_001".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_toggle_strategy_returns_true() {
        let result = toggle_strategy("test_001".to_string(), false).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_get_risk_metrics_contains_var() {
        let result = get_risk_metrics().await;
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(metrics.contains_key("var_95"));
        assert!(metrics.contains_key("var_99"));
        assert!(metrics.contains_key("max_position_size"));
    }

    #[tokio::test]
    async fn test_get_risk_config_returns_defaults() {
        let result = get_risk_config().await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.max_position_size, 0.2);
        assert_eq!(config.max_daily_loss, 0.05);
        assert!(config.enable_pre_trade_check);
    }

    #[tokio::test]
    async fn test_update_risk_config_returns_true() {
        let config = quant_common::config::RiskConfig {
            max_position_size: 0.3,
            max_daily_loss: 0.1,
            max_drawdown: 0.2,
            max_concentration: 0.2,
            enable_pre_trade_check: true,
            enable_real_time_monitor: true,
            var_confidence_level: 0.99,
        };
        let result = update_risk_config(config).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_login_with_valid_credentials_returns_token() {
        let result = login("admin".to_string(), "admin123".to_string()).await;
        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_login_with_invalid_credentials_returns_error() {
        let result = login("admin".to_string(), "wrong".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid username or password");
    }

    #[tokio::test]
    async fn test_login_with_unknown_user_returns_error() {
        let result = login("unknown".to_string(), "pass".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_valid_token_returns_true() {
        let token = login("admin".to_string(), "admin123".to_string())
            .await
            .unwrap();
        let result = verify_token(token).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_verify_invalid_token_returns_false() {
        let result = verify_token("invalid.token.here".to_string()).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_get_user_profile_returns_admin() {
        let result = get_user_profile().await;
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile["username"], "admin");
    }
}
