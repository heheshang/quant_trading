//! Binance REST client.
//!
//! Independent Binance implementation: a trait-based client (`ClientInterface`)
//! with a `reqwest` implementation (`Client`) and an auto-mock for tests.
//! Private endpoints are signed with HMAC-SHA256; public market data needs
//! no authentication.

use crate::types::*;
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use quant_common::{Error, Result};
use reqwest::StatusCode;
use rust_decimal::Decimal;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const SPOT_BASE: &str = "https://api.binance.com";
const FUTURES_BASE: &str = "https://fapi.binance.com";

/// HMAC-SHA256 signature for a raw query string (hex-encoded).
pub(crate) fn sign(secret: &str, query: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Current UNIX timestamp in milliseconds.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_millis() as u64
}

/// Binance client trait — enables mocking in tests.
#[async_trait]
#[cfg_attr(any(test, feature = "test-utils"), mockall::automock)]
pub trait ClientInterface: Send + Sync {
    /// Klines for a symbol (`/api/v3/klines`).
    async fn get_candles(
        &self,
        symbol: &str,
        interval: &str,
        limit: Option<u32>,
    ) -> Result<Vec<BinanceKline>>;
    /// Account balances (`/api/v3/account`, signed).
    async fn get_account_balance(&self) -> Result<Vec<BinanceBalance>>;
    /// Order-book depth (`/api/v3/depth`).
    async fn get_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<BinanceOrderBook>;
    /// 24h ticker (`/api/v3/ticker/24hr`).
    async fn get_ticker_24hr(&self, symbol: &str) -> Result<BinanceTicker24h>;
    /// Place an order (`/api/v3/order`, signed).
    async fn place_order(&self, request: &BinancePlaceOrderRequest) -> Result<BinanceOrder>;
    /// Cancel an order (`/api/v3/order`, signed).
    async fn cancel_order(&self, symbol: &str, order_id: i64) -> Result<()>;
    /// Positions (`/fapi/v2/positionRisk`, signed). Futures only; spot
    /// accounts return an empty vec.
    async fn get_positions(&self, symbol: Option<&str>) -> Result<Vec<BinancePosition>>;
    /// Single order query (`/api/v3/order`, signed).
    async fn get_order(&self, symbol: &str, order_id: i64) -> Result<BinanceOrder>;
    /// Open orders (`/api/v3/openOrders`, signed).
    async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<BinanceOrder>>;
    /// Order history (`/api/v3/allOrders`, signed).
    async fn get_all_orders(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<BinanceOrder>>;
    /// Exchange instruments (`/api/v3/exchangeInfo`, public).
    async fn get_instruments(&self) -> Result<serde_json::Value>;
}

/// Concrete Binance REST client.
pub struct Client {
    http: reqwest::Client,
    base: String,
    api_key: Option<String>,
    api_secret: String,
    environment: BinanceEnvironment,
}

impl Client {
    pub fn new(api_key: String, api_secret: String, environment: BinanceEnvironment) -> Self {
        let base = match environment {
            BinanceEnvironment::Spot => SPOT_BASE,
            BinanceEnvironment::Futures => FUTURES_BASE,
        };
        Self {
            http: reqwest::Client::new(),
            base: base.to_string(),
            api_key: if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            },
            api_secret,
            environment,
        }
    }

    /// Public GET that parses to `T`.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .http
            .get(&url)
            .query(params)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        Self::unwrap(resp, path).await
    }

    /// Signed GET (adds timestamp + signature, requires API key).
    async fn signed_get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: Vec<(&str, String)>,
    ) -> Result<T> {
        let mut query = params;
        query.push(("timestamp", now_ms().to_string()));
        let qs = Self::build_query(&query);
        let signature = sign(&self.api_secret, &qs);
        let url = format!("{}{}?{}&signature={}", self.base, path, qs, signature);
        let resp = self
            .http
            .get(&url)
            .header("X-MBX-APIKEY", self.api_key.as_deref().unwrap_or(""))
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        Self::unwrap(resp, path).await
    }

    /// Build a query string. Binance params are alphanumeric (symbol,
    /// interval, limit, timestamp, orderId), so no percent-encoding is needed.
    fn build_query(params: &[(&str, String)]) -> String {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Uniform error mapping for non-2xx responses.
    async fn unwrap<T: serde::de::DeserializeOwned>(
        resp: reqwest::Response,
        path: &str,
    ) -> Result<T> {
        let status = resp.status();
        if status != StatusCode::OK {
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(path, status = %status, body, "Binance request failed");
            return Err(Error::Network(format!(
                "Binance {path} -> {status}: {body}"
            )));
        }
        resp.json::<T>()
            .await
            .map_err(|e| Error::Network(e.to_string()))
    }
}

#[async_trait]
impl ClientInterface for Client {
    async fn get_candles(
        &self,
        symbol: &str,
        interval: &str,
        limit: Option<u32>,
    ) -> Result<Vec<BinanceKline>> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("interval", interval.to_string()),
            ("limit", limit.unwrap_or(500).to_string()),
        ];
        let rows: Vec<Vec<serde_json::Value>> = self.get_json("/api/v3/klines", &params).await?;
        rows.into_iter().map(|row| parse_kline(&row)).collect()
    }

    async fn get_account_balance(&self) -> Result<Vec<BinanceBalance>> {
        #[derive(serde::Deserialize)]
        struct Account {
            balances: Vec<AssetBalance>,
        }
        #[derive(serde::Deserialize)]
        struct AssetBalance {
            asset: String,
            free: String,
            locked: String,
        }
        let account: Account = self.signed_get("/api/v3/account", vec![]).await?;
        Ok(account
            .balances
            .into_iter()
            .map(|b| BinanceBalance {
                asset: b.asset,
                free: parse_decimal(&b.free),
                locked: parse_decimal(&b.locked),
            })
            .collect())
    }

    async fn get_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<BinanceOrderBook> {
        #[derive(serde::Deserialize)]
        struct Depth {
            bids: Vec<(String, String)>,
            asks: Vec<(String, String)>,
        }
        let params = vec![
            ("symbol", symbol.to_string()),
            ("limit", limit.unwrap_or(50).to_string()),
        ];
        let depth: Depth = self.get_json("/api/v3/depth", &params).await?;
        let bids = depth
            .bids
            .into_iter()
            .map(|(p, q)| (parse_decimal(&p), parse_decimal(&q)))
            .collect();
        let asks = depth
            .asks
            .into_iter()
            .map(|(p, q)| (parse_decimal(&p), parse_decimal(&q)))
            .collect();
        Ok(BinanceOrderBook {
            symbol: symbol.to_string(),
            bids,
            asks,
        })
    }

    async fn get_ticker_24hr(&self, symbol: &str) -> Result<BinanceTicker24h> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Ticker {
            last_price: String,
            price_change: String,
            price_change_percent: String,
            high_price: String,
            low_price: String,
            open_price: String,
            volume: String,
            quote_volume: String,
        }
        let params = vec![("symbol", symbol.to_string())];
        let t: Ticker = self.get_json("/api/v3/ticker/24hr", &params).await?;
        Ok(BinanceTicker24h {
            symbol: symbol.to_string(),
            last_price: parse_decimal(&t.last_price),
            price_change: parse_decimal(&t.price_change),
            price_change_percent: parse_decimal(&t.price_change_percent),
            high: parse_decimal(&t.high_price),
            low: parse_decimal(&t.low_price),
            open: parse_decimal(&t.open_price),
            volume: parse_decimal(&t.volume),
            quote_volume: parse_decimal(&t.quote_volume),
        })
    }

    async fn place_order(&self, request: &BinancePlaceOrderRequest) -> Result<BinanceOrder> {
        let mut params: Vec<(&str, String)> = vec![
            ("symbol", request.symbol.clone()),
            ("side", format!("{:?}", request.side).to_uppercase()),
            ("type", format!("{:?}", request.order_type).to_uppercase()),
            ("quantity", request.quantity.to_string()),
        ];
        if let Some(price) = request.price {
            params.push(("price", price.to_string()));
        }
        let qs = Self::build_query(&params);
        let timestamp = now_ms().to_string();
        let full_qs = format!("{}&timestamp={}", qs, timestamp);
        let signature = sign(&self.api_secret, &full_qs);
        let url = format!(
            "{}/api/v3/order?{}&signature={}",
            self.base, full_qs, signature
        );
        let resp = self
            .http
            .post(&url)
            .header("X-MBX-APIKEY", self.api_key.as_deref().unwrap_or(""))
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        let o: ApiOrder = Self::unwrap(resp, "/api/v3/order").await?;
        Ok(o.into())
    }

    async fn cancel_order(&self, symbol: &str, order_id: i64) -> Result<()> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
        ];
        let qs = Self::build_query(&params);
        let timestamp = now_ms().to_string();
        let full_qs = format!("{}&timestamp={}", qs, timestamp);
        let signature = sign(&self.api_secret, &full_qs);
        let url = format!(
            "{}/api/v3/order?{}&signature={}",
            self.base, full_qs, signature
        );
        let _resp = self
            .http
            .delete(&url)
            .header("X-MBX-APIKEY", self.api_key.as_deref().unwrap_or(""))
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        Ok(())
    }

    async fn get_positions(&self, symbol: Option<&str>) -> Result<Vec<BinancePosition>> {
        // Spot accounts expose no derivative positions.
        if self.environment == BinanceEnvironment::Spot {
            return Ok(vec![]);
        }
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(symbol) = symbol {
            params.push(("symbol", symbol.to_string()));
        }
        let rows: Vec<PositionRisk> = self.signed_get("/fapi/v2/positionRisk", params).await?;
        Ok(rows
            .into_iter()
            .filter(|p| !p.position_amt.is_empty() && p.position_amt != "0")
            .map(BinancePosition::from)
            .collect())
    }

    async fn get_order(&self, symbol: &str, order_id: i64) -> Result<BinanceOrder> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
        ];
        let o: ApiOrder = self.signed_get("/api/v3/order", params).await?;
        Ok(o.into())
    }

    async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<BinanceOrder>> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(symbol) = symbol {
            params.push(("symbol", symbol.to_string()));
        }
        let rows: Vec<ApiOrder> = self.signed_get("/api/v3/openOrders", params).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_all_orders(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<BinanceOrder>> {
        let mut params = vec![("symbol", symbol.to_string())];
        if let Some(limit) = limit {
            params.push(("limit", limit.to_string()));
        }
        let rows: Vec<ApiOrder> = self.signed_get("/api/v3/allOrders", params).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_instruments(&self) -> Result<serde_json::Value> {
        let path = if self.environment == BinanceEnvironment::Futures {
            "/fapi/v1/exchangeInfo"
        } else {
            "/api/v3/exchangeInfo"
        };
        self.get_json(path, &[]).await
    }
}

/// Raw Binance order response field mapping (camelCase API -> serpent casing).
///
/// Used for `place_order`, `get_order`, `openOrders` and `allOrders`; extra
/// fields are tolerated with `#[serde(default)]`.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiOrder {
    symbol: String,
    order_id: i64,
    client_order_id: String,
    status: String,
    executed_qty: String,
    cummulative_quote_qty: String,
    price: String,
    #[serde(default)]
    side: String,
    #[serde(default, rename = "type")]
    order_type: String,
    #[serde(default)]
    orig_qty: String,
    #[serde(default)]
    time: i64,
    #[serde(default)]
    update_time: i64,
    #[serde(default)]
    transact_time: i64,
}

impl From<ApiOrder> for BinanceOrder {
    fn from(o: ApiOrder) -> Self {
        BinanceOrder {
            symbol: o.symbol,
            order_id: o.order_id,
            client_order_id: o.client_order_id,
            status: o.status,
            executed_qty: parse_decimal(&o.executed_qty),
            cummulative_quote_qty: parse_decimal(&o.cummulative_quote_qty),
            price: parse_decimal(&o.price),
            side: o.side,
            order_type: o.order_type,
            orig_qty: parse_decimal(&o.orig_qty),
            time: if o.time != 0 { o.time } else { o.transact_time },
            update_time: o.update_time,
        }
    }
}

/// Raw Binance `/fapi/v2/positionRisk` row.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PositionRisk {
    pub(crate) symbol: String,
    pub(crate) position_amt: String,
    pub(crate) entry_price: String,
    pub(crate) mark_price: String,
    pub(crate) un_realized_profit: String,
    pub(crate) liquidation_price: String,
    pub(crate) leverage: String,
    pub(crate) margin_type: String,
    pub(crate) notional: String,
    pub(crate) position_side: String,
}

impl From<PositionRisk> for BinancePosition {
    fn from(p: PositionRisk) -> Self {
        BinancePosition {
            symbol: p.symbol,
            position_amt: parse_decimal(&p.position_amt),
            entry_price: parse_decimal(&p.entry_price),
            mark_price: parse_decimal(&p.mark_price),
            un_realized_profit: parse_decimal(&p.un_realized_profit),
            liquidation_price: parse_decimal(&p.liquidation_price),
            leverage: p.leverage,
            margin_type: p.margin_type,
            notional: parse_decimal(&p.notional),
            position_side: p.position_side,
        }
    }
}

/// Parse a numeric string to `Decimal`, failing defensively for bad input.
pub(crate) fn parse_decimal(value: &str) -> Decimal {
    value.parse::<Decimal>().unwrap_or(Decimal::ZERO)
}

/// Parse a `/api/v3/klines` row (array of values).
fn parse_kline(row: &[serde_json::Value]) -> Result<BinanceKline> {
    let get = |i: usize| -> Result<String> {
        row.get(i)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::Network(format!("missing kline field at index {i}")))
    };
    Ok(BinanceKline {
        open_time: row.first().and_then(|v| v.as_u64()).unwrap_or(0) as i64,
        open: parse_decimal(&get(1)?),
        high: parse_decimal(&get(2)?),
        low: parse_decimal(&get(3)?),
        close: parse_decimal(&get(4)?),
        volume: parse_decimal(&get(5)?),
        close_time: row.get(6).and_then(|v| v.as_u64()).unwrap_or(0) as i64,
        quote_volume: parse_decimal(&get(7)?),
        trades: row.get(8).and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    const ORDER_JSON: &str = r#"{
        "symbol":"BTCUSDT",
        "orderId":123,
        "clientOrderId":"ord-x",
        "price":"50000.00",
        "origQty":"0.01",
        "executedQty":"0.005",
        "cummulativeQuoteQty":"250.00",
        "status":"NEW",
        "time":1700000000000,
        "updateTime":1700000001000,
        "side":"BUY",
        "type":"LIMIT"
    }"#;

    const ORDER_JSON_PLACE: &str = r#"{
        "symbol":"BTCUSDT",
        "orderId":456,
        "clientOrderId":"ord-y",
        "price":"50000.00",
        "origQty":"0.02",
        "executedQty":"0",
        "cummulativeQuoteQty":"0",
        "status":"NEW",
        "transactTime":1700000002000,
        "side":"SELL",
        "type":"MARKET"
    }"#;

    const POSITION_JSON: &str = r#"{
        "symbol":"BTCUSDT",
        "positionAmt":"0.0010",
        "entryPrice":"50000.00",
        "markPrice":"51000.00",
        "unRealizedProfit":"1.00",
        "liquidationPrice":"0",
        "leverage":"10",
        "marginType":"crossed",
        "notional":"50.00",
        "positionSide":"BOTH"
    }"#;

    #[test]
    fn order_all_orders_parses_fields() {
        let o: ApiOrder = serde_json::from_str(ORDER_JSON).expect("parse");
        let order: BinanceOrder = o.into();
        assert_eq!(order.symbol, "BTCUSDT");
        assert_eq!(order.order_id, 123);
        assert_eq!(order.side, "BUY");
        assert_eq!(order.order_type, "LIMIT");
        assert_eq!(order.orig_qty, Decimal::new(1, 2));
        assert_eq!(order.executed_qty, Decimal::new(5, 3));
        assert_eq!(order.time, 1_700_000_000_000);
        assert_eq!(order.update_time, 1_700_000_001_000);
    }

    #[test]
    fn order_place_response_falls_back_to_transact_time() {
        let o: ApiOrder = serde_json::from_str(ORDER_JSON_PLACE).expect("parse");
        let order: BinanceOrder = o.into();
        assert_eq!(order.order_id, 456);
        assert_eq!(order.side, "SELL");
        assert_eq!(order.order_type, "MARKET");
        // No `time` field; falls back to `transactTime`.
        assert_eq!(order.time, 1_700_000_002_000);
        assert_eq!(order.update_time, 0);
    }

    #[test]
    fn position_risk_parses_fields() {
        let p: PositionRisk = serde_json::from_str(POSITION_JSON).expect("parse");
        let pos: BinancePosition = p.into();
        assert_eq!(pos.symbol, "BTCUSDT");
        assert_eq!(pos.position_amt, Decimal::new(10, 4));
        assert_eq!(pos.entry_price, Decimal::new(50_000, 0));
        assert_eq!(pos.un_realized_profit, Decimal::new(1, 0));
        assert_eq!(pos.leverage, "10");
        assert_eq!(pos.position_side, "BOTH");
    }
}
