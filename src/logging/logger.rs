use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Ring buffer for UI log viewer (last 500 entries)
pub static LOG_BUFFER: Lazy<Mutex<Vec<LogEntry>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

const MAX_BUFFER_ENTRIES: usize = 500;

pub fn add_to_buffer(level: &str, message: &str) {
    if let Ok(mut buf) = LOG_BUFFER.lock() {
        let entry = LogEntry {
            timestamp: chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string(),
            level: level.to_string(),
            message: message.to_string(),
        };
        buf.push(entry);
        if buf.len() > MAX_BUFFER_ENTRIES {
            buf.remove(0);
        }
    }
}

pub fn get_log_entries() -> Vec<LogEntry> {
    LOG_BUFFER
        .lock()
        .map(|buf| buf.clone())
        .unwrap_or_default()
}

pub fn clear_buffer() {
    if let Ok(mut buf) = LOG_BUFFER.lock() {
        buf.clear();
    }
}

pub fn init_logging(logs_dir: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(logs_dir)?;

    let file_appender = tracing_appender::rolling::daily(logs_dir, "game-optimizer");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Leak the guard so it lives for the process lifetime
    Box::leak(Box::new(guard));

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .init();

    Ok(())
}
