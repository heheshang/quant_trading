//! Tauri 命令单元测试。

use super::*;
use crate::state::AppState;
use chrono::Utc;
use exchange_okx::types::*;
use exchange_okx::ClientInterface;
use exchange_okx::MockOkxClient;
use monitor_layer::{AlertManager, LogBuffer};
use quant_common::config::AppConfig;
use quant_common::types::{Order, StrategyParams};
use quant_common::types::{StrategyStatus, StrategyType};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio::sync::RwLock;

type SharedMockClient = Arc<RwLock<Option<Arc<RwLock<dyn ClientInterface + Send + Sync>>>>>;

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

    let okx_client: SharedMockClient = Arc::new(RwLock::new(mock_client.map(|mc| {
        let inner: Arc<RwLock<dyn ClientInterface + Send + Sync>> = Arc::new(RwLock::new(mc));
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
async fn test_check_redis_status_without_redis_returns_error() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = check_redis_status(state_guard).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Redis client not initialized"));
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
        status: StrategyStatus::Draft,
        description: Some("Test".to_string()),
        tags: vec![],
        symbols: vec![],
        instance_label: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        user_id: 0,
        version: 0,
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
    mock.expect_get_account_balance().returning(|_| {
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
    mock.expect_get_positions().returning(|_| {
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
    mock.expect_place_order().returning(|_| {
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
                fee: "0".to_string(),
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

    let result = cancel_okx_order(state_guard, "BTC-USDT".to_string(), "123".to_string()).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_cancel_okx_order_not_initialized() {
    let state = create_mock_okx_state(None);
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };

    let result = cancel_okx_order(state_guard, "BTC-USDT".to_string(), "123".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "OKX client not initialized");
}

#[tokio::test]
async fn test_get_okx_candles_success() {
    let mut mock = MockOkxClient::new();
    mock.expect_get_candles().returning(|_, _, _| {
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

    let result = get_okx_candles(state_guard, "BTC-USDT".to_string(), None, None).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "OKX client not initialized");
}

#[tokio::test]
async fn test_get_okx_candles_invalid_params() {
    let mut mock = MockOkxClient::new();
    mock.expect_get_candles().returning(|_, _, _| {
        Box::pin(async {
            Err(quant_common::Error::Internal(
                "Invalid instrument ID".to_string(),
            ))
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
    mock.expect_get_instruments().returning(|_| {
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
    mock.expect_get_announcements().returning(|| {
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

// ── Strategy Lifecycle Commands ──

#[tokio::test]
async fn test_deploy_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = deploy_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Application services not initialized");
}

#[tokio::test]
async fn test_start_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = start_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Application services not initialized");
}

#[tokio::test]
async fn test_stop_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = stop_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Application services not initialized");
}

#[tokio::test]
async fn test_pause_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = pause_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Application services not initialized");
}

#[tokio::test]
async fn test_resume_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = resume_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Application services not initialized");
}

#[tokio::test]
async fn test_archive_strategy_requires_services() {
    let state = make_test_state();
    let state_guard: tauri::State<'_, AppState> =
        unsafe { std::mem::transmute::<&AppState, tauri::State<'_, AppState>>(&state) };
    let result = archive_strategy(state_guard, "test_001".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Application services not initialized");
}
