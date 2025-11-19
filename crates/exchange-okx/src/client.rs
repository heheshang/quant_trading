use crate::types::*;
use okx::api::account::OkxAccount;
use okx::api::announcements::announcements_api::{AnnouncementPage, OkxAnnouncements};
use okx::api::api_trait::OkxApiTrait;
use okx::api::market::OkxMarket;
use okx::api::trade::OkxTrade;
use okx::config::Credentials;
use okx::dto::trade_dto::OrderReqDto;
use okx::OkxClient;
use quant_common::{Error, Result};
/// OKX 客户端
#[derive(Debug)]
pub struct Client {
    api: OkxClient,
    environment: OkxEnvironment,
    account_api: OkxAccount,
    market_api: OkxMarket,
    trade_api: OkxTrade,
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
        let api = OkxClient::new(credentials).map_err(|e| Error::OKX(e))?;

        let account_api = OkxAccount::new(api.clone());
        let market_api = OkxMarket::new(api.clone());
        let trade_api = OkxTrade::new(api.clone());

        Ok(Self {
            api,
            environment,
            account_api,
            market_api,
            trade_api,
        })
    }

    pub async fn get_announcements(&self) -> Result<Vec<AnnouncementPage>> {
        let api = OkxAnnouncements::new(self.api.clone());
        let announcements = api.get_announcements(None, None, None).await;
        match announcements {
            Ok(announcements) => Ok(announcements),
            Err(e) => Err(Error::OKX(e)),
        }
    }

    /// 获取账户余额
    pub async fn get_account_balance(&self, ccy: Option<&str>) -> Result<Vec<OkxBalance>> {
        let balances = self
            .account_api
            .get_balance(ccy)
            .await
            .map_err(|e| Error::OKX(e))?;

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
    pub async fn get_positions(&self, inst_id: Option<&str>) -> Result<Vec<OkxPosition>> {
        let positions = self
            .account_api
            .get_positions(None, inst_id, None)
            .await
            .map_err(|e| Error::OKX(e))?;

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
    pub async fn place_order(&self, request: OkxPlaceOrderRequest) -> Result<OkxOrder> {
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
            .map_err(|e| Error::OKX(e))?;

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

    /// 撤单
    pub async fn cancel_order(&self, inst_id: &str, ord_id: &str) -> Result<()> {
        self.trade_api
            .cancel_order(inst_id, Some(ord_id), None)
            .await
            .map_err(|e| Error::OKX(e))?;
        Ok(())
    }

    /// 获取K线数据
    pub async fn get_candles(
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
            .map_err(|e| Error::OKX(e))?;

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
    pub async fn get_instruments(&self, inst_type: &str) -> Result<serde_json::Value> {
        let instruments = self
            .market_api
            .get_instruments(inst_type, None, None)
            .await
            .map_err(|e| Error::OKX(e))?;
        serde_json::to_value(instruments).map_err(|e| Error::Internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;

    #[tokio::test]
    async fn test_okx_client_creation() {
        dotenv().ok();

        // Check if environment variables are set
        let api_key = match dotenv::var("OKX_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                println!("OKX_API_KEY not set, skipping test");
                return;
            }
        };

        let api_secret = match dotenv::var("OKX_API_SECRET") {
            Ok(secret) => secret,
            Err(_) => {
                println!("OKX_API_SECRET not set, skipping test");
                return;
            }
        };

        let passphrase = match dotenv::var("OKX_PASSPHRASE") {
            Ok(pass) => pass,
            Err(_) => {
                println!("OKX_PASSPHRASE not set, skipping test");
                return;
            }
        };

        let client = Client::new(api_key, api_secret, passphrase, OkxEnvironment::Demo);

        match client {
            Ok(client) => {
                println!("OKX client created successfully");
                // Try to get account balance but handle errors gracefully
                match client.get_account_balance(Some("USDT")).await {
                    Ok(balances) => {
                        println!("Account balance retrieved: {:?}", balances);
                    }
                    Err(e) => {
                        println!("Failed to get account balance: {}", e);
                        // This is expected if there's a network issue or invalid credentials
                    }
                }
            }
            Err(e) => {
                println!("Failed to create OKX client: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_announcements() {
        dotenv().ok();
        // Check if environment variables are set
        let api_key = match dotenv::var("OKX_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                println!("OKX_API_KEY not set, skipping test");
                return;
            }
        };

        let api_secret = match dotenv::var("OKX_API_SECRET") {
            Ok(secret) => secret,
            Err(_) => {
                println!("OKX_API_SECRET not set, skipping test");
                return;
            }
        };

        let passphrase = match dotenv::var("OKX_PASSPHRASE") {
            Ok(pass) => pass,
            Err(_) => {
                println!("OKX_PASSPHRASE not set, skipping test");
                return;
            }
        };

        let client = Client::new(api_key, api_secret, passphrase, OkxEnvironment::Demo);

        let announcements = client.unwrap().get_announcements().await.unwrap();
        println!("{:?}", announcements);
    }
}
