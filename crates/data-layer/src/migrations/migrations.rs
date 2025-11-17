use quant_common::{Error, Result};
use super::Migration;
use sqlx::PgPool;
use std::sync::Arc;

/// Migration 001: Create core tables
pub struct Migration001;

#[async_trait::async_trait]
impl Migration for Migration001 {
    fn version(&self) -> i32 {
        1
    }
    
    fn name(&self) -> &str {
        "create_core_tables"
    }
    
    async fn up(&self, pool: &PgPool) -> Result<()> {
        // Create accounts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS accounts (
                account_id UUID PRIMARY KEY,
                account_name VARCHAR(255) NOT NULL,
                account_type VARCHAR(50) NOT NULL,
                total_assets DECIMAL(20, 8) NOT NULL DEFAULT 0,
                available_balance DECIMAL(20, 8) NOT NULL DEFAULT 0,
                frozen_balance DECIMAL(20, 8) NOT NULL DEFAULT 0,
                margin_ratio DECIMAL(10, 4) NOT NULL DEFAULT 0,
                daily_pnl DECIMAL(20, 8) NOT NULL DEFAULT 0,
                total_pnl DECIMAL(20, 8) NOT NULL DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
            )
            "#
        )
        .execute(pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create accounts table: {}", e)))?;
        
        // Create orders table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS orders (
                order_id UUID PRIMARY KEY,
                account_id UUID NOT NULL,
                symbol VARCHAR(50) NOT NULL,
                order_type VARCHAR(50) NOT NULL,
                side VARCHAR(10) NOT NULL,
                price DECIMAL(20, 8),
                quantity DECIMAL(20, 8) NOT NULL,
                filled_quantity DECIMAL(20, 8) NOT NULL DEFAULT 0,
                status VARCHAR(50) NOT NULL,
                time_in_force VARCHAR(50),
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                FOREIGN KEY (account_id) REFERENCES accounts(account_id) ON DELETE CASCADE
            )
            "#
        )
        .execute(pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create orders table: {}", e)))?;
        
        // Create positions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS positions (
                position_id UUID PRIMARY KEY,
                account_id UUID NOT NULL,
                symbol VARCHAR(50) NOT NULL,
                side VARCHAR(10) NOT NULL,
                quantity DECIMAL(20, 8) NOT NULL,
                entry_price DECIMAL(20, 8) NOT NULL,
                current_price DECIMAL(20, 8) NOT NULL,
                unrealized_pnl DECIMAL(20, 8) NOT NULL DEFAULT 0,
                realized_pnl DECIMAL(20, 8) NOT NULL DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                FOREIGN KEY (account_id) REFERENCES accounts(account_id) ON DELETE CASCADE,
                UNIQUE (account_id, symbol, side)
            )
            "#
        )
        .execute(pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create positions table: {}", e)))?;
        
        // Create trades table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trades (
                trade_id UUID PRIMARY KEY,
                order_id UUID NOT NULL,
                account_id UUID NOT NULL,
                symbol VARCHAR(50) NOT NULL,
                side VARCHAR(10) NOT NULL,
                price DECIMAL(20, 8) NOT NULL,
                quantity DECIMAL(20, 8) NOT NULL,
                commission DECIMAL(20, 8) NOT NULL DEFAULT 0,
                commission_asset VARCHAR(10),
                executed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                FOREIGN KEY (order_id) REFERENCES orders(order_id) ON DELETE CASCADE,
                FOREIGN KEY (account_id) REFERENCES accounts(account_id) ON DELETE CASCADE
            )
            "#
        )
        .execute(pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create trades table: {}", e)))?;
        
        Ok(())
    }
    
    async fn down(&self, pool: &PgPool) -> Result<()> {
        // Drop tables in reverse order (respecting foreign keys)
        sqlx::query("DROP TABLE IF EXISTS trades")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop trades table: {}", e)))?;
        
        sqlx::query("DROP TABLE IF EXISTS positions")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop positions table: {}", e)))?;
        
        sqlx::query("DROP TABLE IF EXISTS orders")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop orders table: {}", e)))?;
        
        sqlx::query("DROP TABLE IF EXISTS accounts")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop accounts table: {}", e)))?;
        
        Ok(())
    }
}

/// Migration 002: Create indices for performance
pub struct Migration002;

#[async_trait::async_trait]
impl Migration for Migration002 {
    fn version(&self) -> i32 {
        2
    }
    
    fn name(&self) -> &str {
        "create_indices"
    }
    
    async fn up(&self, pool: &PgPool) -> Result<()> {
        // Indices for accounts table
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_accounts_type ON accounts(account_type)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_accounts_updated ON accounts(updated_at)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        // Indices for orders table
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_account ON orders(account_id)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_symbol ON orders(symbol)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_created ON orders(created_at)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        // Indices for positions table
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_positions_account ON positions(account_id)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_positions_symbol ON positions(symbol)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        // Indices for trades table
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_order ON trades(order_id)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_account ON trades(account_id)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_symbol ON trades(symbol)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_executed ON trades(executed_at)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        Ok(())
    }
    
    async fn down(&self, pool: &PgPool) -> Result<()> {
        // Drop all indices
        let indices = vec![
            "idx_accounts_type",
            "idx_accounts_updated",
            "idx_orders_account",
            "idx_orders_symbol",
            "idx_orders_status",
            "idx_orders_created",
            "idx_positions_account",
            "idx_positions_symbol",
            "idx_trades_order",
            "idx_trades_account",
            "idx_trades_symbol",
            "idx_trades_executed",
        ];
        
        for index in indices {
            sqlx::query(&format!("DROP INDEX IF EXISTS {}", index))
                .execute(pool)
                .await
                .map_err(|e| Error::Database(format!("Failed to drop index {}: {}", index, e)))?;
        }
        
        Ok(())
    }
}

/// Migration 003: Add alerts and risk metrics tables
pub struct Migration003;

#[async_trait::async_trait]
impl Migration for Migration003 {
    fn version(&self) -> i32 {
        3
    }
    
    fn name(&self) -> &str {
        "create_alerts_and_risk_tables"
    }
    
    async fn up(&self, pool: &PgPool) -> Result<()> {
        // Create alerts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS alerts (
                alert_id UUID PRIMARY KEY,
                level VARCHAR(50) NOT NULL,
                source VARCHAR(255) NOT NULL,
                message TEXT NOT NULL,
                acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                acknowledged_at TIMESTAMP WITH TIME ZONE
            )
            "#
        )
        .execute(pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create alerts table: {}", e)))?;
        
        // Create risk_metrics table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS risk_metrics (
                id SERIAL PRIMARY KEY,
                account_id UUID NOT NULL,
                metric_type VARCHAR(50) NOT NULL,
                metric_value DECIMAL(20, 8) NOT NULL,
                recorded_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                FOREIGN KEY (account_id) REFERENCES accounts(account_id) ON DELETE CASCADE
            )
            "#
        )
        .execute(pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create risk_metrics table: {}", e)))?;
        
        // Create indices
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_alerts_level ON alerts(level)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_alerts_created ON alerts(created_at)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_risk_metrics_account ON risk_metrics(account_id)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_risk_metrics_recorded ON risk_metrics(recorded_at)")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to create index: {}", e)))?;
        
        Ok(())
    }
    
    async fn down(&self, pool: &PgPool) -> Result<()> {
        sqlx::query("DROP INDEX IF EXISTS idx_risk_metrics_recorded")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop index: {}", e)))?;
        
        sqlx::query("DROP INDEX IF EXISTS idx_risk_metrics_account")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop index: {}", e)))?;
        
        sqlx::query("DROP INDEX IF EXISTS idx_alerts_created")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop index: {}", e)))?;
        
        sqlx::query("DROP INDEX IF EXISTS idx_alerts_level")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop index: {}", e)))?;
        
        sqlx::query("DROP TABLE IF EXISTS risk_metrics")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop risk_metrics table: {}", e)))?;
        
        sqlx::query("DROP TABLE IF EXISTS alerts")
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to drop alerts table: {}", e)))?;
        
        Ok(())
    }
}

/// Get all available migrations
pub fn get_all_migrations() -> Vec<Arc<dyn Migration>> {
    vec![
        Arc::new(Migration001) as Arc<dyn Migration>,
        Arc::new(Migration002) as Arc<dyn Migration>,
        Arc::new(Migration003) as Arc<dyn Migration>,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_migration001_metadata() {
        let migration = Migration001;
        assert_eq!(migration.version(), 1);
        assert_eq!(migration.name(), "create_core_tables");
    }
    
    #[test]
    fn test_migration002_metadata() {
        let migration = Migration002;
        assert_eq!(migration.version(), 2);
        assert_eq!(migration.name(), "create_indices");
    }
    
    #[test]
    fn test_migration003_metadata() {
        let migration = Migration003;
        assert_eq!(migration.version(), 3);
        assert_eq!(migration.name(), "create_alerts_and_risk_tables");
    }
    
    #[test]
    fn test_get_all_migrations() {
        let migrations = get_all_migrations();
        assert_eq!(migrations.len(), 3);
        assert_eq!(migrations[0].version(), 1);
        assert_eq!(migrations[1].version(), 2);
        assert_eq!(migrations[2].version(), 3);
    }
    
    #[test]
    fn test_migrations_ordered() {
        let migrations = get_all_migrations();
        for i in 1..migrations.len() {
            assert!(
                migrations[i].version() > migrations[i - 1].version(),
                "Migrations must be ordered by version"
            );
        }
    }
}
