pub mod manager;
pub mod migrations;

pub use manager::MigrationManager;

use quant_common::Result;

/// Migration trait - all migrations must implement this
#[async_trait::async_trait]
pub trait Migration: Send + Sync {
    /// Migration version number (e.g., 1, 2, 3)
    fn version(&self) -> i32;
    
    /// Migration name/description
    fn name(&self) -> &str;
    
    /// Execute the migration (upgrade)
    async fn up(&self, pool: &sqlx::PgPool) -> Result<()>;
    
    /// Rollback the migration (downgrade)
    async fn down(&self, pool: &sqlx::PgPool) -> Result<()>;
}

/// Migration record in the database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MigrationRecord {
    pub id: i32,
    pub version: i32,
    pub name: String,
    pub applied_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::Error;
    
    struct TestMigration;
    
    #[async_trait::async_trait]
    impl Migration for TestMigration {
        fn version(&self) -> i32 { 1 }
        fn name(&self) -> &str { "test_migration" }
        
        async fn up(&self, _pool: &sqlx::PgPool) -> Result<()> {
            Ok(())
        }
        
        async fn down(&self, _pool: &sqlx::PgPool) -> Result<()> {
            Ok(())
        }
    }
    
    #[test]
    fn test_migration_trait_implementation() {
        let migration = TestMigration;
        assert_eq!(migration.version(), 1);
        assert_eq!(migration.name(), "test_migration");
    }
}
