//! `AppConfig::from_env` — 从环境变量构建配置。
//!
//! 桌面模式保持确定性默认值；容器化部署通过此构造器注入 Postgres/Redis 及服务配置。

use super::{env_csv, env_option, env_parse, env_string, AppConfig};

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
        config.security.encryption_key =
            env_string("ENCRYPTION_KEY", &config.security.encryption_key);

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

        // Exchange credentials / toggles (dotenv-injected at startup).
        config.binance.api_key = env_string("BINANCE_API_KEY", &config.binance.api_key);
        config.binance.api_secret = env_string("BINANCE_API_SECRET", &config.binance.api_secret);
        config.binance.environment = env_string("BINANCE_ENVIRONMENT", &config.binance.environment);
        config.binance.enable = env_parse("BINANCE_ENABLE", config.binance.enable);

        config
    }
}
