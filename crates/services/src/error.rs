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

    #[error("Binance API error: {0}")]
    BinanceApi(String),

    #[error("Binance client not initialized")]
    BinanceNotInitialized,

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Backtest error: {0}")]
    Backtest(String),

    #[error("Data source error: {0}")]
    DataSource(String),

    #[error("Invalid status transition: {from} → {to}")]
    InvalidStatusTransition { from: String, to: String },

    #[error(
        "Concurrent modification detected for strategy {strategy_id}: expected status {expected:?}"
    )]
    ConcurrentModification {
        strategy_id: String,
        expected: quant_common::types::StrategyStatus,
    },

    #[error("Pagination invalid: {reason}")]
    PaginationInvalid { reason: String },

    #[error("Strategy already running: {0}")]
    StrategyAlreadyRunning(String),

    #[error("Scheduler error: {0}")]
    Scheduler(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Service not initialized: {0}")]
    NotInitialized(String),

    #[error("Validation failed: {field} - {reason}")]
    Validation { field: String, reason: String },

    #[error("{0}")]
    Other(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Repository error: {0}")]
    Repository(String),
}

impl From<quant_repository::RepoError> for ServiceError {
    /// Map repository-layer errors to typed service errors.
    ///
    /// The goal is to preserve the typed error so callers can `match` on the
    /// specific failure mode (e.g. retry on `Conflict`, surface a 404 on
    /// `NotFound`) instead of having every repository failure degrade into
    /// the opaque `ServiceError::Other(String)`.
    fn from(err: quant_repository::RepoError) -> Self {
        use quant_repository::RepoError as R;
        match err {
            R::NotFound { entity, id } => Self::NotFound(format!("{entity} '{id}' not found")),
            R::VersionConflict {
                entity,
                id,
                version,
            } => {
                tracing::warn!(entity, id, version, "repo version conflict");
                Self::Conflict(format!("{entity} '{id}' version conflict (v{version})"))
            }
            R::Database(msg) => {
                tracing::error!(error = %msg, "repo database error");
                Self::Repository(msg)
            }
            R::Migration(msg) => {
                tracing::error!(error = %msg, "repo migration error");
                Self::Repository(format!("migration error: {msg}"))
            }
        }
    }
}

impl From<quant_domain::types::StrategyError> for ServiceError {
    fn from(e: quant_domain::types::StrategyError) -> Self {
        match e {
            quant_domain::types::StrategyError::InvalidTransition { from, to } => {
                Self::InvalidStatusTransition {
                    from: format!("{:?}", from),
                    to: format!("{:?}", to),
                }
            }
        }
    }
}

impl From<strategy_engine::registry::FactoryError> for ServiceError {
    /// Map factory/registry errors to typed service errors.
    ///
    /// Preserves the three-way split (`UnknownType` / `InvalidParameters` /
    /// `Initialize`) so service callers can `match` on the specific failure
    /// mode instead of degrading everything to a string.
    fn from(err: strategy_engine::registry::FactoryError) -> Self {
        use strategy_engine::registry::FactoryError as F;
        match err {
            F::UnknownType(name) => Self::NotFound(format!("Unknown strategy type '{name}'")),
            F::InvalidParameters(msg) => Self::Validation {
                field: "strategy_params".to_string(),
                reason: msg,
            },
            F::Initialize(msg) => Self::Strategy(format!("factory initialize failed: {msg}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_repository::RepoError;

    #[test]
    fn from_repo_error_not_found_maps_to_service_not_found() {
        let repo_err = RepoError::NotFound {
            entity: "strategy",
            id: "strat-123".to_string(),
        };
        let svc_err: ServiceError = repo_err.into();
        assert!(
            matches!(svc_err, ServiceError::NotFound(_)),
            "expected NotFound, got: {svc_err:?}"
        );
    }

    #[test]
    fn from_repo_error_version_conflict_maps_to_conflict() {
        let repo_err = RepoError::VersionConflict {
            entity: "strategy",
            id: "strat-123".to_string(),
            version: 5,
        };
        let svc_err: ServiceError = repo_err.into();
        assert!(
            matches!(svc_err, ServiceError::Conflict(_)),
            "expected Conflict, got: {svc_err:?}"
        );
    }

    #[test]
    fn from_repo_error_database_maps_to_repository() {
        let repo_err = RepoError::Database("connection lost".to_string());
        let svc_err: ServiceError = repo_err.into();
        assert!(
            matches!(svc_err, ServiceError::Repository(_)),
            "expected Repository, got: {svc_err:?}"
        );
    }

    #[test]
    fn from_repo_error_migration_maps_to_repository() {
        let repo_err = RepoError::Migration("schema mismatch".to_string());
        let svc_err: ServiceError = repo_err.into();
        assert!(
            matches!(svc_err, ServiceError::Repository(_)),
            "expected Repository, got: {svc_err:?}"
        );
    }
}
