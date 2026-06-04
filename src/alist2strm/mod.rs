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
    #[serde(default)]
    pub wait_time: f64,
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
    /// 从配置文件中的字符串解析 `.strm` 内容模式。
    ///
    /// 这里保持 Python 版本的宽松行为：未知值不会报错，而是回退到
    /// `AlistURL`，避免配置拼写问题导致整个任务无法启动。
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_str(&value))
    }
}

impl Mode {
    /// 将用户配置的模式字符串转换为内部枚举。
    ///
    /// 支持大小写不敏感的 `RawURL`、`AlistPath` 和默认 `AlistURL`。
    /// 未识别的值会返回 `AlistURL`，保持与 Python 版本一致的容错效果。
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
}

/// 返回默认的普通文件处理并发数。
///
/// 该值限制同时生成 `.strm`、获取 RawURL 和处理本地文件的任务数量，
/// 防止一次扫描中创建过多并发操作。
fn default_max_workers() -> usize {
    50
}

/// 返回默认的伴生文件下载并发数。
///
/// 下载字幕、图片、nfo 等伴生文件时会使用独立限流，避免下载连接占满
/// AList 或上游存储资源。
fn default_max_downloaders() -> usize {
    5
}

/// 返回智能删除保护的默认触发阈值。
///
/// 当一次同步准备删除的 `.strm` 数量达到该阈值时，会进入宽限确认流程，
/// 用来降低 AList 故障或扫描不完整时的大量误删风险。
fn default_protection_threshold() -> usize {
    100
}

/// 返回智能删除保护的默认宽限扫描次数。
///
/// 超过删除阈值的 `.strm` 需要连续多次仍被判定为可删除，才会真正移除。
fn default_grace_scans() -> usize {
    3
}

impl Default for DownloadOption {
    /// 构造默认下载选项。
    ///
    /// 默认只生成视频对应的 `.strm`，不额外下载字幕、图片、nfo 或自定义
    /// 扩展名文件。
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
    /// 构造默认智能保护配置。
    ///
    /// 默认不开启保护，但保留阈值和宽限次数的默认值，便于配置中只显式打开
    /// `enabled` 时获得稳定行为。
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: default_protection_threshold(),
            grace_scans: default_grace_scans(),
        }
    }
}

/// 反序列化自定义伴生文件扩展名配置。
///
/// 配置可以为空、逗号分隔字符串，或字符串列表；函数会统一清理空白并规范为
/// 小写且带 `.` 前缀的扩展名，方便后续直接和文件后缀集合匹配。
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

/// 规范化单个文件扩展名。
///
/// 该函数会去除首尾空白、转成小写，并补齐缺失的 `.` 前缀，使配置中的
/// `srt` 和 `.SRT` 都能匹配同一种后缀。
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
