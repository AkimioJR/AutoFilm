mod alist2strm;
mod app_info;
mod config;
mod extensions;
mod logging;
use chrono_tz::Tz;
use clap::Parser;

use std::path::PathBuf;

use alist2strm::Alist2Strm;
use config::Config;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = CliArgs::parse();
    app_info::print_banner();

    if args.show_version {
        let version_info = serde_json::to_string_pretty(&app_info::VERSION_INFO)?;
        println!("{version_info}");
        return Ok(());
    }

    let _logging_guard = logging::init(args.log_level(), &args.log_path)?;
    debug!(
        debug = args.debug,
        config_path = %args.config_path.display(),
        log_path = %args.log_path,
        timezone = ?args.timezone,
        "启动参数解析完成"
    );
    let config = Config::load(&args.config_path)?;

    if config.alist2strm_tasks.is_empty() {
        warn!("未检测到 Alist2Strm 任务配置");
        return Ok(());
    }

    let tz = args.app_timezone();
    info!(timezone = %tz, "使用应用时区");

    let mut scheduler = JobScheduler::new().await?;
    let mut scheduled_count = 0usize;

    for task in config.alist2strm_tasks {
        let task_id = task.id.clone();
        let Some(cron) = task.cron.as_ref() else {
            warn!(task_id = %task_id, "Alist2Strm 任务缺少 cron，已跳过");
            continue;
        };

        info!(task_id = %task_id, cron = %cron, "添加 Alist2Strm 定时任务");
        scheduler
            .add(Job::new_async_tz(
                cron.to_string(),
                tz,
                move |_uuid, _lock| {
                    let task = task.clone();
                    let task_id = task_id.clone();
                    Box::pin(async move {
                        info!(task_id = %task_id, "开始执行 Alist2Strm 任务");
                        if let Err(err) = Alist2Strm::new(task).run().await {
                            error!(task_id = %task_id, error = %err, "Alist2Strm 任务失败");
                        } else {
                            info!(task_id = %task_id, "Alist2Strm 任务完成");
                        }
                    })
                },
            )?)
            .await?;
        scheduled_count += 1;
    }

    if scheduled_count == 0 {
        warn!("没有可调度的 Alist2Strm 任务");
        return Ok(());
    }

    scheduler.start().await?;
    info!(scheduled_count, "AutoFilm 调度器启动完成");

    // 阻塞主任务，直到收到 Ctrl-C；调度器会在后台按 cron 触发任务。
    tokio::signal::ctrl_c().await?;
    info!("AutoFilm 收到退出信号");
    scheduler.shutdown().await?;
    Ok(())
}

#[derive(Debug, Parser)]
#[command(
    name = app_info::VERSION_INFO.app_name,
    disable_version_flag = true
)]
struct CliArgs {
    /// 是否启用调试日志
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// 指定配置文件路径
    #[arg(
        long = "config",
        value_name = "PATH",
        default_value = "config/config.yaml"
    )]
    config_path: PathBuf,

    /// 指定日志文件目录路径（为空表示禁用日志文件写入）
    #[arg(long = "log", value_name = "PATH", default_value = "logs")]
    log_path: String,

    /// 指定时区（默认为系统本地时区，或 UTC 作为后备）
    /// 支持解析 IANA 时区字符串，如 "Asia/Shanghai"、"America/New_York"、 "UTC" 等
    #[arg(long, value_name = "TZ")]
    timezone: Option<Tz>,

    /// 显示版本、Git 与编译信息
    #[arg(short = 'v', long = "version", default_value_t = false)]
    show_version: bool,
}

impl CliArgs {
    fn log_level(&self) -> tracing::Level {
        if self.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        }
    }

    fn app_timezone(&self) -> Tz {
        self.timezone
            .or_else(|| iana_time_zone::get_timezone().ok()?.parse().ok())
            .unwrap_or(chrono_tz::UTC)
    }
}

#[cfg(test)]
mod tests {
    use super::CliArgs;
    use clap::Parser;

    #[test]
    fn parses_debug_flag_and_config_path() {
        let args = CliArgs::parse_from(["autofilm", "--debug", "--config", "config/demo.yaml"]);
        assert!(args.debug);
        assert_eq!(
            args.config_path,
            std::path::PathBuf::from("config/demo.yaml")
        );
    }

    #[test]
    fn rejects_positional_config_path() {
        let result = CliArgs::try_parse_from(["autofilm", "config/demo.yaml"]);
        assert!(result.is_err());
    }

    #[test]
    fn defaults_config_path() {
        let args = CliArgs::parse_from(["autofilm"]);
        assert_eq!(
            args.config_path,
            std::path::PathBuf::from("config/config.yaml")
        );
    }

    #[test]
    fn defaults_log_path() {
        let args = CliArgs::parse_from(["autofilm"]);
        assert_eq!(args.log_path, std::path::PathBuf::from("logs"));
    }

    #[test]
    fn parses_custom_log_path() {
        let args = CliArgs::parse_from(["autofilm", "--log", "/tmp/autofilm-logs"]);
        assert_eq!(
            args.log_path,
            std::path::PathBuf::from("/tmp/autofilm-logs")
        );
    }

    #[test]
    fn defaults_timezone_to_none() {
        let args = CliArgs::parse_from(["autofilm"]);
        assert_eq!(args.timezone, None);
    }

    #[test]
    fn parses_cli_timezone_shanghai() {
        let args = CliArgs::parse_from(["autofilm", "--timezone", "Asia/Shanghai"]);
        assert_eq!(args.timezone, Some(chrono_tz::Asia::Shanghai));
    }

    #[test]
    fn parses_cli_timezone_new_york() {
        let args = CliArgs::parse_from(["autofilm", "--timezone", "America/New_York"]);
        assert_eq!(args.timezone, Some(chrono_tz::America::New_York));
    }

    #[test]
    fn parses_cli_timezone_utc() {
        let args = CliArgs::parse_from(["autofilm", "--timezone", "UTC"]);
        assert_eq!(args.timezone, Some(chrono_tz::UTC));
    }

    #[test]
    fn rejects_invalid_cli_timezone() {
        let result = CliArgs::try_parse_from(["autofilm", "--timezone", "Not/AZone"]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_version_flag() {
        let args = CliArgs::parse_from(["autofilm", "--version"]);
        assert!(args.show_version);
    }
}
