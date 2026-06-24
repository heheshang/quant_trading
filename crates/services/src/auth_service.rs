use crate::error::{ServiceError, ServiceResult};
use quant_common::config::AppConfig;
use quant_repository::PostgresClient;
use security::encryption::PasswordHasher;
use security::AuthService as SecAuthService;
use sqlx::Row;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, instrument};

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

    #[instrument(skip(self, password), fields(username = %username))]
    pub async fn login(&self, username: &str, password: &str) -> ServiceResult<String> {
        let cfg = self.config.read().await;
        let auth_service = self.make_auth_service(&cfg);
        drop(cfg);

        if let Some(ref client) = self.postgres {
            let pool = client.pool();
            let row =
                sqlx::query("SELECT user_id, role, password_hash FROM users WHERE username = $1")
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

            let user_id: String = row.get("user_id");
            let role: String = row.get("role");
            let stored_hash: String = row.get("password_hash");

            let valid = PasswordHasher::verify_password(password, &stored_hash)
                .map_err(|e| {
                    error!("Password verification error: {}", e);
                    ServiceError::PasswordVerification(e.to_string())
                })?;
            if !valid {
                error!("Password mismatch for user: {}", username);
                return Err(ServiceError::InvalidCredentials);
            }

            let token = auth_service
                .generate_token(&user_id, username, vec![role])
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

    pub async fn verify_token(&self, token: &str) -> bool {
        let cfg = self.config.read().await;
        let auth_service = self.make_auth_service(&cfg);
        drop(cfg);
        auth_service.verify_token(token).is_ok()
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

        let valid = PasswordHasher::verify_password(current_password, &stored_hash)
            .map_err(|e| {
                error!("Password verification error: {}", e);
                ServiceError::PasswordVerification(e.to_string())
            })?;
        if !valid {
            error!("Current password incorrect for user: {}", username);
            return Err(ServiceError::Other("Current password is incorrect".into()));
        }

        let new_hash = PasswordHasher::hash_password(new_password)
            .map_err(|e| {
                error!("Password hash failed: {}", e);
                ServiceError::PasswordHash(e.to_string())
            })?;
        sqlx::query("UPDATE users SET password_hash = $1 WHERE username = $2")
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_login_without_db_returns_error() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let auth_service = AuthService::new(config, None);
        let result = auth_service.login("admin", "admin123").await;
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
        let result = auth_service.login("admin", "wrong_password").await;
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
}
