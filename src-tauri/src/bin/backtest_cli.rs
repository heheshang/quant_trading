//! Headless backtest CLI using the persisted `market_data` table.
//!
//! Reads env for DB + symbol + date range, fetches candles, and runs a
//! strategy backtest. Example:
//!   BACKTEST_SYMBOL=BTC-USDT BACKTEST_DAYS=30 BACKTEST_STRATEGY=TrendFollowing \
//!     cargo run -q -p quant-trading-system --bin backtest_cli
use chrono::Utc;
use data_layer::market_data_repo::MarketDataRepository;
use data_layer::MarketDataRecord;
use quant_common::types::{MarketData, StrategyParams, StrategyType};
use rust_decimal::Decimal;
use sqlx::postgres::PgPoolOptions;
use std::env;

fn to_market_data(r: &MarketDataRecord) -> MarketData {
    MarketData {
        symbol: r.instrument_id.clone(),
        timestamp: r.timestamp,
        open: r.open,
        high: r.high,
        low: r.low,
        close: r.close,
        volume: r.volume,
        turnover: Decimal::ZERO,
        open_interest: None,
        bid_prices: vec![],
        bid_volumes: vec![],
        ask_prices: vec![],
        ask_volumes: vec![],
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let host = env::var("DATABASE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = env::var("DATABASE_PORT").unwrap_or_else(|_| "15432".into());
    let user = env::var("DATABASE_USERNAME").unwrap_or_else(|_| "quant".into());
    let pass = env::var("DATABASE_PASSWORD").unwrap_or_else(|_| "quant_password".into());
    let db = env::var("DATABASE_NAME").unwrap_or_else(|_| "quant_trading".into());
    let url = format!("postgres://{user}:{pass}@{host}:{port}/{db}");

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await?;
    let repo = MarketDataRepository::new(pool);

    let symbol = env::var("BACKTEST_SYMBOL").unwrap_or_else(|_| "BTC-USDT".into());
    let days: i64 = env::var("BACKTEST_DAYS").unwrap_or_else(|_| "30".into()).parse()?;
    let timeframe = env::var("BACKTEST_TIMEFRAME").unwrap_or_else(|_| "1H".into());
    let strategy_type = env::var("BACKTEST_STRATEGY").unwrap_or_else(|_| "TrendFollowing".into());
    let initial: f64 = env::var("BACKTEST_CAPITAL").unwrap_or_else(|_| "10000".into()).parse()?;

    let end = Utc::now();
    let start = end - chrono::Duration::days(days);
    let records = repo
        .query_by_range(&symbol, &timeframe, start, end, Some(100_000))
        .await?;
    println!("fetched {} candles for {} ({days}d, {timeframe})", records.len(), symbol);

    if records.len() < 50 {
        println!("WARN: not enough candles (<50); backtest may be trivial");
    }
    let market_data: Vec<MarketData> = records.iter().map(to_market_data).collect();

    let registry = strategy_layer::registry::default_registry();
    let stype = StrategyType::from_type_name(&strategy_type)
        .ok_or_else(|| anyhow::anyhow!("unknown strategy: {strategy_type}"))?;
    let params = StrategyParams::builder("cli-backtest", "CLI Backtest", stype)
        .params(serde_json::json!({
            "lookback_period": 20,
            "entry_threshold": 2.0,
            "exit_threshold": 0.5,
        }))
        .symbols(vec![symbol.clone()])
        .max_position(Decimal::new(2000, 0))
        .max_daily_loss(Decimal::new(500, 0))
        .build();
    let strategy = registry.create(&strategy_type, params).await?;

    let mut engine = strategy_layer::BacktestEngine::new(
        Decimal::from_f64_retain(initial).ok_or_else(|| anyhow::anyhow!("bad capital"))?,
        Decimal::new(3, 4),  // 0.03% commission
        Decimal::new(1, 4),  // 0.01% slippage
    );
    let result = engine
        .run_with_options(&*strategy, market_data, Default::default())
        .await?;

    println!("=== BACKTEST RESULT ===");
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
