use quant_common::types::LogEntry;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

pub struct LoggingConfig {
    pub log_level: String,
    pub log_dir: String,
    pub service_name: String,
    pub enable_json_logging: bool,
    pub enable_file_logging: bool,
    pub enable_stdout_logging: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_dir: "./logs".to_string(),
            service_name: "quant-trading".to_string(),
            enable_json_logging: false,
            enable_file_logging: true,
            enable_stdout_logging: true,
        }
    }
}

pub fn init_logging(config: LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    // File logging layer
    if config.enable_file_logging {
        let file_appender = RollingFileAppender::new(
            Rotation::DAILY,
            &config.log_dir,
            format!("{}-log", config.service_name),
        );

        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        if config.enable_json_logging {
            let file_layer = fmt::layer().json().with_writer(non_blocking);

            let level_filter = parse_log_level(&config.log_level);

            tracing_subscriber::registry()
                .with(file_layer)
                .with(EnvFilter::from_default_env().add_directive(level_filter.into()))
                .try_init()?;
        } else {
            let file_layer = fmt::layer().with_writer(non_blocking);

            let level_filter = parse_log_level(&config.log_level);

            tracing_subscriber::registry()
                .with(file_layer)
                .with(EnvFilter::from_default_env().add_directive(level_filter.into()))
                .try_init()?;
        }
    } else if config.enable_stdout_logging {
        // Only stdout logging
        if config.enable_json_logging {
            let stdout_layer = fmt::layer().json().with_writer(std::io::stdout);

            let level_filter = parse_log_level(&config.log_level);

            tracing_subscriber::registry()
                .with(stdout_layer)
                .with(EnvFilter::from_default_env().add_directive(level_filter.into()))
                .try_init()?;
        } else {
            let stdout_layer = fmt::layer().with_writer(std::io::stdout);

            let level_filter = parse_log_level(&config.log_level);

            tracing_subscriber::registry()
                .with(stdout_layer)
                .with(EnvFilter::from_default_env().add_directive(level_filter.into()))
                .try_init()?;
        }
    }

    tracing::info!(
        service_name = %config.service_name,
        log_level = %config.log_level,
        log_dir = %config.log_dir,
        "Logging initialized"
    );

    Ok(())
}

fn parse_log_level(level: &str) -> LevelFilter {
    match level.to_lowercase().as_str() {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "info" => LevelFilter::INFO,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        _ => LevelFilter::INFO,
    }
}

/// Create a structured log entry
#[macro_export]
macro_rules! log_structured {
    ($level:ident, $message:expr, $($key:expr => $value:expr),*) => {
        tracing::$level!(
            message = $message,
            $($key = $value),*
        );
    };
}

/// Convenience macro for debug logging with structured data
#[macro_export]
macro_rules! log_debug {
    ($message:expr, $($key:expr => $value:expr),*) => {
        $crate::log_structured!(debug, $message, $($key => $value),*)
    };
}

/// Convenience macro for info logging with structured data
#[macro_export]
macro_rules! log_info {
    ($message:expr, $($key:expr => $value:expr),*) => {
        $crate::log_structured!(info, $message, $($key => $value),*)
    };
}

/// Convenience macro for warning logging with structured data
#[macro_export]
macro_rules! log_warn {
    ($message:expr, $($key:expr => $value:expr),*) => {
        $crate::log_structured!(warn, $message, $($key => $value),*)
    };
}

/// Convenience macro for error logging with structured data
#[macro_export]
macro_rules! log_error {
    ($message:expr, $($key:expr => $value:expr),*) => {
        $crate::log_structured!(error, $message, $($key => $value),*)
    };
}

/// Log buffer for storing recent logs in memory
pub struct LogBuffer {
    entries: Arc<RwLock<Vec<LogEntry>>>,
    max_size: usize,
}

impl LogBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_size,
        }
    }

    pub async fn add_entry(&self, entry: LogEntry) {
        let mut entries = self.entries.write().await;
        entries.push(entry);

        // Keep only the last max_size entries
        let len = entries.len();
        if len > self.max_size {
            entries.drain(0..len - self.max_size);
        }
    }

    pub async fn get_entries(&self) -> Vec<LogEntry> {
        self.entries.read().await.clone()
    }

    pub async fn get_entries_by_level(&self, level: &str) -> Vec<LogEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.level.eq_ignore_ascii_case(level))
            .cloned()
            .collect()
    }

    pub async fn get_entries_by_module(&self, module: &str) -> Vec<LogEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| {
                if let Some(m) = &e.module {
                    m.contains(module)
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    pub async fn get_count(&self) -> usize {
        self.entries.read().await.len()
    }
}
