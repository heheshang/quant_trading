use crate::state::AppState;
use chrono::Utc;
use quant_common::config::AppConfig;
use quant_common::types::{Account, MarketData, Order, OrderStatus, Position};
use rust_decimal::prelude::ToPrimitive;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    state.require_auth().await?;
    let config = state.config.read().await;
    // Redact sensitive values (DB password, JWT secret, exchange API
    // keys/secrets/passphrase) so they never leave the backend.
    Ok(config.redacted())
}

/// 用后端 ENCRYPTION_KEY 加密任意字符串（用于把会话 token 等敏感值加密后落盘）。
#[tauri::command]
pub async fn secure_encrypt(state: State<'_, AppState>, value: String) -> Result<String, String> {
    state.require_auth().await?;
    let key = state.config.read().await.security.encryption_key.clone();
    let de = security::DataEncryption::from_key_string(&key).map_err(|e| format!("加密初始化失败: {}", e))?;
    de.encrypt_string(&value).map_err(|e| format!("加密失败: {}", e))
}

/// 用后端 ENCRYPTION_KEY 解密（配合 `secure_encrypt`）。
#[tauri::command]
pub async fn secure_decrypt(state: State<'_, AppState>, value: String) -> Result<String, String> {
    state.require_auth().await?;
    let key = state.config.read().await.security.encryption_key.clone();
    let de = security::DataEncryption::from_key_string(&key).map_err(|e| format!("解密初始化失败: {}", e))?;
    de.decrypt_string(&value).map_err(|e| format!("解密失败: {}", e))
}

/// 更新系统配置
#[tauri::command]
pub async fn update_config(state: State<'_, AppState>, config: AppConfig) -> Result<bool, String> {
    state.require_role("admin").await?;
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
    // Route through the services layer so the command never touches the
    // data-source / infrastructure layer directly (layering + DIP).
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| "Market service not initialized".to_string())?;

    services
        .market_service
        .get_realtime_data(&symbol)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn submit_order(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    order: Order,
) -> Result<String, String> {
    let user = state.require_auth().await?;
    // The order-placement pipeline (market data → risk check → submit →
    //   persist → emit → async execution) lives in `OrderProcessor` so the
    //   command stays a *thin adapter* (SRP) and never reaches into the
    //   domain / engine / infrastructure layers directly.
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| "Order service not initialized (no database connection)".to_string())?;

    let symbol = order.symbol.clone();
    let side = format!("{:?}", order.side);
    let quantity = order.quantity.to_string();

    let placement = match services.order_processor.place_order(order).await {
        Ok(p) => p,
        Err(e) => {
            // 被拒/失败也审计（成功/失败 + 错误信息），避免失联的下单审计盲区。
            let msg = e.to_string();
            let _ = state
                .audit_logger
                .log_order_submit(
                    &user.user_id.to_string(),
                    &user.username,
                    "N/A",
                    &symbol,
                    &side,
                    &quantity,
                    false,
                    Some(msg.clone()),
                )
                .await;
            return Err(msg);
        }
    };

    let _ = state
        .audit_logger
        .log_order_submit(
            &user.user_id.to_string(),
            &user.username,
            &placement.order_id.to_string(),
            &symbol,
            &side,
            &quantity,
            true,
            None,
        )
        .await;

    // Forward the UI event; the use-case already ran persistence + async execution.
    let _ = app.emit("order:submitted", placement.event);

    Ok(placement.order_id.to_string())
}

/// 运行算法订单（TWAP / VWAP / Iceberg）。
///
/// 将一笔大单按所选算法拆分为若干普通 Market / Limit 子订单，每个子订单经
/// [`OrderProcessor::place_order`] 走完整下单链路（风控 + 纸面/实盘执行 + 持久化）。
/// 子订单类型永远不是 `TWAP/VWAP/Iceberg`，故 Binance 实盘能将其作为普通市价/限价单接收。
///
/// 返回每个子订单的 `order_id`，并按子订单逐个发送 `order:submitted` UI 事件。
#[tauri::command]
pub async fn run_algorithmic_order(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    order: Order,
    algorithm: String,
    params: quant_services::order_processor::AlgorithmicOrderParams,
) -> Result<Vec<i64>, String> {
    state.require_auth().await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| "Order service not initialized (no database connection)".to_string())?;

    let placements = services
        .order_processor
        .place_algorithmic_order(order, &algorithm, &params)
        .await
        .map_err(|e| e.to_string())?;

    // Forward one UI event per placed slice so the frontend can track children.
    for placement in &placements {
        let _ = app.emit("order:submitted", placement.event.clone());
    }

    Ok(placements.iter().map(|p| p.order_id).collect())
}

#[tauri::command]
pub async fn get_account_info(state: State<'_, AppState>) -> Result<Account, String> {
    state.require_auth().await?;
    match state.app_services.as_ref() {
        Some(services) => match services.account_service.get_account_info().await {
            Ok(account) => {
                // 单一写者：position_value 仅在此处写（账户真实持仓市值）；账户余额/当日盈亏
                // 由 start_monitor_metrics（连续快照）写，避免两个来源互相覆盖。
                monitor_layer::MetricsCollector::set_position_value(
                    account.market_value.to_f64().unwrap_or(0.0),
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
    state.require_auth().await?;
    match state.app_services.as_ref() {
        Some(services) => match services.account_service.get_positions().await {
            Ok(positions) => Ok(positions),
            Err(e) => Err(format!("Positions unavailable: {}", e)),
        },
        None => Err("Account service not initialized (no database connection)".to_string()),
    }
}

/// 最近订单（含已成交/撤单/拒绝），按时间倒序，供「最近交易」等展示。
#[tauri::command]
pub async fn get_recent_orders(
    state: State<'_, AppState>,
    limit: Option<u32>,
    exchange: Option<String>,
) -> Result<Vec<Order>, String> {
    state.require_auth().await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .account_service
        .get_recent_orders(limit.unwrap_or(50), exchange.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_active_orders(
    state: State<'_, AppState>,
    exchange: Option<String>,
) -> Result<Vec<Order>, String> {
    state.require_auth().await?;
    // 活跃订单以数据库为准（持久化，重启后仍可读）；可按种类(paper/live/algorithm)过滤。
    if let Some(services) = state.app_services.as_ref() {
        match services.account_service.get_active_orders(exchange.as_deref()).await {
            Ok(orders) => return Ok(orders),
            Err(_) => {
                // DB 不可用时降级到内存 OrderManager（保证不阻塞页面）。
            }
        }
    }

    // DB 不可用 → 内存 OrderManager 兜底。
    Ok(state.order_manager.get_active_orders().await)
}

/// 撤销订单（paper / OrderManager 订单）
///
/// 对未成交/进行中的订单（Submitted / PartiallyFilled）执行取消：
/// 优先从内存 `OrderManager` 取消；否则降级到数据库订单表。
#[tauri::command]
pub async fn cancel_order(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    order_id: i64,
) -> Result<bool, String> {
    let user = state.require_auth().await?;
    let cancelled = cancel_order_core(state.inner(), order_id).await?;
    if cancelled {
        let _ = app.emit("order:cancelled", order_id);
        let _ = state
            .audit_logger
            .log(
                &user.user_id.to_string(),
                &user.username,
                security::audit::AuditAction::OrderCancel,
                &order_id.to_string(),
                serde_json::json!({}),
                None,
                true,
                None,
            )
            .await;
    }
    Ok(cancelled)
}

pub(crate) async fn cancel_order_core(state: &AppState, order_id: i64) -> Result<bool, String> {
    // Paper / in-memory orders live in the OrderManager.
    if let Ok(order) = state.order_manager.get_order(order_id).await {
        if !matches!(
            order.status,
            OrderStatus::Submitted | OrderStatus::PartiallyFilled
        ) {
            return Err(format!(
                "Order {} is not in a cancellable state: {:?}",
                order_id, order.status
            ));
        }
        state
            .order_manager
            .cancel_order(order_id)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(true);
    }

    // Fallback: order persisted in the database.
    if let Some(services) = state.app_services.as_ref() {
        services
            .account_service
            .cancel_order(order_id)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(true);
    }

    Err(format!(
        "Cannot cancel order {}: no active order found",
        order_id
    ))
}
