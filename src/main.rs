mod alist;
mod alist2strm;
mod ani2alist;
mod app_info;
mod args;
mod config;
mod extensions;
mod logging;
mod schedule;

use args::CliArgs;
use clap::Parser;
use config::{Config, Error as ConfigError};
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

    let tz = args.app_timezone();

    let _logging_guard = logging::init(args.log_level(), &args.log_path, tz, args.colorful_log)?;
    debug!(
        debug = args.debug,
        config_path = %args.config_path.display(),
        log_path = %args.log_path,
        timezone = ?args.timezone,
        "启动参数解析完成"
    );
    let config = match Config::load(&args.config_path) {
        Ok(config) => config,
        Err(ConfigError::CreatedExample { path }) => {
            error!(config_path = %path, "配置文件不存在，已生成示例配置文件，请编辑后重新启动程序");
            return Ok(());
        }
        Err(err) => return Err(Box::new(err) as Box<dyn std::error::Error + Send + Sync>),
    };

    info!(timezone = %tz, "使用应用时区");

    let (mut scheduler, scheduled_count) = schedule::create_scheduler(config, tz).await?;

    if scheduled_count == 0 {
        warn!("没有可调度的任务");
        return Ok(());
    }

    scheduler.start().await?;
    info!(scheduled_count, "AutoFilm 调度器启动完成");

    // 阻塞主任务，直到收到 Ctrl-C；调度器会在后台按 cron 触发任务。
    tokio::signal::ctrl_c().await?;
    info!("AutoFilm 收到退出信号，正在退出中...");
    scheduler.shutdown().await?;
    info!("AutoFilm 已成功退出");
    Ok(())
}
