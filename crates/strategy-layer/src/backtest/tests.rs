//! BacktestEngine 单元测试。

use super::*;
use crate::strategy::MeanReversionStrategy;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

fn make_market_data(timestamp: DateTime<Utc>, close: Decimal, symbol: &str) -> MarketData {
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

fn make_bar(timestamp: DateTime<Utc>, open: Decimal, close: Decimal) -> MarketData {
    MarketData {
        timestamp,
        symbol: "BTC-USDT".to_string(),
        open,
        high: open.max(close),
        low: open.min(close),
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

fn make_order(
    symbol: &str,
    side: quant_common::types::OrderSide,
    price: Decimal,
    quantity: Decimal,
) -> Order {
    Order {
        order_id: 0,
        strategy_id: "test".to_string(),
        symbol: symbol.to_string(),
        order_type: quant_common::types::OrderType::Limit,
        side,
        price: Some(price),
        quantity,
        filled_quantity: Decimal::ZERO,
        status: quant_common::types::OrderStatus::Pending,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        commission: Decimal::ZERO,
        slippage: Decimal::ZERO,
        exchange: "paper".to_string(),
    }
}

#[tokio::test]
async fn test_empty_market_data_returns_error() {
    let mut engine = BacktestEngine::new(
        Decimal::from(10000),
        Decimal::from_f64(0.001).unwrap(),
        Decimal::from_f64(0.0001).unwrap(),
    );
    let strategy = MeanReversionStrategy::new();
    let result = engine
        .run_with_options(&strategy, vec![], BacktestOptions::default())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_single_buy_and_sell_produces_profit_loss_ratio() {
    let now = Utc::now();
    let data = vec![
        make_market_data(now, Decimal::from(100), "BTC/USDT"),
        make_market_data(
            now + chrono::Duration::hours(1),
            Decimal::from(100),
            "BTC/USDT",
        ),
    ];

    let mut engine = BacktestEngine::new(Decimal::from(10000), Decimal::ZERO, Decimal::ZERO);

    // Buy at 100, quantity = 10
    engine
        .execute_order(
            make_order(
                "BTC/USDT",
                quant_common::types::OrderSide::Buy,
                Decimal::from(100),
                Decimal::from(10),
            ),
            &data,
        )
        .unwrap();

    // Sell at 110, profit = (110 - 100) * 10 = 100
    engine
        .execute_order(
            make_order(
                "BTC/USDT",
                quant_common::types::OrderSide::Sell,
                Decimal::from(110),
                Decimal::from(10),
            ),
            &data,
        )
        .unwrap();

    assert_eq!(engine.winning_trades, 1);
    assert_eq!(engine.total_profit, Decimal::from(100));
    assert_eq!(engine.total_loss, Decimal::ZERO);
}

#[tokio::test]
async fn test_multiple_buys_weighted_avg_price() {
    let now = Utc::now();
    let data = vec![
        make_market_data(now, Decimal::from(100), "BTC/USDT"),
        make_market_data(
            now + chrono::Duration::hours(1),
            Decimal::from(200),
            "BTC/USDT",
        ),
    ];

    let mut engine = BacktestEngine::new(Decimal::from(10000), Decimal::ZERO, Decimal::ZERO);

    // First buy: 10 units at 100
    engine
        .execute_order(
            make_order(
                "BTC/USDT",
                quant_common::types::OrderSide::Buy,
                Decimal::from(100),
                Decimal::from(10),
            ),
            &data,
        )
        .unwrap();

    // Second buy: 10 units at 200
    engine
        .execute_order(
            make_order(
                "BTC/USDT",
                quant_common::types::OrderSide::Buy,
                Decimal::from(200),
                Decimal::from(10),
            ),
            &data,
        )
        .unwrap();

    // Weighted avg = (10*100 + 10*200) / 20 = 150
    let btc_pos = engine.positions.get("BTC/USDT").unwrap();
    assert_eq!(btc_pos.avg_price, Decimal::from(150));
    assert_eq!(btc_pos.quantity, Decimal::from(20));
}

#[tokio::test]
async fn test_profit_loss_ratio_calculation() {
    let now = Utc::now();
    let data = vec![
        make_market_data(now, Decimal::from(100), "BTC/USDT"),
        make_market_data(
            now + chrono::Duration::hours(1),
            Decimal::from(100),
            "BTC/USDT",
        ),
    ];

    let mut engine = BacktestEngine::new(Decimal::from(10000), Decimal::ZERO, Decimal::ZERO);

    // Buy at 100
    engine
        .execute_order(
            make_order(
                "BTC/USDT",
                quant_common::types::OrderSide::Buy,
                Decimal::from(100),
                Decimal::from(10),
            ),
            &data,
        )
        .unwrap();

    // Win trade: sell at 150, profit = 500
    engine
        .execute_order(
            make_order(
                "BTC/USDT",
                quant_common::types::OrderSide::Sell,
                Decimal::from(150),
                Decimal::from(5),
            ),
            &data,
        )
        .unwrap();

    // Loss trade: sell at 50, loss = 250
    engine
        .execute_order(
            make_order(
                "BTC/USDT",
                quant_common::types::OrderSide::Sell,
                Decimal::from(50),
                Decimal::from(5),
            ),
            &data,
        )
        .unwrap();

    // total_profit = 250, total_loss = 250
    // profit_loss_ratio = 250 / 250 = 1.0
    assert_eq!(engine.total_profit, Decimal::from(250));
    assert_eq!(engine.total_loss, Decimal::from(250));
}

/// 验证前视偏差修复：bar t 收盘生成的信号应在 bar t+1 开盘价成交。
#[tokio::test]
async fn test_backtest_executes_at_next_bar_open() {
    use crate::strategy::{Strategy, StrategyContext};
    use async_trait::async_trait;
    use quant_common::types::{OrderSide, OrderStatus, OrderType, StrategyParams, StrategyType};
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

    struct BuyOnceStrategy {
        params: StrategyParams,
        signaled: AtomicBool,
    }

    #[async_trait]
    impl Strategy for BuyOnceStrategy {
        async fn initialize(&mut self, params: StrategyParams) -> Result<()> {
            self.params = params;
            Ok(())
        }

        async fn generate_signals(&self, context: &StrategyContext) -> Result<Vec<Order>> {
            if self.signaled.swap(true, AtomicOrdering::SeqCst) {
                return Ok(vec![]);
            }
            let price = context.market_data[0].close;
            Ok(vec![Order {
                order_id: 0,
                strategy_id: "buy_once".to_string(),
                symbol: "BTC-USDT".to_string(),
                order_type: OrderType::Limit,
                side: OrderSide::Buy,
                price: Some(price),
                quantity: Decimal::ONE,
                filled_quantity: Decimal::ZERO,
                status: OrderStatus::Pending,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                commission: Decimal::ZERO,
                slippage: Decimal::ZERO,
                exchange: "paper".to_string(),
            }])
        }

        fn name(&self) -> &str {
            &self.params.strategy_name
        }
        fn params(&self) -> &StrategyParams {
            &self.params
        }
        fn params_mut(&mut self) -> &mut StrategyParams {
            &mut self.params
        }
    }

    let now = Utc::now();
    let params = StrategyParams::builder(
        "buy_once".to_string(),
        "BuyOnce".to_string(),
        StrategyType::MeanReversion,
    )
    .params(serde_json::json!({}))
    .max_position(Decimal::from(10000))
    .max_daily_loss(Decimal::from(1000))
    .symbols(vec!["BTC-USDT".to_string()])
    .build();
    let strategy = BuyOnceStrategy {
        params,
        signaled: AtomicBool::new(false),
    };

    // 三根 K 线，open 与 close 不同：bar0 收盘 110，bar1 开盘 111
    let data = vec![
        make_bar(now, Decimal::from(100), Decimal::from(110)),
        make_bar(
            now + chrono::Duration::hours(1),
            Decimal::from(111),
            Decimal::from(120),
        ),
        make_bar(
            now + chrono::Duration::hours(2),
            Decimal::from(121),
            Decimal::from(130),
        ),
    ];

    let mut engine = BacktestEngine::new(Decimal::from(10000), Decimal::ZERO, Decimal::ZERO);
    let result = engine
        .run_with_options(&strategy, data, BacktestOptions::default())
        .await
        .unwrap();

    // 买入应在 bar1 开盘价 111 成交，而非 bar0 收盘价 110（前视偏差修复验证）
    let pos = engine
        .positions
        .get("BTC-USDT")
        .expect("position should exist");
    assert_eq!(pos.avg_price, Decimal::from(111));
    assert_eq!(pos.quantity, Decimal::ONE);
    assert_eq!(result.total_trades, 1);
}

struct RoundTripStrategy {
    params: quant_common::types::StrategyParams,
}

#[async_trait::async_trait]
impl Strategy for RoundTripStrategy {
    async fn initialize(&mut self, params: quant_common::types::StrategyParams) -> Result<()> {
        self.params = params;
        Ok(())
    }

    async fn generate_signals(
        &self,
        context: &crate::strategy::StrategyContext,
    ) -> Result<Vec<Order>> {
        use quant_common::types::OrderSide;
        let side = match context.market_data.len() {
            1 => OrderSide::Buy,
            2 => OrderSide::Sell,
            _ => return Ok(vec![]),
        };
        let bar = context.market_data.last().unwrap();
        Ok(vec![make_order(&bar.symbol, side, bar.close, Decimal::ONE)])
    }

    fn name(&self) -> &str {
        &self.params.strategy_name
    }

    fn params(&self) -> &quant_common::types::StrategyParams {
        &self.params
    }

    fn params_mut(&mut self) -> &mut quant_common::types::StrategyParams {
        &mut self.params
    }
}

async fn run_portfolio_round_trips(quotes: &[(&str, &[i64])]) -> BacktestResult {
    use quant_common::types::{StrategyParams, StrategyType};
    let params = StrategyParams::builder("round_trip", "RoundTrip", StrategyType::MeanReversion)
        .symbols(
            quotes
                .iter()
                .map(|(symbol, _)| symbol.to_string())
                .collect(),
        )
        .build();
    let start = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    let mut data = Vec::new();
    for (symbol, prices) in quotes {
        for (index, price) in prices.iter().enumerate() {
            data.push(make_market_data(
                start + chrono::Duration::hours(index as i64),
                Decimal::from(*price),
                symbol,
            ));
        }
    }
    run_backtest_multi(
        vec![(Box::new(RoundTripStrategy { params }), "round_trip".into())],
        data,
        Decimal::from(10000),
        Decimal::ZERO,
        Decimal::ZERO,
    )
    .await
    .unwrap()
    .results
    .remove(0)
    .1
}

#[tokio::test]
async fn portfolio_losing_sale_is_not_profit() {
    let result =
        run_portfolio_round_trips(&[("A", &[100, 100, 80]), ("B", &[100, 100, 100])]).await;
    assert_eq!(result.final_capital, Decimal::from(9980));
    assert_eq!(result.winning_trades, 0);
    assert_eq!(result.losing_trades, 1);
    assert_eq!(result.profit_loss_ratio, Decimal::ZERO);
}

#[tokio::test]
async fn portfolio_combines_realized_profits_and_losses_and_orders_trades() {
    let result =
        run_portfolio_round_trips(&[("A", &[100, 100, 140]), ("B", &[100, 100, 90])]).await;
    assert_eq!(result.final_capital, Decimal::from(10030));
    assert_eq!(result.winning_trades, 1);
    assert_eq!(result.losing_trades, 1);
    assert_eq!(result.profit_loss_ratio, Decimal::from(4));
    let start = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    assert_eq!(
        result
            .trades
            .iter()
            .map(|trade| trade.date)
            .collect::<Vec<_>>(),
        vec![
            start + chrono::Duration::hours(1),
            start + chrono::Duration::hours(1),
            start + chrono::Duration::hours(2),
            start + chrono::Duration::hours(2),
        ]
    );
}

#[tokio::test]
async fn portfolio_all_winners_keep_bounded_profit_ratio() {
    let result =
        run_portfolio_round_trips(&[("A", &[100, 100, 120]), ("B", &[100, 100, 110])]).await;
    assert_eq!(result.winning_trades, 2);
    assert_eq!(result.losing_trades, 0);
    assert_eq!(result.profit_loss_ratio, Decimal::from(100));
}

#[tokio::test]
async fn portfolio_without_fills_has_zero_profit_ratio() {
    let result = run_portfolio_round_trips(&[("A", &[100]), ("B", &[100])]).await;
    assert_eq!(result.total_trades, 0);
    assert!(result.trades.is_empty());
    assert_eq!(result.profit_loss_ratio, Decimal::ZERO);
}
