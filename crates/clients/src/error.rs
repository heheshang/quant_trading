use thiserror::Error;

/// Client-layer errors for external service calls.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("cache operation failed: {0}")]
    Cache(String),

    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("time-series database error: {0}")]
    TimeSeries(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("internal error: {0}")]
    Internal(String),
}
