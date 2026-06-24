use super::{Migration, MigrationRecord};
use quant_common::{Error, Result};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Migration Manager - handles database schema migrations
pub struct MigrationManager {
    pool: Arc<PgPool>,
    migrations: Vec<Arc<dyn Migration>>,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            migrations: Vec::new(),
        }
    }

    /// Add a migration to the manager
    pub fn add_migration(&mut self, migration: Arc<dyn Migration>) {
        self.migrations.push(migration);
    }

    /// Initialize the migrations table
    pub async fn init(&self) -> Result<()> {
        info!("Initializing migrations table");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS migrations (
                id SERIAL PRIMARY KEY,
                version INTEGER NOT NULL UNIQUE,
                name VARCHAR(255) NOT NULL,
                applied_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create migrations table: {}", e)))?;

        info!("Migrations table initialized successfully");
        Ok(())
    }

    /// Get current database version
    pub async fn get_current_version(&self) -> Result<i32> {
        let result =
            sqlx::query_scalar::<_, i32>("SELECT COALESCE(MAX(version), 0) FROM migrations")
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| Error::Database(format!("Failed to get current version: {}", e)))?;

        Ok(result)
    }

    /// Get all applied migrations
    pub async fn get_applied_migrations(&self) -> Result<Vec<MigrationRecord>> {
        let records = sqlx::query_as::<_, MigrationRecord>(
            "SELECT id, version, name, applied_at FROM migrations ORDER BY version ASC",
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to fetch migrations: {}", e)))?;

        Ok(records)
    }

    /// Check if a specific version is applied
    pub async fn is_version_applied(&self, version: i32) -> Result<bool> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM migrations WHERE version = $1")
                .bind(version)
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| Error::Database(format!("Failed to check version: {}", e)))?;

        Ok(count > 0)
    }

    /// Record a migration as applied
    async fn record_migration(&self, migration: &dyn Migration) -> Result<()> {
        sqlx::query("INSERT INTO migrations (version, name) VALUES ($1, $2)")
            .bind(migration.version())
            .bind(migration.name())
            .execute(&*self.pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to record migration: {}", e)))?;

        Ok(())
    }

    /// Remove a migration record
    async fn remove_migration(&self, version: i32) -> Result<()> {
        sqlx::query("DELETE FROM migrations WHERE version = $1")
            .bind(version)
            .execute(&*self.pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to remove migration: {}", e)))?;

        Ok(())
    }

    /// Run all pending migrations
    pub async fn migrate(&self) -> Result<()> {
        self.init().await?;

        let current_version = self.get_current_version().await?;
        info!("Current database version: {}", current_version);

        // Sort migrations by version
        let mut sorted_migrations = self.migrations.clone();
        sorted_migrations.sort_by_key(|m| m.version());

        let mut applied_count = 0;

        for migration in sorted_migrations {
            let version = migration.version();

            if version <= current_version {
                continue;
            }

            info!("Applying migration {}: {}", version, migration.name());

            // Start transaction
            let tx = self
                .pool
                .begin()
                .await
                .map_err(|e| Error::Database(format!("Failed to start transaction: {}", e)))?;

            // Run migration
            match migration.up(&*self.pool).await {
                Ok(_) => {
                    info!("Migration {} applied successfully", version);
                }
                Err(e) => {
                    error!("Migration {} failed: {}", version, e);
                    tx.rollback()
                        .await
                        .map_err(|e| Error::Database(format!("Rollback failed: {}", e)))?;
                    return Err(Error::Database(format!(
                        "Migration {} failed: {}",
                        version, e
                    )));
                }
            }

            // Record migration
            self.record_migration(migration.as_ref()).await?;

            // Commit transaction
            tx.commit()
                .await
                .map_err(|e| Error::Database(format!("Failed to commit transaction: {}", e)))?;

            applied_count += 1;
        }

        if applied_count > 0 {
            info!("Successfully applied {} migration(s)", applied_count);
        } else {
            info!("No pending migrations");
        }

        Ok(())
    }

    /// Rollback to a specific version
    pub async fn rollback_to(&self, target_version: i32) -> Result<()> {
        let current_version = self.get_current_version().await?;

        if target_version >= current_version {
            warn!(
                "Target version {} is >= current version {}, nothing to rollback",
                target_version, current_version
            );
            return Ok(());
        }

        info!(
            "Rolling back from version {} to version {}",
            current_version, target_version
        );

        // Sort migrations by version in descending order
        let mut sorted_migrations = self.migrations.clone();
        sorted_migrations.sort_by_key(|m| std::cmp::Reverse(m.version()));

        for migration in sorted_migrations {
            let version = migration.version();

            if version <= target_version {
                break;
            }

            if !self.is_version_applied(version).await? {
                continue;
            }

            info!("Rolling back migration {}: {}", version, migration.name());

            // Start transaction
            let tx = self
                .pool
                .begin()
                .await
                .map_err(|e| Error::Database(format!("Failed to start transaction: {}", e)))?;

            // Run rollback
            match migration.down(&*self.pool).await {
                Ok(_) => {
                    info!("Migration {} rolled back successfully", version);
                }
                Err(e) => {
                    error!("Rollback of migration {} failed: {}", version, e);
                    tx.rollback()
                        .await
                        .map_err(|e| Error::Database(format!("Rollback failed: {}", e)))?;
                    return Err(Error::Database(format!("Rollback failed: {}", e)));
                }
            }

            // Remove migration record
            self.remove_migration(version).await?;

            // Commit transaction
            tx.commit()
                .await
                .map_err(|e| Error::Database(format!("Failed to commit transaction: {}", e)))?;
        }

        info!("Rollback completed successfully");
        Ok(())
    }

    /// Get pending migrations
    pub async fn get_pending_migrations(&self) -> Result<Vec<Arc<dyn Migration>>> {
        let current_version = self.get_current_version().await?;

        let pending: Vec<_> = self
            .migrations
            .iter()
            .filter(|m| m.version() > current_version)
            .cloned()
            .collect();

        Ok(pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock migration for testing
    struct MockMigration {
        version: i32,
        name: String,
        should_fail: bool,
    }

    impl MockMigration {
        fn new(version: i32, name: &str) -> Self {
            Self {
                version,
                name: name.to_string(),
                should_fail: false,
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }
    }

    #[async_trait::async_trait]
    impl Migration for MockMigration {
        fn version(&self) -> i32 {
            self.version
        }

        fn name(&self) -> &str {
            &self.name
        }

        async fn up(&self, _pool: &PgPool) -> Result<()> {
            if self.should_fail {
                Err(Error::Database("Migration failed".to_string()))
            } else {
                Ok(())
            }
        }

        async fn down(&self, _pool: &PgPool) -> Result<()> {
            if self.should_fail {
                Err(Error::Database("Rollback failed".to_string()))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_migration_manager_creation() {
        // Since we can't create a real PgPool in unit tests without a database,
        // we'll test the structure
        let migrations = vec![
            MockMigration::new(1, "first_migration"),
            MockMigration::new(2, "second_migration"),
        ];

        assert_eq!(migrations[0].version(), 1);
        assert_eq!(migrations[0].name(), "first_migration");
        assert_eq!(migrations[1].version(), 2);
        assert_eq!(migrations[1].name(), "second_migration");
    }

    #[test]
    fn test_migration_sorting() {
        let migrations = vec![
            Arc::new(MockMigration::new(3, "third")) as Arc<dyn Migration>,
            Arc::new(MockMigration::new(1, "first")) as Arc<dyn Migration>,
            Arc::new(MockMigration::new(2, "second")) as Arc<dyn Migration>,
        ];

        let mut sorted = migrations.clone();
        sorted.sort_by_key(|m| m.version());

        assert_eq!(sorted[0].version(), 1);
        assert_eq!(sorted[1].version(), 2);
        assert_eq!(sorted[2].version(), 3);
    }

    #[test]
    fn test_migration_filtering() {
        let current_version = 2;
        let migrations = vec![
            Arc::new(MockMigration::new(1, "first")) as Arc<dyn Migration>,
            Arc::new(MockMigration::new(2, "second")) as Arc<dyn Migration>,
            Arc::new(MockMigration::new(3, "third")) as Arc<dyn Migration>,
        ];

        let pending: Vec<_> = migrations
            .iter()
            .filter(|m| m.version() > current_version)
            .collect();

        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].version(), 3);
    }
}
