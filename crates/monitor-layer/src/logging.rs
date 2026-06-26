use quant_common::types::LogEntry;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{
    field::{Field, Visit},
    Event, Subscriber,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
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
    let level_filter = parse_log_level(&config.log_level);
    let env_filter = EnvFilter::from_default_env().add_directive(level_filter.into());

    if config.enable_file_logging {
        let file_appender = RollingFileAppender::new(
            Rotation::DAILY,
            &config.log_dir,
            format!("{}-log", config.service_name),
        );

        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        if config.enable_stdout_logging {
            // Both file and stdout layers simultaneously
            if config.enable_json_logging {
                tracing_subscriber::registry()
                    .with(fmt::layer().json().with_writer(non_blocking))
                    .with(fmt::layer().json().with_writer(std::io::stdout))
                    .with(env_filter)
                    .try_init()?;
            } else {
                tracing_subscriber::registry()
                    .with(fmt::layer().with_writer(non_blocking))
                    .with(fmt::layer().with_writer(std::io::stdout))
                    .with(env_filter)
                    .try_init()?;
            }
        } else {
            // File layer only
            if config.enable_json_logging {
                tracing_subscriber::registry()
                    .with(fmt::layer().json().with_writer(non_blocking))
                    .with(env_filter)
                    .try_init()?;
            } else {
                tracing_subscriber::registry()
                    .with(fmt::layer().with_writer(non_blocking))
                    .with(env_filter)
                    .try_init()?;
            }
        }
    } else if config.enable_stdout_logging {
        // Stdout layer only
        if config.enable_json_logging {
            tracing_subscriber::registry()
                .with(fmt::layer().json().with_writer(std::io::stdout))
                .with(env_filter)
                .try_init()?;
        } else {
            tracing_subscriber::registry()
                .with(fmt::layer().with_writer(std::io::stdout))
                .with(env_filter)
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
        _ => {
            tracing::warn!("Unknown log level '{}', defaulting to info", level);
            LevelFilter::INFO
        }
    }
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

struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

pub struct LogBufferLayer {
    buffer: Arc<LogBuffer>,
}

impl LogBufferLayer {
    pub fn new(buffer: Arc<LogBuffer>) -> Self {
        Self { buffer }
    }
}

impl<S: Subscriber> Layer<S> for LogBufferLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();
        let level = metadata.level().to_string();
        let target = metadata.target().to_string();
        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);
        let message = if visitor.message.is_empty() {
            metadata.name().to_string()
        } else {
            visitor.message
        };
        let buffer = self.buffer.clone();
        let entry = LogEntry {
            timestamp: chrono::Utc::now(),
            level,
            message,
            module: Some(target),
        };
        tokio::spawn(async move {
            buffer.add_entry(entry).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ── parse_log_level ──

    #[test]
    fn test_parse_log_level_valid_levels() {
        assert_eq!(parse_log_level("trace"), LevelFilter::TRACE);
        assert_eq!(parse_log_level("debug"), LevelFilter::DEBUG);
        assert_eq!(parse_log_level("info"), LevelFilter::INFO);
        assert_eq!(parse_log_level("warn"), LevelFilter::WARN);
        assert_eq!(parse_log_level("error"), LevelFilter::ERROR);
    }

    #[test]
    fn test_parse_log_level_case_insensitive() {
        assert_eq!(parse_log_level("INFO"), LevelFilter::INFO);
        assert_eq!(parse_log_level("Debug"), LevelFilter::DEBUG);
        assert_eq!(parse_log_level("TRACE"), LevelFilter::TRACE);
    }

    #[test]
    fn test_parse_log_level_invalid_falls_back_to_info() {
        assert_eq!(parse_log_level("superdebug"), LevelFilter::INFO);
        assert_eq!(parse_log_level(""), LevelFilter::INFO);
        assert_eq!(parse_log_level("verbose"), LevelFilter::INFO);
    }

    // ── MessageVisitor ──

    // ── LogBuffer ──

    #[tokio::test]
    async fn test_log_buffer_add_and_get_entries() {
        let buffer = LogBuffer::new(100);
        let entry = LogEntry::new("INFO", "test message", Some("test".into()));
        buffer.add_entry(entry).await;

        let entries = buffer.get_entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, "INFO");
        assert_eq!(entries[0].message, "test message");
    }

    #[tokio::test]
    async fn test_log_buffer_max_size_drain() {
        let buffer = LogBuffer::new(3);
        for i in 0..5 {
            let entry = LogEntry::new("INFO", format!("msg {}", i), None);
            buffer.add_entry(entry).await;
        }

        let entries = buffer.get_entries().await;
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].message, "msg 2");
        assert_eq!(entries[1].message, "msg 3");
        assert_eq!(entries[2].message, "msg 4");
    }

    #[tokio::test]
    async fn test_log_buffer_get_entries_by_level() {
        let buffer = LogBuffer::new(100);
        buffer
            .add_entry(LogEntry::new("INFO", "info msg", None))
            .await;
        buffer
            .add_entry(LogEntry::new("WARN", "warn msg", None))
            .await;
        buffer
            .add_entry(LogEntry::new("ERROR", "error msg", None))
            .await;

        let warn_entries = buffer.get_entries_by_level("WARN").await;
        assert_eq!(warn_entries.len(), 1);
        assert_eq!(warn_entries[0].message, "warn msg");
    }

    #[tokio::test]
    async fn test_log_buffer_get_entries_by_level_case_insensitive() {
        let buffer = LogBuffer::new(100);
        buffer
            .add_entry(LogEntry::new("info", "lowercase", None))
            .await;
        buffer
            .add_entry(LogEntry::new("INFO", "uppercase", None))
            .await;

        let entries = buffer.get_entries_by_level("info").await;
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_log_buffer_get_entries_by_module() {
        let buffer = LogBuffer::new(100);
        buffer
            .add_entry(LogEntry::new("INFO", "main msg", Some("main".into())))
            .await;
        buffer
            .add_entry(LogEntry::new("INFO", "trading msg", Some("trading".into())))
            .await;

        let main_entries = buffer.get_entries_by_module("main").await;
        assert_eq!(main_entries.len(), 1);
        assert_eq!(main_entries[0].message, "main msg");
    }

    #[tokio::test]
    async fn test_log_buffer_clear() {
        let buffer = LogBuffer::new(100);
        buffer.add_entry(LogEntry::new("INFO", "msg", None)).await;
        assert_eq!(buffer.get_count().await, 1);

        buffer.clear().await;
        assert_eq!(buffer.get_count().await, 0);
    }

    #[tokio::test]
    async fn test_log_buffer_get_count() {
        let buffer = LogBuffer::new(100);
        assert_eq!(buffer.get_count().await, 0);
        buffer.add_entry(LogEntry::new("INFO", "a", None)).await;
        buffer.add_entry(LogEntry::new("INFO", "b", None)).await;
        assert_eq!(buffer.get_count().await, 2);
    }

    // ── LogBufferLayer ──

    #[tokio::test]
    async fn test_log_buffer_layer_captures_events() {
        let buffer = Arc::new(LogBuffer::new(100));
        let layer = LogBufferLayer::new(buffer.clone());

        let subscriber = tracing_subscriber::registry().with(layer);
        let dispatch = tracing::Dispatch::new(subscriber);

        tracing::dispatcher::with_default(&dispatch, || {
            tracing::info!("test message via layer");
        });

        // Wait for tokio::spawn to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        let entries = buffer.get_entries().await;
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.contains("test message via layer"));
        assert_eq!(entries[0].level, "INFO");
    }

    #[tokio::test]
    async fn test_log_buffer_layer_captures_multiple_events() {
        let buffer = Arc::new(LogBuffer::new(100));
        let layer = LogBufferLayer::new(buffer.clone());

        let subscriber = tracing_subscriber::registry().with(layer);
        let dispatch = tracing::Dispatch::new(subscriber);

        tracing::dispatcher::with_default(&dispatch, || {
            tracing::info!("first");
            tracing::warn!("second");
            tracing::error!("third");
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let entries = buffer.get_entries().await;
        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn test_log_buffer_layer_captures_level() {
        let buffer = Arc::new(LogBuffer::new(100));
        let layer = LogBufferLayer::new(buffer.clone());

        let subscriber = tracing_subscriber::registry().with(layer);
        let dispatch = tracing::Dispatch::new(subscriber);

        tracing::dispatcher::with_default(&dispatch, || {
            tracing::error!("something went wrong");
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let entries = buffer.get_entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, "ERROR");
    }

    #[tokio::test]
    async fn test_log_buffer_layer_does_not_exceed_max_size() {
        let buffer = Arc::new(LogBuffer::new(2));
        let layer = LogBufferLayer::new(buffer.clone());

        let subscriber = tracing_subscriber::registry().with(layer);
        let dispatch = tracing::Dispatch::new(subscriber);

        tracing::dispatcher::with_default(&dispatch, || {
            tracing::info!("msg 0");
            tracing::info!("msg 1");
            tracing::info!("msg 2");
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let entries = buffer.get_entries().await;
        assert_eq!(entries.len(), 2);
        assert!(entries[0].message.contains("msg 1") || entries[0].message.contains("msg 2"));
    }
}
