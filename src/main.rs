mod alist2strm;
mod config;
mod extensions;

use std::env;
use std::path::PathBuf;

use alist2strm::Alist2Strm;
use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Rust 版当前是一次执行模型：启动后读取配置并顺序跑完所有 Alist2Strm 任务。
    let config_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/config.yaml"));
    let config = Config::load(config_path)?;

    if config.alist2strm_tasks.is_empty() {
        println!("未检测到 Alist2Strm 任务配置");
        return Ok(());
    }

    for task in config.alist2strm_tasks {
        let task_id = task.id.clone();
        println!("开始执行 Alist2Strm 任务：{task_id}");
        Alist2Strm::new(task).run().await?;
        println!("Alist2Strm 任务完成：{task_id}");
    }

    Ok(())
}

