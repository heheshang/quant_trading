use quant_common::config::DatabaseConfig;
use sqlx::postgres::PgPoolOptions;
use tracing::{error, info};

#[tokio::main]
async fn main() {
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

    let db_config = DatabaseConfig {
        host: std::env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: std::env::var("DATABASE_PORT")
            .unwrap_or_else(|_| "5432".to_string())
            .parse()
            .unwrap_or(5432),
        username: std::env::var("DATABASE_USERNAME").unwrap_or_else(|_| "quant".to_string()),
        password: std::env::var("DATABASE_PASSWORD")
            .unwrap_or_else(|_| "quant_password".to_string()),
        database: std::env::var("DATABASE_NAME").unwrap_or_else(|_| "quant_trading".to_string()),
        max_connections: 5,
    };

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
        Ok(pool) => pool,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            eprintln!("Error: Failed to connect to database: {}", e);
            eprintln!("Make sure PostgreSQL is running and credentials are correct.");
            std::process::exit(1);
        }
    };

    info!("Database connection established");

    match command.as_str() {
        "migrate" | "up" => {
            info!("Running migrations...");
            match sqlx::migrate!("../crates/data-layer/migrations")
                .run(&pool)
                .await
            {
                Ok(_) => {
                    info!("All migrations completed successfully");
                }
                Err(e) => {
                    error!("Migration failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "rollback" => {
            let target_version = if args.len() >= 3 {
                args[2].parse::<i64>().unwrap_or(0)
            } else {
                0
            };
            info!("Rolling back to version {}...", target_version);
            match sqlx::migrate!("../crates/data-layer/migrations")
                .undo(&pool, target_version)
                .await
            {
                Ok(_) => {
                    info!("Rollback completed successfully");
                }
                Err(e) => {
                    error!("Rollback failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "status" => {
            println!("Migrations are in: crates/data-layer/migrations/");
            println!("Run 'migrate' to apply pending migrations or 'rollback' to undo.");
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
    println!("  rollback [version]      Rollback to a specific version");
    println!("  status                  Show migration status");
    println!();
    println!("Environment Variables (match main.rs / .env):");
    println!("  DATABASE_HOST           Database host (default: localhost)");
    println!("  DATABASE_PORT           Database port (default: 5432)");
    println!("  DATABASE_USERNAME       Database username (default: quant)");
    println!("  DATABASE_PASSWORD       Database password (default: quant_password)");
    println!("  DATABASE_NAME           Database name (default: quant_trading)");
}
