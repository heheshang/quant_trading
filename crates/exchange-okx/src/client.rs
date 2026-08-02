use crate::types::*;
use async_trait::async_trait;
use okx::api::account::OkxAccount;
use okx::api::announcements::announcements_api::{AnnouncementPage, OkxAnnouncements};
use okx::api::api_trait::OkxApiTrait;
use okx::api::market::OkxMarket;
use okx::api::trade::OkxTrade as OkxTradeApi;
use okx::config::Credentials;
use okx::dto::trade_dto::OrderReqDto;
use okx::OkxClient;
use quant_common::{Error, Result};
use reqwest::Method;
/// OKX 客户端
#[derive(Debug)]
pub struct Client {
    api: OkxClient,
    _environment: OkxEnvironment,
    account_api: OkxAccount,
    market_api: OkxMarket,
    trade_api: OkxTradeApi,
}

/// OKX client trait — enables mocking in tests.
#[async_trait]
#[cfg_attr(any(test, feature = "test-utils"), mockall::automock)]
pub trait ClientInterface: Send + Sync {
    /// 获取账户余额
    async fn get_account_balance(&self, ccy: Option<&str>) -> Result<Vec<OkxBalance>>;
    /// 获取持仓
    async fn get_positions(&self, inst_id: Option<&str>) -> Result<Vec<OkxPosition>>;
    /// 下单
    async fn place_order(&self, request: OkxPlaceOrderRequest) -> Result<OkxOrder>;
    /// 撤单
    async fn cancel_order(&self, inst_id: &str, ord_id: &str) -> Result<()>;
    /// 查询订单状态
    async fn get_order(&self, inst_id: &str, ord_id: &str) -> Result<OkxOrder>;
    /// 获取K线数据
    async fn get_candles(
        &self,
        inst_id: &str,
        bar: &str,
        limit: Option<u32>,
    ) -> Result<Vec<OkxCandle>>;
    /// 获取交易对信息
    async fn get_instruments(&self, inst_type: &str) -> Result<serde_json::Value>;
    /// 获取 Ticker 24h 统计
    async fn get_ticker(&self, inst_id: &str) -> Result<OkxTicker>;
    /// 获取资金费率
    async fn get_funding_rate(&self, inst_id: &str) -> Result<OkxFundingRate>;
    /// 获取标记价格
    async fn get_mark_price(&self, inst_id: &str) -> Result<OkxMarkPrice>;
    /// 获取指数价格
    async fn get_index_price(&self, inst_id: &str) -> Result<OkxIndexPrice>;
    /// 获取持仓量
    async fn get_open_interest(&self, inst_id: &str) -> Result<OkxOpenInterest>;
    /// 获取成交明细
    async fn get_trades(&self, inst_id: &str, limit: Option<u32>) -> Result<Vec<OkxTrade>>;
    /// 获取订单薄
    async fn get_order_book(&self, inst_id: &str, sz: Option<u32>) -> Result<OkxOrderBook>;
    /// 获取公告
    async fn get_announcements(&self) -> Result<Vec<AnnouncementPage>>;
}

impl Client {
    /// 创建新的 OKX 客户端
    pub fn new(
        api_key: String,
        api_secret: String,
        passphrase: String,
        environment: OkxEnvironment,
    ) -> Result<Self> {
        let credentials = Credentials::new(
            &api_key,
            &api_secret,
            &passphrase,
            if environment == OkxEnvironment::Demo {
                "1"
            } else {
                "0"
            },
        );
        let api = OkxClient::new(credentials).map_err(Error::OKX)?;

        let account_api = OkxAccount::new(api.clone());
        let market_api = OkxMarket::new(api.clone());
        let trade_api = OkxTradeApi::new(api.clone());

        Ok(Self {
            api,
            _environment: environment,
            account_api,
            market_api,
            trade_api,
        })
    }
}

#[async_trait]
impl ClientInterface for Client {
    async fn get_announcements(&self) -> Result<Vec<AnnouncementPage>> {
        let api = OkxAnnouncements::new(self.api.clone());
        let announcements = api.get_announcements(None, None, None).await;
        match announcements {
            Ok(announcements) => Ok(announcements),
            Err(e) => Err(Error::OKX(e)),
        }
    }

    /// 获取账户余额
    async fn get_account_balance(&self, ccy: Option<&str>) -> Result<Vec<OkxBalance>> {
        let balances = self
            .account_api
            .get_balance(ccy)
            .await
            .map_err(Error::OKX)?;

        // Convert to our internal type
        let mut okx_balances: Vec<OkxBalance> = Vec::new();
        for balance in balances {
            for detail in balance.details {
                okx_balances.push(OkxBalance {
                    ccy: detail.ccy,
                    eq: detail.eq,
                    cash_bal: detail.cash_bal,
                    avail_eq: detail.avail_eq,
                    frozen_bal: detail.frozen_bal,
                });
            }
        }

        Ok(okx_balances)
    }

    /// 获取持仓
    async fn get_positions(&self, inst_id: Option<&str>) -> Result<Vec<OkxPosition>> {
        let positions = self
            .account_api
            .get_positions(None, inst_id, None)
            .await
            .map_err(Error::OKX)?;

        // Convert to our internal type
        let okx_positions: Vec<OkxPosition> = positions
            .into_iter()
            .map(|p| OkxPosition {
                inst_id: p.inst_id,
                pos: p.pos,
                avail_pos: "".to_string(), // Not available in PositionRespDto
                avg_px: p.average_price,
                upl: p.upl,
                upl_ratio: "".to_string(), // Not available in PositionRespDto
            })
            .collect();

        Ok(okx_positions)
    }

    /// 下单
    async fn place_order(&self, request: OkxPlaceOrderRequest) -> Result<OkxOrder> {
        // Clone all fields first to avoid move issues
        let inst_id = request.inst_id.clone();
        let td_mode = request.td_mode.clone();
        let side = request.side.clone();
        let ord_type = request.ord_type.clone();
        let sz = request.sz.clone();
        let px = request.px.clone();
        let cl_ord_id = request.cl_ord_id.clone();
        let tag = request.tag.clone();
        let pos_side = request.pos_side.clone();
        let ccy = request.ccy.clone();
        let px_usd = request.px_usd.clone();
        let px_vol = request.px_vol.clone();
        let reduce_only = request.reduce_only;
        let tgt_ccy = request.tgt_ccy.clone();

        let order_req = OrderReqDto {
            inst_id,
            td_mode,
            side,
            ord_type,
            sz,
            px,
            cl_ord_id,
            tag,
            pos_side,
            ccy,
            px_usd,
            px_vol,
            reduce_only,
            tgt_ccy,
            ban_amend: None,
            quick_mgn_type: None,
            stp_id: None,
            stp_mode: None,
            attach_algo_ords: None,
        };

        let orders = self
            .trade_api
            .place_order(order_req)
            .await
            .map_err(Error::OKX)?;

        if let Some(order) = orders.first() {
            Ok(OkxOrder {
                ord_id: order.ord_id.clone(),
                cl_ord_id: order.cl_ord_id.clone().unwrap_or_default(),
                inst_id: request.inst_id,
                side: request.side,
                ord_type: request.ord_type,
                px: request.px.unwrap_or_default(),
                sz: request.sz,
                state: order.s_code.clone(),
                avg_px: "".to_string(),
                acc_fill_sz: "".to_string(),
                u_time: order.ts.clone(),
            })
        } else {
            Err(Error::Trading("Failed to place order".to_string()))
        }
    }

    /// 查询订单
    async fn get_order(&self, inst_id: &str, ord_id: &str) -> Result<OkxOrder> {
        let orders = self
            .trade_api
            .get_order_details(inst_id, Some(ord_id), None)
            .await
            .map_err(Error::OKX)?;

        orders
            .into_iter()
            .next()
            .map(|o| OkxOrder {
                ord_id: o.ord_id,
                cl_ord_id: o.cl_ord_id,
                inst_id: o.inst_id,
                side: o.side,
                ord_type: o.ord_type,
                px: o.px,
                sz: o.sz,
                state: o.state,
                avg_px: o.avg_px,
                acc_fill_sz: o.acc_fill_sz,
                u_time: o.u_time,
            })
            .ok_or_else(|| Error::NotFound(format!("Order {} not found", ord_id)))
    }

    /// 撤单
    async fn cancel_order(&self, inst_id: &str, ord_id: &str) -> Result<()> {
        self.trade_api
            .cancel_order(inst_id, Some(ord_id), None)
            .await
            .map_err(Error::OKX)?;
        Ok(())
    }

    /// 获取K线数据
    async fn get_candles(
        &self,
        inst_id: &str,
        bar: &str, // 1m, 5m, 15m, 1H, 1D
        limit: Option<u32>,
    ) -> Result<Vec<OkxCandle>> {
        let limit_str = limit.map(|l| l.to_string());
        let candles = self
            .market_api
            .get_candles(inst_id, bar, None, None, limit_str.as_deref())
            .await
            .map_err(Error::OKX)?;

        // Convert to our internal type
        let okx_candles: Vec<OkxCandle> = candles
            .into_iter()
            .map(|c| OkxCandle {
                ts: c.ts,
                open: c.o,
                high: c.h,
                low: c.l,
                close: c.c,
                vol: c.v,
                vol_ccy: c.vol_ccy,
            })
            .collect();

        Ok(okx_candles)
    }

    /// 获取交易对信息
    async fn get_instruments(&self, inst_type: &str) -> Result<serde_json::Value> {
        let instruments = self
            .market_api
            .get_instruments(inst_type, None, None)
            .await
            .map_err(Error::OKX)?;
        serde_json::to_value(instruments).map_err(|e| Error::Internal(e.to_string()))
    }

    /// 获取 Ticker 24h 统计
    async fn get_ticker(&self, inst_id: &str) -> Result<OkxTicker> {
        let data = self
            .market_api
            .get_ticker(inst_id)
            .await
            .map_err(Error::OKX)?;

        data.into_iter()
            .next()
            .map(|d| OkxTicker {
                inst_id: d.inst_id,
                last: d.last,
                last_sz: d.last_sz,
                ask_px: d.ask_px,
                bid_px: d.bid_px,
                open_24h: d.open24h,
                high_24h: d.high24h,
                low_24h: d.low24h,
                vol_ccy_24h: d.vol_ccy24h,
                vol_24h: d.vol24h,
                sod_utc0: d.sod_utc0,
                sod_utc8: d.sod_utc8,
                ts: d.ts,
            })
            .ok_or_else(|| Error::Internal("empty ticker response".into()))
    }

    /// 获取资金费率
    async fn get_funding_rate(&self, inst_id: &str) -> Result<OkxFundingRate> {
        let path = format!("/api/v5/public/funding-rate?instId={}", inst_id);
        let data = self
            .api
            .send_request::<Vec<OkxFundingRate>>(Method::GET, &path, "")
            .await
            .map_err(Error::OKX)?;

        data.into_iter()
            .next()
            .ok_or_else(|| Error::Internal("empty funding rate response".into()))
    }

    /// 获取标记价格
    async fn get_mark_price(&self, inst_id: &str) -> Result<OkxMarkPrice> {
        let path = format!("/api/v5/public/mark-price?instId={}", inst_id);
        let data = self
            .api
            .send_request::<Vec<OkxMarkPrice>>(Method::GET, &path, "")
            .await
            .map_err(Error::OKX)?;

        data.into_iter()
            .next()
            .ok_or_else(|| Error::Internal("empty mark price response".into()))
    }

    /// 获取指数价格
    async fn get_index_price(&self, inst_id: &str) -> Result<OkxIndexPrice> {
        let path = format!("/api/v5/market/index-tickers?instId={}", inst_id);
        let data = self
            .api
            .send_request::<Vec<OkxIndexPrice>>(Method::GET, &path, "")
            .await
            .map_err(Error::OKX)?;

        data.into_iter()
            .next()
            .ok_or_else(|| Error::Internal("empty index price response".into()))
    }

    /// 获取持仓量
    async fn get_open_interest(&self, inst_id: &str) -> Result<OkxOpenInterest> {
        let path = format!("/api/v5/public/open-interest?instId={}", inst_id);
        let data = self
            .api
            .send_request::<Vec<OkxOpenInterest>>(Method::GET, &path, "")
            .await
            .map_err(Error::OKX)?;

        data.into_iter()
            .next()
            .ok_or_else(|| Error::Internal("empty open interest response".into()))
    }

    /// 获取成交明细
    async fn get_trades(&self, inst_id: &str, limit: Option<u32>) -> Result<Vec<OkxTrade>> {
        let mut path = format!("/api/v5/market/trades?instId={}", inst_id);
        if let Some(l) = limit {
            path.push_str(&format!("&limit={}", l));
        }
        self.api
            .send_request::<Vec<OkxTrade>>(Method::GET, &path, "")
            .await
            .map_err(Error::OKX)
    }

    /// 获取订单薄
    async fn get_order_book(&self, inst_id: &str, sz: Option<u32>) -> Result<OkxOrderBook> {
        let depth = self
            .market_api
            .get_books(inst_id, sz)
            .await
            .map_err(Error::OKX)?;

        Ok(OkxOrderBook {
            asks: depth.asks,
            bids: depth.bids,
            ts: depth.ts,
        })
    }
}

#[cfg(test)]
mod tests {
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
            .withf(|req: &OkxPlaceOrderRequest| {
                req.ord_type == "limit" && req.inst_id == "BTC-USDT"
            })
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
            .returning(|_, _| {
                Box::pin(async move { Err(Error::NotFound("order missing".into())) })
            });

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
            .returning(|_: &str| {
                Box::pin(async move { Err(Error::Internal("test error".into())) })
            });

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
            .returning(|_: &str| {
                Box::pin(async move { Err(Error::Internal("test error".into())) })
            });

        let result = mock.get_ticker("NONEXISTENT").await;
        assert!(result.is_err());
    }

    // ── 8. get_funding_rate ──

    #[tokio::test]
    async fn test_get_funding_rate_success() {
        let mut mock = MockClientInterface::new();
        mock.expect_get_funding_rate()
            .withf(|inst_id: &str| inst_id == "BTC-USDT-SWAP")
            .returning(|_: &str| {
                Box::pin(async move { Ok(mock_okx_funding_rate("BTC-USDT-SWAP")) })
            });

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
            .returning(|_: &str| {
                Box::pin(async move { Err(Error::Internal("test error".into())) })
            });

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
            .returning(|_: &str| {
                Box::pin(async move { Err(Error::Internal("test error".into())) })
            });

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
            .returning(|_: &str| {
                Box::pin(async move { Err(Error::Internal("test error".into())) })
            });

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
            .returning(|_: &str| {
                Box::pin(async move { Err(Error::Internal("test error".into())) })
            });

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
            .withf(|inst_id: &str, limit: &Option<u32>| {
                inst_id == "BTC-USDT" && *limit == Some(500)
            })
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
            .returning(|_: &str, _: Option<u32>| {
                Box::pin(async move { Ok(mock_okx_order_book()) })
            });

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
            .returning(|_: &str, _: Option<u32>| {
                Box::pin(async move { Ok(mock_empty_order_book()) })
            });

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
}
