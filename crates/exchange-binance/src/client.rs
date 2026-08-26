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
use quant_common::utils::{floor_to_step, now_millis, round_to_tick};
use reqwest::StatusCode;
use rust_decimal::Decimal;
use sha2::Sha256;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use ed25519_dalek::{SigningKey, Signer as _};
use ed25519_dalek::pkcs8::DecodePrivateKey;
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

/// Exchange-level symbol filters (from `/api/v3/exchangeInfo`), used to round
/// order quantity/price to the symbol's step before placing (avoids
/// `-1013 Filter failure: LOT_SIZE / PRICE_FILTER`).
#[derive(Debug, Clone, Default)]
pub struct SymbolFilters {
    pub tick_size: rust_decimal::Decimal,
    pub step_size: rust_decimal::Decimal,
    pub min_qty: rust_decimal::Decimal,
    pub min_notional: rust_decimal::Decimal,
}

/// HMAC-SHA256 signature for a raw query string (hex-encoded).
pub(crate) fn sign(secret: &str, query: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
/// Authentication scheme for signed Binance requests.
enum SigningScheme {
    /// HMAC-SHA256 (hex signature). Default / legacy.
    Hmac(String),
    /// Ed25519 asymmetric (base64 signature) — Binance's recommended key type.
    Ed25519(SigningKey),
}

/// Decode a PEM body (base64 between the `-----BEGIN/END-----` markers) to DER.
fn pem_to_der(pem: &str) -> Result<Vec<u8>> {
    let body: String = pem
        .lines()
        .filter(|l| !l.contains("-----"))
        .collect::<Vec<_>>()
        .join("");
    BASE64
        .decode(body)
        .map_err(|e| Error::Config(format!("invalid Ed25519 private key base64: {e}")))
}

impl SigningScheme {
    fn new(key_type: &str, secret: &str, private_key_pem: Option<&str>) -> Result<Self> {
        match key_type {
            "" | "hmac" => Ok(SigningScheme::Hmac(secret.to_string())),
            "ed25519" => {
                let pem = private_key_pem.ok_or_else(|| {
                    Error::Config(
                        "BINANCE_KEY_TYPE=ed25519 requires BINANCE_PRIVATE_KEY_PATH".to_string(),
                    )
                })?;
                let der = pem_to_der(pem)?;
                let key = SigningKey::from_pkcs8_der(&der)
                    .map_err(|e| Error::Config(format!("invalid Ed25519 private key: {e}")))?;
                Ok(SigningScheme::Ed25519(key))
            }
            other => Err(Error::Config(format!("unsupported BINANCE_KEY_TYPE: {other}"))),
        }
    }

    fn sign(&self, payload: &str) -> String {
        match self {
            SigningScheme::Hmac(secret) => sign(secret, payload),
            SigningScheme::Ed25519(key) => BASE64.encode(key.sign(payload.as_bytes()).to_bytes()),
        }
    }
}

/// Current UNIX timestamp in milliseconds.
fn now_ms() -> u64 {
    now_millis()
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
    /// Latest price for all symbols (`/api/v3/ticker/price`, no symbol → all).
    async fn get_all_ticker_prices(&self) -> Result<HashMap<String, Decimal>>;
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
    signer: SigningScheme,
    environment: BinanceEnvironment,
    /// Per-symbol exchange filters cache (avoids refetching exchangeInfo).
    filters: parking_lot::RwLock<std::collections::HashMap<String, SymbolFilters>>,
}

impl Client {
    pub fn new(
        api_key: String,
        api_secret: String,
        environment: BinanceEnvironment,
        base_url: Option<String>,
        key_type: String,
        private_key_pem: Option<String>,
    ) -> Result<Self> {
        let base = base_url.unwrap_or_else(|| environment.base_url().to_string());
        let signer = SigningScheme::new(&key_type, &api_secret, private_key_pem.as_deref())?;
        Ok(Self {
            http: reqwest::Client::new(),
            base,
            api_key: if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            },
            signer,
            environment,
            filters: parking_lot::RwLock::new(std::collections::HashMap::new()),
        })
    }

    /// Max retries for transient rate-limit (429) responses.
    const RATE_LIMIT_MAX_RETRIES: usize = 3;

    /// Read `Retry-After` seconds from a rate-limit/ban response.
    fn retry_after_secs(resp: &reqwest::Response) -> Option<u64> {
        resp.headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
    }

    /// Fetch (and cache) exchange filters for a symbol. Best-effort: on error
    /// returns `SymbolFilters::default()` so order placement still proceeds.
    async fn symbol_filters(&self, symbol: &str) -> SymbolFilters {
        if let Some(f) = self.filters.read().get(symbol) {
            return f.clone();
        }
        let v: serde_json::Value = match self
            .get_json("/api/v3/exchangeInfo", &[("symbol", symbol.to_string())])
            .await
        {
            Ok(v) => v,
            Err(_) => return SymbolFilters::default(),
        };
        let si = v.get("symbols").and_then(|a| a.get(0)).cloned();
        let mut f = SymbolFilters::default();
        if let Some(filters) = si
            .and_then(|s| s.get("filters").cloned())
            .and_then(|a| a.as_array().cloned())
        {
            for filt in filters {
                match filt.get("filterType").and_then(|t| t.as_str()) {
                    Some("LOT_SIZE") => {
                        f.step_size =
                            parse_decimal(filt.get("stepSize").and_then(|v| v.as_str()).unwrap_or("1"));
                        f.min_qty =
                            parse_decimal(filt.get("minQty").and_then(|v| v.as_str()).unwrap_or("0"));
                    }
                    Some("PRICE_FILTER") => {
                        f.tick_size = parse_decimal(
                            filt.get("tickSize")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0.00000001"),
                        );
                    }
                    Some("NOTIONAL") | Some("MIN_NOTIONAL") => {
                        f.min_notional =
                            parse_decimal(filt.get("minNotional").and_then(|v| v.as_str()).unwrap_or("0"));
                    }
                    _ => {}
                }
            }
        }
        let _ = self
            .filters
            .write()
            .insert(symbol.to_string(), f.clone());
        f
    }

    /// Round a quantity DOWN to the symbol's lot step (safe: never over-buys).
    fn round_down(value: Decimal, step: Decimal) -> Decimal {
        floor_to_step(value, step)
    }

    /// Round a price to the nearest symbol tick.
    fn round_tick(value: Decimal, tick: Decimal) -> Decimal {
        round_to_tick(value, tick)
    }

    /// Send a request, retrying transient 429 (rate limit) with exponential
    /// backoff; a 418 (IP ban) fails immediately so we never keep hammering a
    /// banned IP (which would extend the ban to 3 days).
    async fn send_with_rate_limit_backoff<F>(&self, mut build: F) -> Result<reqwest::Response>
    where
        F: FnMut() -> reqwest::RequestBuilder,
    {
        let mut attempt = 0usize;
        loop {
            let resp = build().send().await.map_err(|e| Error::Network(e.to_string()))?;
            let status = resp.status();
            if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::IM_A_TEAPOT {
                let retry_after = Self::retry_after_secs(&resp)
                    .unwrap_or_else(|| 1u64 << attempt.min(3));
                let banned = status == StatusCode::IM_A_TEAPOT;
                if banned || attempt >= Self::RATE_LIMIT_MAX_RETRIES {
                    return Err(Error::RateLimited {
                        message: format!("HTTP {} (rate limited / banned)", status.as_u16()),
                        retry_after_secs: retry_after,
                    });
                }
                tracing::warn!(attempt, retry_after, "Binance rate limited; backing off");
                tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                attempt += 1;
                continue;
            }
            return Ok(resp);
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
            .send_with_rate_limit_backoff(|| self.http.get(&url).query(params))
            .await?;
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
        let signature = self.signer.sign(&qs);
        let url = format!("{}{}?{}&signature={}", self.base, path, qs, signature);
        let api_key = self.api_key.clone();
        let resp = self
            .send_with_rate_limit_backoff(|| {
                self.http
                    .get(&url)
                    .header("X-MBX-APIKEY", api_key.as_deref().unwrap_or(""))
            })
            .await?;
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

    async fn get_all_ticker_prices(&self) -> Result<HashMap<String, Decimal>> {
        #[derive(serde::Deserialize)]
        struct PriceRow {
            symbol: String,
            price: String,
        }
        let rows: Vec<PriceRow> = self.get_json("/api/v3/ticker/price", &[]).await?;
        Ok(rows
            .into_iter()
            .map(|p| (p.symbol, parse_decimal(&p.price)))
            .collect())
    }

    async fn place_order(&self, request: &BinancePlaceOrderRequest) -> Result<BinanceOrder> {
        // Round quantity/price to the symbol's exchange rules so Binance does
        // not reject with `-1013 Filter failure: LOT_SIZE / PRICE_FILTER`.
        let f = self.symbol_filters(&request.symbol).await;
        let quantity = Self::round_down(request.quantity, f.step_size);
        if quantity < f.min_qty {
            return Err(Error::Validation(format!(
                "Quantity {} invalid for {}: min {} step {}",
                request.quantity, request.symbol, f.min_qty, f.step_size
            )));
        }
        let price = request.price.map(|p| Self::round_tick(p, f.tick_size));
        // MIN_NOTIONAL: for LIMIT orders the notional (qty × price) must meet the
        // exchange minimum, otherwise Binance rejects with `-1013 NOTIONAL`.
        if let (Some(p), false) = (price, f.min_notional <= Decimal::ZERO) {
            if quantity * p < f.min_notional {
                return Err(Error::Validation(format!(
                    "Order notional {} below min {} for {}",
                    quantity * p,
                    f.min_notional,
                    request.symbol
                )));
            }
        }
        let mut params: Vec<(&str, String)> = vec![
            ("symbol", request.symbol.clone()),
            ("side", format!("{:?}", request.side).to_uppercase()),
            ("type", format!("{:?}", request.order_type).to_uppercase()),
            ("quantity", quantity.to_string()),
        ];
        if let Some(price) = price {
            params.push(("price", price.to_string()));
        }
        // Binance requires `timeInForce` for LIMIT orders.
        if matches!(request.order_type, BinanceOrderType::Limit) {
            params.push(("timeInForce", "GTC".to_string()));
        }
        let qs = Self::build_query(&params);
        let timestamp = now_ms().to_string();
        let full_qs = format!("{}&timestamp={}", qs, timestamp);
        let signature = self.signer.sign(&full_qs);
        let url = format!(
            "{}/api/v3/order?{}&signature={}",
            self.base, full_qs, signature
        );
        let api_key = self.api_key.clone();
        let resp = self
            .send_with_rate_limit_backoff(|| {
                self.http
                    .post(&url)
                    .header("X-MBX-APIKEY", api_key.as_deref().unwrap_or(""))
            })
            .await?;
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
        let signature = self.signer.sign(&full_qs);
        let url = format!(
            "{}/api/v3/order?{}&signature={}",
            self.base, full_qs, signature
        );
        let api_key = self.api_key.clone();
        let _resp = self
            .send_with_rate_limit_backoff(|| {
                self.http
                    .delete(&url)
                    .header("X-MBX-APIKEY", api_key.as_deref().unwrap_or(""))
            })
            .await?;
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
