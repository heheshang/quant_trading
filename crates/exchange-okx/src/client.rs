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
                fee: "0".to_string(),
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

        // OKX 按 ord_id 查询（instrument 内唯一），返回列表最多一个元素，
        // 取首个即可；为空时视为订单不存在。
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
                fee: o.fee,
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
mod tests;
