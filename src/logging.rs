mod daily_file_appender;

use chrono::Utc;
use chrono_tz::Tz;
use tracing::level_filters::LevelFilter;
use tracing::{Level, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use daily_file_appender::DailyFileAppender;
use tracing_subscriber::fmt::time::FormatTime;

/// 自定义一个结构体，用来存放 chrono_tz 的时区
#[derive(Debug, Clone)]
struct ChronoTzTimer {
    timezone: chrono_tz::Tz,
}

impl ChronoTzTimer {
    fn new(timezone: chrono_tz::Tz) -> Self {
        Self { timezone }
    }
}

// 为我们的结构体实现 tracing-subscriber 的 FormatTime 特征
impl FormatTime for ChronoTzTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        // 1. 获取当前的 UTC 时间
        let utc_now = Utc::now();

        // 2. 将 UTC 时间转换为经由 chrono_tz::Tz 指定的时区时间
        let local_now = utc_now.with_timezone(&self.timezone);

        // 3. 格式化为你想要的字符串样式并写入日志（例如: 2026-06-05 15:30:00）
        write!(w, "{}", local_now.format("%+"))
    }
}

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
    colorful_log: bool,
) -> Result<Option<LoggingGuard>, Box<dyn std::error::Error + Send + Sync>> {
    let level_filter = LevelFilter::from_level(level);
    let tz_timer = ChronoTzTimer::new(tz);

    let subscriber = tracing_subscriber::registry();

    let console_layer = tracing_subscriber::fmt::layer()
        .with_timer(tz_timer.clone())
        .with_target(true)
        .with_line_number(true)
        .with_ansi(colorful_log)
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
        .with_timer(tz_timer)
        .with_target(true)
        .with_line_number(true)
        .with_ansi(false)
        .with_writer(file_writer)
        .with_filter(level_filter);

    subscriber.with(file_layer).init();

    info!(log_path = %log_path.display(), "日志文件输出已启用");

    Ok(Some(LoggingGuard {
        _file_guard: file_guard,
    }))
}
