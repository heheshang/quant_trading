use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub influxdb: InfluxDBConfig,
    pub trading: TradingConfig,
    pub risk: RiskConfig,
    pub monitoring: MonitoringConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub db: i64,
    pub pool_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluxDBConfig {
    pub url: String,
    pub token: String,
    pub database: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub enable_paper_trading: bool,
    pub max_orders_per_second: u32,
    pub default_commission_rate: f64,
    pub default_slippage: f64,
    pub order_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub max_position_size: f64,
    pub max_daily_loss: f64,
    pub max_drawdown: f64,
    pub enable_pre_trade_check: bool,
    pub enable_real_time_monitor: bool,
    pub var_confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_prometheus: bool,
    pub prometheus_port: u16,
    pub log_level: String,
    pub alert_email: Option<String>,
    pub alert_webhook: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_encryption: bool,
    pub jwt_secret: String,
    pub token_expiry_hours: u64,
    pub enable_2fa: bool,
    pub allowed_ips: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                username: "quant".to_string(),
                password: "quant_password".to_string(),
                database: "quant_trading".to_string(),
                max_connections: 50,
            },
            redis: RedisConfig {
                host: "localhost".to_string(),
                port: 6379,
                password: None,
                db: 0,
                pool_size: 20,
            },
            influxdb: InfluxDBConfig {
                url: "http://localhost:8086".to_string(),
                token: "".to_string(),
                database: "market-data".to_string(),
            },
            trading: TradingConfig {
                enable_paper_trading: true,
                max_orders_per_second: 100,
                default_commission_rate: 0.0003,
                default_slippage: 0.0001,
                order_timeout_seconds: 30,
            },
            risk: RiskConfig {
                max_position_size: 0.2,
                max_daily_loss: 0.05,
                max_drawdown: 0.15,
                enable_pre_trade_check: true,
                enable_real_time_monitor: true,
                var_confidence_level: 0.95,
            },
            monitoring: MonitoringConfig {
                enable_prometheus: true,
                prometheus_port: 9090,
                log_level: "info".to_string(),
                alert_email: None,
                alert_webhook: None,
            },
            security: SecurityConfig {
                enable_encryption: true,
                jwt_secret: "change_this_secret_in_production".to_string(),
                token_expiry_hours: 24,
                enable_2fa: false,
                allowed_ips: vec!["127.0.0.1".to_string()],
            },
        }
    }
}
