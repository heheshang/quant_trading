//! ClientInterface 单元测试。

use super::*;
use dotenv::dotenv;
use tracing::info;

#[tokio::test]
#[ignore = "requires real OKX API credentials and network; use for manual integration testing only"]
async fn test_okx_client_creation() {
    dotenv().ok();

    // Check if environment variables are set
    let api_key = match dotenv::var("OKX_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            info!("OKX_API_KEY not set, skipping test");
            return;
        }
    };

    let api_secret = match dotenv::var("OKX_API_SECRET") {
        Ok(secret) => secret,
        Err(_) => {
            info!("OKX_API_SECRET not set, skipping test");
            return;
        }
    };

    let passphrase = match dotenv::var("OKX_PASSPHRASE") {
        Ok(pass) => pass,
        Err(_) => {
            info!("OKX_PASSPHRASE not set, skipping test");
            return;
        }
    };

    let client = Client::new(api_key, api_secret, passphrase, OkxEnvironment::Demo);

    match client {
        Ok(client) => {
            info!("OKX client created successfully");
            match client.get_account_balance(Some("USDT")).await {
                Ok(balances) => {
                    info!("Account balance retrieved: {:?}", balances);
                }
                Err(e) => {
                    info!("Failed to get account balance: {}", e);
                }
            }
        }
        Err(e) => {
            info!("Failed to create OKX client: {}", e);
        }
    }
}

#[tokio::test]
#[ignore = "requires real OKX API credentials and network; use for manual integration testing only"]
async fn test_get_announcements() {
    dotenv().ok();
    let api_key = match dotenv::var("OKX_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            info!("OKX_API_KEY not set, skipping test");
            return;
        }
    };

    let api_secret = match dotenv::var("OKX_API_SECRET") {
        Ok(secret) => secret,
        Err(_) => {
            info!("OKX_API_SECRET not set, skipping test");
            return;
        }
    };

    let passphrase = match dotenv::var("OKX_PASSPHRASE") {
        Ok(pass) => pass,
        Err(_) => {
            info!("OKX_PASSPHRASE not set, skipping test");
            return;
        }
    };

    let client = Client::new(api_key, api_secret, passphrase, OkxEnvironment::Demo);

    let announcements = client.unwrap().get_announcements().await.unwrap();
    info!("{:?}", announcements);
}

// ──────────────────────────────────────────────
// Mock-based unit tests for ClientInterface
// ──────────────────────────────────────────────

use crate::mock_data::*;

// ── 1. get_account_balance ──

#[tokio::test]
async fn test_get_account_balance_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_account_balance()
        .withf(|ccy: &Option<&str>| *ccy == Some("BTC"))
        .returning(|_: Option<&str>| {
            Box::pin(async move { Ok(vec![mock_okx_balance("BTC", "1.5")]) })
        });

    let result = mock.get_account_balance(Some("BTC")).await;
    assert!(result.is_ok());
    let balances = result.unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].ccy, "BTC");
    assert_eq!(balances[0].eq, "1.5");
}

#[tokio::test]
async fn test_get_account_balance_multi() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_account_balance()
        .withf(|ccy: &Option<&str>| ccy.is_none())
        .returning(|_: Option<&str>| Box::pin(async move { Ok(mock_okx_balance_list()) }));

    let result = mock.get_account_balance(None).await;
    assert!(result.is_ok());
    let balances = result.unwrap();
    assert_eq!(balances.len(), 3);
    assert_eq!(balances[0].ccy, "BTC");
    assert_eq!(balances[1].ccy, "ETH");
    assert_eq!(balances[2].ccy, "USDT");
}

#[tokio::test]
async fn test_get_account_balance_large_number() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_account_balance()
        .withf(|ccy: &Option<&str>| *ccy == Some("BTC"))
        .returning(|_: Option<&str>| {
            Box::pin(async move { Ok(vec![mock_large_number_balance()]) })
        });

    let result = mock.get_account_balance(Some("BTC")).await;
    assert!(result.is_ok());
    // Value exceeds 2^53 = 9007199254740992
    assert_eq!(result.unwrap()[0].eq, "123456789012345678");
}

#[tokio::test]
async fn test_get_account_balance_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_account_balance()
        .withf(|ccy: &Option<&str>| *ccy == Some("INVALID"))
        .returning(|_: Option<&str>| {
            Box::pin(async move { Err(Error::Internal("test error".into())) })
        });

    let result = mock.get_account_balance(Some("INVALID")).await;
    assert!(result.is_err());
}

// ── 2. get_positions ──

#[tokio::test]
async fn test_get_positions_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_positions()
        .withf(|inst_id: &Option<&str>| *inst_id == Some("BTC-USDT"))
        .returning(|_: Option<&str>| {
            Box::pin(async move { Ok(vec![mock_okx_position("BTC-USDT", "1")]) })
        });

    let result = mock.get_positions(Some("BTC-USDT")).await;
    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].inst_id, "BTC-USDT");
    assert_eq!(positions[0].pos, "1");
}

#[tokio::test]
async fn test_get_positions_multi() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_positions()
        .withf(|inst_id: &Option<&str>| inst_id.is_none())
        .returning(|_: Option<&str>| Box::pin(async move { Ok(mock_okx_position_list()) }));

    let result = mock.get_positions(None).await;
    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0].inst_id, "BTC-USDT");
    assert_eq!(positions[1].inst_id, "ETH-USDT");
}

#[tokio::test]
async fn test_get_positions_empty() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_positions()
        .withf(|inst_id: &Option<&str>| *inst_id == Some("EMPTY"))
        .returning(|_: Option<&str>| Box::pin(async move { Ok(vec![]) }));

    let result = mock.get_positions(Some("EMPTY")).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_positions_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_positions()
        .withf(|inst_id: &Option<&str>| *inst_id == Some("INVALID"))
        .returning(|_: Option<&str>| {
            Box::pin(async move { Err(Error::Internal("test error".into())) })
        });

    let result = mock.get_positions(Some("INVALID")).await;
    assert!(result.is_err());
}

// ── 3. place_order ──

#[tokio::test]
async fn test_place_order_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_place_order()
        .withf(|req: &OkxPlaceOrderRequest| req.inst_id == "BTC-USDT" && req.side == "buy")
        .returning(|_| Box::pin(async move { Ok(mock_okx_order("BTC-USDT", "buy")) }));

    let req = mock_place_order_request("BTC-USDT", "buy", "1");
    let result = mock.place_order(req).await;
    assert!(result.is_ok());
    let order = result.unwrap();
    assert_eq!(order.inst_id, "BTC-USDT");
    assert_eq!(order.side, "buy");
    assert_eq!(order.state, "live");
}

#[tokio::test]
async fn test_place_order_limit() {
    let mut mock = MockClientInterface::new();
    mock.expect_place_order()
        .withf(|req: &OkxPlaceOrderRequest| req.ord_type == "limit" && req.inst_id == "BTC-USDT")
        .returning(|_| {
            let mut order = mock_filled_order("BTC-USDT", "sell");
            order.ord_type = "limit".to_string();
            Box::pin(async move { Ok(order) })
        });

    let req = mock_limit_order_request("BTC-USDT", "sell", "0.5", "46000");
    let result = mock.place_order(req).await;
    assert!(result.is_ok());
    let order = result.unwrap();
    assert_eq!(order.ord_type, "limit");
    assert_eq!(order.state, "filled");
}

#[tokio::test]
async fn test_place_order_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_place_order()
        .withf(|req: &OkxPlaceOrderRequest| req.inst_id == "INVALID")
        .returning(|_| Box::pin(async move { Err(Error::Internal("test error".into())) }));

    let req = mock_place_order_request("INVALID", "buy", "1");
    let result = mock.place_order(req).await;
    assert!(result.is_err());
}

// ── 4. cancel_order ──

#[tokio::test]
async fn test_cancel_order_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_cancel_order()
        .withf(|inst_id: &str, ord_id: &str| inst_id == "BTC-USDT" && ord_id == "123456789")
        .returning(|_: &str, _: &str| Box::pin(async move { Ok(()) }));

    let result = mock.cancel_order("BTC-USDT", "123456789").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cancel_order_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_cancel_order()
        .withf(|inst_id: &str, ord_id: &str| inst_id == "INVALID" && ord_id == "999")
        .returning(|_: &str, _: &str| {
            Box::pin(async move { Err(Error::Internal("test error".into())) })
        });

    let result = mock.cancel_order("INVALID", "999").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_order_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_order()
        .withf(|inst_id: &str, ord_id: &str| inst_id == "BTC-USDT" && ord_id == "123456789")
        .returning(|_, _| Box::pin(async move { Ok(mock_okx_order("BTC-USDT", "buy")) }));

    let result = mock.get_order("BTC-USDT", "123456789").await;
    assert!(result.is_ok());
    let order = result.unwrap();
    assert_eq!(order.ord_id, "123456789");
    assert_eq!(order.state, "live");
}

#[tokio::test]
async fn test_get_order_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_order()
        .withf(|inst_id: &str, ord_id: &str| inst_id == "INVALID" && ord_id == "999")
        .returning(|_, _| Box::pin(async move { Err(Error::NotFound("order missing".into())) }));

    let result = mock.get_order("INVALID", "999").await;
    assert!(result.is_err());
}

// ── 5. get_candles ──

#[tokio::test]
async fn test_get_candles_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_candles()
        .withf(|inst_id: &str, bar: &str, limit: &Option<u32>| {
            inst_id == "BTC-USDT" && bar == "1m" && *limit == Some(5)
        })
        .returning(|_: &str, _: &str, _: Option<u32>| {
            Box::pin(async move { Ok(mock_okx_candles(5)) })
        });

    let result = mock.get_candles("BTC-USDT", "1m", Some(5)).await;
    assert!(result.is_ok());
    let candles = result.unwrap();
    assert_eq!(candles.len(), 5);
    // Verify sequential timestamps
    let ts0: u64 = candles[0].ts.parse().unwrap();
    let ts1: u64 = candles[1].ts.parse().unwrap();
    assert_eq!(ts1 - ts0, 3600000);
}

#[tokio::test]
async fn test_get_candles_limit_zero() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_candles()
        .withf(|inst_id: &str, bar: &str, limit: &Option<u32>| {
            inst_id == "BTC-USDT" && bar == "1H" && *limit == Some(0)
        })
        .returning(|_: &str, _: &str, _: Option<u32>| Box::pin(async move { Ok(vec![]) }));

    let result = mock.get_candles("BTC-USDT", "1H", Some(0)).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_candles_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_candles()
        .withf(|inst_id: &str, _bar: &str, _limit: &Option<u32>| inst_id == "INVALID")
        .returning(|_: &str, _: &str, _: Option<u32>| {
            Box::pin(async move { Err(Error::Internal("test error".into())) })
        });

    let result = mock.get_candles("INVALID", "1m", Some(1)).await;
    assert!(result.is_err());
}

// ── 6. get_instruments ──

#[tokio::test]
async fn test_get_instruments_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_instruments()
        .withf(|inst_type: &str| inst_type == "SPOT")
        .returning(|_: &str| {
            Box::pin(async move {
                Ok(serde_json::json!([
                    {"instId": "BTC-USDT", "instType": "SPOT", "state": "live"}
                ]))
            })
        });

    let result = mock.get_instruments("SPOT").await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.is_array());
    assert_eq!(value[0]["instId"], "BTC-USDT");
}

#[tokio::test]
async fn test_get_instruments_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_instruments()
        .withf(|inst_type: &str| inst_type == "BAD")
        .returning(|_: &str| Box::pin(async move { Err(Error::Internal("test error".into())) }));

    let result = mock.get_instruments("BAD").await;
    assert!(result.is_err());
}

// ── 7. get_ticker ──

#[tokio::test]
async fn test_get_ticker_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_ticker()
        .withf(|inst_id: &str| inst_id == "BTC-USDT")
        .returning(|_: &str| Box::pin(async move { Ok(mock_okx_ticker("BTC-USDT")) }));

    let result = mock.get_ticker("BTC-USDT").await;
    assert!(result.is_ok());
    let ticker = result.unwrap();
    assert_eq!(ticker.inst_id, "BTC-USDT");
    assert_eq!(ticker.last, "45200.0");
    assert_eq!(ticker.ask_px, "45210.0");
    assert_eq!(ticker.bid_px, "45190.0");
}

#[tokio::test]
async fn test_get_ticker_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_ticker()
        .withf(|inst_id: &str| inst_id == "NONEXISTENT")
        .returning(|_: &str| Box::pin(async move { Err(Error::Internal("test error".into())) }));

    let result = mock.get_ticker("NONEXISTENT").await;
    assert!(result.is_err());
}

// ── 8. get_funding_rate ──

#[tokio::test]
async fn test_get_funding_rate_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_funding_rate()
        .withf(|inst_id: &str| inst_id == "BTC-USDT-SWAP")
        .returning(|_: &str| Box::pin(async move { Ok(mock_okx_funding_rate("BTC-USDT-SWAP")) }));

    let result = mock.get_funding_rate("BTC-USDT-SWAP").await;
    assert!(result.is_ok());
    let rate = result.unwrap();
    assert_eq!(rate.inst_id, "BTC-USDT-SWAP");
    assert_eq!(rate.funding_rate, "0.0001");
    assert_eq!(rate.inst_type, "SWAP");
}

#[tokio::test]
async fn test_get_funding_rate_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_funding_rate()
        .withf(|inst_id: &str| inst_id == "INVALID")
        .returning(|_: &str| Box::pin(async move { Err(Error::Internal("test error".into())) }));

    let result = mock.get_funding_rate("INVALID").await;
    assert!(result.is_err());
}

// ── 9. get_mark_price ──

#[tokio::test]
async fn test_get_mark_price_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_mark_price()
        .withf(|inst_id: &str| inst_id == "BTC-USDT")
        .returning(|_: &str| Box::pin(async move { Ok(mock_okx_mark_price("BTC-USDT")) }));

    let result = mock.get_mark_price("BTC-USDT").await;
    assert!(result.is_ok());
    let mp = result.unwrap();
    assert_eq!(mp.inst_id, "BTC-USDT");
    assert_eq!(mp.mark_px, "45200.0");
}

#[tokio::test]
async fn test_get_mark_price_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_mark_price()
        .withf(|inst_id: &str| inst_id == "INVALID")
        .returning(|_: &str| Box::pin(async move { Err(Error::Internal("test error".into())) }));

    let result = mock.get_mark_price("INVALID").await;
    assert!(result.is_err());
}

// ── 10. get_index_price ──

#[tokio::test]
async fn test_get_index_price_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_index_price()
        .withf(|inst_id: &str| inst_id == "BTC-USDT")
        .returning(|_: &str| Box::pin(async move { Ok(mock_okx_index_price("BTC-USDT")) }));

    let result = mock.get_index_price("BTC-USDT").await;
    assert!(result.is_ok());
    let ip = result.unwrap();
    assert_eq!(ip.inst_id, "BTC-USDT");
    assert_eq!(ip.idx_px, "45205.0");
}

#[tokio::test]
async fn test_get_index_price_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_index_price()
        .withf(|inst_id: &str| inst_id == "INVALID")
        .returning(|_: &str| Box::pin(async move { Err(Error::Internal("test error".into())) }));

    let result = mock.get_index_price("INVALID").await;
    assert!(result.is_err());
}

// ── 11. get_open_interest ──

#[tokio::test]
async fn test_get_open_interest_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_open_interest()
        .withf(|inst_id: &str| inst_id == "BTC-USDT")
        .returning(|_: &str| Box::pin(async move { Ok(mock_okx_open_interest("BTC-USDT")) }));

    let result = mock.get_open_interest("BTC-USDT").await;
    assert!(result.is_ok());
    let oi = result.unwrap();
    assert_eq!(oi.inst_id, "BTC-USDT");
    assert_eq!(oi.oi, "50000");
}

#[tokio::test]
async fn test_get_open_interest_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_open_interest()
        .withf(|inst_id: &str| inst_id == "INVALID")
        .returning(|_: &str| Box::pin(async move { Err(Error::Internal("test error".into())) }));

    let result = mock.get_open_interest("INVALID").await;
    assert!(result.is_err());
}

// ── 12. get_trades ──

#[tokio::test]
async fn test_get_trades_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_trades()
        .withf(|inst_id: &str, limit: &Option<u32>| inst_id == "BTC-USDT" && *limit == Some(10))
        .returning(|_: &str, _: Option<u32>| {
            Box::pin(
                async move { Ok(vec![mock_okx_trade("BTC-USDT"), mock_okx_trade("BTC-USDT")]) },
            )
        });

    let result = mock.get_trades("BTC-USDT", Some(10)).await;
    assert!(result.is_ok());
    let trades = result.unwrap();
    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].inst_id, "BTC-USDT");
}

#[tokio::test]
async fn test_get_trades_limit_boundary() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_trades()
        .withf(|inst_id: &str, limit: &Option<u32>| inst_id == "BTC-USDT" && *limit == Some(0))
        .returning(|_: &str, _: Option<u32>| Box::pin(async move { Ok(vec![]) }));

    let result = mock.get_trades("BTC-USDT", Some(0)).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_trades_limit_max() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_trades()
        .withf(|inst_id: &str, limit: &Option<u32>| inst_id == "BTC-USDT" && *limit == Some(500))
        .returning(|_: &str, _: Option<u32>| {
            Box::pin(async move { Ok(vec![mock_okx_trade("BTC-USDT")]) })
        });

    let result = mock.get_trades("BTC-USDT", Some(500)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[tokio::test]
async fn test_get_trades_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_trades()
        .withf(|inst_id: &str, _limit: &Option<u32>| inst_id == "INVALID")
        .returning(|_: &str, _: Option<u32>| {
            Box::pin(async move { Err(Error::Internal("test error".into())) })
        });

    let result = mock.get_trades("INVALID", Some(10)).await;
    assert!(result.is_err());
}

// ── 13. get_order_book ──

#[tokio::test]
async fn test_get_order_book_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_order_book()
        .withf(|inst_id: &str, sz: &Option<u32>| inst_id == "BTC-USDT" && *sz == Some(5))
        .returning(|_: &str, _: Option<u32>| Box::pin(async move { Ok(mock_okx_order_book()) }));

    let result = mock.get_order_book("BTC-USDT", Some(5)).await;
    assert!(result.is_ok());
    let ob = result.unwrap();
    assert_eq!(ob.asks.len(), 3);
    assert_eq!(ob.bids.len(), 3);
    assert_eq!(ob.asks[0][0], "45210.0");
}

#[tokio::test]
async fn test_get_order_book_sz_zero() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_order_book()
        .withf(|inst_id: &str, sz: &Option<u32>| inst_id == "BTC-USDT" && *sz == Some(0))
        .returning(|_: &str, _: Option<u32>| Box::pin(async move { Ok(mock_empty_order_book()) }));

    let result = mock.get_order_book("BTC-USDT", Some(0)).await;
    assert!(result.is_ok());
    let ob = result.unwrap();
    assert!(ob.asks.is_empty());
    assert!(ob.bids.is_empty());
}

#[tokio::test]
async fn test_get_order_book_sz_max() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_order_book()
        .withf(|inst_id: &str, sz: &Option<u32>| inst_id == "BTC-USDT" && *sz == Some(400))
        .returning(|_: &str, _: Option<u32>| {
            Box::pin(async move { Ok(mock_single_level_order_book()) })
        });

    let result = mock.get_order_book("BTC-USDT", Some(400)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().asks.len(), 1);
}

#[tokio::test]
async fn test_get_order_book_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_order_book()
        .withf(|inst_id: &str, _sz: &Option<u32>| inst_id == "INVALID")
        .returning(|_: &str, _: Option<u32>| {
            Box::pin(async move { Err(Error::Internal("test error".into())) })
        });

    let result = mock.get_order_book("INVALID", Some(5)).await;
    assert!(result.is_err());
}

// ── 14. get_announcements ──

#[tokio::test]
async fn test_get_announcements_success() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_announcements()
        .returning(|| Box::pin(async move { Ok(vec![mock_announcement_page()]) }));

    let result = mock.get_announcements().await;
    assert!(result.is_ok());
    let pages = result.unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].total_page, "1");
    assert_eq!(pages[0].details.len(), 2);
}

#[tokio::test]
async fn test_get_announcements_error() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_announcements()
        .returning(|| Box::pin(async move { Err(Error::Internal("API unavailable".into())) }));

    let result = mock.get_announcements().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_announcements_multi_page() {
    let mut mock = MockClientInterface::new();
    mock.expect_get_announcements().returning(|| {
        Box::pin(async move { Ok(vec![mock_announcement_page(), mock_announcement_page()]) })
    });

    let result = mock.get_announcements().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}
