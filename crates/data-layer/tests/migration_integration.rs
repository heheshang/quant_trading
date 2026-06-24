use data_layer::migrations::migrations::get_all_migrations;
use data_layer::{Migration, MigrationManager};
use quant_common::Result;
use sqlx::PgPool;
use std::sync::Arc;

/// Integration tests for database migrations
/// These tests require a running PostgreSQL instance
///
/// To run these tests:
/// 1. Start PostgreSQL
/// 2. Create a test database: CREATE DATABASE quant_trading_test;
/// 3. Set environment variables:
///    - TEST_DATABASE_URL=postgres://quant:postgres@localhost/quant_trading_test
/// 4. Run: cargo test --package data-layer --test migration_integration -- --ignored

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
    // Drop all tables to ensure clean state
    let _ = sqlx::query("DROP TABLE IF EXISTS risk_metrics CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS alerts CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS trades CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS positions CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS orders CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS accounts CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS migrations CASCADE")
        .execute(pool)
        .await;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_full_migration_cycle() {
    let pool = get_test_pool().await.expect("Failed to get test pool");

    // Clean up before test
    cleanup_test_db(&*pool).await.expect("Failed to cleanup");

    // Create migration manager
    let mut manager = MigrationManager::new(pool.clone());

    // Add all migrations
    for migration in get_all_migrations() {
        manager.add_migration(migration);
    }

    // Test 1: Initialize migrations table
    manager
        .init()
        .await
        .expect("Failed to initialize migrations table");

    // Verify migrations table exists
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM migrations")
        .fetch_one(&*pool)
        .await
        .expect("Migrations table should exist");
    assert_eq!(count, 0, "Migrations table should be empty initially");

    // Test 2: Check initial version
    let version = manager
        .get_current_version()
        .await
        .expect("Failed to get version");
    assert_eq!(version, 0, "Initial version should be 0");

    // Test 3: Get pending migrations
    let pending = manager
        .get_pending_migrations()
        .await
        .expect("Failed to get pending");
    assert_eq!(pending.len(), 3, "Should have 3 pending migrations");

    // Test 4: Run all migrations
    manager.migrate().await.expect("Failed to run migrations");

    // Verify all tables exist
    let tables = vec![
        "accounts",
        "orders",
        "positions",
        "trades",
        "alerts",
        "risk_metrics",
    ];
    for table in tables {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
        )
        .bind(table)
        .fetch_one(&*pool)
        .await
        .expect(&format!("Failed to check {} table", table));
        assert!(exists, "Table {} should exist after migration", table);
    }

    // Test 5: Verify version is updated
    let new_version = manager
        .get_current_version()
        .await
        .expect("Failed to get version");
    assert_eq!(new_version, 3, "Version should be 3 after all migrations");

    // Test 6: Verify no pending migrations
    let pending = manager
        .get_pending_migrations()
        .await
        .expect("Failed to get pending");
    assert_eq!(pending.len(), 0, "Should have no pending migrations");

    // Test 7: Verify migration records
    let records = manager
        .get_applied_migrations()
        .await
        .expect("Failed to get records");
    assert_eq!(records.len(), 3, "Should have 3 migration records");
    assert_eq!(records[0].version, 1);
    assert_eq!(records[1].version, 2);
    assert_eq!(records[2].version, 3);

    // Test 8: Verify indices exist (from migration 002)
    let indices = vec![
        "idx_accounts_type",
        "idx_orders_account",
        "idx_positions_account",
        "idx_trades_order",
    ];
    for index in indices {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_indexes WHERE indexname = $1)")
                .bind(index)
                .fetch_one(&*pool)
                .await
                .expect(&format!("Failed to check {} index", index));
        assert!(exists, "Index {} should exist after migration", index);
    }

    // Test 9: Rollback to version 1
    manager.rollback_to(1).await.expect("Failed to rollback");
    let version = manager
        .get_current_version()
        .await
        .expect("Failed to get version");
    assert_eq!(version, 1, "Version should be 1 after rollback");

    // Verify alerts and risk_metrics tables are gone
    let alerts_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'alerts')",
    )
    .fetch_one(&*pool)
    .await
    .expect("Failed to check alerts table");
    assert!(
        !alerts_exists,
        "Alerts table should not exist after rollback"
    );

    // Test 10: Rollback to version 0
    manager
        .rollback_to(0)
        .await
        .expect("Failed to rollback to 0");
    let version = manager
        .get_current_version()
        .await
        .expect("Failed to get version");
    assert_eq!(version, 0, "Version should be 0 after full rollback");

    // Verify all tables are gone
    for table in vec!["accounts", "orders", "positions", "trades"] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
        )
        .bind(table)
        .fetch_one(&*pool)
        .await
        .expect(&format!("Failed to check {} table", table));
        assert!(
            !exists,
            "Table {} should not exist after full rollback",
            table
        );
    }

    // Clean up after test
    cleanup_test_db(&*pool).await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_migration_idempotency() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup_test_db(&*pool).await.expect("Failed to cleanup");

    let mut manager = MigrationManager::new(pool.clone());
    for migration in get_all_migrations() {
        manager.add_migration(migration);
    }

    // Run migrations twice
    manager
        .migrate()
        .await
        .expect("First migration should succeed");
    manager
        .migrate()
        .await
        .expect("Second migration should succeed (no-op)");

    // Verify version is still correct
    let version = manager
        .get_current_version()
        .await
        .expect("Failed to get version");
    assert_eq!(version, 3, "Version should still be 3");

    // Verify migration count
    let records = manager
        .get_applied_migrations()
        .await
        .expect("Failed to get records");
    assert_eq!(
        records.len(),
        3,
        "Should still have exactly 3 migration records"
    );

    cleanup_test_db(&*pool).await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_table_structure() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup_test_db(&*pool).await.expect("Failed to cleanup");

    let mut manager = MigrationManager::new(pool.clone());
    for migration in get_all_migrations() {
        manager.add_migration(migration);
    }

    manager.migrate().await.expect("Migration should succeed");

    // Test accounts table structure
    let columns: Vec<(String,)> = sqlx::query_as(
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'accounts' ORDER BY ordinal_position"
    )
    .fetch_all(&*pool)
    .await
    .expect("Failed to get columns");

    let expected_columns = vec![
        "account_id",
        "account_name",
        "account_type",
        "total_assets",
        "available_balance",
        "frozen_balance",
        "margin_ratio",
        "daily_pnl",
        "total_pnl",
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

    // Test foreign key constraints
    let fk_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM information_schema.table_constraints 
        WHERE constraint_type = 'FOREIGN KEY' 
        AND table_name IN ('orders', 'positions', 'trades', 'risk_metrics')
        "#,
    )
    .fetch_one(&*pool)
    .await
    .expect("Failed to count foreign keys");

    assert!(fk_count >= 4, "Should have foreign key constraints");

    cleanup_test_db(&*pool).await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_data_insertion_after_migration() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup_test_db(&*pool).await.expect("Failed to cleanup");

    let mut manager = MigrationManager::new(pool.clone());
    for migration in get_all_migrations() {
        manager.add_migration(migration);
    }

    manager.migrate().await.expect("Migration should succeed");

    // Insert test data into accounts
    let account_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO accounts (account_id, account_name, account_type, total_assets)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(account_id)
    .bind("Test Account")
    .bind("SPOT")
    .bind("10000")
    .execute(&*pool)
    .await
    .expect("Failed to insert account");

    // Verify data
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
        .fetch_one(&*pool)
        .await
        .expect("Failed to count accounts");
    assert_eq!(count, 1, "Should have 1 account");

    // Insert order (tests foreign key)
    let order_id = uuid::Uuid::new_v4();
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
    .bind("1")
    .bind("PENDING")
    .execute(&*pool)
    .await
    .expect("Failed to insert order");

    // Verify cascade delete works
    sqlx::query("DELETE FROM accounts WHERE account_id = $1")
        .bind(account_id)
        .execute(&*pool)
        .await
        .expect("Failed to delete account");

    let order_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE order_id = $1")
        .bind(order_id)
        .fetch_one(&*pool)
        .await
        .expect("Failed to count orders");
    assert_eq!(order_count, 0, "Order should be deleted via CASCADE");

    cleanup_test_db(&*pool).await.expect("Failed to cleanup");
}
