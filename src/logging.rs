use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub struct LoggingGuard {
    _file_guard: WorkerGuard,
}

pub fn init(level: Level) -> Result<LoggingGuard, Box<dyn std::error::Error + Send + Sync>> {
    // RollingFileAppender 会在进程跨天运行时自动切换到新的日期文件。
    std::fs::create_dir_all("logs")?;
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("")
        .filename_suffix("log")
        .build("logs")?;
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);

    let level_filter = LevelFilter::from_level(level);

    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_ansi(true)
        .with_filter(level_filter);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_ansi(false)
        .with_writer(file_writer)
        .with_filter(level_filter);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    Ok(LoggingGuard {
        _file_guard: file_guard,
    })
}
