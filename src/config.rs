use crate::{alist, alist2strm, ani2alist};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rust_nested_example_config() {
        let config: Config = serde_yaml::from_str(include_str!("../config/config.example.yaml"))
            .expect("example config should parse");

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
}
