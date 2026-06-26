use chrono::{TimeDelta, Utc};
use data_layer::{MarketDataRepository, NewMarketDataRecord};
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

async fn cleanup(pool: &PgPool) -> Result<()> {
    let _ = sqlx::query("DROP TABLE IF EXISTS market_data CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(pool)
        .await;
    Ok(())
}

async fn setup_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| quant_common::Error::Database(format!("Migration failed: {}", e)))
}

#[tokio::test]
#[ignore]
async fn test_repo_insert_batch() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup(&*pool).await.expect("Failed to cleanup");

    setup_migrations(&*pool)
        .await
        .expect("Migrations should succeed");

    let repo = MarketDataRepository::new((*pool).clone());

    let now = Utc::now();
    let items = vec![
        NewMarketDataRecord {
            instrument_id: "BTC-USDT".to_string(),
            timeframe: "1h".to_string(),
            timestamp: now,
            open: Decimal::new(50000, 2),
            high: Decimal::new(51000, 2),
            low: Decimal::new(49000, 2),
            close: Decimal::new(50500, 2),
            volume: Decimal::new(1000, 0),
        },
        NewMarketDataRecord {
            instrument_id: "BTC-USDT".to_string(),
            timeframe: "1h".to_string(),
            timestamp: now + TimeDelta::try_hours(1).unwrap(),
            open: Decimal::new(50500, 2),
            high: Decimal::new(51500, 2),
            low: Decimal::new(50000, 2),
            close: Decimal::new(51000, 2),
            volume: Decimal::new(1500, 0),
        },
        NewMarketDataRecord {
            instrument_id: "ETH-USDT".to_string(),
            timeframe: "15m".to_string(),
            timestamp: now,
            open: Decimal::new(3000, 2),
            high: Decimal::new(3100, 2),
            low: Decimal::new(2950, 2),
            close: Decimal::new(3050, 2),
            volume: Decimal::new(5000, 0),
        },
    ];

    let inserted = repo
        .insert_batch(&items)
        .await
        .expect("Batch insert should succeed");
    assert_eq!(inserted, 3, "Should insert 3 records");

    cleanup(&*pool).await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn test_repo_insert_batch_empty() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup(&*pool).await.expect("Failed to cleanup");

    let repo = MarketDataRepository::new((*pool).clone());

    let inserted = repo
        .insert_batch(&[])
        .await
        .expect("Empty batch should succeed");
    assert_eq!(inserted, 0, "Empty batch should return 0");

    cleanup(&*pool).await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn test_repo_insert_duplicate() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup(&*pool).await.expect("Failed to cleanup");

    setup_migrations(&*pool)
        .await
        .expect("Migrations should succeed");

    let repo = MarketDataRepository::new((*pool).clone());

    let now = Utc::now();
    let item = NewMarketDataRecord {
        instrument_id: "BTC-USDT".to_string(),
        timeframe: "1h".to_string(),
        timestamp: now,
        open: Decimal::new(50000, 2),
        high: Decimal::new(51000, 2),
        low: Decimal::new(49000, 2),
        close: Decimal::new(50500, 2),
        volume: Decimal::new(1000, 0),
    };

    let first = repo
        .insert_batch(&[item.clone()])
        .await
        .expect("First insert should succeed");
    assert_eq!(first, 1, "First insert should return 1");

    let second = repo
        .insert_batch(&[item.clone()])
        .await
        .expect("Duplicate insert should succeed");
    assert_eq!(second, 0, "ON CONFLICT DO NOTHING should skip duplicate");

    cleanup(&*pool).await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn test_repo_query_by_range() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup(&*pool).await.expect("Failed to cleanup");

    setup_migrations(&*pool)
        .await
        .expect("Migrations should succeed");

    let repo = MarketDataRepository::new((*pool).clone());

    let base = Utc::now();
    let mut items = Vec::new();
    for i in 0..10 {
        items.push(NewMarketDataRecord {
            instrument_id: "BTC-USDT".to_string(),
            timeframe: "1h".to_string(),
            timestamp: base + TimeDelta::try_hours(i).unwrap(),
            open: Decimal::new(50000 + i as i64 * 100, 2),
            high: Decimal::new(51000 + i as i64 * 100, 2),
            low: Decimal::new(49000 + i as i64 * 100, 2),
            close: Decimal::new(50500 + i as i64 * 100, 2),
            volume: Decimal::new(1000, 0),
        });
    }

    repo.insert_batch(&items)
        .await
        .expect("Batch insert should succeed");

    let from = base;
    let to = base + TimeDelta::try_hours(10).unwrap();

    let results = repo
        .query_by_range("BTC-USDT", "1h", from, to, None)
        .await
        .expect("Query should succeed");
    assert_eq!(results.len(), 10, "Should return all 10 records");

    let first = &results[0];
    assert_eq!(first.instrument_id, "BTC-USDT");
    assert_eq!(first.timeframe, "1h");

    cleanup(&*pool).await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn test_repo_query_empty_range() {
    let pool = get_test_pool().await.expect("Failed to get test pool");
    cleanup(&*pool).await.expect("Failed to cleanup");

    setup_migrations(&*pool)
        .await
        .expect("Migrations should succeed");

    let repo = MarketDataRepository::new((*pool).clone());

    let now = Utc::now();
    let from = now - TimeDelta::try_hours(48).unwrap();
    let to = now - TimeDelta::try_hours(24).unwrap();

    let results = repo
        .query_by_range("NONEXISTENT", "1h", from, to, None)
        .await
        .expect("Query should succeed");
    assert!(results.is_empty(), "Should return empty for no data");

    cleanup(&*pool).await.expect("Failed to cleanup");
}
