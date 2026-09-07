use super::*;
use crate::state::{AuthedUser, BinanceWsState};
use data_layer::PostgresClient;
use monitor_layer::{AlertManager, LogBuffer};
use quant_common::config::AppConfig;
use quant_services::{AppServices, SharedInfra};
use security::AuditLogger;
use std::sync::Arc;
use tokio::sync::RwLock;
use trading_layer::OrderManager;

fn test_state(postgres: Option<Arc<PostgresClient>>) -> AppState {
    let config = Arc::new(RwLock::new(AppConfig::default()));
    let log_buffer = Arc::new(LogBuffer::new(100));
    let binance_client = Arc::new(RwLock::new(None));
    let order_manager = OrderManager::new();
    let services = AppServices::new(SharedInfra {
        config: config.clone(),
        postgres: postgres.clone(),
        redis: None,
        market_data: None,
        market_data_source: Arc::new(RwLock::new(None)),
        live_market_data_provider: None,
        binance_client: binance_client.clone(),
        binance_executor: Arc::new(RwLock::new(None)),
        order_manager: Arc::new(order_manager.clone()),
        log_buffer: log_buffer.clone(),
    });
    AppState {
        config,
        alert_manager: Arc::new(AlertManager::new(false, vec![])),
        log_buffer,
        audit_logger: Arc::new(AuditLogger::new(None)),
        pg_client: postgres,
        redis_cache: None,
        binance_client,
        order_manager,
        app_services: Some(services),
        binance_ws_state: BinanceWsState::new(),
        auth_session: Arc::new(RwLock::new(Some(AuthedUser::admin()))),
    }
}

// Match the existing command-test State adapter; production commands remain unchanged.
fn command_state(state: &AppState) -> State<'_, AppState> {
    unsafe { std::mem::transmute::<&AppState, State<'_, AppState>>(state) }
}

#[tokio::test]
async fn offline_risk_check_uses_explicit_config_but_update_fails() {
    let state = test_state(None);
    state.config.write().await.risk.max_position_size = 0.013;
    assert_eq!(
        get_risk_config(command_state(&state))
            .await
            .unwrap()
            .data
            .unwrap()
            .max_position_size,
        0.013
    );
    let mut proposed = state.config.read().await.risk.clone();
    proposed.max_position_size = 0.4;
    assert!(update_risk_config(command_state(&state), proposed)
        .await
        .is_err());
    assert_eq!(state.config.read().await.risk.max_position_size, 0.013);
}

#[tokio::test]
async fn binance_order_query_requires_authentication() {
    let state = test_state(None);
    *state.auth_session.write().await = None;
    let result =
        crate::commands::get_binance_order(command_state(&state), "BTC-USDT".into(), 1).await;
    assert_eq!(
        result.unwrap_err().code,
        quant_common::api::code::UNAUTHORIZED
    );
}

#[tokio::test]
#[ignore = "requires AUDIT_DATABASE_URL pointing to an isolated PostgreSQL database"]
async fn failed_risk_config_write_preserves_effective_configuration() {
    let url = std::env::var("AUDIT_DATABASE_URL").expect("isolated PostgreSQL URL");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::raw_sql(
        "CREATE TEMP TABLE risk_config (
            id INTEGER PRIMARY KEY,
            max_position_size DOUBLE PRECISION CHECK (max_position_size <= 0.5),
            max_daily_loss DOUBLE PRECISION, max_drawdown DOUBLE PRECISION,
            max_concentration DOUBLE PRECISION, enable_pre_trade_check BOOLEAN,
            enable_real_time_monitor BOOLEAN, var_confidence_level DOUBLE PRECISION,
            updated_at TIMESTAMPTZ DEFAULT NOW()
        );
        INSERT INTO risk_config VALUES (1, 0.01, 0.05, 0.15, 0.2, true, true, 0.95, NOW());",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = test_state(Some(Arc::new(PostgresClient::from_pool(pool.clone()))));
    let initial = get_risk_config(command_state(&state))
        .await
        .unwrap()
        .data
        .unwrap();
    let mut proposed = initial.clone();
    proposed.max_position_size = 0.9;
    let failure = update_risk_config(command_state(&state), proposed)
        .await
        .unwrap_err();
    assert_eq!(failure.code, quant_common::api::code::DATABASE);
    assert_eq!(state.config.read().await.risk.max_position_size, 0.01);
    assert_eq!(
        get_risk_config(command_state(&state))
            .await
            .unwrap()
            .data
            .unwrap()
            .max_position_size,
        0.01
    );

    let mut committed = initial.clone();
    committed.max_position_size = 0.3;
    assert!(update_risk_config(command_state(&state), committed)
        .await
        .unwrap()
        .data
        .unwrap());
    assert_eq!(state.config.read().await.risk.max_position_size, 0.3);
    assert_eq!(
        state
            .app_services
            .as_ref()
            .unwrap()
            .risk_service
            .get_risk_config()
            .await
            .unwrap()
            .max_position_size,
        0.3
    );

    sqlx::query("DELETE FROM pg_temp.risk_config")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        update_risk_config(command_state(&state), initial)
            .await
            .unwrap_err()
            .code,
        quant_common::api::code::NOT_FOUND
    );
    assert_eq!(
        get_risk_config(command_state(&state))
            .await
            .unwrap_err()
            .code,
        quant_common::api::code::NOT_FOUND
    );
    assert_eq!(state.config.read().await.risk.max_position_size, 0.3);

    pool.close().await;
    assert_eq!(
        get_risk_config(command_state(&state))
            .await
            .unwrap_err()
            .code,
        quant_common::api::code::DATABASE
    );
    assert_eq!(state.config.read().await.risk.max_position_size, 0.3);
}
