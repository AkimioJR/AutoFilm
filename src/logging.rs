use std::fs::OpenOptions;
use std::path::Path;

use chrono::Local;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub struct LoggingGuard {
    _file_guard: WorkerGuard,
}

pub fn init(debug: bool) -> Result<LoggingGuard, Box<dyn std::error::Error + Send + Sync>> {
    // 行为对齐 Python 版本：控制台和 logs/YYYY-MM-DD.log 同时输出。
    std::fs::create_dir_all("logs")?;
    let log_file = Path::new("logs").join(format!("{}.log", Local::now().format("%Y-%m-%d")));
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    let (file_writer, file_guard) = tracing_appender::non_blocking(file);

    let level = if debug {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_ansi(true)
        .with_filter(level);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_ansi(false)
        .with_writer(file_writer)
        .with_filter(level);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    Ok(LoggingGuard {
        _file_guard: file_guard,
    })
}
