mod path;
mod protection;
mod runner;

pub use runner::Alist2Strm;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlistConfig {
    // AList 服务地址，允许不写协议；运行时会默认补 https://。
    pub base_url: String,
    #[serde(default)]
    pub public_url: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub otp_code: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadOption {
    #[serde(default)]
    pub subtitle: bool,
    #[serde(default)]
    pub image: bool,
    #[serde(default)]
    pub nfo: bool,
    #[serde(default, deserialize_with = "deserialize_exts")]
    pub other_ext: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize)]
pub enum Mode {
    #[default]
    AlistURL,
    RawURL,
    AlistPath,
}

impl<'de> Deserialize<'de> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_str(&value))
    }
}

impl Mode {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "rawurl" => Self::RawURL,
            "alistpath" => Self::AlistPath,
            _ => Self::AlistURL,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmartProtection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_protection_threshold")]
    pub threshold: usize,
    #[serde(default = "default_grace_scans")]
    pub grace_scans: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub ignore: Option<String>,
    #[serde(default)]
    pub smart_protection: Option<SmartProtection>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    // 任务 ID 用于日志、保护状态文件名等场景。
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub cron: Option<String>,
    pub alist: AlistConfig,
    pub source_dir: String,
    pub target_dir: PathBuf,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub flatten_mode: bool,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub download: DownloadOption,
    #[serde(default)]
    pub sync: Option<SyncConfig>,
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,
    #[serde(default = "default_max_downloaders")]
    pub max_downloaders: usize,
    #[serde(default)]
    pub wait_time: f64,
}

fn default_max_workers() -> usize {
    50
}

fn default_max_downloaders() -> usize {
    5
}

fn default_protection_threshold() -> usize {
    100
}

fn default_grace_scans() -> usize {
    3
}

impl Default for DownloadOption {
    fn default() -> Self {
        Self {
            subtitle: false,
            image: false,
            nfo: false,
            other_ext: Vec::new(),
        }
    }
}

impl Default for SmartProtection {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: default_protection_threshold(),
            grace_scans: default_grace_scans(),
        }
    }
}

fn deserialize_exts<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Exts {
        Empty(Option<String>),
        String(String),
        List(Vec<String>),
    }

    let exts = match Exts::deserialize(deserializer)? {
        Exts::Empty(None) => Vec::new(),
        Exts::Empty(Some(value)) | Exts::String(value) => value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(normalize_ext)
            .collect(),
        Exts::List(values) => values
            .into_iter()
            .map(|value| normalize_ext(&value))
            .collect(),
    };
    Ok(exts)
}

fn normalize_ext(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    if value.starts_with('.') {
        value
    } else {
        format!(".{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mode_case_insensitively() {
        assert_eq!(Mode::from_str("rawurl"), Mode::RawURL);
        assert_eq!(Mode::from_str("AlistPath"), Mode::AlistPath);
        assert_eq!(Mode::from_str("unknown"), Mode::AlistURL);
    }
}
