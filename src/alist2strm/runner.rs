use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use alist::models::fs::{FsGetReq, FsListReq};
use alist::{Authentication, Client};
use futures_util::{StreamExt, TryStreamExt};
use regex::Regex;
use reqwest::StatusCode;
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use crate::alist2strm::path::{AlistPath, bdmv_root, is_bdmv_file};
use crate::alist2strm::protection::ProtectionManager;
use crate::alist2strm::{AlistConfig, Config, Mode};
use crate::extensions::{IMAGE_EXTS, NFO_EXTS, SUBTITLE_EXTS, VIDEO_EXTS};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("alist client error: {0}")]
    Alist(#[from] alist::ClientError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("missing alist authentication, provide token or username/password")]
    MissingAuthentication,
    #[error("download failed with status {status}: {url}")]
    DownloadStatus { status: StatusCode, url: String },
}

#[derive(Debug, Clone)]
pub struct Alist2Strm {
    config: Config,
}

#[derive(Debug)]
struct RunContext {
    client: Arc<Client>,
    http: reqwest::Client,
    server_url: String,
    base_path: String,
    download_semaphore: Arc<Semaphore>,
    download_exts: HashSet<String>,
    process_exts: HashSet<String>,
    sync_ignore_pattern: Option<Regex>,
}

impl Alist2Strm {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> Result<()> {
        // 每次 run 都重新创建上下文，确保 token、base_path 和配置是当前值。
        let context = Arc::new(self.create_context().await?);
        info!(task_id = %self.config.id, source_dir = %self.config.source_dir, "开始扫描 AList 目录");
        let mut processed_local_paths = HashSet::new();
        let mut bdmv_collections: HashMap<String, Vec<AlistPath>> = HashMap::new();
        let mut process_paths = Vec::new();

        // 第一阶段遍历远端文件：普通文件立即加入处理队列，BDMV 文件先收集不处理。
        self.collect_paths(
            &context,
            &mut processed_local_paths,
            &mut bdmv_collections,
            &mut process_paths,
        )
        .await?;

        self.process_paths(context.clone(), process_paths).await?;

        // 第二阶段为每个 BDMV 目录挑选最大的 m2ts，生成单个电影标题 .strm。
        let bdmv_paths = self.finalize_bdmv_paths(bdmv_collections);
        if !bdmv_paths.is_empty() {
            info!(count = bdmv_paths.len(), "开始处理 BDMV 主片文件");
        }
        for path in &bdmv_paths {
            let local_path = self.local_path(path);
            processed_local_paths.insert(local_path);
        }
        self.process_paths(context.clone(), bdmv_paths).await?;

        if self.sync_enabled() {
            self.cleanup_local_files(&context, processed_local_paths)
                .await?;
        }

        info!(task_id = %self.config.id, "Alist2Strm 扫描处理完成");
        Ok(())
    }

    async fn create_context(&self) -> Result<RunContext> {
        let client = Arc::new(build_client(&self.config.alist)?);
        let me = client.me().await?;
        let server_url = normalize_url(&self.config.alist.base_url);
        let download_exts = self.download_exts();
        let mut process_exts = VIDEO_EXTS
            .iter()
            .map(|ext| ext.to_string())
            .collect::<HashSet<_>>();
        process_exts.extend(download_exts.iter().cloned());
        let sync_ignore_pattern = self
            .config
            .sync
            .as_ref()
            .and_then(|sync| sync.ignore.as_deref())
            .filter(|pattern| !pattern.trim().is_empty())
            .map(Regex::new)
            .transpose()?;

        Ok(RunContext {
            client,
            http: reqwest::Client::builder()
                .user_agent(format!("AutoFilm/{}", env!("CARGO_PKG_VERSION")))
                .build()?,
            server_url,
            base_path: me.base_path,
            download_semaphore: Arc::new(Semaphore::new(self.config.max_downloaders.max(1))),
            download_exts,
            process_exts,
            sync_ignore_pattern,
        })
    }

    async fn collect_paths(
        &self,
        context: &RunContext,
        processed_local_paths: &mut HashSet<PathBuf>,
        bdmv_collections: &mut HashMap<String, Vec<AlistPath>>,
        process_paths: &mut Vec<AlistPath>,
    ) -> Result<()> {
        // 使用显式栈递归遍历，避免深层目录导致函数递归过深。
        let mut stack = vec![self.config.source_dir.clone()];

        while let Some(dir_path) = stack.pop() {
            debug!(dir_path = %dir_path, "获取 AList 目录文件列表");
            let resp = context.client.fs_list(FsListReq::all(&dir_path)).await?;
            for item in resp.content {
                let path = AlistPath::from_obj(
                    context.server_url.clone(),
                    context.base_path.clone(),
                    &dir_path,
                    item.object,
                );

                if path.is_dir {
                    stack.push(path.full_path.clone());
                    continue;
                }

                if self.should_process_path(
                    context,
                    &path,
                    processed_local_paths,
                    bdmv_collections,
                )? {
                    process_paths.push(path);
                }
            }
        }

        Ok(())
    }

    fn should_process_path(
        &self,
        context: &RunContext,
        path: &AlistPath,
        processed_local_paths: &mut HashSet<PathBuf>,
        bdmv_collections: &mut HashMap<String, Vec<AlistPath>>,
    ) -> Result<bool> {
        if path.full_path.contains("@eaDir")
            || path.full_path.contains("Thumbs.db")
            || path.full_path.contains(".DS_Store")
        {
            debug!(path = %path.full_path, "跳过系统文件");
            return Ok(false);
        }

        if path.full_path.contains("/BDMV/") && !is_bdmv_file(path) {
            debug!(path = %path.full_path, "跳过 BDMV 内部非主片候选文件");
            return Ok(false);
        }

        let suffix = path.suffix();
        if !context.process_exts.contains(&suffix) {
            debug!(path = %path.full_path, suffix = %suffix, "文件后缀不在处理列表中");
            return Ok(false);
        }

        if is_bdmv_file(path) {
            // BDMV 需要等同目录所有 m2ts 都收集完，才能按大小选主片。
            if let Some(root) = bdmv_root(path) {
                bdmv_collections.entry(root).or_default().push(path.clone());
            }
            return Ok(false);
        }

        let local_path = self.local_path(path);
        processed_local_paths.insert(local_path.clone());

        if !self.config.overwrite && local_path.exists() {
            if context.download_exts.contains(&suffix)
                && companion_file_is_stale(&local_path, path)?
            {
                debug!(path = %path.full_path, local_path = %local_path.display(), "伴生文件已过期或大小不一致，重新处理");
                return Ok(true);
            }
            debug!(path = %path.full_path, local_path = %local_path.display(), "本地文件已存在，跳过处理");
            return Ok(false);
        }

        Ok(true)
    }

    async fn process_paths(&self, context: Arc<RunContext>, paths: Vec<AlistPath>) -> Result<()> {
        // 用 bounded concurrency 限制并发处理数量，避免压垮 AList 或本地 IO。
        futures_util::stream::iter(paths)
            .map(Ok)
            .try_for_each_concurrent(self.config.max_workers.max(1), |path| {
                let context = context.clone();
                async move { self.process_path(&context, path).await }
            })
            .await
    }

    async fn process_path(&self, context: &RunContext, mut path: AlistPath) -> Result<()> {
        if self.config.mode == Mode::RawURL && path.raw_url.is_none() {
            // RawURL 只有 `/api/fs/get` 才会返回，遍历列表时按需补取详情。
            let detail = context
                .client
                .fs_get(FsGetReq {
                    path: path.full_path.clone(),
                    password: String::new(),
                    page: None,
                    per_page: None,
                    refresh: None,
                })
                .await?;
            path = path.with_raw_url(detail.raw_url);
        }

        let local_path = self.local_path(&path);
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if local_path.extension().and_then(|ext| ext.to_str()) == Some("strm") {
            // 视频文件写入 .strm 内容；伴生文件则直接下载到本地。
            let Some(content) = self.strm_content(context, &path) else {
                warn!(path = %path.full_path, "生成 .strm 的内容为空，跳过");
                return Ok(());
            };
            fs::write(local_path, content).await?;
            info!(path = %path.full_path, "strm 文件创建成功");
        } else {
            self.download_file(context, &path.download_url(), &local_path)
                .await?;
            info!(path = %path.full_path, local_path = %local_path.display(), "伴生文件下载成功");
        }

        Ok(())
    }

    fn strm_content(&self, context: &RunContext, path: &AlistPath) -> Option<String> {
        // 三种模式对应 Python 版本：AList 下载链接、后端原始链接、AList 路径。
        match self.config.mode {
            Mode::AlistURL => {
                let content = path.download_url();
                Some(match self.public_url() {
                    Some(public_url) => content.replacen(&context.server_url, &public_url, 1),
                    None => content,
                })
            }
            Mode::RawURL => path.raw_url.clone(),
            Mode::AlistPath => Some(path.full_path.clone()),
        }
    }

    async fn download_file(
        &self,
        context: &RunContext,
        url: &str,
        local_path: &Path,
    ) -> Result<()> {
        // 下载伴生文件有独立限流，避免字幕/图片批量同步时占满连接。
        let _permit = context
            .download_semaphore
            .acquire()
            .await
            .expect("semaphore open");
        let response = context.http.get(url).send().await?;
        let status = response.status();
        if status != StatusCode::OK {
            return Err(Error::DownloadStatus {
                status,
                url: url.to_string(),
            });
        }

        let mut file = fs::File::create(local_path).await?;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            file.write_all(&chunk?).await?;
        }
        Ok(())
    }

    async fn cleanup_local_files(
        &self,
        context: &RunContext,
        processed_local_paths: HashSet<PathBuf>,
    ) -> Result<()> {
        // sync.enabled 开启时，本地多出来的文件会被清理；可配合保护防止大规模误删。
        let all_local_files =
            collect_local_files(&self.config.target_dir, self.config.flatten_mode).await?;
        let mut files_to_delete = all_local_files
            .difference(&processed_local_paths)
            .cloned()
            .collect::<HashSet<_>>();

        if let Some(protection_config) = self.sync_smart_protection() {
            let strm_to_delete = files_to_delete
                .iter()
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("strm"))
                .cloned()
                .collect::<HashSet<_>>();
            let other_files = files_to_delete
                .difference(&strm_to_delete)
                .cloned()
                .collect::<HashSet<_>>();
            let strm_present = processed_local_paths
                .iter()
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("strm"))
                .cloned()
                .collect::<HashSet<_>>();
            let mut protection = ProtectionManager::new(
                self.config.target_dir.clone(),
                &self.config.id,
                protection_config,
            )
            .await?;
            let ready_strm = protection.process(strm_to_delete, &strm_present).await?;
            files_to_delete = other_files.union(&ready_strm).cloned().collect();
        }

        for file_path in files_to_delete {
            if context.sync_ignore_pattern.as_ref().is_some_and(|regex| {
                regex.is_match(
                    file_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(""),
                )
            }) {
                continue;
            }

            if fs::try_exists(&file_path).await? {
                fs::remove_file(&file_path).await?;
                info!(path = %file_path.display(), "删除本地过期文件");
                remove_empty_parents(file_path.parent(), &self.config.target_dir).await?;
            }
        }

        Ok(())
    }

    fn finalize_bdmv_paths(&self, collections: HashMap<String, Vec<AlistPath>>) -> Vec<AlistPath> {
        collections
            .into_values()
            .filter_map(|paths| {
                let selected = paths.into_iter().max_by_key(|path| path.size);
                if let Some(path) = selected.as_ref() {
                    info!(path = %path.full_path, size = path.size, "选择 BDMV 最大 m2ts 作为主片");
                }
                selected
            })
            .collect()
    }

    fn local_path(&self, path: &AlistPath) -> PathBuf {
        let bdmv_root = is_bdmv_file(path).then(|| bdmv_root(path)).flatten();
        path.local_path(
            &self.config.source_dir,
            &self.config.target_dir,
            self.config.flatten_mode,
            bdmv_root.as_deref(),
        )
    }

    fn download_exts(&self) -> HashSet<String> {
        // 平铺模式下不下载伴生文件，与 Python 行为保持一致。
        if self.config.flatten_mode {
            return HashSet::new();
        }

        let mut exts = HashSet::new();
        if self.config.download.subtitle {
            exts.extend(SUBTITLE_EXTS.iter().map(|ext| ext.to_string()));
        }
        if self.config.download.image {
            exts.extend(IMAGE_EXTS.iter().map(|ext| ext.to_string()));
        }
        if self.config.download.nfo {
            exts.extend(NFO_EXTS.iter().map(|ext| ext.to_string()));
        }
        exts.extend(
            self.config
                .download
                .other_ext
                .iter()
                .map(|ext| ext.to_ascii_lowercase()),
        );
        exts
    }

    fn public_url(&self) -> Option<String> {
        self.config
            .alist
            .public_url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .map(normalize_url)
    }

    fn sync_enabled(&self) -> bool {
        self.config.sync.as_ref().is_some_and(|sync| sync.enabled)
    }

    fn sync_smart_protection(&self) -> Option<&crate::alist2strm::SmartProtection> {
        self.config
            .sync
            .as_ref()
            .and_then(|sync| sync.smart_protection.as_ref())
            .filter(|config| config.enabled)
    }
}

fn build_client(config: &AlistConfig) -> Result<Client> {
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

fn normalize_url(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

fn companion_file_is_stale(local_path: &Path, remote_path: &AlistPath) -> Result<bool> {
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

async fn collect_local_files(target_dir: &Path, flatten_mode: bool) -> Result<HashSet<PathBuf>> {
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

async fn remove_empty_parents(parent: Option<&Path>, target_dir: &Path) -> Result<()> {
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
