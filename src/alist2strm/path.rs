use alist::models::fs::ObjResp;
use std::path::{Path, PathBuf};

use crate::extensions::VIDEO_EXTS;

#[derive(Debug, Clone)]
pub struct AlistPath {
    // AList 站点根地址，用于拼接 /d 下载链接。
    pub server_url: String,
    // 当前用户 base_path；非根目录用户生成下载链接时必须保留。
    pub base_path: String,
    // 文件相对当前用户根目录的完整路径。
    pub full_path: String,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified_timestamp: i64,
    pub sign: String,
    pub raw_url: Option<String>,
}

impl AlistPath {
    /// 将 AList `/api/fs/list` 返回的对象转换为运行期路径模型。
    pub fn from_obj(
        server_url: impl Into<String>,
        base_path: impl Into<String>,
        parent_path: &str,
        object: ObjResp,
    ) -> Self {
        let full_path = join_alist_path(parent_path, &object.name);
        Self {
            server_url: server_url.into(),
            base_path: base_path.into(),
            full_path,
            name: object.name,
            size: object.size.max(0) as u64,
            is_dir: object.is_dir,
            modified_timestamp: object.modified.timestamp(),
            sign: object.sign,
            raw_url: None,
        }
    }

    pub fn with_raw_url(mut self, raw_url: String) -> Self {
        self.raw_url = Some(raw_url);
        self
    }

    pub fn suffix(&self) -> String {
        suffix_of(&self.name)
    }

    pub fn download_url(&self) -> String {
        // Python 版本的下载链接规则是：server + "/d" + base_path + full_path + sign。
        let abs_path = format!(
            "{}{}",
            self.base_path.trim_end_matches('/'),
            ensure_leading_slash(&self.full_path)
        );
        let mut url = format!(
            "{}/d{}",
            self.server_url.trim_end_matches('/'),
            encode_path(&abs_path)
        );
        if !self.sign.is_empty() {
            url.push_str("?sign=");
            url.push_str(&urlencoding::encode(&self.sign));
        }
        url
    }

    pub fn local_path(
        &self,
        source_dir: &str,
        target_dir: &Path,
        flatten_mode: bool,
        bdmv_root: Option<&str>,
    ) -> PathBuf {
        // BDMV 只为最大 m2ts 生成一个电影标题 .strm，避免整盘结构被扫成多个视频。
        if let Some(bdmv_root) = bdmv_root {
            let movie_title = movie_title_from_bdmv_root(bdmv_root);
            return if flatten_mode {
                target_dir.join(format!("{movie_title}.strm"))
            } else {
                let relative_path = relative_from_source(bdmv_root, source_dir);
                target_dir
                    .join(relative_path)
                    .join(format!("{movie_title}.strm"))
            };
        }

        let mut local_path = if flatten_mode {
            target_dir.join(&self.name)
        } else {
            target_dir.join(relative_from_source(&self.full_path, source_dir))
        };

        if VIDEO_EXTS.contains(&self.suffix().as_str()) {
            local_path.set_extension("strm");
        }
        local_path
    }
}

pub fn suffix_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((_, ext)) => format!(".{}", ext.to_ascii_lowercase()),
        None => String::new(),
    }
}

pub fn is_bdmv_file(path: &AlistPath) -> bool {
    // 只处理 BDMV/STREAM 下的 m2ts；其它 BDMV 内部文件全部跳过。
    path.full_path.contains("/BDMV/STREAM/") && path.suffix() == ".m2ts"
}

pub fn bdmv_root(path: &AlistPath) -> Option<String> {
    path.full_path
        .find("/BDMV/")
        .map(|index| path.full_path[..index].to_string())
        .filter(|root| !root.is_empty())
}

pub fn movie_title_from_bdmv_root(root: &str) -> String {
    root.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("BDMV")
        .to_string()
}

pub fn join_alist_path(parent: &str, name: &str) -> String {
    let parent = parent.trim_end_matches('/');
    if parent.is_empty() {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    }
}

fn relative_from_source(path: &str, source_dir: &str) -> PathBuf {
    // 非平铺模式下，本地目录结构与 source_dir 下的云端结构保持一致。
    let source = source_dir.trim_end_matches('/');
    let relative = path
        .strip_prefix(source)
        .unwrap_or(path)
        .trim_start_matches('/');
    PathBuf::from(relative)
}

fn ensure_leading_slash(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn encode_path(path: &str) -> String {
    path.split('/')
        .map(urlencoding::encode)
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_alist_paths_without_double_slashes() {
        assert_eq!(join_alist_path("/", "movie.mkv"), "/movie.mkv");
        assert_eq!(join_alist_path("/media", "movie.mkv"), "/media/movie.mkv");
    }

    #[test]
    fn extracts_bdmv_root_and_title() {
        let path = AlistPath {
            server_url: "http://alist".into(),
            base_path: String::new(),
            full_path: "/电影/Example/BDMV/STREAM/00001.m2ts".into(),
            name: "00001.m2ts".into(),
            size: 1,
            is_dir: false,
            modified_timestamp: 0,
            sign: String::new(),
            raw_url: None,
        };
        assert_eq!(bdmv_root(&path).as_deref(), Some("/电影/Example"));
        assert_eq!(movie_title_from_bdmv_root("/电影/Example"), "Example");
    }
}
