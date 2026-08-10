use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub trading: TradingConfig,
    pub risk: RiskConfig,
    pub monitoring: MonitoringConfig,
    pub security: SecurityConfig,
    pub okx: OkxConfig,
    #[serde(default)]
    pub data_puller: DataPullerConfig,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub param_optimizer: ParamOptimizerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
    #[serde(default = "default_database_connect_timeout_seconds")]
    pub connect_timeout_seconds: u64,
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
pub struct TradingConfig {
    pub enable_paper_trading: bool,
    pub max_orders_per_second: u32,
    pub default_commission_rate: f64,
    pub default_slippage: f64,
    pub order_timeout_seconds: u64,
    /// Simulated fill delay for paper trading (milliseconds).
    /// Orders remain in `Submitted` state for this duration
    /// before being automatically filled in simulation mode.
    pub simulation_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub max_position_size: f64,
    pub max_daily_loss: f64,
    pub max_drawdown: f64,
    pub max_concentration: f64,
    pub enable_pre_trade_check: bool,
    pub enable_real_time_monitor: bool,
    pub var_confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_prometheus: bool,
    pub prometheus_port: u16,
    pub log_level: String,
    pub log_dir: String,
    pub service_name: String,
    pub enable_json_logging: bool,
    pub enable_file_logging: bool,
    pub enable_stdout_logging: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkxConfig {
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: String,
    pub environment: String, // "live" or "demo"
    pub enable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPullerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub candle: CandlePullConfig,
    #[serde(default)]
    pub ticker: TickerPullConfig,
    #[serde(default)]
    pub account_balance: IntervalConfig,
    #[serde(default)]
    pub positions: IntervalConfig,
    #[serde(default)]
    pub funding_rate: IntervalConfig,
    #[serde(default)]
    pub mark_price: IntervalConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CandlePullConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_candle_bars")]
    pub bars: Vec<String>,
    #[serde(default = "default_candle_limit")]
    pub limit: u32,
    #[serde(default = "default_candle_interval")]
    pub interval_secs: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TickerPullConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ticker_interval")]
    pub interval_secs: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntervalConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,
}

// ─── Scheduler & Optimizer Config ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    #[serde(default = "default_scheduler_enabled")]
    pub enabled: bool,
    #[serde(default = "default_scheduler_max_concurrent")]
    pub max_concurrent_strategies: usize,
    #[serde(default = "default_scheduler_default_interval_secs")]
    pub default_interval_secs: u64,
    #[serde(default = "default_scheduler_circuit_breaker_threshold")]
    pub circuit_breaker_threshold: u32,
    #[serde(default = "default_scheduler_circuit_breaker_window_secs")]
    pub circuit_breaker_window_secs: u64,
    #[serde(default = "default_scheduler_circuit_breaker_cooldown_secs")]
    pub circuit_breaker_cooldown_secs: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: default_scheduler_enabled(),
            max_concurrent_strategies: default_scheduler_max_concurrent(),
            default_interval_secs: default_scheduler_default_interval_secs(),
            circuit_breaker_threshold: default_scheduler_circuit_breaker_threshold(),
            circuit_breaker_window_secs: default_scheduler_circuit_breaker_window_secs(),
            circuit_breaker_cooldown_secs: default_scheduler_circuit_breaker_cooldown_secs(),
        }
    }
}

fn default_scheduler_enabled() -> bool {
    false
}
fn default_scheduler_max_concurrent() -> usize {
    10
}
fn default_scheduler_default_interval_secs() -> u64 {
    60
}
fn default_scheduler_circuit_breaker_threshold() -> u32 {
    5
}
fn default_scheduler_circuit_breaker_window_secs() -> u64 {
    300
}
fn default_scheduler_circuit_breaker_cooldown_secs() -> u64 {
    600
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamOptimizerConfig {
    #[serde(default = "default_optimizer_enabled")]
    pub enabled: bool,
    #[serde(default = "default_optimizer_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_optimizer_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_optimizer_parallel_jobs")]
    pub parallel_jobs: u32,
}

impl Default for ParamOptimizerConfig {
    fn default() -> Self {
        Self {
            enabled: default_optimizer_enabled(),
            max_iterations: default_optimizer_max_iterations(),
            timeout_secs: default_optimizer_timeout_secs(),
            parallel_jobs: default_optimizer_parallel_jobs(),
        }
    }
}

fn default_optimizer_enabled() -> bool {
    false
}
fn default_optimizer_max_iterations() -> u32 {
    100
}
fn default_optimizer_timeout_secs() -> u64 {
    3600
}
fn default_optimizer_parallel_jobs() -> u32 {
    4
}

fn default_candle_bars() -> Vec<String> {
    vec!["1m".into(), "5m".into(), "1H".into()]
}

fn default_candle_limit() -> u32 {
    100
}

fn default_candle_interval() -> u64 {
    60
}

fn default_ticker_interval() -> u64 {
    30
}

fn default_interval_secs() -> u64 {
    60
}

fn default_database_connect_timeout_seconds() -> u64 {
    3
}

impl Default for DataPullerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            symbols: vec![],
            candle: CandlePullConfig {
                enabled: false,
                bars: default_candle_bars(),
                limit: default_candle_limit(),
                interval_secs: default_candle_interval(),
            },
            ticker: TickerPullConfig {
                enabled: false,
                interval_secs: default_ticker_interval(),
            },
            account_balance: IntervalConfig {
                enabled: false,
                interval_secs: default_interval_secs(),
            },
            positions: IntervalConfig {
                enabled: false,
                interval_secs: default_interval_secs(),
            },
            funding_rate: IntervalConfig {
                enabled: false,
                interval_secs: 3600,
            },
            mark_price: IntervalConfig {
                enabled: false,
                interval_secs: 10,
            },
        }
    }
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
                connect_timeout_seconds: default_database_connect_timeout_seconds(),
            },
            redis: RedisConfig {
                host: "localhost".to_string(),
                port: 6379,
                password: None,
                db: 0,
                pool_size: 20,
            },
            trading: TradingConfig {
                enable_paper_trading: true,
                max_orders_per_second: 100,
                default_commission_rate: 0.0003,
                default_slippage: 0.0001,
                order_timeout_seconds: 30,
                simulation_delay_ms: 30_000,
            },
            risk: RiskConfig {
                max_position_size: 0.2,
                max_daily_loss: 0.05,
                max_drawdown: 0.15,
                max_concentration: 0.2,
                enable_pre_trade_check: true,
                enable_real_time_monitor: true,
                var_confidence_level: 0.95,
            },
            monitoring: MonitoringConfig {
                enable_prometheus: true,
                prometheus_port: 9090,
                log_level: "info".to_string(),
                log_dir: "./logs".to_string(),
                service_name: "quant-trading".to_string(),
                enable_json_logging: false,
                enable_file_logging: true,
                enable_stdout_logging: true,
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
            okx: OkxConfig {
                api_key: std::env::var("OKX_API_KEY").unwrap_or_default(),
                api_secret: std::env::var("OKX_API_SECRET").unwrap_or_default(),
                passphrase: std::env::var("OKX_PASSPHRASE").unwrap_or_default(),
                environment: std::env::var("OKX_ENVIRONMENT")
                    .unwrap_or_else(|_| "demo".to_string()),
                enable: std::env::var("OKX_ENABLE").unwrap_or_else(|_| "false".to_string())
                    == "true",
            },
            data_puller: DataPullerConfig::default(),
            scheduler: SchedulerConfig::default(),
            param_optimizer: ParamOptimizerConfig::default(),
        }
    }
}

impl AppConfig {
    /// Build configuration from conventional environment variables.
    ///
    /// Desktop mode keeps deterministic defaults; containerized deployment uses
    /// this constructor so Postgres/Redis and service settings can be injected
    /// without rewriting config files.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        config.database.host = env_string("DATABASE_HOST", &config.database.host);
        config.database.port = env_parse("DATABASE_PORT", config.database.port);
        config.database.username = env_string("DATABASE_USERNAME", &config.database.username);
        config.database.password = env_string("DATABASE_PASSWORD", &config.database.password);
        config.database.database = env_string("DATABASE_NAME", &config.database.database);
        config.database.max_connections =
            env_parse("DATABASE_MAX_CONNECTIONS", config.database.max_connections);
        config.database.connect_timeout_seconds = env_parse(
            "DATABASE_CONNECT_TIMEOUT_SECONDS",
            config.database.connect_timeout_seconds,
        );

        config.redis.host = env_string("REDIS_HOST", &config.redis.host);
        config.redis.port = env_parse("REDIS_PORT", config.redis.port);
        config.redis.password = env_option("REDIS_PASSWORD");
        config.redis.db = env_parse("REDIS_DB", config.redis.db);
        config.redis.pool_size = env_parse("REDIS_POOL_SIZE", config.redis.pool_size);

        config.trading.enable_paper_trading =
            env_parse("ENABLE_PAPER_TRADING", config.trading.enable_paper_trading);
        config.trading.max_orders_per_second = env_parse(
            "MAX_ORDERS_PER_SECOND",
            config.trading.max_orders_per_second,
        );
        config.trading.default_commission_rate = env_parse(
            "DEFAULT_COMMISSION_RATE",
            config.trading.default_commission_rate,
        );
        config.trading.default_slippage =
            env_parse("DEFAULT_SLIPPAGE", config.trading.default_slippage);
        config.trading.order_timeout_seconds = env_parse(
            "ORDER_TIMEOUT_SECONDS",
            config.trading.order_timeout_seconds,
        );
        config.trading.simulation_delay_ms =
            env_parse("SIMULATION_DELAY_MS", config.trading.simulation_delay_ms);

        config.risk.max_position_size =
            env_parse("MAX_POSITION_SIZE", config.risk.max_position_size);
        config.risk.max_daily_loss = env_parse("MAX_DAILY_LOSS", config.risk.max_daily_loss);
        config.risk.max_drawdown = env_parse("MAX_DRAWDOWN", config.risk.max_drawdown);
        config.risk.max_concentration =
            env_parse("MAX_CONCENTRATION", config.risk.max_concentration);
        config.risk.enable_pre_trade_check =
            env_parse("ENABLE_PRE_TRADE_CHECK", config.risk.enable_pre_trade_check);
        config.risk.enable_real_time_monitor = env_parse(
            "ENABLE_REAL_TIME_MONITOR",
            config.risk.enable_real_time_monitor,
        );
        config.risk.var_confidence_level =
            env_parse("VAR_CONFIDENCE_LEVEL", config.risk.var_confidence_level);

        config.monitoring.enable_prometheus =
            env_parse("ENABLE_PROMETHEUS", config.monitoring.enable_prometheus);
        config.monitoring.prometheus_port =
            env_parse("PROMETHEUS_PORT", config.monitoring.prometheus_port);
        config.monitoring.log_level = env_string("LOG_LEVEL", &config.monitoring.log_level);
        config.monitoring.log_dir = env_string("LOG_DIR", &config.monitoring.log_dir);
        config.monitoring.service_name =
            env_string("SERVICE_NAME", &config.monitoring.service_name);
        config.monitoring.enable_json_logging =
            env_parse("ENABLE_JSON_LOGGING", config.monitoring.enable_json_logging);
        config.monitoring.enable_file_logging =
            env_parse("ENABLE_FILE_LOGGING", config.monitoring.enable_file_logging);
        config.monitoring.enable_stdout_logging = env_parse(
            "ENABLE_STDOUT_LOGGING",
            config.monitoring.enable_stdout_logging,
        );
        config.monitoring.alert_email = env_option("ALERT_EMAIL");
        config.monitoring.alert_webhook = env_option("ALERT_WEBHOOK");

        config.security.enable_encryption =
            env_parse("ENABLE_ENCRYPTION", config.security.enable_encryption);
        config.security.jwt_secret = env_string("JWT_SECRET", &config.security.jwt_secret);
        config.security.token_expiry_hours =
            env_parse("TOKEN_EXPIRY_HOURS", config.security.token_expiry_hours);
        config.security.enable_2fa = env_parse("ENABLE_2FA", config.security.enable_2fa);
        config.security.allowed_ips = env_csv("ALLOWED_IPS", &config.security.allowed_ips);

        config.data_puller.enabled = env_parse("DATA_PULLER_ENABLED", config.data_puller.enabled);
        config.data_puller.symbols = env_csv("DATA_PULLER_SYMBOLS", &config.data_puller.symbols);
        config.data_puller.candle.enabled = env_parse(
            "DATA_PULLER_CANDLE_ENABLED",
            config.data_puller.candle.enabled,
        );
        config.data_puller.candle.bars =
            env_csv("DATA_PULLER_CANDLE_BARS", &config.data_puller.candle.bars);
        config.data_puller.candle.limit =
            env_parse("DATA_PULLER_CANDLE_LIMIT", config.data_puller.candle.limit);
        config.data_puller.candle.interval_secs = env_parse(
            "DATA_PULLER_CANDLE_INTERVAL_SECS",
            config.data_puller.candle.interval_secs,
        );
        config.data_puller.ticker.enabled = env_parse(
            "DATA_PULLER_TICKER_ENABLED",
            config.data_puller.ticker.enabled,
        );
        config.data_puller.ticker.interval_secs = env_parse(
            "DATA_PULLER_TICKER_INTERVAL_SECS",
            config.data_puller.ticker.interval_secs,
        );
        config.data_puller.account_balance.enabled = env_parse(
            "DATA_PULLER_ACCOUNT_BALANCE_ENABLED",
            config.data_puller.account_balance.enabled,
        );
        config.data_puller.account_balance.interval_secs = env_parse(
            "DATA_PULLER_ACCOUNT_BALANCE_INTERVAL_SECS",
            config.data_puller.account_balance.interval_secs,
        );
        config.data_puller.positions.enabled = env_parse(
            "DATA_PULLER_POSITIONS_ENABLED",
            config.data_puller.positions.enabled,
        );
        config.data_puller.positions.interval_secs = env_parse(
            "DATA_PULLER_POSITIONS_INTERVAL_SECS",
            config.data_puller.positions.interval_secs,
        );
        config.data_puller.funding_rate.enabled = env_parse(
            "DATA_PULLER_FUNDING_RATE_ENABLED",
            config.data_puller.funding_rate.enabled,
        );
        config.data_puller.funding_rate.interval_secs = env_parse(
            "DATA_PULLER_FUNDING_RATE_INTERVAL_SECS",
            config.data_puller.funding_rate.interval_secs,
        );
        config.data_puller.mark_price.enabled = env_parse(
            "DATA_PULLER_MARK_PRICE_ENABLED",
            config.data_puller.mark_price.enabled,
        );
        config.data_puller.mark_price.interval_secs = env_parse(
            "DATA_PULLER_MARK_PRICE_INTERVAL_SECS",
            config.data_puller.mark_price.interval_secs,
        );

        config.scheduler.enabled = env_parse("SCHEDULER_ENABLED", config.scheduler.enabled);
        config.scheduler.max_concurrent_strategies = env_parse(
            "SCHEDULER_MAX_CONCURRENT_STRATEGIES",
            config.scheduler.max_concurrent_strategies,
        );
        config.scheduler.default_interval_secs = env_parse(
            "SCHEDULER_DEFAULT_INTERVAL_SECS",
            config.scheduler.default_interval_secs,
        );
        config.scheduler.circuit_breaker_threshold = env_parse(
            "SCHEDULER_CIRCUIT_BREAKER_THRESHOLD",
            config.scheduler.circuit_breaker_threshold,
        );
        config.scheduler.circuit_breaker_window_secs = env_parse(
            "SCHEDULER_CIRCUIT_BREAKER_WINDOW_SECS",
            config.scheduler.circuit_breaker_window_secs,
        );
        config.scheduler.circuit_breaker_cooldown_secs = env_parse(
            "SCHEDULER_CIRCUIT_BREAKER_COOLDOWN_SECS",
            config.scheduler.circuit_breaker_cooldown_secs,
        );

        config.param_optimizer.enabled =
            env_parse("OPTIMIZER_ENABLED", config.param_optimizer.enabled);
        config.param_optimizer.max_iterations = env_parse(
            "OPTIMIZER_MAX_ITERATIONS",
            config.param_optimizer.max_iterations,
        );
        config.param_optimizer.timeout_secs = env_parse(
            "OPTIMIZER_TIMEOUT_SECS",
            config.param_optimizer.timeout_secs,
        );
        config.param_optimizer.parallel_jobs = env_parse(
            "OPTIMIZER_PARALLEL_JOBS",
            config.param_optimizer.parallel_jobs,
        );

        config
    }
}

fn env_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_parse<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr,
{
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_option(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn env_csv(key: &str, default: &[String]) -> Vec<String> {
    std::env::var(key)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(|_| default.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default_monitoring_log_level() {
        let app_config = AppConfig::default();
        let config = &app_config.monitoring;
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_app_config_default_monitoring_log_dir() {
        let app_config = AppConfig::default();
        assert_eq!(app_config.monitoring.log_dir, "./logs");
    }

    #[test]
    fn test_app_config_default_monitoring_service_name() {
        let app_config = AppConfig::default();
        assert_eq!(app_config.monitoring.service_name, "quant-trading");
    }

    #[test]
    fn test_app_config_default_monitoring_json_disabled() {
        let app_config = AppConfig::default();
        assert!(!app_config.monitoring.enable_json_logging);
    }

    #[test]
    fn test_app_config_default_monitoring_file_enabled() {
        let app_config = AppConfig::default();
        assert!(app_config.monitoring.enable_file_logging);
    }

    #[test]
    fn test_app_config_default_monitoring_stdout_enabled() {
        let app_config = AppConfig::default();
        assert!(app_config.monitoring.enable_stdout_logging);
    }

    #[test]
    fn test_app_config_default_monitoring_alert_fields() {
        let app_config = AppConfig::default();
        assert!(app_config.monitoring.enable_prometheus);
        assert_eq!(app_config.monitoring.prometheus_port, 9090);
        assert_eq!(app_config.monitoring.alert_email, None);
        assert_eq!(app_config.monitoring.alert_webhook, None);
    }

    // ── Scheduler Config ─────────────────────────────────────────────────

    #[test]
    fn test_scheduler_config_default() {
        let cfg = SchedulerConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.max_concurrent_strategies, 10);
        assert_eq!(cfg.default_interval_secs, 60);
        assert_eq!(cfg.circuit_breaker_threshold, 5);
        assert_eq!(cfg.circuit_breaker_window_secs, 300);
        assert_eq!(cfg.circuit_breaker_cooldown_secs, 600);
    }

    #[test]
    fn test_app_config_includes_scheduler() {
        let cfg = AppConfig::default();
        assert!(!cfg.scheduler.enabled);
        assert_eq!(cfg.scheduler.default_interval_secs, 60);
    }

    // ── Param Optimizer Config ────────────────────────────────────────────

    #[test]
    fn test_param_optimizer_config_default() {
        let cfg = ParamOptimizerConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.max_iterations, 100);
        assert_eq!(cfg.timeout_secs, 3600);
        assert_eq!(cfg.parallel_jobs, 4);
    }

    #[test]
    fn test_app_config_includes_optimizer() {
        let cfg = AppConfig::default();
        assert!(!cfg.param_optimizer.enabled);
        assert_eq!(cfg.param_optimizer.max_iterations, 100);
    }
}
