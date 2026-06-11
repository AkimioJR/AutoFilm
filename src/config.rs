use crate::{alist, alist2strm, ani2alist};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

const EXAMPLE_CONFIG: &str = include_str!("../config/config.example.yaml");

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("配置文件不存在，已在 {path} 生成示例配置文件，请编辑后重新启动程序")]
    CreatedExample { path: String },

    #[error("读取配置文件失败: {0}")]
    Read(#[source] std::io::Error),

    #[error("创建示例配置文件失败: {0}")]
    CreateExample(#[source] std::io::Error),

    #[error("解析配置文件失败: {0}")]
    Parse(#[from] serde_yaml::Error),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    // Rust 版统一使用 snake_case 根字段；旧 Python 平铺配置不再兼容。
    #[serde(default)]
    pub alist: Vec<alist::AlistConfig>,
    #[serde(default)]
    pub alist2strm_tasks: Vec<alist2strm::Config>,
    #[serde(default)]
    pub ani2alist_tasks: Vec<ani2alist::Config>,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                write_example_config(path)?;
                return Err(Error::CreatedExample {
                    path: path.display().to_string(),
                });
            }
            Err(err) => return Err(Error::Read(err)),
        };

        Ok(serde_yaml::from_str(&content)?)
    }
}

fn write_example_config(path: &Path) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(Error::CreateExample)?;
    }

    fs::write(path, EXAMPLE_CONFIG).map_err(Error::CreateExample)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_rust_nested_example_config() {
        let config: Config =
            serde_yaml::from_str(EXAMPLE_CONFIG).expect("example config should parse");

        assert_eq!(config.alist2strm_tasks.len(), 2);
        assert_eq!(config.ani2alist_tasks.len(), 3);
        assert_eq!(config.alist.len(), 3);
        assert_eq!(config.alist[0].id, "我的Alist");
        assert_eq!(config.alist[0].base_url, "http://alist:5244");
        assert_eq!(config.alist[0].wait_time, 0.0);
        assert_eq!(config.alist2strm_tasks[0].alist, "我的Alist");
        assert!(!config.alist2strm_tasks[0].download.enable);
        assert_eq!(config.alist2strm_tasks[0].download.concurrency, 5);
        assert!(config.alist2strm_tasks[1].download.enable);
        assert!(config.alist2strm_tasks[1].download.subtitle);
        assert!(
            config.alist2strm_tasks[0]
                .sync
                .as_ref()
                .expect("sync config should exist")
                .enabled
        );
        assert!(
            config.alist2strm_tasks[0]
                .sync
                .as_ref()
                .and_then(|sync| sync.smart_protection.as_ref())
                .expect("sync smart protection should exist")
                .enabled
        );
        assert_eq!(config.ani2alist_tasks[0].alist, "OpenList");
        assert_eq!(
            config.ani2alist_tasks[0].source.rss_url,
            "https://api.ani.rip/ani-download.xml"
        );
    }

    #[test]
    fn creates_example_config_when_missing() {
        let config_path = std::env::temp_dir()
            .join(format!(
                "autofilm-config-test-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("system time should be after unix epoch")
                    .as_nanos()
            ))
            .join("config.yaml");

        let result = Config::load(&config_path);

        assert!(matches!(result, Err(Error::CreatedExample { .. })));
        assert_eq!(
            fs::read_to_string(&config_path).expect("example config should be written"),
            EXAMPLE_CONFIG
        );

        fs::remove_file(&config_path).ok();
        if let Some(parent) = config_path.parent() {
            fs::remove_dir(parent).ok();
        }
    }
}
