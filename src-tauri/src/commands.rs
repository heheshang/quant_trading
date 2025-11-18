use quant_common::config::AppConfig;
use quant_common::types::{Account, Order, MarketData, BacktestResult, Position, Alert, StrategyParams, StrategyType};
use strategy_layer::{BacktestEngine, Strategy};
use rust_decimal::Decimal;
use strategy_layer::strategy::MeanReversionStrategy;
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;
use rust_decimal_macros::dec;
use chrono::Utc;
use std::collections::HashMap;

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
pub async fn submit_order(order: Order) -> Result<String, String> {
    use trading_layer::{OrderManager, ExecutionEngine};
    use std::sync::Arc;
    use quant_common::config::TradingConfig;
    use quant_common::types::MarketData;
    use rust_decimal_macros::dec;
    
    // 创建订单管理器
    let order_manager = Arc::new(OrderManager::new(100));
    
    // 创建交易配置
    let config = TradingConfig {
        enable_paper_trading: true,
        max_orders_per_second: 100,
        default_commission_rate: 0.001,
        default_slippage: 0.0005,
        order_timeout_seconds: 30,
    };
    
    // 创建执行引擎
    let execution_engine = ExecutionEngine::new(order_manager.clone(), config);
    
    // 提交订单到订单管理器
    let order_id = order_manager.submit_order(order.clone()).await
        .map_err(|e| format!("Failed to submit order: {}", e))?
        .to_string();
    
    // 模拟市场数据用于执行
    let market_data = MarketData {
        symbol: order.symbol.clone(),
        timestamp: chrono::Utc::now(),
        open: order.price.unwrap_or(dec!(0)),
        high: order.price.unwrap_or(dec!(0)),
        low: order.price.unwrap_or(dec!(0)),
        close: order.price.unwrap_or(dec!(0)),
        volume: dec!(1000000),
        turnover: dec!(1000000000),
        open_interest: None,
        bid_prices: vec![],
        bid_volumes: vec![],
        ask_prices: vec![],
        ask_volumes: vec![],
    };
    
    // 执行订单
    tokio::spawn(async move {
        // 模拟一些延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // 执行订单
        if let Err(e) = execution_engine.execute_order(order, &market_data).await {
            eprintln!("Order execution failed: {}", e);
        }
    });
    
    Ok(order_id)
}

#[tauri::command]
pub async fn get_account_info() -> Result<Account, String> {
    // Mock account data for demonstration
    Ok(Account {
        account_id: Uuid::new_v4(),
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

#[tauri::command]
pub async fn get_positions() -> Result<Vec<Position>, String> {
    // Mock positions data for demonstration
    Ok(vec![
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
        }
    ])
}

#[tauri::command]
pub async fn get_active_orders() -> Result<Vec<Order>, String> {
    // Mock orders data for demonstration
    Ok(vec![
        Order {
            order_id: Uuid::new_v4(),
            strategy_id: "trend_following".to_string(),
            symbol: "600519.SH".to_string(),
            order_type: quant_common::types::OrderType::Limit,
            side: quant_common::types::OrderSide::Buy,
            price: Some(dec!(1685.00)),
            quantity: dec!(100),
            filled_quantity: dec!(0),
            status: quant_common::types::OrderStatus::Submitted,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: dec!(0),
            slippage: dec!(0),
        }
    ])
}

#[tauri::command]
pub async fn run_backtest(strategy_id: String, _start_date: String, _end_date: String) -> Result<BacktestResult, String> {
    // 模拟市场数据
    let market_data = vec![
        MarketData {
            symbol: "600519.SH".to_string(),
            timestamp: chrono::Utc::now() - chrono::Duration::days(30),
            open: dec!(1600.00),
            high: dec!(1650.00),
            low: dec!(1580.00),
            close: dec!(1620.00),
            volume: dec!(1000000),
            turnover: dec!(1620000000),
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        },
        MarketData {
            symbol: "600519.SH".to_string(),
            timestamp: chrono::Utc::now() - chrono::Duration::days(29),
            open: dec!(1620.00),
            high: dec!(1680.00),
            low: dec!(1610.00),
            close: dec!(1650.00),
            volume: dec!(1200000),
            turnover: dec!(1980000000),
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        }
    ];
    
    // 创建回测引擎
    let mut engine = BacktestEngine::new(
        dec!(1000000), // 初始资金
        dec!(0.001),   // 手续费率
        dec!(0.0005)   // 滑点
    );
    
    // 创建策略实例
    let mut strategy = MeanReversionStrategy::new();
    
    // 初始化策略参数
    let strategy_params = StrategyParams {
        strategy_id: strategy_id.clone(),
        strategy_name: "回测策略".to_string(),
        strategy_type: StrategyType::MeanReversion,
        params: serde_json::json!({
            "lookback_period": 20,
            "entry_threshold": 2.0,
            "exit_threshold": 0.5
        }),
        enabled: true,
        max_position: dec!(100000),
        max_daily_loss: dec!(5000),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // 初始化策略
    strategy.initialize(strategy_params).await
        .map_err(|e| format!("Failed to initialize strategy: {}", e))?;
    
    // 运行回测
    let result = engine.run(&strategy, market_data).await
        .map_err(|e| format!("Backtest failed: {}", e))?;
    
    Ok(result)
}

/// 获取实时指标数据
#[tauri::command]
pub async fn get_metrics() -> Result<HashMap<String, f64>, String> {
    let mut metrics = HashMap::new();
    
    // 模拟指标数据
    metrics.insert("orders_total".to_string(), monitor_layer::ORDERS_TOTAL.get() as f64);
    metrics.insert("orders_filled".to_string(), monitor_layer::ORDERS_FILLED.get() as f64);
    metrics.insert("orders_cancelled".to_string(), monitor_layer::ORDERS_CANCELLED.get() as f64);
    metrics.insert("account_balance".to_string(), monitor_layer::ACCOUNT_BALANCE.get() as f64);
    metrics.insert("position_value".to_string(), monitor_layer::POSITION_VALUE.get() as f64);
    metrics.insert("daily_pnl".to_string(), monitor_layer::DAILY_PNL.get() as f64);
    
    Ok(metrics)
}

/// 获取告警信息
#[tauri::command]
pub async fn get_alerts() -> Result<Vec<Alert>, String> {
    // 模拟告警数据
    let alerts = vec![Alert {
        alert_id: Uuid::new_v4(),
        level: quant_common::types::AlertLevel::Warning,
        source: "Risk Management".to_string(),
        message: "Account margin ratio approaching limit".to_string(),
        timestamp: Utc::now(),
        acknowledged: false,
    }];
    
    Ok(alerts)
}

/// 确认告警
#[tauri::command]
pub async fn acknowledge_alert(alert_id: String) -> Result<bool, String> {
    // 模拟告警确认
    println!("Acknowledging alert: {}", alert_id);
    Ok(true)
}

/// 获取日志信息
#[tauri::command]
pub async fn get_logs(_level: Option<String>, _limit: Option<u32>) -> Result<Vec<quant_common::types::LogEntry>, String> {
    // 模拟日志数据
    let logs = vec![
        quant_common::types::LogEntry {
            timestamp: Utc::now(),
            level: "info".to_string(),
            message: "System started successfully222".to_string(),
            module: Some("main".to_string()),
        },
        quant_common::types::LogEntry {
            timestamp: Utc::now() - chrono::Duration::minutes(1),
            level: "warning".to_string(),
            message: "Account margin ratio approaching limit".to_string(),
            module: Some("risk".to_string()),
        },
        quant_common::types::LogEntry {
            timestamp: Utc::now() - chrono::Duration::minutes(2),
            level: "error".to_string(),
            message: "Order execution failed for symbol 60051911.SH".to_string(),
            module: Some("trading".to_string()),
        }
    ];
    
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
        }
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

/// 获取风险指标
#[tauri::command]
pub async fn get_risk_metrics() -> Result<HashMap<String, f64>, String> {
    use risk_layer::VaRCalculator;
    use quant_common::config::RiskConfig;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::ToPrimitive;
    
    let mut metrics = HashMap::new();
    
    // 模拟收益率数据用于VaR计算
    let returns = vec![
        dec!(0.01), dec!(-0.005), dec!(0.02), dec!(-0.01), dec!(0.008),
        dec!(-0.015), dec!(0.012), dec!(0.003), dec!(-0.007), dec!(0.011)
    ];
    
    // 计算VaR
    let config = RiskConfig {
        max_position_size: 0.2,
        max_daily_loss: 0.05,
        max_drawdown: 0.15,
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
pub async fn pre_trade_check(order: Order, account: Account, positions: Vec<Position>) -> Result<bool, String> {
    use risk_layer::PreTradeRiskChecker;
    use quant_common::config::RiskConfig;
    
    let config = RiskConfig {
        max_position_size: 0.2,
        max_daily_loss: 0.05,
        max_drawdown: 0.15,
        enable_pre_trade_check: true,
        enable_real_time_monitor: true,
        var_confidence_level: 0.95,
    };
    
    let checker = PreTradeRiskChecker::new(config);
    
    match checker.check_order(&order, &account, &positions) {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Pre-trade check failed: {}", e);
            Ok(false)
        }
    }
}

/// 用户登录
#[tauri::command]
pub async fn login(username: String, password: String) -> Result<String, String> {
    use security::AuthService;
    use quant_common::config::AppConfig;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    // In a real implementation, this would check against a user database
    // For now, we'll use a simple check
    if username == "admin" && password == "admin123" {
        // Create auth service with config values
        let config = AppConfig::default();
        let auth_service = AuthService::new(
            config.security.jwt_secret,
            config.security.token_expiry_hours as i64
        );
        
        // Generate JWT token
        let token = auth_service.generate_token(
            "admin_id",
            &username,
            vec!["admin".to_string()]
        ).map_err(|e| format!("Token generation failed: {}", e))?;
        
        Ok(token)
    } else {
        Err("Invalid username or password".to_string())
    }
}

/// 验证 Token
#[tauri::command]
pub async fn verify_token(token: String) -> Result<bool, String> {
    use security::AuthService;
    use quant_common::config::AppConfig;
    
    let config = AppConfig::default();
    let auth_service = AuthService::new(
        config.security.jwt_secret,
        config.security.token_expiry_hours as i64
    );
    
    match auth_service.verify_token(&token) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false)
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
pub async fn change_password(current_password: String, new_password: String) -> Result<bool, String> {
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
