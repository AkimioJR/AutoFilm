mod daily_file_appender;

use chrono_tz::Tz;
use tracing::level_filters::LevelFilter;
use tracing::{Level, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use daily_file_appender::DailyFileAppender;

pub struct LoggingGuard {
    _file_guard: WorkerGuard,
}

/// 初始化日志系统，支持控制台和文件输出。返回一个 `LoggingGuard` 以保持文件 appender 的生命周期。
/// `level` 决定了日志的详细程度
/// `log_path` 指定了日志文件的目录（为空表示禁用文件输出）
/// `tz` 指定使用时区
pub fn init(
    level: Level,
    log_path: &str,
    tz: Tz,
) -> Result<Option<LoggingGuard>, Box<dyn std::error::Error + Send + Sync>> {
    let level_filter = LevelFilter::from_level(level);

    let subscriber = tracing_subscriber::registry();

    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_ansi(true)
        .with_filter(level_filter);

    let subscriber = subscriber.with(console_layer);

    if log_path.is_empty() {
        subscriber.init();
        warn!("日志文件输出已禁用");
        return Ok(None);
    }

    let log_path = std::path::absolute(log_path)?;
    std::fs::create_dir_all(&log_path)?;

    let file_appender = DailyFileAppender::new(log_path.clone(), tz)?;
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_ansi(false)
        .with_writer(file_writer)
        .with_filter(level_filter);

    subscriber.with(file_layer).init();

    info!(log_path = %log_path.display(), "日志文件输出已启用");

    Ok(Some(LoggingGuard {
        _file_guard: file_guard,
    }))
}
