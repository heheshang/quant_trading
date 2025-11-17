use quant_common::{Error, Result};
use security::ApiKeyManager;
use crate::types::*;
use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use tracing::{debug, error, info};

/// OKX 客户端
pub struct OkxClient {
    http_client: Client,
    api_key: String,
    api_secret: String,
    passphrase: String,
    environment: OkxEnvironment,
    api_key_manager: ApiKeyManager,
}

impl OkxClient {
    /// 创建新的 OKX 客户端
    pub fn new(
        api_key: String,
        api_secret: String,
        passphrase: String,
        environment: OkxEnvironment,
    ) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;

        let api_key_manager = ApiKeyManager::new("okx_encryption_key")?;

        Ok(Self {
            http_client,
            api_key,
            api_secret,
            passphrase,
            environment,
            api_key_manager,
        })
    }

    /// 发送签名请求
    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<String>,
    ) -> Result<OkxResponse<T>> {
        let url = format!("{}{}", self.environment.base_url(), path);
        let timestamp = ApiKeyManager::generate_timestamp();
        let body_str = body.as_deref().unwrap_or("");

        // 生成签名
        let signature = self.api_key_manager.generate_signature(
            &self.api_secret,
            &timestamp,
            method.as_str(),
            path,
            body_str,
        )?;

        debug!("OKX Request: {} {}", method, url);

        // 构建请求
        let mut request = self.http_client
            .request(method.clone(), &url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json");

        // 模拟盘标识
        if matches!(self.environment, OkxEnvironment::Demo) {
            request = request.header("x-simulated-trading", "1");
        }

        // 添加请求体
        if let Some(body) = body {
            request = request.body(body);
        }

        // 发送请求
        let response = request.send().await
            .map_err(|e| Error::Network(format!("Request failed: {}", e)))?;

        let status = response.status();
        let text = response.text().await
            .map_err(|e| Error::Network(format!("Read response failed: {}", e)))?;

        debug!("OKX Response: {} - {}", status, text);

        if !status.is_success() {
            error!("OKX API Error: {} - {}", status, text);
            return Err(Error::Network(format!("HTTP {}: {}", status, text)));
        }

        serde_json::from_str(&text)
            .map_err(|e| Error::Internal(format!("Parse response failed: {}", e)))
    }

    /// 获取账户余额
    pub async fn get_account_balance(&self, ccy: Option<&str>) -> Result<Vec<OkxBalance>> {
        let path = if let Some(currency) = ccy {
            format!("/api/v5/account/balance?ccy={}", currency)
        } else {
            "/api/v5/account/balance".to_string()
        };

        let response: OkxResponse<OkxBalance> = self.request(Method::GET, &path, None).await?;
        
        if response.code != "0" {
            return Err(Error::Network(format!("OKX API Error: {}", response.msg)));
        }

        Ok(response.data.unwrap_or_default())
    }

    /// 获取持仓
    pub async fn get_positions(&self, inst_id: Option<&str>) -> Result<Vec<OkxPosition>> {
        let path = if let Some(id) = inst_id {
            format!("/api/v5/account/positions?instId={}", id)
        } else {
            "/api/v5/account/positions".to_string()
        };

        let response: OkxResponse<OkxPosition> = self.request(Method::GET, &path, None).await?;
        
        if response.code != "0" {
            return Err(Error::Network(format!("OKX API Error: {}", response.msg)));
        }

        Ok(response.data.unwrap_or_default())
    }

    /// 下单
    pub async fn place_order(&self, request: OkxPlaceOrderRequest) -> Result<OkxOrder> {
        let body = serde_json::to_string(&request)
            .map_err(|e| Error::Internal(format!("Serialize request failed: {}", e)))?;

        info!("Placing order: {}", body);

        let response: OkxResponse<OkxOrder> = self.request(
            Method::POST,
            "/api/v5/trade/order",
            Some(body),
        ).await?;

        if response.code != "0" {
            return Err(Error::Trading(format!("Place order failed: {}", response.msg)));
        }

        response.data
            .and_then(|mut data| data.pop())
            .ok_or_else(|| Error::Trading("No order data returned".to_string()))
    }

    /// 撤单
    pub async fn cancel_order(&self, inst_id: &str, ord_id: &str) -> Result<()> {
        let body = serde_json::json!({
            "instId": inst_id,
            "ordId": ord_id,
        }).to_string();

        let response: OkxResponse<serde_json::Value> = self.request(
            Method::POST,
            "/api/v5/trade/cancel-order",
            Some(body),
        ).await?;

        if response.code != "0" {
            return Err(Error::Trading(format!("Cancel order failed: {}", response.msg)));
        }

        Ok(())
    }

    /// 获取K线数据
    pub async fn get_candles(
        &self,
        inst_id: &str,
        bar: &str, // 1m, 5m, 15m, 1H, 1D
        limit: Option<u32>,
    ) -> Result<Vec<OkxCandle>> {
        let mut path = format!("/api/v5/market/candles?instId={}&bar={}", inst_id, bar);
        
        if let Some(lim) = limit {
            path.push_str(&format!("&limit={}", lim));
        }

        let response: OkxResponse<OkxCandle> = self.request(Method::GET, &path, None).await?;
        
        if response.code != "0" {
            return Err(Error::Network(format!("Get candles failed: {}", response.msg)));
        }

        Ok(response.data.unwrap_or_default())
    }

    /// 获取交易对信息
    pub async fn get_instruments(&self, inst_type: &str) -> Result<serde_json::Value> {
        let path = format!("/api/v5/public/instruments?instType={}", inst_type);
        let response: OkxResponse<serde_json::Value> = self.request(Method::GET, &path, None).await?;
        
        if response.code != "0" {
            return Err(Error::Network(format!("Get instruments failed: {}", response.msg)));
        }

        Ok(serde_json::json!(response.data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_okx_client_creation() {
        let client = OkxClient::new(
            "test_key".to_string(),
            "test_secret".to_string(),
            "test_pass".to_string(),
            OkxEnvironment::Demo,
        );
        
        assert!(client.is_ok());
    }
}
