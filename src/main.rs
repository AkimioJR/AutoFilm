mod app_info;
mod alist2strm;
mod config;
mod extensions;
mod logging;

use std::env;
use std::path::PathBuf;

use alist2strm::Alist2Strm;
use config::Config;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    app_info::print_banner();

    let args = CliArgs::parse();
    let _logging_guard = logging::init(args.debug)?;
    for arg in &args.ignored_args {
        warn!(arg = %arg, "忽略无法识别的启动参数");
    }

    // Rust 版默认读取 config/config.yaml，也可以通过非 --debug 的第一个参数指定配置文件。
    debug!(config_path = %args.config_path.display(), debug = args.debug, "启动参数解析完成");
    let config = Config::load(&args.config_path)?;

    if config.alist2strm_tasks.is_empty() {
        warn!("未检测到 Alist2Strm 任务配置");
        return Ok(());
    }

    let mut scheduler = JobScheduler::new().await?;
    let mut scheduled_count = 0usize;

    for task in config.alist2strm_tasks {
        let task_id = task.id.clone();
        let Some(cron) = task.cron.as_deref().and_then(normalize_cron) else {
            warn!(task_id = %task_id, "Alist2Strm 任务缺少 cron，已跳过");
            continue;
        };

        // tokio-cron-scheduler 使用带秒字段的 cron；normalize_cron 会兼容常见 5 字段写法。
        info!(task_id = %task_id, cron = %cron, "添加 Alist2Strm 定时任务");
        scheduler
            .add(Job::new_async(cron.as_str(), move |_uuid, _lock| {
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
            })?)
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

struct CliArgs {
    debug: bool,
    config_path: PathBuf,
    ignored_args: Vec<String>,
}

impl CliArgs {
    fn parse() -> Self {
        Self::parse_from(env::args().skip(1))
    }

    fn parse_from(args: impl IntoIterator<Item = String>) -> Self {
        let mut debug = false;
        let mut config_path = None;
        let mut ignored_args = Vec::new();

        for arg in args {
            match arg.as_str() {
                "--debug" => debug = true,
                _ if config_path.is_none() => config_path = Some(PathBuf::from(arg)),
                _ => ignored_args.push(arg),
            }
        }

        Self {
            debug,
            config_path: config_path.unwrap_or_else(|| PathBuf::from("config/config.yaml")),
            ignored_args,
        }
    }
}

fn normalize_cron(cron: &str) -> Option<String> {
    let cron = cron.trim();
    if cron.is_empty() {
        return None;
    }

    let fields = cron.split_whitespace().count();
    match fields {
        // 兼容 Python crontab 常用的 5 字段格式：分 时 日 月 周。
        5 => Some(format!("0 {cron}")),
        // tokio-cron-scheduler 支持带秒字段的 6/7 字段格式。
        6 | 7 => Some(cron.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{CliArgs, normalize_cron};

    #[test]
    fn normalizes_five_field_cron_with_seconds() {
        assert_eq!(normalize_cron("0 20 * * *").as_deref(), Some("0 0 20 * * *"));
    }

    #[test]
    fn keeps_six_field_cron_unchanged() {
        assert_eq!(
            normalize_cron("5 0 20 * * *").as_deref(),
            Some("5 0 20 * * *")
        );
    }

    #[test]
    fn rejects_empty_or_invalid_cron() {
        assert_eq!(normalize_cron(""), None);
        assert_eq!(normalize_cron("* * * *"), None);
    }

    #[test]
    fn parses_debug_flag_and_config_path() {
        let args = CliArgs::parse_from(["--debug".to_string(), "config/demo.yaml".to_string()]);
        assert!(args.debug);
        assert_eq!(args.config_path, std::path::PathBuf::from("config/demo.yaml"));
    }

    #[test]
    fn collects_extra_startup_args() {
        let args = CliArgs::parse_from([
            "config/demo.yaml".to_string(),
            "--unknown".to_string(),
            "extra".to_string(),
        ]);
        assert_eq!(args.ignored_args, ["--unknown", "extra"]);
    }
}
