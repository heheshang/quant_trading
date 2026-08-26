//! Exchange API-key storage service.
//!
//! Combines the `security::ApiKeyManager` (AES-256-GCM encryption) with the
//! `data_layer::ApiKeyRepository` (persistence) so exchange secrets are
//! stored encrypted in the `api_keys` table and never returned in plaintext.

use crate::error::{ServiceError, ServiceResult};
use data_layer::{ApiKeyRecord, ApiKeyRepository, NewApiKey};
use security::ApiKeyManager;
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

/// A redacted view of a stored API key. The secret is never included; the
/// public key and passphrase are masked.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MaskedApiKey {
    pub id: i64,
    pub exchange: String,
    pub api_key: String,
    pub passphrase: Option<String>,
    pub is_active: bool,
}

fn mask(value: &str) -> String {
    let len = value.chars().count();
    if len <= 8 {
        "****".to_string()
    } else {
        let prefix: String = value.chars().take(4).collect();
        let suffix: String = value.chars().skip(len - 4).collect();
        format!("{}...{}", prefix, suffix)
    }
}

/// Exchange API-key storage service.
pub struct ApiKeyService {
    manager: Option<ApiKeyManager>,
    repo: Option<Arc<dyn ApiKeyRepository>>,
}

impl ApiKeyService {
    pub fn new(encryption_key: Option<String>, repo: Option<Arc<dyn ApiKeyRepository>>) -> Self {
        let manager = encryption_key.map(|key| {
            let effective = if key.is_empty() {
                warn!(
                    "ENCRYPTION_KEY is empty; using a default fallback key. \
                     Set ENCRYPTION_KEY for production API-key encryption."
                );
                "quant_encryption_fallback_key_change_me".to_string()
            } else {
                key
            };
            match ApiKeyManager::new(&effective) {
                Ok(m) => m,
                Err(e) => {
                    error!("Failed to initialize API key manager: {}", e);
                    // Fall back to a manager built from a fixed key so the
                    // service remains usable (best-effort).
                    ApiKeyManager::new("quant_encryption_fallback_key_change_me")
                        .expect("fallback key must be valid")
                }
            }
        });
        Self { manager, repo }
    }

    /// Encrypt an exchange secret and persist it to the `api_keys` table.
    #[instrument(skip(self, secret, passphrase), fields(exchange = %exchange))]
    pub async fn save_api_key(
        &self,
        user_id: i64,
        exchange: &str,
        api_key: &str,
        secret: &str,
        passphrase: Option<String>,
    ) -> ServiceResult<()> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| ServiceError::Other("API key manager not initialized".into()))?;
        let repo = self.repo.as_ref().ok_or_else(|| {
            ServiceError::Other("API key repository not available (no database)".into())
        })?;

        let encrypted_secret = manager.encrypt_api_key(secret).map_err(|e| {
            error!("Failed to encrypt API secret: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        let row = NewApiKey {
            user_id,
            exchange: exchange.to_string(),
            api_key: api_key.to_string(),
            encrypted_secret,
            passphrase,
        };
        repo.insert(&row).await.map_err(|e| {
            error!("Failed to persist API key: {}", e);
            ServiceError::Other(e.to_string())
        })?;
        info!(user_id, exchange = %exchange, "API key saved (encrypted)");
        Ok(())
    }

    /// List stored API keys for a user, redacted (no secret, masked keys).
    #[instrument(skip(self), fields(user_id))]
    pub async fn get_api_keys(&self, user_id: i64) -> ServiceResult<Vec<MaskedApiKey>> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            ServiceError::Other("API key repository not available (no database)".into())
        })?;

        let records = repo.find_all(user_id).await.map_err(|e| {
            error!("Failed to query API keys: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        Ok(records.into_iter().map(mask_record).collect())
    }

    /// Decrypt an exchange secret by user + exchange (+ api key). Intended for
    /// building exchange clients from the encrypted store (#12); currently the
    /// client construction path still reads env plaintext (see TODO in main.rs).
    #[instrument(skip(self), fields(user_id, exchange))]
    pub async fn get_decrypted_secret(
        &self,
        user_id: i64,
        exchange: &str,
        api_key: &str,
    ) -> ServiceResult<String> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| ServiceError::Other("API key manager not initialized".into()))?;
        let repo = self.repo.as_ref().ok_or_else(|| {
            ServiceError::Other("API key repository not available (no database)".into())
        })?;

        let records = repo
            .find_by_exchange(user_id, exchange)
            .await
            .map_err(|e| {
                error!("Failed to query API key for {}: {}", exchange, e);
                ServiceError::Other(e.to_string())
            })?;
        let record = records
            .into_iter()
            .find(|r| r.api_key == api_key)
            .ok_or_else(|| ServiceError::NotFound(format!("no active api key for {}", exchange)))?;

        manager
            .decrypt_api_key(&record.encrypted_secret)
            .map_err(|e| {
                error!("Failed to decrypt API secret: {}", e);
                ServiceError::Other(e.to_string())
            })
    }
}

fn mask_record(record: ApiKeyRecord) -> MaskedApiKey {
    MaskedApiKey {
        id: record.id,
        exchange: record.exchange,
        api_key: mask(&record.api_key),
        passphrase: record.passphrase.as_deref().map(mask),
        is_active: record.is_active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use parking_lot::Mutex;
    use data_layer::RepoError;

    #[derive(Debug, Clone)]
    struct ApiKeyRecordClone {
        id: i64,
        user_id: i64,
        exchange: String,
        api_key: String,
        encrypted_secret: String,
        passphrase: Option<String>,
        is_active: bool,
        created_at: chrono::DateTime<Utc>,
    }

    impl From<ApiKeyRecordClone> for ApiKeyRecord {
        fn from(r: ApiKeyRecordClone) -> Self {
            ApiKeyRecord {
                id: r.id,
                user_id: r.user_id,
                exchange: r.exchange,
                api_key: r.api_key,
                encrypted_secret: r.encrypted_secret,
                passphrase: r.passphrase,
                is_active: r.is_active,
                created_at: r.created_at,
            }
        }
    }

    struct InMemoryApiKeyRepository {
        keys: Mutex<Vec<NewApiKey>>,
    }

    impl InMemoryApiKeyRepository {
        fn new() -> Self {
            Self {
                keys: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ApiKeyRepository for InMemoryApiKeyRepository {
        async fn insert(&self, row: &NewApiKey) -> Result<i64, RepoError> {
            let mut keys = self.keys.lock();
            keys.push(row.clone());
            Ok(keys.len() as i64)
        }

        async fn find_all(&self, user_id: i64) -> Result<Vec<ApiKeyRecord>, RepoError> {
            let keys = self.keys.lock();
            Ok(keys
                .iter()
                .enumerate()
                .filter(|(_, k)| k.user_id == user_id)
                .map(|(i, k)| ApiKeyRecordClone {
                    id: i as i64 + 1,
                    user_id: k.user_id,
                    exchange: k.exchange.clone(),
                    api_key: k.api_key.clone(),
                    encrypted_secret: k.encrypted_secret.clone(),
                    passphrase: k.passphrase.clone(),
                    is_active: true,
                    created_at: Utc::now(),
                })
                .map(ApiKeyRecord::from)
                .collect())
        }

        async fn find_by_exchange(
            &self,
            user_id: i64,
            exchange: &str,
        ) -> Result<Vec<ApiKeyRecord>, RepoError> {
            let keys = self.keys.lock();
            Ok(keys
                .iter()
                .enumerate()
                .filter(|(_, k)| k.user_id == user_id && k.exchange == exchange)
                .map(|(i, k)| ApiKeyRecordClone {
                    id: i as i64 + 1,
                    user_id: k.user_id,
                    exchange: k.exchange.clone(),
                    api_key: k.api_key.clone(),
                    encrypted_secret: k.encrypted_secret.clone(),
                    passphrase: k.passphrase.clone(),
                    is_active: true,
                    created_at: Utc::now(),
                })
                .map(ApiKeyRecord::from)
                .collect())
        }

        async fn set_active(&self, id: i64, active: bool) -> Result<bool, RepoError> {
            let _ = (id, active);
            Ok(false)
        }
    }

    #[test]
    fn test_mask_short_value() {
        assert_eq!(mask("secret"), "****");
    }

    #[test]
    fn test_mask_long_value() {
        assert_eq!(mask("abcdefghij"), "abcd...ghij");
    }

    #[tokio::test]
    async fn test_save_api_key_encrypts_secret() {
        let repo = Arc::new(InMemoryApiKeyRepository::new());
        let svc = ApiKeyService::new(Some("test_encryption_key".to_string()), Some(repo.clone()));
        svc.save_api_key(
            1,
            "BINANCE",
            "pub-key-12345",
            "super-secret",
            Some("pp".to_string()),
        )
        .await
        .unwrap();

        let keys = repo.keys.lock();
        assert_eq!(keys.len(), 1);
        // Secret is encrypted, never stored raw.
        assert_ne!(keys[0].encrypted_secret, "super-secret");
        assert!(keys[0].encrypted_secret.len() > 10);
    }

    #[tokio::test]
    async fn test_get_api_keys_redacts_secret() {
        let repo = Arc::new(InMemoryApiKeyRepository::new());
        let svc = ApiKeyService::new(Some("test_encryption_key".to_string()), Some(repo.clone()));
        svc.save_api_key(
            1,
            "BINANCE",
            "pub-key-12345",
            "super-secret",
            Some("pp".to_string()),
        )
        .await
        .unwrap();

        let keys = svc.get_api_keys(1).await.unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].api_key, "pub-...2345");
        // No secret field is exposed.
        assert_eq!(keys[0].passphrase, Some("****".to_string()));
    }

    #[tokio::test]
    async fn test_get_decrypted_secret_roundtrips() {
        let repo = Arc::new(InMemoryApiKeyRepository::new());
        let svc = ApiKeyService::new(Some("test_encryption_key".to_string()), Some(repo.clone()));
        svc.save_api_key(
            1,
            "BINANCE",
            "pub-key-12345",
            "super-secret",
            Some("pp".to_string()),
        )
        .await
        .unwrap();

        let secret = svc
            .get_decrypted_secret(1, "BINANCE", "pub-key-12345")
            .await
            .unwrap();
        assert_eq!(secret, "super-secret");
    }
}
