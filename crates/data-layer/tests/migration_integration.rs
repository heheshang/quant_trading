use quant_common::Result;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::Arc;

async fn get_test_pool() -> Result<Arc<PgPool>> {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://quant:postgres@localhost/quant_trading_test".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| quant_common::Error::Database(format!("Failed to connect: {}", e)))?;

    Ok(Arc::new(pool))
}

async fn cleanup_test_db(pool: &PgPool) -> Result<()> {
    let _ = sqlx::query("DROP TABLE IF EXISTS market_data CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS risk_metrics CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS alerts CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS trades CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS backtest_results CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS api_keys CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS audit_logs CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS orders CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS positions CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS strategies CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS instruments CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS risk_config CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS accounts CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS users CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(pool)
        .await;
    Ok(())
}

async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| quant_common::Error::Database(format!("Migration failed: {}", e)))
}

#[tokio::test]
#[ignore]
async fn test_full_migration_cycle() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");

    run_migrations(pool.as_ref())
        .await
        .expect("Migrations should succeed");

    let tables = vec![
        "users",
        "accounts",
        "orders",
        "positions",
        "instruments",
        "strategies",
        "backtest_results",
        "api_keys",
        "audit_logs",
        "alerts",
        "market_data",
        "risk_config",
    ];
    for table in tables {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
        )
        .bind(table)
        .fetch_one(pool.as_ref())
        .await
        .unwrap_or_else(|_| panic!("Failed to check {} table", table));
        assert!(exists, "Table {} should exist after migration", table);
    }

    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn test_migration_idempotency() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");

    run_migrations(pool.as_ref())
        .await
        .expect("First migration should succeed");
    run_migrations(pool.as_ref())
        .await
        .expect("Second migration should succeed (no-op)");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(pool.as_ref())
        .await
        .expect("Failed to count migrations");
    assert!(
        count >= 6,
        "Should have at least 6 migration records, got {}",
        count
    );

    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn test_table_structure() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");

    run_migrations(pool.as_ref())
        .await
        .expect("Migrations should succeed");

    let columns: Vec<(String,)> = sqlx::query_as(
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'accounts' ORDER BY ordinal_position"
    )
    .fetch_all(pool.as_ref())
    .await
    .expect("Failed to get columns");

    let expected_columns = vec![
        "account_id",
        "user_id",
        "account_type",
        "total_assets",
        "available_cash",
        "frozen_cash",
        "market_value",
        "total_pnl",
        "daily_pnl",
        "margin",
        "margin_ratio",
        "created_at",
        "updated_at",
    ];
    assert_eq!(
        columns.len(),
        expected_columns.len(),
        "Accounts table should have correct number of columns"
    );
    for (i, col) in columns.iter().enumerate() {
        assert_eq!(col.0, expected_columns[i], "Column {} should match", i);
    }

    let fk_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM information_schema.table_constraints
        WHERE constraint_type = 'FOREIGN KEY'
        AND table_name IN ('accounts', 'orders', 'positions', 'strategies', 'backtest_results')
        "#,
    )
    .fetch_one(pool.as_ref())
    .await
    .expect("Failed to count foreign keys");
    assert!(
        fk_count >= 5,
        "Should have at least 5 foreign key constraints, got {}",
        fk_count
    );

    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn test_data_insertion_after_migration() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");

    run_migrations(pool.as_ref())
        .await
        .expect("Migrations should succeed");

    // The demo-data migration already created an `admin` user plus its account,
    // so insert a dedicated test user for isolation.
    let user_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO users (username, email, password_hash)
        VALUES ($1, $2, $3)
        RETURNING user_id
        "#,
    )
    .bind("integration_test_user")
    .bind("integration_test_user@example.com")
    .bind("test-password-hash")
    .fetch_one(pool.as_ref())
    .await
    .expect("Failed to insert test user");

    // accounts.user_id is NOT NULL FK -> users(user_id).
    let account_id: i64 = 0;
    sqlx::query(
        r#"
        INSERT INTO accounts (account_id, user_id, account_type, total_assets)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(account_id)
    .bind(user_id)
    .bind("SPOT")
    .bind(Decimal::new(10000, 0))
    .execute(pool.as_ref())
    .await
    .expect("Failed to insert account");

    // The demo migration also creates an account for `admin`, so a global
    // COUNT(*) would be 2; assert on our own test user's account instead.
    let account_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool.as_ref())
        .await
        .expect("Failed to count accounts");
    assert_eq!(account_count, 1, "Should have 1 account for test user");

    // orders.symbol is FK -> instruments(symbol); 'BTC-USDT' is populated by the
    // demo-data migration. quantity is DECIMAL(20,8).
    let order_id: i64 = 0;
    sqlx::query(
        r#"
        INSERT INTO orders (order_id, account_id, symbol, order_type, side, quantity, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(order_id)
    .bind(account_id)
    .bind("BTC-USDT")
    .bind("LIMIT")
    .bind("BUY")
    .bind(Decimal::new(1, 0))
    .bind("PENDING")
    .execute(pool.as_ref())
    .await
    .expect("Failed to insert order");

    // NOTE: orders.account_id REFERENCES accounts(account_id) has NO
    // ON DELETE CASCADE in migration 20240101000001, so deleting the account
    // directly would violate the FK constraint (23503). Remove the dependent
    // order first, then the account.
    sqlx::query("DELETE FROM orders WHERE order_id = $1")
        .bind(order_id)
        .execute(pool.as_ref())
        .await
        .expect("Failed to delete order");

    sqlx::query("DELETE FROM accounts WHERE account_id = $1")
        .bind(account_id)
        .execute(pool.as_ref())
        .await
        .expect("Failed to delete account");

    let order_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE order_id = $1")
        .bind(order_id)
        .fetch_one(pool.as_ref())
        .await
        .expect("Failed to count orders");
    assert_eq!(order_count, 0, "Order should be removed after delete");

    let remaining_accounts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM accounts WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(pool.as_ref())
            .await
            .expect("Failed to count accounts");
    assert_eq!(
        remaining_accounts, 0,
        "Account should be removed after delete"
    );

    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn test_market_data_partitions() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");

    run_migrations(pool.as_ref())
        .await
        .expect("Migrations should succeed");

    let parent_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'market_data')",
    )
    .fetch_one(pool.as_ref())
    .await
    .expect("Failed to check market_data table");
    assert!(parent_exists, "market_data parent table should exist");

    let partition_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM pg_class
        WHERE oid IN (
            SELECT inhrelid FROM pg_inherits
            WHERE inhparent = 'market_data'::regclass
        )
        "#,
    )
    .fetch_one(pool.as_ref())
    .await
    .expect("Failed to count partitions");
    assert!(
        partition_count >= 24,
        "Should have at least 24 monthly partitions, got {}",
        partition_count
    );

    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn test_alter_table_json_fields() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");

    run_migrations(pool.as_ref())
        .await
        .expect("Migrations should succeed");

    let param_col_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT FROM information_schema.columns
            WHERE table_name = 'backtest_results' AND column_name = 'parameters_json'
        )
        "#,
    )
    .fetch_one(pool.as_ref())
    .await
    .expect("Failed to check column");
    assert!(
        param_col_exists,
        "backtest_results.parameters_json should exist"
    );

    let config_col_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT FROM information_schema.columns
            WHERE table_name = 'strategies' AND column_name = 'indicator_config_json'
        )
        "#,
    )
    .fetch_one(pool.as_ref())
    .await
    .expect("Failed to check column");
    assert!(
        config_col_exists,
        "strategies.indicator_config_json should exist"
    );

    cleanup_test_db(pool.as_ref())
        .await
        .expect("Failed to cleanup");
}
