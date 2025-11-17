use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub struct LoggingConfig {
    pub log_level: String,
    pub log_dir: String,
}

pub fn init_logging(config: LoggingConfig) {
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &config.log_dir,
        "quant-trading.log"
    );

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking))
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(&config.log_level)
        }))
        .init();
}
