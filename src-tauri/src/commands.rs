use quant_common::config::AppConfig;
use quant_common::types::{Account, Order, MarketData, BacktestResult, Position, Alert};
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;
use rust_decimal_macros::dec;
use rust_decimal::Decimal;
use chrono::Utc;
use std::collections::HashMap;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.read().await;
    Ok(config.clone())
}

#[tauri::command]
pub async fn get_market_data(_symbol: String) -> Result<MarketData, String> {
    // TODO: 实现真实数据获取
    Err("Not implemented".to_string())
}

#[tauri::command]
pub async fn submit_order(order: Order) -> Result<String, String> {
    // TODO: 实现订单提交
    Ok(order.order_id.to_string())
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
pub async fn run_backtest(_strategy_id: String, _start_date: String, _end_date: String) -> Result<BacktestResult, String> {
    // TODO: 实现回测
    Err("Not implemented".to_string())
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
pub async fn get_logs(level: Option<String>, limit: Option<u32>) -> Result<Vec<quant_common::types::LogEntry>, String> {
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