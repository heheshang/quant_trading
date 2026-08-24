//! Tauri 命令单元测试。

use super::*;
use crate::state::AppState;
use crate::state::AuthedUser;
use chrono::Utc;
use monitor_layer::{AlertManager, LogBuffer};
use quant_common::config::AppConfig;
use quant_common::types::StrategyType;
use quant_common::types::{Order, StrategyParams};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use security::AuditLogger;
use std::sync::Arc;
use tokio::sync::RwLock;

fn make_test_state() -> AppState {
    use trading_layer::OrderManager;

    let alert_manager = Arc::new(AlertManager::new(false, vec![]));
    let log_buffer = Arc::new(LogBuffer::new(1000));
    AppState {
        config: Arc::new(RwLock::new(AppConfig::default())),
        alert_manager,
        log_buffer,
        audit_logger: Arc::new(AuditLogger::new(None)),
        pg_client: None,
        redis_cache: None,
        binance_client: Arc::new(RwLock::new(None)),
        order_manager: OrderManager::new(),
        app_services: None,
        binance_ws_state: crate::state::BinanceWsState::new(),
        auth_session: Arc::new(RwLock::new(Some(AuthedUser::admin()))),
    }
}

#[tokio::test]
async fn test_get_market_data_without_service_returns_error() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_market_data(state_guard, "BTC-USDT".to_string()).await;
    assert!(result.is_err());
    // Layered: with no services wired the market service reports not initialized.
    assert_eq!(
        result.unwrap_err().code,
        quant_common::api::code::NOT_INITIALIZED
    );
}

#[tokio::test]
async fn test_binance_positions_uninitialized_returns_error() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_binance_positions(state_guard, None).await;
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code,
        quant_common::api::code::NOT_INITIALIZED
    );
}

#[tokio::test]
async fn test_binance_orders_uninitialized_returns_error() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_binance_orders(state_guard, "BTC-USDT".to_string(), None, None).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_get_config_redacts_sensitive_fields() {
    let mut state = make_test_state();
    let mut cfg = AppConfig::default();
    cfg.database.password = "db_secret".to_string();
    cfg.redis.password = Some("redis_secret".to_string());
    cfg.security.jwt_secret = "jwt_secret".to_string();
    cfg.binance.api_key = "bin_key".to_string();
    cfg.binance.api_secret = "bin_secret".to_string();
    state.config = Arc::new(RwLock::new(cfg));

    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let cfg_out = get_config(state_guard).await.unwrap().data.unwrap();

    // Sensitive values must not leak.
    assert_eq!(cfg_out.database.password, "");
    assert_eq!(cfg_out.redis.password, None);
    assert_eq!(cfg_out.security.jwt_secret, "");
    assert_eq!(cfg_out.binance.api_key, "");
    assert_eq!(cfg_out.binance.api_secret, "");

    // Non-sensitive display fields preserved.
    assert_eq!(cfg_out.database.host, "localhost");
    assert_eq!(cfg_out.trading.max_orders_per_second, 100);
}

#[tokio::test]
async fn test_get_account_info_without_db_returns_error() {
    let state = make_test_state();
    // SAFETY: State is a transparent wrapper around &T
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_account_info(state_guard).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_get_positions_without_db_returns_error() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_positions(state_guard).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_get_active_orders_returns_submitted() {
    let state = make_test_state();
    // Submit an order first so OrderManager has a submitted order
    let order = Order { order_id: 0,
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
    slippage: dec!(0), exchange: "paper".to_string(), };
    state.order_manager.submit_order(order).await.unwrap();

    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_active_orders(state_guard, None).await;
    assert!(result.is_ok());
    let orders = result.unwrap().data.unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(
        orders[0].status,
        quant_common::types::OrderStatus::Submitted
    );
}

#[tokio::test]
async fn test_cancel_order_cancels_paper_order() {
    let state = make_test_state();
    // Submit a paper order into the OrderManager.
    let order = Order { order_id: 0,
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
    slippage: dec!(0), exchange: "paper".to_string(), };
    let order_id = state.order_manager.submit_order(order).await.unwrap();

    let result = crate::commands::core::cancel_order_core(&state, order_id).await;
    assert!(result.is_ok());
    assert!(result.unwrap());

    let cancelled = state.order_manager.get_order(order_id).await.unwrap();
    assert_eq!(
        cancelled.status,
        quant_common::types::OrderStatus::Cancelled
    );
}

#[tokio::test]
async fn test_cancel_order_rejects_already_filled() {
    let state = make_test_state();
    let order = Order { order_id: 0,
    strategy_id: "test_strategy".to_string(),
    symbol: "600519.SH".to_string(),
    order_type: quant_common::types::OrderType::Limit,
    side: quant_common::types::OrderSide::Buy,
    price: Some(dec!(1685.00)),
    quantity: dec!(100),
    filled_quantity: dec!(100),
    status: quant_common::types::OrderStatus::Filled,
    created_at: Utc::now(),
    updated_at: Utc::now(),
    commission: dec!(0),
    slippage: dec!(0), exchange: "paper".to_string(), };
    let order_id = state.order_manager.submit_order(order).await.unwrap();
    // Mark it filled so it is no longer cancellable.
    state
        .order_manager
        .update_order_status(order_id, quant_common::types::OrderStatus::Filled)
        .await
        .unwrap();

    let result = crate::commands::core::cancel_order_core(&state, order_id).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not in a cancellable state"));
}

#[tokio::test]
async fn test_cancel_order_not_found_returns_error() {
    let state = make_test_state();
    let result = crate::commands::core::cancel_order_core(&state, 999_999).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no active order found"));
}

#[tokio::test]
async fn test_check_redis_status_without_redis_returns_error() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = check_redis_status(state_guard).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_get_strategies_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_strategies(state_guard).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_save_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let strategy = StrategyParams::builder(
        "test_001".to_string(),
        "Test Strategy".to_string(),
        StrategyType::MeanReversion,
    )
    .params(serde_json::json!({}))
    .max_position(dec!(100000))
    .max_daily_loss(dec!(5000))
    .description(Some("Test".to_string()))
    .build();
    let result = save_strategy(state_guard, strategy).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_delete_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = delete_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_toggle_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = toggle_strategy(state_guard, "test_001".to_string(), false).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_get_risk_metrics_contains_var() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_risk_metrics(state_guard).await;
    assert!(result.is_ok());
    let metrics = result.unwrap().data.unwrap();
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
    let config = result.unwrap().data.unwrap();
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
    assert!(result.unwrap().data.unwrap());
}

#[tokio::test]
async fn test_login_without_db_returns_error() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = login(
        state_guard,
        "admin".to_string(),
        "admin123".to_string(),
        None,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().message, "Authentication unavailable: no database connection");
}

#[tokio::test]
async fn test_verify_invalid_token_without_db_returns_false() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = verify_token(state_guard, "invalid.token.here".to_string()).await;
    assert!(result.is_ok());
    assert!(!result.unwrap().data.unwrap());
}

#[tokio::test]
async fn test_verify_empty_token_without_db_returns_false() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = verify_token(state_guard, String::new()).await;
    assert!(result.is_ok());
    assert!(!result.unwrap().data.unwrap());
}

#[tokio::test]
async fn test_get_user_profile_without_db_returns_error() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_user_profile(state_guard, None).await;
    assert!(result.is_err());
}

// ── Strategy Lifecycle Commands ──

#[tokio::test]
async fn test_deploy_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = deploy_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_start_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = start_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_stop_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = stop_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_pause_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = pause_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_resume_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = resume_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}

#[tokio::test]
async fn test_archive_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = archive_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, quant_common::api::code::NOT_INITIALIZED);
}
// ── RBAC / Auth Session Tests ──

#[tokio::test]
async fn test_unauthenticated_access_to_protected_command_rejected() {
    let state = make_test_state();
    *state.auth_session.write().await = None;
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = get_config(state_guard).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().message, "Authentication required: not logged in");
}

#[tokio::test]
async fn test_low_role_rejected_from_admin_command() {
    let state = make_test_state();
    *state.auth_session.write().await = Some(AuthedUser {
        user_id: 7,
        username: "trader".into(),
        role: "trader".into(),
    });
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = update_config(state_guard, AppConfig::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Permission denied"));
}

#[tokio::test]
async fn test_admin_session_can_run_admin_command() {
    let state = make_test_state(); // make_test_state seeds an authed admin session
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = update_config(state_guard, AppConfig::default()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().data.unwrap());
}

#[tokio::test]
async fn test_verify_token_restores_session_and_enforces_rbac() {
    let mut state = make_test_state();
    *state.auth_session.write().await = None;
    {
        let mut cfg = AppConfig::default();
        cfg.security.jwt_secret = "test-secret".to_string();
        cfg.security.token_expiry_hours = 1;
        state.config = Arc::new(RwLock::new(cfg));
    }

    let auth = security::AuthService::new("test-secret".to_string(), 1);
    let token = auth
        .generate_token(7, "trader_user", vec!["trader".to_string()])
        .unwrap();

    // verify_token validates the token and re-establishes the session.
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let valid = verify_token(state_guard, token.clone()).await.unwrap().data.unwrap();
    assert!(valid);
    let session = state.auth_session.read().await.clone().unwrap();
    assert_eq!(session.role, "trader");
    assert_eq!(session.user_id, 7);
    assert_eq!(session.username, "trader_user");

    // The restored trader session must be rejected from an admin-only command.
    let state_guard2: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = update_config(state_guard2, AppConfig::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Permission denied"));
}
