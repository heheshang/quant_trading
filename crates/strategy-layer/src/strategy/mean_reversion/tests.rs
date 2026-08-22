//! MeanReversionStrategy 单元测试。

use super::*;
use chrono::DateTime;
use quant_common::types::MarketData;

fn make_market_data(close: Decimal) -> MarketData {
    MarketData {
        timestamp: Utc::now(),
        symbol: "BTC/USDT".to_string(),
        open: close,
        high: close,
        low: close,
        close,
        volume: Decimal::from(1000),
        turnover: Decimal::ZERO,
        open_interest: None,
        bid_prices: vec![],
        bid_volumes: vec![],
        ask_prices: vec![],
        ask_volumes: vec![],
    }
}

fn make_full_market_data(timestamp: DateTime<Utc>, close: Decimal, symbol: &str) -> MarketData {
    MarketData {
        timestamp,
        symbol: symbol.to_string(),
        open: close,
        high: close,
        low: close,
        close,
        volume: Decimal::from(1000),
        turnover: Decimal::ZERO,
        open_interest: None,
        bid_prices: vec![],
        bid_volumes: vec![],
        ask_prices: vec![],
        ask_volumes: vec![],
    }
}

/// Build a regime-change data series:
///   phase 1: `stable` bars all at `stable_price`
///   phase 2: `trend` bars moving linearly from `stable_price` toward `extreme_price`
fn build_regime_series(
    stable: usize,
    trend: usize,
    stable_price: i64,
    extreme_price: i64,
    symbol: &str,
) -> Vec<MarketData> {
    let utc = Utc::now();
    let mut data = Vec::with_capacity(stable + trend);

    for i in 0..stable {
        data.push(make_full_market_data(
            utc + chrono::Duration::hours(i as i64),
            Decimal::from(stable_price),
            symbol,
        ));
    }

    for i in 0..trend {
        let t = (i + 1) as f64 / trend as f64;
        let close_f = stable_price as f64 + (extreme_price - stable_price) as f64 * t;
        let close = Decimal::from_f64(close_f).unwrap();
        data.push(make_full_market_data(
            utc + chrono::Duration::hours((stable + i) as i64),
            close,
            symbol,
        ));
    }

    data
}

#[tokio::test]
async fn test_insufficient_data_returns_empty_signals() {
    let strategy = MeanReversionStrategy::new();
    let context = StrategyContext {
        current_time: Utc::now(),
        positions: vec![],
        market_data: vec![make_market_data(Decimal::from(100))], // only 1 point, lookback=20
    };
    let orders = strategy.generate_signals(&context).await.unwrap();
    assert!(orders.is_empty());
}

#[tokio::test]
async fn test_rsi_oversold_triggers_buy() {
    let mut strategy = MeanReversionStrategy::new();
    let params = StrategyParams::builder(
        "test".to_string(),
        "Test".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({
        "lookback_period": 5,
        "entry_threshold": 0.5,
        "exit_threshold": 0.5,
    }))
    .max_position(Decimal::from(100000))
    .max_daily_loss(Decimal::from(5000))
    .build();
    strategy.initialize(params).await.unwrap();

    let data = build_regime_series(20, 15, 100, 1, "BTC/USDT");
    let context = StrategyContext {
        current_time: Utc::now(),
        positions: vec![],
        market_data: data,
    };
    let orders = strategy.generate_signals(&context).await.unwrap();
    assert!(
        orders.iter().any(|o| o.side == OrderSide::Buy),
        "Expected Buy signal: price crashed from 100 to 1 after stable period"
    );
}

#[tokio::test]
async fn test_rsi_overbought_triggers_sell() {
    let mut strategy = MeanReversionStrategy::new();
    let params = StrategyParams::builder(
        "test".to_string(),
        "Test".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({
        "lookback_period": 5,
        "entry_threshold": 0.5,
        "exit_threshold": 0.5,
    }))
    .max_position(Decimal::from(100000))
    .max_daily_loss(Decimal::from(5000))
    .build();
    strategy.initialize(params).await.unwrap();

    let data = build_regime_series(20, 15, 100, 199, "BTC/USDT");
    let context = StrategyContext {
        current_time: Utc::now(),
        positions: vec![],
        market_data: data,
    };
    let orders = strategy.generate_signals(&context).await.unwrap();
    assert!(
        orders.iter().any(|o| o.side == OrderSide::Sell),
        "Expected Sell signal: price surged from 100 to 199 after stable period"
    );
}

#[tokio::test]
async fn test_update_params_preserves_runtime_state() {
    let mut strategy = MeanReversionStrategy::new();
    let initial_params = StrategyParams::builder(
        "orig-001".to_string(),
        "Original".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({
        "lookback_period": 10,
        "entry_threshold": 1.5,
        "exit_threshold": 0.3,
    }))
    .max_position(Decimal::from(100000))
    .max_daily_loss(Decimal::from(5000))
    .build();
    strategy.initialize(initial_params).await.unwrap();

    assert_eq!(strategy.lookback_period, 10);
    assert!((strategy.entry_threshold - 1.5).abs() < f64::EPSILON);

    let new_params = StrategyParams::builder(
        "orig-001".to_string(),
        "Original".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({
        "lookback_period": 50,
        "entry_threshold": 4.0,
        "exit_threshold": 1.0,
    }))
    .max_position(Decimal::from(100000))
    .max_daily_loss(Decimal::from(5000))
    .build();
    strategy.update_params(new_params.clone()).await.unwrap();

    assert_eq!(strategy.params().strategy_id, "orig-001");
    assert_eq!(strategy.params().params, new_params.params);

    assert_eq!(
        strategy.lookback_period, 10,
        "update_params must not reset parsed lookback_period"
    );
    assert!(
        (strategy.entry_threshold - 1.5).abs() < f64::EPSILON,
        "update_params must not reset parsed entry_threshold"
    );
    assert!(
        (strategy.exit_threshold - 0.3).abs() < f64::EPSILON,
        "update_params must not reset parsed exit_threshold"
    );
}

#[tokio::test]
async fn test_reinitialize_resets_state_with_new_params() {
    let mut strategy = MeanReversionStrategy::new();
    let initial_params = StrategyParams::builder(
        "orig-002".to_string(),
        "Original".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({
        "lookback_period": 10,
        "entry_threshold": 1.5,
        "exit_threshold": 0.3,
    }))
    .max_position(Decimal::from(100000))
    .max_daily_loss(Decimal::from(5000))
    .build();
    strategy.initialize(initial_params).await.unwrap();
    assert_eq!(strategy.lookback_period, 10);

    let new_params = StrategyParams::builder(
        "orig-002".to_string(),
        "Original".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({
        "lookback_period": 50,
        "entry_threshold": 4.0,
        "exit_threshold": 1.0,
    }))
    .max_position(Decimal::from(100000))
    .max_daily_loss(Decimal::from(5000))
    .build();
    strategy.reinitialize(new_params).await.unwrap();

    assert_eq!(
        strategy.lookback_period, 50,
        "reinitialize must re-parse lookback_period"
    );
    assert!((strategy.entry_threshold - 4.0).abs() < f64::EPSILON);
    assert!((strategy.exit_threshold - 1.0).abs() < f64::EPSILON);
}
