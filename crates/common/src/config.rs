use serde::{Deserialize, Serialize};

mod from_env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub trading: TradingConfig,
    pub risk: RiskConfig,
    pub monitoring: MonitoringConfig,
    pub security: SecurityConfig,
    #[serde(default)]
    pub binance: BinanceConfig,
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
    pub encryption_key: String,
    pub token_expiry_hours: u64,
    pub enable_2fa: bool,
    pub allowed_ips: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinanceConfig {
    pub api_key: String,
    pub api_secret: String,
    pub environment: String, // "spot" or "futures"
    pub enable: bool,
    /// Optional REST base URL override (e.g. Spot Testnet `testnet.binance.vision`).
    /// When unset, the environment-derived URL is used.
    pub base_url: Option<String>,
    /// Optional WebSocket base URL override (e.g. Spot Testnet stream).
    /// When unset, the environment-derived stream URL is used.
    pub ws_url: Option<String>,
    /// Optional WebSocket-API base URL override (user data stream, e.g. Spot
    /// Testnet `wss://ws-api.testnet.binance.vision/ws-api/v3`). When unset,
    /// derived from the REST base/host.
    pub ws_api_url: Option<String>,
    /// Signing scheme: `"hmac"` (default) or `"ed25519"`.
    pub key_type: String,
    /// Path to the Ed25519 PKCS#8 PEM private key (used when `key_type="ed25519"`).
    pub private_key_path: Option<String>,
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
    true
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
                encryption_key: "change_this_encryption_key_in_production".to_string(),
                token_expiry_hours: 24,
                enable_2fa: false,
                allowed_ips: vec!["127.0.0.1".to_string()],
            },
            // Deterministic dev defaults — environment variables are applied
            // by `AppConfig::from_env()` (dotenv-based) at startup.
            binance: BinanceConfig {
                api_key: String::new(),
                api_secret: String::new(),
                environment: "spot".to_string(),
                enable: false,
                base_url: None,
                ws_url: None,
                ws_api_url: None,
                key_type: "hmac".to_string(),
                private_key_path: None,
            },
            data_puller: DataPullerConfig::default(),
            scheduler: SchedulerConfig::default(),
            param_optimizer: ParamOptimizerConfig::default(),
        }
    }
}

impl AppConfig {
    /// Return a copy of this config with all sensitive values blanked.
    ///
    /// Used to redact the config before it is returned to the frontend via
    /// `get_config`, so database passwords, JWT secrets and exchange
    /// `api_key`/`api_secret`/`passphrase` never leave the backend.
    #[must_use]
    pub fn redacted(&self) -> Self {
        let mut c = self.clone();
        c.database.password.clear();
        c.redis.password = None;
        c.security.jwt_secret.clear();
        c.security.encryption_key.clear();
        c.binance.api_key.clear();
        c.binance.api_secret.clear();
        c
    }

    /// 校验安全密钥：拒绝空/占位/过短（<32 字节）的 JWT 密钥与加密密钥。
    ///
    /// 防止使用仓库中公开的默认占位值运行（否则 JWT 可伪造、密钥可解密）。
    pub fn validate_secrets(&self) -> Result<(), String> {
        // 拦截仓库/文档中公开的占位（如 change_this_* / change_me / docker_test_*）。
        let is_placeholder = |s: &str| {
            let s = s.trim().to_lowercase();
            s.is_empty()
                || s.contains("change_this")
                || s.contains("change_me")
                || s.contains("docker_test")
                || s.contains("test_change")
        };
        if is_placeholder(&self.security.jwt_secret) {
            return Err("JWT 密钥为空或仍为占位值，拒绝启动。请在环境变量设置足够强(≥32字节)的 JWT_SECRET。".to_string());
        }
        if self.security.jwt_secret.len() < 32 {
            return Err("JWT 密钥过短(<32字节)，拒绝启动。".to_string());
        }
        if is_placeholder(&self.security.encryption_key) {
            return Err("加密密钥为空或仍为占位值，拒绝启动。请设置足够强(≥32字节)的 ENCRYPTION_KEY。".to_string());
        }
        if self.security.encryption_key.len() < 32 {
            return Err("加密密钥过短(<32字节)，拒绝启动。".to_string());
        }
        Ok(())
    }

    /// Restore secret values from `previous` where `self` has them blanked.
    ///
    /// When the frontend round-trips a redacted config back on save, empty
    /// secret fields must not clobber the running configuration; this merges
    /// the previous non-empty secrets back into `self`.
    #[must_use]
    pub fn with_secrets_from(&self, previous: &Self) -> Self {
        let mut c = self.clone();
        if c.database.password.is_empty() {
            c.database.password = previous.database.password.clone();
        }
        if c.redis.password.is_none() {
            c.redis.password = previous.redis.password.clone();
        }
        if c.security.jwt_secret.is_empty() {
            c.security.jwt_secret = previous.security.jwt_secret.clone();
        }
        if c.security.encryption_key.is_empty() {
            c.security.encryption_key = previous.security.encryption_key.clone();
        }
        if c.binance.api_key.is_empty() {
            c.binance.api_key = previous.binance.api_key.clone();
        }
        if c.binance.api_secret.is_empty() {
            c.binance.api_secret = previous.binance.api_secret.clone();
        }
        c
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
        assert!(cfg.enabled);
        assert_eq!(cfg.max_concurrent_strategies, 10);
        assert_eq!(cfg.default_interval_secs, 60);
        assert_eq!(cfg.circuit_breaker_threshold, 5);
        assert_eq!(cfg.circuit_breaker_window_secs, 300);
        assert_eq!(cfg.circuit_breaker_cooldown_secs, 600);
    }

    #[test]
    fn test_app_config_includes_scheduler() {
        let cfg = AppConfig::default();
        assert!(cfg.scheduler.enabled);
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

    #[test]
    fn test_default_is_neutral_for_exchange_credentials() {
        // `AppConfig::default()` must NOT read environment variables; it is a
        // deterministic dev baseline that `from_env()` (dotenv) overrides.
        let cfg = AppConfig::default();
        assert_eq!(cfg.binance.api_key, "");
        assert!(!cfg.binance.enable);
        assert_eq!(cfg.binance.environment, "spot");
    }

    #[test]
    fn test_redacted_blanks_sensitive_fields() {
        let mut cfg = AppConfig::default();
        cfg.database.password = "db_pw".to_string();
        cfg.redis.password = Some("redis_pw".to_string());
        cfg.security.jwt_secret = "jwt_secret".to_string();
        cfg.binance.api_key = "bin_key".to_string();
        cfg.binance.api_secret = "bin_secret".to_string();

        let redacted = cfg.redacted();

        // Sensitive values must be blanked.
        assert_eq!(redacted.database.password, "");
        assert_eq!(redacted.redis.password, None);
        assert_eq!(redacted.security.jwt_secret, "");
        assert_eq!(redacted.binance.api_key, "");
        assert_eq!(redacted.binance.api_secret, "");

        // Non-sensitive display fields preserved.
        assert_eq!(redacted.database.host, "localhost");
        assert_eq!(redacted.database.port, 5432);
        assert_eq!(redacted.trading.max_orders_per_second, 100);
        assert!(redacted.risk.enable_pre_trade_check);
        assert_eq!(redacted.monitoring.log_level, "info");
    }

    #[test]
    fn test_with_secrets_from_restores_blanked_values() {
        let mut previous = AppConfig::default();
        previous.database.password = "db_pw".to_string();
        previous.security.jwt_secret = "jwt".to_string();
        previous.binance.api_key = "bin_key".to_string();

        // Incoming config has secrets blanked (round-tripped from the UI).
        let incoming = previous.redacted();
        let restored = incoming.with_secrets_from(&previous);

        assert_eq!(restored.database.password, "db_pw");
        assert_eq!(restored.security.jwt_secret, "jwt");
        assert_eq!(restored.binance.api_key, "bin_key");

        // When the previous config also has a blank secret, it stays blank
        // (nothing to restore), and non-secret values pass through unchanged.
        let mut previous2 = AppConfig::default();
        previous2.database.password.clear();
        let incoming2 = previous2.redacted();
        let restored2 = incoming2.with_secrets_from(&previous2);
        assert_eq!(restored2.database.password, "");
        // Non-secret values are left untouched by the secret restore.
        let mut incoming2 = AppConfig::default();
        incoming2.trading.max_orders_per_second = 7;
        let mut previous2 = AppConfig::default();
        previous2.trading.max_orders_per_second = 100;
        let out2 = incoming2.with_secrets_from(&previous2);
        assert_eq!(out2.trading.max_orders_per_second, 7);
    }
}
