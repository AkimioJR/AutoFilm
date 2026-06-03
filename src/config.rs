use crate::alist2strm;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    // Rust 版统一使用 snake_case 根字段；旧 Python 平铺配置不再兼容。
    #[serde(default)]
    pub alist2strm_tasks: Vec<alist2strm::Config>,
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
        assert_eq!(
            config.alist2strm_tasks[0].alist.base_url,
            "http://alist:5244"
        );
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
    }
}
