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
    ///
    /// 函数会结合父目录和对象名称得到完整远端路径，并保存生成下载链接、
    /// 判断本地文件是否过期和创建 `.strm` 所需的元数据。
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

    /// 为路径对象补充 RawURL 后返回新的路径对象。
    ///
    /// RawURL 只能通过 `/api/fs/get` 获取；在 `RawURL` 模式下处理文件前会调用
    /// 该函数，把详情接口得到的原始下载地址合并到扫描得到的路径模型中。
    pub fn with_raw_url(mut self, raw_url: String) -> Self {
        self.raw_url = Some(raw_url);
        self
    }

    /// 返回文件名的小写扩展名。
    ///
    /// 结果包含前导 `.`，没有扩展名时返回空字符串，用于和视频、字幕、图片等
    /// 扩展名集合直接匹配。
    pub fn suffix(&self) -> String {
        suffix_of(&self.name)
    }

    /// 生成 AList `/d/...` 下载链接。
    ///
    /// 该链接遵循 Python 版本的规则：站点地址 + `/d` + 用户 `base_path` +
    /// 远端完整路径，并在存在签名时附加 `sign` 查询参数。路径组件会做 URL
    /// 编码，确保中文和空格等字符可用于 `.strm` 内容或伴生文件下载。
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

    /// 计算该远端路径对应的本地目标路径。
    ///
    /// 非平铺模式会保留 `source_dir` 之下的目录结构；平铺模式只使用文件名。
    /// 视频文件会转换为 `.strm` 后缀。BDMV 主片会以 BDMV 根目录名生成单个
    /// 电影标题 `.strm`，避免把整盘结构里的多个 m2ts 都暴露给媒体库。
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

/// 返回文件名的小写扩展名。
///
/// 结果包含前导 `.`；文件名没有扩展名时返回空字符串。该函数用于统一处理
/// AList 返回的文件名和配置中的扩展名匹配。
pub fn suffix_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((_, ext)) => format!(".{}", ext.to_ascii_lowercase()),
        None => String::new(),
    }
}

/// 判断路径是否是 BDMV 主片候选文件。
///
/// 只有 `BDMV/STREAM` 目录下的 `.m2ts` 会作为候选参与后续按大小选择；
/// BDMV 目录里的其它文件会被跳过，避免产生大量无意义 `.strm`。
pub fn is_bdmv_file(path: &AlistPath) -> bool {
    // 只处理 BDMV/STREAM 下的 m2ts；其它 BDMV 内部文件全部跳过。
    path.full_path.contains("/BDMV/STREAM/") && path.suffix() == ".m2ts"
}

/// 提取 BDMV 目录的根路径。
///
/// 返回 `/BDMV/` 之前的远端目录，用作同一蓝光原盘内 m2ts 文件的分组键。
/// 该分组会在扫描结束后选出最大 m2ts 作为主片。
pub fn bdmv_root(path: &AlistPath) -> Option<String> {
    path.full_path
        .find("/BDMV/")
        .map(|index| path.full_path[..index].to_string())
        .filter(|root| !root.is_empty())
}

/// 从 BDMV 根路径中提取电影标题。
///
/// 标题来自根路径的最后一级目录；如果路径为空或无法提取，则回退为 `BDMV`。
/// 结果用于生成 BDMV 主片对应的本地 `.strm` 文件名。
pub fn movie_title_from_bdmv_root(root: &str) -> String {
    root.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("BDMV")
        .to_string()
}

/// 拼接 AList 父目录和子项名称。
///
/// 函数会避免父目录末尾的 `/` 造成双斜杠，并确保根目录下的子项仍以
/// `/name` 形式表示。
pub fn join_alist_path(parent: &str, name: &str) -> String {
    let parent = parent.trim_end_matches('/');
    if parent.is_empty() {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    }
}

/// 计算远端路径相对 `source_dir` 的本地相对路径。
///
/// 非平铺模式下用这个结果保持云端目录结构；如果远端路径不在 `source_dir`
/// 之下，则使用原路径去掉开头 `/` 后的形式，避免生成绝对本地路径。
fn relative_from_source(path: &str, source_dir: &str) -> PathBuf {
    // 非平铺模式下，本地目录结构与 source_dir 下的云端结构保持一致。
    let source = source_dir.trim_end_matches('/');
    let relative = path
        .strip_prefix(source)
        .unwrap_or(path)
        .trim_start_matches('/');
    PathBuf::from(relative)
}

/// 确保路径字符串以 `/` 开头。
///
/// 生成 AList 下载链接时需要绝对路径形态；该函数把缺失前导斜杠的路径补齐。
fn ensure_leading_slash(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

/// 对远端路径的每个片段做 URL 编码。
///
/// 函数按 `/` 拆分路径，只编码各个路径片段本身，从而保留目录分隔符并兼容
/// 中文、空格、特殊符号等文件名。
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
