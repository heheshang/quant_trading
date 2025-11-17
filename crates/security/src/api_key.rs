use quant_common::{Error, Result};
use crate::encryption::DataEncryption;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;

type HmacSha256 = Hmac<Sha256>;

/// API 密钥管理器
pub struct ApiKeyManager {
    encryptor: DataEncryption,
}

impl ApiKeyManager {
    pub fn new(encryption_key: &str) -> Result<Self> {
        let encryptor = DataEncryption::from_key_string(encryption_key)?;
        Ok(Self { encryptor })
    }

    /// 加密存储 API Key
    pub fn encrypt_api_key(&self, api_key: &str) -> Result<String> {
        self.encryptor.encrypt_string(api_key)
    }

    /// 解密 API Key
    pub fn decrypt_api_key(&self, encrypted: &str) -> Result<String> {
        self.encryptor.decrypt_string(encrypted)
    }

    /// 生成 API 签名（用于交易所 API 调用）
    pub fn generate_signature(
        &self,
        secret: &str,
        timestamp: &str,
        method: &str,
        request_path: &str,
        body: &str,
    ) -> Result<String> {
        // OKX 签名格式: timestamp + method + requestPath + body
        let prehash = format!("{}{}{}{}", timestamp, method, request_path, body);
        
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| Error::Internal(format!("HMAC init failed: {}", e)))?;
        
        mac.update(prehash.as_bytes());
        let result = mac.finalize();
        let signature = general_purpose::STANDARD.encode(result.into_bytes());
        
        Ok(signature)
    }

    /// 生成当前时间戳（ISO 8601 格式）
    pub fn generate_timestamp() -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    }
}

/// API 密钥存储结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiCredentials {
    pub api_key: String,
    pub encrypted_secret: String,
    pub passphrase: Option<String>,  // OKX 需要
    pub exchange: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl ApiCredentials {
    pub fn new(
        api_key: String,
        secret: String,
        passphrase: Option<String>,
        exchange: String,
        encryptor: &DataEncryption,
    ) -> Result<Self> {
        let encrypted_secret = encryptor.encrypt_string(&secret)?;
        
        Ok(Self {
            api_key,
            encrypted_secret,
            passphrase,
            exchange,
            created_at: Utc::now(),
        })
    }

    pub fn get_decrypted_secret(&self, encryptor: &DataEncryption) -> Result<String> {
        encryptor.decrypt_string(&self.encrypted_secret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_encryption() {
        let manager = ApiKeyManager::new("test_encryption_key").unwrap();
        
        let api_key = "my_secret_api_key";
        let encrypted = manager.encrypt_api_key(api_key).unwrap();
        let decrypted = manager.decrypt_api_key(&encrypted).unwrap();
        
        assert_eq!(api_key, decrypted);
    }

    #[test]
    fn test_signature_generation() {
        let manager = ApiKeyManager::new("test_key").unwrap();
        
        let signature = manager.generate_signature(
            "secret",
            "2024-01-01T00:00:00.000Z",
            "GET",
            "/api/v5/account/balance",
            "",
        ).unwrap();
        
        assert!(!signature.is_empty());
    }
}
