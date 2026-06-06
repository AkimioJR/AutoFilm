use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use alist::{Authentication, Client};
use tokio::fs;
use tracing::info;

use crate::alist2strm::config::AlistConfig;
use crate::alist2strm::errors::{Error, Result};
use crate::alist2strm::path::AlistPath;

/// 根据配置构建 AList API 客户端。
///
/// 优先使用永久 token；未配置 token 时使用用户名、密码和可选 OTP 登录。
/// 同时会应用请求间隔配置，用于降低对 AList 或上游存储的请求压力。
pub(super) fn build_client(config: &AlistConfig) -> Result<Client> {
    let base_url = normalize_url(&config.base_url);
    let request_interval =
        (config.wait_time > 0.0).then(|| Duration::from_secs_f64(config.wait_time));
    if let Some(token) = config
        .token
        .as_deref()
        .filter(|token| !token.trim().is_empty())
    {
        return Ok(Client::with_token(base_url, token.to_string())?
            .with_api_request_interval(request_interval));
    }

    let username = config.username.as_deref().filter(|value| !value.is_empty());
    let password = config.password.as_deref().filter(|value| !value.is_empty());
    match (username, password) {
        (Some(username), Some(password)) => Ok(Client::with_authentication(
            base_url,
            Authentication::username_password(
                username.to_string(),
                password.to_string(),
                config.otp_code.clone(),
            ),
        )?
        .with_api_request_interval(request_interval)),
        _ => Err(Error::MissingAuthentication),
    }
}

/// 规范化 AList 地址。
///
/// 函数会去除首尾空白和末尾 `/`；如果用户没有写协议，则默认补 `https://`。
/// 这样后续拼接 API 地址和下载地址时可以使用稳定格式。
pub(super) fn normalize_url(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

/// 判断本地伴生文件是否已经过期或疑似损坏。
///
/// 如果本地文件大小小于远端大小，或本地修改时间不晚于远端修改时间，则认为
/// 需要重新下载。该逻辑只在 `overwrite=false` 且本地文件已存在时用于伴生文件。
pub(super) fn companion_file_is_stale(local_path: &Path, remote_path: &AlistPath) -> Result<bool> {
    let metadata = std::fs::metadata(local_path)?;
    if metadata.len() < remote_path.size {
        return Ok(true);
    }
    if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
            return Ok(duration.as_secs() as i64 <= remote_path.modified_timestamp);
        }
    }
    Ok(false)
}

/// 收集目标目录中的本地文件集合。
///
/// 平铺模式只扫描目标目录第一层文件；非平铺模式会递归扫描所有子目录。
/// 返回结果用于和远端扫描得到的 `processed_local_paths` 比较，找出需要清理的
/// 过期本地文件。
pub(super) async fn collect_local_files(
    target_dir: &Path,
    flatten_mode: bool,
) -> Result<HashSet<PathBuf>> {
    let mut files = HashSet::new();
    if !fs::try_exists(target_dir).await? {
        return Ok(files);
    }

    if flatten_mode {
        let mut entries = fs::read_dir(target_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if entry.file_type().await?.is_file() {
                files.insert(path);
            }
        }
        return Ok(files);
    }

    let mut dirs = vec![target_dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            let path = entry.path();
            if file_type.is_dir() {
                dirs.push(path);
            } else if file_type.is_file() {
                files.insert(path);
            }
        }
    }
    Ok(files)
}

/// 从指定父目录开始向上删除空目录。
///
/// 删除会在遇到非空目录或到达任务目标目录时停止。该函数用于移除过期文件后
/// 清理遗留的空目录层级，不会删除 `target_dir` 本身。
pub(super) async fn remove_empty_parents(parent: Option<&Path>, target_dir: &Path) -> Result<()> {
    let mut current = parent.map(Path::to_path_buf);
    while let Some(dir) = current {
        if dir == target_dir {
            break;
        }

        let mut entries = fs::read_dir(&dir).await?;
        if entries.next_entry().await?.is_some() {
            break;
        }

        fs::remove_dir(&dir).await?;
        info!(path = %dir.display(), "删除空目录");
        current = dir.parent().map(Path::to_path_buf);
    }
    Ok(())
}
