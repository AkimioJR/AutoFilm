mod app_info;
mod alist2strm;
mod config;
mod extensions;

use std::env;
use std::path::PathBuf;

use alist2strm::Alist2Strm;
use config::Config;
use tokio_cron_scheduler::{Job, JobScheduler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    app_info::print_banner();

    // Rust 版默认读取 config/config.yaml，也可以通过第一个命令行参数指定配置文件。
    let config_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/config.yaml"));
    let config = Config::load(config_path)?;

    if config.alist2strm_tasks.is_empty() {
        println!("未检测到 Alist2Strm 任务配置");
        return Ok(());
    }

    let mut scheduler = JobScheduler::new().await?;
    let mut scheduled_count = 0usize;

    for task in config.alist2strm_tasks {
        let task_id = task.id.clone();
        let Some(cron) = task.cron.as_deref().and_then(normalize_cron) else {
            println!("Alist2Strm 任务缺少 cron，已跳过：{task_id}");
            continue;
        };

        // tokio-cron-scheduler 使用带秒字段的 cron；normalize_cron 会兼容常见 5 字段写法。
        scheduler
            .add(Job::new_async(cron.as_str(), move |_uuid, _lock| {
                let task = task.clone();
                let task_id = task_id.clone();
                Box::pin(async move {
                    println!("开始执行 Alist2Strm 任务：{task_id}");
                    if let Err(err) = Alist2Strm::new(task).run().await {
                        eprintln!("Alist2Strm 任务失败：{task_id}，错误：{err}");
                    } else {
                        println!("Alist2Strm 任务完成：{task_id}");
                    }
                })
            })?)
            .await?;
        scheduled_count += 1;
    }

    if scheduled_count == 0 {
        println!("没有可调度的 Alist2Strm 任务");
        return Ok(());
    }

    scheduler.start().await?;
    println!("AutoFilm 调度器启动完成，已添加 {scheduled_count} 个 Alist2Strm 任务");

    // 阻塞主任务，直到收到 Ctrl-C；调度器会在后台按 cron 触发任务。
    tokio::signal::ctrl_c().await?;
    println!("AutoFilm 收到退出信号");
    scheduler.shutdown().await?;
    Ok(())
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
    use super::normalize_cron;

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
}
