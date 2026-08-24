use crate::error::{ServiceError, ServiceResult};
use quant_common::config::AppConfig;
use quant_repository::PostgresClient;
use security::encryption::{DataEncryption, PasswordHasher};
use security::totp::{generate_totp_secret, verify_totp};
use security::AuthService as SecAuthService;
use sqlx::Row;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, instrument, warn};

/// Result of starting the two-factor authentication enable flow.
///
/// The raw `secret` is what the QR code / manual entry needs; `encrypted_secret`
/// is an AES-encrypted copy that may be rendered for display; `otpauth_uri` is
/// the ready-to-encode `otpauth://totp/...` string a QR generator can consume.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Enable2faResult {
    pub secret: String,
    pub encrypted_secret: String,
    pub otpauth_uri: String,
}

/// Authentication and user management service.
pub struct AuthService {
    config: Arc<RwLock<AppConfig>>,
    postgres: Option<Arc<PostgresClient>>,
}

impl AuthService {
    pub fn new(config: Arc<RwLock<AppConfig>>, postgres: Option<Arc<PostgresClient>>) -> Self {
        Self { config, postgres }
    }

    fn make_auth_service(&self, config: &AppConfig) -> SecAuthService {
        SecAuthService::new(
            config.security.jwt_secret.clone(),
            config.security.token_expiry_hours as i64,
        )
    }

    #[instrument(skip(self, password, code), fields(username = %username))]
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        code: Option<&str>,
    ) -> ServiceResult<String> {
        let cfg = self.config.read().await;
        let auth_service = self.make_auth_service(&cfg);
        drop(cfg);

        if let Some(ref client) = self.postgres {
            let pool = client.pool();
            let row = sqlx::query(
                "SELECT user_id, role, password_hash, token_version, totp_enabled, totp_secret \
                 FROM users WHERE username = $1",
            )
            .bind(username)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                error!("Login DB query failed: {}", e);
                ServiceError::from(e)
            })?
            .ok_or_else(|| {
                error!("Invalid credentials for user: {}", username);
                ServiceError::InvalidCredentials
            })?;

            let user_id: i64 = row.get("user_id");
            let role: String = row.get("role");
            let stored_hash: String = row.get("password_hash");
            let token_version: i64 = row.get("token_version");
            let totp_enabled: bool = row.get("totp_enabled");
            let totp_secret: Option<String> = row.get("totp_secret");

            let valid = PasswordHasher::verify_password(password, &stored_hash).map_err(|e| {
                error!("Password verification error: {}", e);
                ServiceError::PasswordVerification(e.to_string())
            })?;
            if !valid {
                error!("Password mismatch for user: {}", username);
                return Err(ServiceError::InvalidCredentials);
            }

            // 2FA gate: once a user has TOTP enabled, a valid code is required
            // before a session token is issued.
            if totp_enabled {
                let secret = totp_secret.ok_or_else(|| {
                    error!(
                        user_id = %user_id,
                        "totp_enabled is set but no secret is provisioned"
                    );
                    ServiceError::TwoFactorRequired
                })?;
                let code = code.ok_or(ServiceError::TwoFactorRequired)?;
                if !verify_totp(&secret, code, 1) {
                    warn!(user_id = %user_id, "Invalid 2FA code during login");
                    return Err(ServiceError::TwoFactorInvalid);
                }
            }

            let token = auth_service
                .generate_token_with_version(user_id, username, vec![role], Some(token_version))
                .map_err(|e| {
                    error!("Token generation failed: {}", e);
                    ServiceError::TokenGeneration(e.to_string())
                })?;
            info!(username = %username, "User logged in successfully");
            Ok(token)
        } else {
            error!("Database not connected during login");
            Err(ServiceError::DatabaseNotConnected)
        }
    }

    /// Resolve a user's numeric id from their username (for audit attribution).
    #[instrument(skip(self), fields(username = %username))]
    pub async fn resolve_user_id(&self, username: &str) -> ServiceResult<Option<i64>> {
        let Some(client) = &self.postgres else {
            return Ok(None);
        };
        let pool = client.pool();
        let row = sqlx::query("SELECT user_id FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                error!(username = %username, error = %e, "Failed to resolve user id");
                ServiceError::from(e)
            })?;
        Ok(row.map(|r| r.get("user_id")))
    }

    pub async fn verify_token(&self, token: &str) -> bool {
        let cfg = self.config.read().await;
        let auth_service = self.make_auth_service(&cfg);
        drop(cfg);

        match auth_service.verify_token(token) {
            Ok(claims) => {
                let Some(ref client) = self.postgres else {
                    return true;
                };

                let pool = client.pool();
                let row = sqlx::query("SELECT token_version FROM users WHERE username = $1")
                    .bind(&claims.username)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        error!(username = %claims.username, error = %e, "Token version lookup failed");
                        e
                    })
                    .ok();

                match row {
                    Some(Some(row)) => {
                        let current_version: i64 = row.get("token_version");
                        current_version == claims.version
                    }
                    Some(None) | None => false,
                }
            }
            Err(_) => false,
        }
    }

    #[instrument(skip(self), fields(username = %username))]
    pub async fn get_user_profile(&self, username: &str) -> ServiceResult<serde_json::Value> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let row = sqlx::query(
            r#"
            SELECT username, email, phone, full_name, company, address,
                   created_at, last_login
            FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch user profile for {}: {}", username, e);
            ServiceError::from(e)
        })?
        .ok_or_else(|| {
            error!("User not found: {}", username);
            ServiceError::NotFound("User not found".into())
        })?;

        let username: String = row.get("username");
        let email: Option<String> = row.get("email");
        let phone: Option<String> = row.get("phone");
        let full_name: Option<String> = row.get("full_name");
        let company: Option<String> = row.get("company");
        let address: Option<String> = row.get("address");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let last_login: Option<chrono::DateTime<chrono::Utc>> = row.get("last_login");

        let profile = serde_json::json!({
            "username": username,
            "email": email.unwrap_or_default(),
            "phone": phone.unwrap_or_default(),
            "full_name": full_name.unwrap_or_default(),
            "company": company.unwrap_or_default(),
            "address": address.unwrap_or_default(),
            "created_at": created_at.to_rfc3339(),
            "last_login": last_login.map(|t| t.to_rfc3339()).unwrap_or_default(),
        });
        info!(username = %username, "User profile retrieved");
        Ok(profile)
    }

    pub async fn update_profile(
        &self,
        username: &str,
        profile_data: &serde_json::Value,
    ) -> ServiceResult<bool> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let email = profile_data
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let phone = profile_data
            .get("phone")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let full_name = profile_data
            .get("full_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let company = profile_data
            .get("company")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let address = profile_data
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let affected = sqlx::query(
            "UPDATE users SET email=$1, phone=$2, full_name=$3, company=$4, address=$5 WHERE username=$6",
        )
        .bind(email)
        .bind(phone)
        .bind(full_name)
        .bind(company)
        .bind(address)
        .bind(username)
        .execute(pool)
        .await
        .map_err(ServiceError::Database)?;

        Ok(affected.rows_affected() > 0)
    }

    #[instrument(skip(self, current_password, new_password), fields(username = %username))]
    pub async fn change_password(
        &self,
        username: &str,
        current_password: &str,
        new_password: &str,
    ) -> ServiceResult<bool> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let row = sqlx::query("SELECT password_hash FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch user for password change: {}", e);
                ServiceError::from(e)
            })?
            .ok_or_else(|| {
                error!("User not found for password change: {}", username);
                ServiceError::NotFound("User not found".into())
            })?;
        let stored_hash: String = row.get("password_hash");

        let valid =
            PasswordHasher::verify_password(current_password, &stored_hash).map_err(|e| {
                error!("Password verification error: {}", e);
                ServiceError::PasswordVerification(e.to_string())
            })?;
        if !valid {
            error!("Current password incorrect for user: {}", username);
            return Err(ServiceError::Other("Current password is incorrect".into()));
        }

        let new_hash = PasswordHasher::hash_password(new_password).map_err(|e| {
            error!("Password hash failed: {}", e);
            ServiceError::PasswordHash(e.to_string())
        })?;
        sqlx::query(
            "UPDATE users SET password_hash = $1, token_version = COALESCE(token_version, 0) + 1 WHERE username = $2",
        )
        .bind(&new_hash)
        .bind(username)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to update password: {}", e);
            ServiceError::Database(e)
        })?;

        info!(username = %username, "Password changed successfully");
        Ok(true)
    }

    /// Build an AES-256-GCM encryptor from the configured encryption key.
    fn encrypt_secret(&self, secret: &str) -> ServiceResult<String> {
        let key = self
            .config
            .try_read()
            .map(|c| c.security.encryption_key.clone())
            .ok()
            .ok_or_else(|| ServiceError::Other("Encryption key unavailable for 2FA".to_string()))?;
        if key.is_empty() {
            warn!("ENCRYPTION_KEY is empty; returning 2FA secret unencrypted");
            return Ok(secret.to_string());
        }
        let enc = DataEncryption::from_key_string(&key)
            .map_err(|e| ServiceError::Other(e.to_string()))?;
        enc.encrypt_string(secret)
            .map_err(|e| ServiceError::Other(e.to_string()))
    }

    /// Start the 2FA enable flow: provision a TOTP secret and persist it
    /// (pending verification), returning the secret + otpauth URI for the
    /// frontend to render a QR code.
    ///
    /// The account is **not** marked enabled until
    /// [`verify_2fa_code`](Self::verify_2fa_code) successfully validates a
    /// live code.
    #[instrument(skip(self), fields(user_id = %user_id))]
    pub async fn enable_2fa(&self, user_id: i64) -> ServiceResult<Enable2faResult> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        // Look up the user for the otpauth label and existence check.
        let row = sqlx::query("SELECT username, email FROM users WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(ServiceError::Database)?
            .ok_or_else(|| {
                error!("User not found for 2FA enable: {}", user_id);
                ServiceError::NotFound(format!("User '{user_id}' not found"))
            })?;
        let username: String = row.get("username");
        let email: Option<String> = row.get("email");

        let secret = generate_totp_secret().map_err(|e| {
            error!("Failed to generate TOTP secret: {}", e);
            ServiceError::Other(e.to_string())
        })?;

        // Persist the pending secret; reset the enabled flag so re-provisioning
        // forces a fresh verification cycle.
        sqlx::query("UPDATE users SET totp_secret = $1, totp_enabled = FALSE WHERE user_id = $2")
            .bind(&secret)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(ServiceError::Database)?;

        let account = email
            .as_deref()
            .filter(|e| !e.is_empty())
            .unwrap_or(username.as_str());
        let issuer = "QuantTrading";
        let otpauth_uri = format!(
            "otpauth://totp/{issuer}:{account}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits=6&period=30"
        );
        let encrypted_secret = self.encrypt_secret(&secret)?;

        info!(
            user_id = %user_id,
            secret_provisioned = true,
            "2FA enable flow started: secret provisioned, awaiting code verification"
        );
        Ok(Enable2faResult {
            secret,
            encrypted_secret,
            otpauth_uri,
        })
    }

    /// Verify a user-supplied 6-digit TOTP code against the stored secret.
    ///
    /// On success the account is marked enabled (the challenge is solved).
    /// Returns `Ok(false)` when the code is invalid (caller may re-prompt);
    /// returns a `NotFound` error if no secret has been provisioned.
    #[instrument(skip(self, code), fields(user_id = %user_id))]
    pub async fn verify_2fa_code(&self, user_id: i64, code: &str) -> ServiceResult<bool> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let row = sqlx::query("SELECT totp_secret FROM users WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(ServiceError::Database)?
            .ok_or_else(|| {
                error!("User not found for 2FA verify: {}", user_id);
                ServiceError::NotFound(format!("User '{user_id}' not found"))
            })?;
        let secret: Option<String> = row.get("totp_secret");
        let secret = secret.ok_or_else(|| {
            error!("TOTP secret not provisioned for user {}", user_id);
            ServiceError::NotFound("Two-factor authentication is not set up".into())
        })?;

        if !verify_totp(&secret, code, 1) {
            warn!(user_id = %user_id, "Invalid 2FA code during enable/verify");
            return Ok(false);
        }

        sqlx::query("UPDATE users SET totp_enabled = TRUE WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(ServiceError::Database)?;

        info!(user_id = %user_id, "2FA enabled after code verification");
        Ok(true)
    }

    /// Disable 2FA for a user, **only after** the supplied code is verified.
    ///
    /// Returns `Ok(true)` when 2FA was disabled (or was already absent), and
    /// `Ok(false)` when the supplied code is invalid (the disable is refused).
    #[instrument(skip(self, code), fields(user_id = %user_id))]
    pub async fn disable_2fa(&self, user_id: i64, code: &str) -> ServiceResult<bool> {
        let client = self
            .postgres
            .as_ref()
            .ok_or(ServiceError::DatabaseNotConnected)?;
        let pool = client.pool();

        let row = sqlx::query("SELECT totp_secret FROM users WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(ServiceError::Database)?
            .ok_or_else(|| {
                error!("User not found for 2FA disable: {}", user_id);
                ServiceError::NotFound(format!("User '{user_id}' not found"))
            })?;
        let secret: Option<String> = row.get("totp_secret");
        let Some(secret) = secret else {
            // Already disabled / nothing provisioned — idempotent success.
            return Ok(true);
        };

        if !verify_totp(&secret, code, 1) {
            warn!(user_id = %user_id, "Refusing to disable 2FA: invalid code");
            return Ok(false);
        }

        sqlx::query("UPDATE users SET totp_secret = NULL, totp_enabled = FALSE WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(ServiceError::Database)?;

        info!(user_id = %user_id, "2FA disabled after code verification");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::config::DatabaseConfig;
    use quant_repository::PostgresClient;
    use security::encryption::PasswordHasher;

    #[tokio::test]
    async fn test_login_without_db_returns_error() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let result = auth_service.login("admin", "admin123", None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_login_wrong_password_without_db() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let result = auth_service.login("admin", "wrong_password", None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_verify_token_without_db() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let valid = auth_service.verify_token("invalid.jwt.token").await;
        assert!(!valid);
    }

    #[tokio::test]
    async fn test_verify_empty_token() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let valid = auth_service.verify_token("").await;
        assert!(!valid);
    }

    #[tokio::test]
    async fn test_get_user_profile_without_db_returns_error() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let result = auth_service.get_user_profile("admin").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_update_profile_without_db_returns_error() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let profile = serde_json::json!({"email": "test@test.com"});
        let result = auth_service.update_profile("admin", &profile).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_change_password_without_db_returns_error() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let result = auth_service.change_password("admin", "old", "new").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    // ── Two-factor authentication ──────────────────────────────────────────

    #[tokio::test]
    async fn test_enable_2fa_without_db_returns_error() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let result = auth_service.enable_2fa(1).await;
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_verify_2fa_code_without_db_returns_error() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let result = auth_service.verify_2fa_code(1, "123456").await;
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    async fn test_disable_2fa_without_db_returns_error() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let result = auth_service.disable_2fa(1, "123456").await;
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::DatabaseNotConnected
        ));
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL integration environment"]
    async fn test_2fa_enable_verify_disable_round_trip_with_real_db() {
        use security::totp::totp_code;
        use std::time::{SystemTime, UNIX_EPOCH};

        let db_config = DatabaseConfig {
            host: std::env::var("DATABASE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("DATABASE_PORT")
                .unwrap_or_else(|_| "15432".to_string())
                .parse::<u16>()
                .unwrap_or(15432),
            username: std::env::var("DATABASE_USERNAME").unwrap_or_else(|_| "quant".to_string()),
            password: std::env::var("DATABASE_PASSWORD")
                .unwrap_or_else(|_| "quant_password".to_string()),
            database: std::env::var("DATABASE_NAME")
                .unwrap_or_else(|_| "quant_trading".to_string()),
            max_connections: 5,
            connect_timeout_seconds: 5,
        };

        let postgres = PostgresClient::new(&db_config)
            .await
            .expect("expected docker PostgreSQL to be reachable");
        let postgres = Arc::new(postgres);
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, Some(postgres.clone()));
        let pool = postgres.pool();

        // Seed a user and ensure a clean 2FA slate.
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("twofa_flow_{suffix}");
        let email = format!("{username}@example.com");
        let password_hash = PasswordHasher::hash_password("test-password")
            .expect("password hash should be created");
        sqlx::query(
            "INSERT INTO users (username, password_hash, email, role) VALUES ($1, $2, $3, $4)",
        )
        .bind(&username)
        .bind(&password_hash)
        .bind(&email)
        .bind("trader")
        .execute(pool)
        .await
        .expect("user should be inserted for 2FA flow test");
        let user_id: i64 = sqlx::query_scalar("SELECT user_id FROM users WHERE username = $1")
            .bind(&username)
            .fetch_one(pool)
            .await
            .expect("user_id should be read back");

        // Enable → secret provisioned but not yet enabled.
        let enabled = auth_service
            .enable_2fa(user_id)
            .await
            .expect("enable_2fa should succeed");
        assert_eq!(enabled.secret.len(), 32, "secret should be 32 base32 chars");
        assert!(enabled.otpauth_uri.starts_with("otpauth://totp/"));
        assert!(!enabled.encrypted_secret.is_empty());
        let stored: Option<String> =
            sqlx::query_scalar("SELECT totp_secret FROM users WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .expect("stored secret should be readable");
        assert_eq!(stored.as_deref(), Some(enabled.secret.as_str()));
        let enabled_flag: bool =
            sqlx::query_scalar("SELECT totp_enabled FROM users WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .expect("enabled flag should be readable");
        assert!(
            !enabled_flag,
            "account must not be enabled before code verification"
        );

        // A wrong code fails (and does NOT enable).
        assert!(!auth_service
            .verify_2fa_code(user_id, "000000")
            .await
            .expect("verify should return Ok(false) on bad code"));
        let still_disabled: bool =
            sqlx::query_scalar("SELECT totp_enabled FROM users WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .expect("enabled flag should be readable");
        assert!(!still_disabled);

        // The correct live code succeeds and marks the account enabled.
        let step = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 30;
        let valid_code = totp_code(&enabled.secret, step).expect("code should be computed");
        assert!(auth_service
            .verify_2fa_code(user_id, &valid_code)
            .await
            .expect("verify should succeed"));

        // A wrong code cannot disable an enabled account.
        assert!(!auth_service
            .disable_2fa(user_id, "000000")
            .await
            .expect("disable should return Ok(false) on bad code"));
        let no_longer_disabled: bool =
            sqlx::query_scalar("SELECT totp_enabled FROM users WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .expect("enabled flag should be readable");
        assert!(no_longer_disabled);

        // A valid code disables 2FA and clears the secret.
        let step2 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 30;
        let disable_code = totp_code(&enabled.secret, step2).expect("code should be computed");
        assert!(auth_service
            .disable_2fa(user_id, &disable_code)
            .await
            .expect("disable should succeed"));
        let cleared: Option<String> =
            sqlx::query_scalar("SELECT totp_secret FROM users WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .expect("stored secret should be readable");
        assert!(cleared.is_none(), "secret should be cleared after disable");

        sqlx::query("DELETE FROM users WHERE username = $1")
            .bind(&username)
            .execute(pool)
            .await
            .expect("temporary user should be cleaned up");
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL integration environment"]
    async fn test_login_change_password_invalidates_old_token_with_real_db() {
        let db_config = DatabaseConfig {
            host: std::env::var("DATABASE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("DATABASE_PORT")
                .unwrap_or_else(|_| "15432".to_string())
                .parse::<u16>()
                .unwrap_or(15432),
            username: std::env::var("DATABASE_USERNAME").unwrap_or_else(|_| "quant".to_string()),
            password: std::env::var("DATABASE_PASSWORD")
                .unwrap_or_else(|_| "quant_password".to_string()),
            database: std::env::var("DATABASE_NAME")
                .unwrap_or_else(|_| "quant_trading".to_string()),
            max_connections: 5,
            connect_timeout_seconds: 5,
        };

        let postgres = PostgresClient::new(&db_config)
            .await
            .expect("expected docker PostgreSQL to be reachable");
        let postgres = Arc::new(postgres);

        let mut config = AppConfig::default();
        config.security.jwt_secret = "docker-test-secret".to_string();
        config.security.token_expiry_hours = 24;
        let config = Arc::new(RwLock::new(config));

        let auth_service = AuthService::new(config, Some(postgres.clone()));
        let pool = postgres.pool();

        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("auth_flow_{suffix}");
        let email = format!("{username}@example.com");
        let initial_password = "old-password";
        let new_password = "new-password";
        let password_hash = PasswordHasher::hash_password(initial_password)
            .expect("password hash should be created");

        sqlx::query(
            "INSERT INTO users (username, password_hash, email, role) VALUES ($1, $2, $3, $4)",
        )
        .bind(&username)
        .bind(&password_hash)
        .bind(&email)
        .bind("trader")
        .execute(pool)
        .await
        .expect("user should be inserted for auth flow test");

        let old_token = auth_service
            .login(&username, initial_password, None)
            .await
            .expect("login should succeed for seeded user");
        assert!(auth_service.verify_token(&old_token).await);

        let password_changed = auth_service
            .change_password(&username, initial_password, new_password)
            .await
            .expect("password change should succeed");
        assert!(password_changed);

        assert!(!auth_service.verify_token(&old_token).await);

        let new_token = auth_service
            .login(&username, new_password, None)
            .await
            .expect("login should succeed with the updated password");
        assert!(auth_service.verify_token(&new_token).await);

        sqlx::query("DELETE FROM users WHERE username = $1")
            .bind(&username)
            .execute(pool)
            .await
            .expect("temporary user should be cleaned up");
    }
    #[tokio::test]
    #[ignore = "requires PostgreSQL integration environment"]
    async fn test_login_enforces_2fa_with_real_db() {
        use security::totp::totp_code;
        use std::time::{SystemTime, UNIX_EPOCH};

        let db_config = DatabaseConfig {
            host: std::env::var("DATABASE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("DATABASE_PORT")
                .unwrap_or_else(|_| "15432".to_string())
                .parse::<u16>()
                .unwrap_or(15432),
            username: std::env::var("DATABASE_USERNAME").unwrap_or_else(|_| "quant".to_string()),
            password: std::env::var("DATABASE_PASSWORD")
                .unwrap_or_else(|_| "quant_password".to_string()),
            database: std::env::var("DATABASE_NAME")
                .unwrap_or_else(|_| "quant_trading".to_string()),
            max_connections: 5,
            connect_timeout_seconds: 5,
        };

        let postgres = PostgresClient::new(&db_config)
            .await
            .expect("expected docker PostgreSQL to be reachable");
        let postgres = Arc::new(postgres);
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, Some(postgres.clone()));
        let pool = postgres.pool();

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("twofa_login_{suffix}");
        let email = format!("{username}@example.com");
        let password_hash = PasswordHasher::hash_password("test-password")
            .expect("password hash should be created");
        sqlx::query(
            "INSERT INTO users (username, password_hash, email, role) VALUES ($1, $2, $3, $4)",
        )
        .bind(&username)
        .bind(&password_hash)
        .bind(&email)
        .bind("trader")
        .execute(pool)
        .await
        .expect("user should be inserted for 2FA login test");
        let user_id: i64 = sqlx::query_scalar("SELECT user_id FROM users WHERE username = $1")
            .bind(&username)
            .fetch_one(pool)
            .await
            .expect("user_id should be read back");

        // Non-2FA user logs in without a code.
        assert!(
            auth_service
                .login(&username, "test-password", None)
                .await
                .is_ok(),
            "2FA-disabled user should log in without a code"
        );

        // Enable 2FA for the user.
        let enabled = auth_service
            .enable_2fa(user_id)
            .await
            .expect("enable_2fa should succeed");
        let step = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 30;
        let verify_code = totp_code(&enabled.secret, step).expect("code should be computed");
        assert!(auth_service
            .verify_2fa_code(user_id, &verify_code)
            .await
            .expect("verify_2fa_code should succeed"));

        // Missing code → TwoFactorRequired.
        let err = auth_service
            .login(&username, "test-password", None)
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::TwoFactorRequired));

        // Wrong code → TwoFactorInvalid.
        let err = auth_service
            .login(&username, "test-password", Some("000000"))
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::TwoFactorInvalid));

        // Correct code → success; token verifies.
        let step2 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 30;
        let login_code = totp_code(&enabled.secret, step2).expect("code should be computed");
        let token = auth_service
            .login(&username, "test-password", Some(&login_code))
            .await
            .expect("login with valid 2FA code should succeed");
        assert!(auth_service.verify_token(&token).await);

        sqlx::query("DELETE FROM users WHERE username = $1")
            .bind(&username)
            .execute(pool)
            .await
            .expect("temporary user should be cleaned up");
    }
}
