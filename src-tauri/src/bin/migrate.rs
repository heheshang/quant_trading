use data_layer::migrations::migrations::get_all_migrations;
use data_layer::MigrationManager;
use quant_common::config::DatabaseConfig;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Initialize logging
    monitor_layer::logging::init_logging(monitor_layer::logging::LoggingConfig {
        log_level: "info".to_string(),
        log_dir: "./logs".to_string(),
        service_name: "migrate-db".to_string(),
        enable_json_logging: false,
        enable_file_logging: true,
        enable_stdout_logging: true,
    })
    .expect("Failed to initialize logging");

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let command = &args[1];

    // Load database config from environment or default
    let db_config = DatabaseConfig {
        host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: std::env::var("DB_PORT")
            .unwrap_or_else(|_| "5432".to_string())
            .parse()
            .unwrap_or(5432),
        username: std::env::var("DB_USERNAME").unwrap_or_else(|_| "quant".to_string()),
        password: std::env::var("DB_PASSWORD").unwrap_or_else(|_| "quant_password".to_string()),
        database: std::env::var("DB_DATABASE").unwrap_or_else(|_| "quant_trading".to_string()),
        max_connections: 5,
    };

    // Create connection pool
    let connection_string = format!(
        "postgres://{}:{}@{}:{}/{}",
        db_config.username, db_config.password, db_config.host, db_config.port, db_config.database
    );

    info!(
        "Connecting to database: {}@{}:{}/{}",
        db_config.username, db_config.host, db_config.port, db_config.database
    );

    let pool = match PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .connect(&connection_string)
        .await
    {
        Ok(pool) => Arc::new(pool),
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            eprintln!("Error: Failed to connect to database: {}", e);
            eprintln!("Make sure PostgreSQL is running and credentials are correct.");
            std::process::exit(1);
        }
    };

    info!("Database connection established");

    // Create migration manager
    let mut manager = MigrationManager::new(pool);

    // Add all migrations
    for migration in get_all_migrations() {
        manager.add_migration(migration);
    }

    // Execute command
    match command.as_str() {
        "migrate" | "up" => {
            info!("Running migrations...");
            match manager.migrate().await {
                Ok(_) => {
                    info!("✅ All migrations completed successfully");
                }
                Err(e) => {
                    error!("❌ Migration failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "rollback" => {
            let target_version = if args.len() >= 3 {
                args[2].parse::<i32>().unwrap_or(0)
            } else {
                match manager.get_current_version().await {
                    Ok(v) => v - 1,
                    Err(e) => {
                        error!("Failed to get current version: {}", e);
                        std::process::exit(1);
                    }
                }
            };

            info!("Rolling back to version {}...", target_version);
            match manager.rollback_to(target_version).await {
                Ok(_) => {
                    info!("✅ Rollback completed successfully");
                }
                Err(e) => {
                    error!("❌ Rollback failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "status" => {
            match manager.init().await {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to initialize: {}", e);
                    std::process::exit(1);
                }
            }

            match manager.get_current_version().await {
                Ok(version) => {
                    println!("Current database version: {}", version);
                }
                Err(e) => {
                    error!("Failed to get version: {}", e);
                    std::process::exit(1);
                }
            }

            match manager.get_applied_migrations().await {
                Ok(migrations) => {
                    if migrations.is_empty() {
                        println!("No migrations applied yet.");
                    } else {
                        println!("\nApplied migrations:");
                        for m in migrations {
                            println!(
                                "  [v{}] {} - applied at {}",
                                m.version,
                                m.name,
                                m.applied_at.format("%Y-%m-%d %H:%M:%S")
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get migrations: {}", e);
                }
            }

            match manager.get_pending_migrations().await {
                Ok(pending) => {
                    if pending.is_empty() {
                        println!("\nNo pending migrations.");
                    } else {
                        println!("\nPending migrations:");
                        for m in pending {
                            println!("  [v{}] {}", m.version(), m.name());
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get pending migrations: {}", e);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Database Migration Tool");
    println!();
    println!("Usage:");
    println!("  migrate-db <command> [options]");
    println!();
    println!("Commands:");
    println!("  migrate | up            Run all pending migrations");
    println!("  rollback [version]      Rollback to a specific version (or previous version)");
    println!("  status                  Show migration status");
    println!();
    println!("Environment Variables:");
    println!("  DB_HOST                 Database host (default: localhost)");
    println!("  DB_PORT                 Database port (default: 5432)");
    println!("  DB_USERNAME             Database username (default: quant)");
    println!("  DB_PASSWORD             Database password (default: quant_password)");
    println!("  DB_DATABASE             Database name (default: quant_trading)");
}
