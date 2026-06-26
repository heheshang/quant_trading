use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use quant_common::{Error, Result};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, warn};

/// JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,        // 用户 ID
    pub username: String,   // 用户名
    pub exp: i64,           // 过期时间
    pub iat: i64,           // 签发时间
    pub jti: String,        // JWT ID
    pub roles: Vec<String>, // 用户角色
}

/// 认证服务
pub struct AuthService {
    jwt_secret: String,
    token_expiry_hours: i64,
}

impl AuthService {
    pub fn new(jwt_secret: String, token_expiry_hours: i64) -> Self {
        Self {
            jwt_secret,
            token_expiry_hours,
        }
    }

    /// 生成 JWT Token
    #[instrument(skip(self, roles))]
    pub fn generate_token(
        &self,
        user_id: &str,
        username: &str,
        roles: Vec<String>,
    ) -> Result<String> {
        info!(user = %username, "generating JWT token");
        let now = Utc::now();
        let exp = now + Duration::hours(self.token_expiry_hours);

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            roles,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            error!(error = %e, "token generation failed");
            Error::Auth(format!("Token generation failed: {}", e))
        })
    }

    /// 验证 JWT Token
    #[instrument(skip(self, token))]
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        info!("verifying JWT token");
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| {
            warn!(error = %e, "token verification failed");
            Error::Auth(format!("Token verification failed: {}", e))
        })
    }

    /// 检查权限
    #[instrument(skip(self, claims))]
    pub fn check_permission(&self, claims: &Claims, required_role: &str) -> Result<()> {
        if claims.roles.contains(&required_role.to_string())
            || claims.roles.contains(&"admin".to_string())
        {
            Ok(())
        } else {
            warn!(
                user = %claims.username,
                required_role = %required_role,
                "permission denied"
            );
            Err(Error::Permission(format!(
                "Required role: {}",
                required_role
            )))
        }
    }

    /// 刷新 Token
    #[instrument(skip(self, old_token))]
    pub fn refresh_token(&self, old_token: &str) -> Result<String> {
        let claims = self.verify_token(old_token)?;
        self.generate_token(&claims.sub, &claims.username, claims.roles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_auth() {
        let auth = AuthService::new("test_secret".to_string(), 24);

        let token = auth
            .generate_token("user123", "testuser", vec!["trader".to_string()])
            .unwrap();

        let claims = auth.verify_token(&token).unwrap();
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.roles, vec!["trader".to_string()]);
    }

    #[test]
    fn test_permission_check() {
        let auth = AuthService::new("test_secret".to_string(), 24);

        let claims = Claims {
            sub: "user123".to_string(),
            username: "testuser".to_string(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            roles: vec!["trader".to_string()],
        };

        assert!(auth.check_permission(&claims, "trader").is_ok());
        assert!(auth.check_permission(&claims, "admin").is_err());
    }
}
