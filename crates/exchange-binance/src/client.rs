//! Binance REST client.
//!
//! Mirrors `exchange-okx::client`: a trait-based client (`ClientInterface`)
//! with a `reqwest` implementation (`Client`) and an auto-mock for tests.
//! Private endpoints are signed with HMAC-SHA256; public market data needs
//! no authentication.

use crate::types::*;
use async_trait::async_trait;
use rust_decimal::Decimal;
use hmac::{Hmac, Mac};
use quant_common::{Error, Result};
use reqwest::StatusCode;
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
    /// Place an order (`/api/v3/order`, signed).
    async fn place_order(&self, request: &BinancePlaceOrderRequest) -> Result<BinanceOrder>;
    /// Cancel an order (`/api/v3/order`, signed).
    async fn cancel_order(&self, symbol: &str, order_id: i64) -> Result<()>;
}

/// Concrete Binance REST client.
pub struct Client {
    http: reqwest::Client,
    base: String,
    api_key: Option<String>,
    api_secret: String,
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
            api_key: if api_key.is_empty() { None } else { Some(api_key) },
            api_secret,
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
            return Err(Error::Network(format!("Binance {path} -> {status}: {body}")));
        }
        resp.json::<T>().await.map_err(|e| Error::Network(e.to_string()))
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
        rows.into_iter()
            .map(|row| parse_kline(&row))
            .collect()
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

    async fn get_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<BinanceOrderBook> {
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
        #[derive(serde::Deserialize)]
        struct NewOrder {
            symbol: String,
            order_id: i64,
            client_order_id: String,
            status: String,
            executed_qty: String,
            cummulative_quote_qty: String,
            price: String,
        }
        let o: NewOrder = Self::unwrap(resp, "/api/v3/order").await?;
        Ok(BinanceOrder {
            symbol: o.symbol,
            order_id: o.order_id,
            client_order_id: o.client_order_id,
            status: o.status,
            executed_qty: parse_decimal(&o.executed_qty),
            cummulative_quote_qty: parse_decimal(&o.cummulative_quote_qty),
            price: parse_decimal(&o.price),
        })
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
        open_time: row
            .first()
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as i64,
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


