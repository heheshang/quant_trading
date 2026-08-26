use thiserror::Error;

/// Repository-layer errors.
#[derive(Debug, Clone, Error)]
pub enum RepoError {
    #[error("database error: {0}")]
    Database(String),

    #[error("record not found: {entity} id={id}")]
    NotFound { entity: &'static str, id: String },

    #[error("optimistic lock conflict: {entity} id={id}, version={version}")]
    VersionConflict {
        entity: &'static str,
        id: String,
        version: i32,
    },

    #[error("migration error: {0}")]
    Migration(String),
}

impl From<sqlx::Error> for RepoError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}
