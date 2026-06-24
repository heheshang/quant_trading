use thiserror::Error;
pub type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Database not connected")]
    DatabaseNotConnected,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("{0}")]
    NotFound(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token generation failed: {0}")]
    TokenGeneration(String),

    #[error("Password verification error: {0}")]
    PasswordVerification(String),

    #[error("Failed to hash password: {0}")]
    PasswordHash(String),

    #[error("Failed to deserialize {field}: {source}")]
    Deserialization {
        field: &'static str,
        #[source]
        source: serde_json::Error,
    },

    #[error("Failed to serialize {what}: {source}")]
    Serialization {
        what: &'static str,
        #[source]
        source: serde_json::Error,
    },

    #[error("OKX client not initialized")]
    OkxNotInitialized,

    #[error("OKX executor not initialized")]
    OkxExecutorNotInitialized,

    #[error("OKX data source not initialized")]
    OkxDataSourceNotInitialized,

    #[error("OKX API error: {0}")]
    OkxApi(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Backtest error: {0}")]
    Backtest(String),

    #[error("Data source error: {0}")]
    DataSource(String),

    #[error("{0}")]
    Other(String),
}
