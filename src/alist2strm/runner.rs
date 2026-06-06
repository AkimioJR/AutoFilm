use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::Ordering};

use alist::Client;
use alist::models::fs::{FsGetReq, FsListReq};
use futures_util::future::{BoxFuture, FutureExt};
use futures_util::stream::{FuturesUnordered, StreamExt};
use regex::Regex;
use reqwest::StatusCode;

use crate::alist2strm::config::{Config, Mode, SmartProtectionConfig};
use crate::alist2strm::errors::{Error, Result};
use crate::alist2strm::path::{AlistPath, bdmv_root, is_bdmv_file};
use crate::alist2strm::protection::ProtectionManager;
use crate::alist2strm::summary::{RunStats, RunSummary};
use crate::alist2strm::utils::{
    collect_local_files, companion_file_is_stale, remove_empty_parents,
};
use crate::extensions::{IMAGE_EXTS, NFO_EXTS, SUBTITLE_EXTS, VIDEO_EXTS};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
#[derive(Debug, Clone)]
pub struct Alist2Strm {
    config: Config,
    client: Arc<Client>,
    server_url: String,
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
    stats: Arc<RunStats>,
}

impl Alist2Strm {
    /// 创建一个 Alist2Strm 任务执行器。
    ///
    /// 执行器保存任务配置和共享的 AList 客户端；真正的扫描和本地文件处理
    /// 发生在 `run` 中。
    pub fn new(config: Config, client: Arc<Client>, server_url: String) -> Self {
        Self {
            config,
            client,
            server_url,
        }
    }

    /// 执行一次完整的 Alist2Strm 同步任务。
    ///
    /// 流程包括创建 AList/HTTP 上下文、扫描远端目录、边扫描边处理普通文件、
    /// 收集并处理 BDMV 主片、按需清理本地过期文件。单个远端目录或文件失败会
    /// 记录日志并跳过，初始化和本地清理等关键错误仍会返回给调度器。
    pub async fn run(&self) -> Result<RunSummary> {
        // 每次 run 都重新创建轻量上下文，确保 base_path 和配置派生项是当前值。
        let stats = Arc::new(RunStats::new());
        let context = Arc::new(self.create_context(stats.clone()).await?);
        info!(task_id = %self.config.id, source_dir = %self.config.source_dir, "开始扫描 AList 目录");
        let mut processed_local_paths = HashSet::new();
        let mut bdmv_collections: HashMap<String, Vec<AlistPath>> = HashMap::new();

        // 第一阶段边遍历远端文件边处理普通文件；BDMV 文件先收集，扫描结束后再选主片处理。
        self.collect_and_process_paths(
            context.clone(),
            &mut processed_local_paths,
            &mut bdmv_collections,
        )
        .await?;

        // 第二阶段为每个 BDMV 目录挑选最大的 m2ts，生成单个电影标题 .strm。
        let bdmv_paths = self.finalize_bdmv_paths(bdmv_collections);
        context
            .stats
            .bdmv_selected_count
            .store(bdmv_paths.len(), Ordering::Relaxed);
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

        let summary = stats.summarize(&self.config);
        Ok(summary)
    }

    /// 创建本次运行共享的上下文。
    ///
    /// 该函数会复用 AList 客户端、读取当前用户 `base_path`、准备 HTTP 下载
    /// 客户端、计算需要处理和下载的扩展名集合，并编译同步忽略规则。
    async fn create_context(&self, stats: Arc<RunStats>) -> Result<RunContext> {
        let client = self.client.clone();
        let me = client.me().await?;
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
            server_url: self.server_url.clone(),
            base_path: me.base_path,
            download_semaphore: Arc::new(Semaphore::new(self.config.max_downloaders.max(1))),
            download_exts,
            process_exts,
            sync_ignore_pattern,
            stats,
        })
    }

    /// 扫描远端目录并立即处理普通文件。
    ///
    /// 函数使用显式栈遍历 `source_dir` 下的 AList 目录；扫描到普通文件后会
    /// 立刻提交到有上限的并发处理队列，避免等全量扫描完成才开始写入本地。
    /// BDMV 候选文件会先收集到分组中，等待扫描结束后统一选择主片。
    async fn collect_and_process_paths(
        &self,
        context: Arc<RunContext>,
        processed_local_paths: &mut HashSet<PathBuf>,
        bdmv_collections: &mut HashMap<String, Vec<AlistPath>>,
    ) -> Result<()> {
        // 使用显式栈递归遍历，避免深层目录导致函数递归过深。
        let mut stack = vec![self.config.source_dir.clone()];
        let max_workers = self.config.max_workers.max(1);
        let mut pending: FuturesUnordered<BoxFuture<'_, ()>> = FuturesUnordered::new();

        while let Some(dir_path) = stack.pop() {
            debug!(
                task_id = %self.config.id,
                dir_path = %dir_path,
                "正在扫描 AList 目录"
            );
            let resp = match context.client.fs_list(FsListReq::all(&dir_path)).await {
                Ok(resp) => resp,
                Err(err) => {
                    RunStats::inc(&context.stats.skipped_dir_count);
                    warn!(
                        task_id = %self.config.id,
                        dir_path = %dir_path,
                        error = %err,
                        "获取 AList 目录文件列表失败，已跳过该目录"
                    );
                    continue;
                }
            };
            RunStats::inc(&context.stats.scanned_dir_count);
            debug!(
                task_id = %self.config.id,
                dir_path = %dir_path,
                total = resp.total,
                item_count = resp.content.len(),
                "AList 目录扫描完成"
            );
            for item in resp.content {
                let path = AlistPath::from_obj(
                    context.server_url.clone(),
                    context.base_path.clone(),
                    &dir_path,
                    item.object,
                );

                if path.is_dir {
                    debug!(
                        task_id = %self.config.id,
                        dir_path = %path.full_path,
                        "发现 AList 子目录，加入扫描队列"
                    );
                    stack.push(path.full_path.clone());
                    continue;
                }
                RunStats::inc(&context.stats.discovered_file_count);

                match self.should_process_path(
                    &context,
                    &path,
                    processed_local_paths,
                    bdmv_collections,
                ) {
                    Ok(true) => {
                        debug!(
                            task_id = %self.config.id,
                            path = %path.full_path,
                            "AList 路径加入处理队列"
                        );
                        pending.push(self.process_path_logged(context.clone(), path).boxed());
                        while pending.len() >= max_workers {
                            pending.next().await;
                        }
                    }
                    Ok(false) => {}
                    Err(err) => {
                        RunStats::inc(&context.stats.failed_path_count);
                        warn!(
                            task_id = %self.config.id,
                            path = %path.full_path,
                            error = %err,
                            "判断 AList 路径是否需要处理失败，已跳过该路径"
                        );
                    }
                }
            }
        }

        while pending.next().await.is_some() {}

        Ok(())
    }

    /// 判断一个远端路径是否需要进入处理流程。
    ///
    /// 函数会过滤系统文件、非目标扩展名、BDMV 内部非主片候选文件，并维护
    /// `processed_local_paths` 用于后续同步清理。对于本地已存在的文件，
    /// `overwrite=false` 时会跳过；伴生文件如果过期或大小偏小则仍会重新处理。
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
            RunStats::inc(&context.stats.filtered_file_count);
            return Ok(false);
        }

        if path.full_path.contains("/BDMV/") && !is_bdmv_file(path) {
            debug!(path = %path.full_path, "跳过 BDMV 内部非主片候选文件");
            RunStats::inc(&context.stats.filtered_file_count);
            return Ok(false);
        }

        let suffix = path.suffix();
        if !context.process_exts.contains(&suffix) {
            debug!(path = %path.full_path, suffix = %suffix, "文件后缀不在处理列表中");
            RunStats::inc(&context.stats.filtered_file_count);
            return Ok(false);
        }
        RunStats::inc(&context.stats.matched_file_count);

        if is_bdmv_file(path) {
            // BDMV 需要等同目录所有 m2ts 都收集完，才能按大小选主片。
            if let Some(root) = bdmv_root(path) {
                bdmv_collections.entry(root).or_default().push(path.clone());
                RunStats::inc(&context.stats.bdmv_collection_count);
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
            if local_path.extension().and_then(|ext| ext.to_str()) == Some("strm") {
                RunStats::inc(&context.stats.strm_skipped_count);
            } else {
                RunStats::inc(&context.stats.attachment_skipped_count);
            }
            return Ok(false);
        }

        Ok(true)
    }

    /// 并发处理一组已经确定需要处理的路径。
    ///
    /// 该函数主要用于扫描结束后的 BDMV 主片批处理；普通文件在扫描阶段已经
    /// 通过 `collect_and_process_paths` 边扫描边提交。每个路径的失败会被记录并
    /// 跳过，不会中断同批次其它路径。
    async fn process_paths(&self, context: Arc<RunContext>, paths: Vec<AlistPath>) -> Result<()> {
        // 用 bounded concurrency 限制并发处理数量，避免压垮 AList 或本地 IO。
        futures_util::stream::iter(paths)
            .for_each_concurrent(self.config.max_workers.max(1), |path| {
                let context = context.clone();
                self.process_path_logged(context, path)
            })
            .await;

        Ok(())
    }

    /// 包装单路径处理并记录错误。
    ///
    /// 该函数把 `process_path` 的错误降级为警告日志，使单个文件的 RawURL 获取、
    /// `.strm` 写入或伴生文件下载失败不会影响后续路径继续处理。
    async fn process_path_logged(&self, context: Arc<RunContext>, path: AlistPath) {
        let full_path = path.full_path.clone();
        if let Err(err) = self.process_path(&context, path).await {
            RunStats::inc(&context.stats.failed_path_count);
            warn!(
                task_id = %self.config.id,
                path = %full_path,
                error = %err,
                "处理 AList 路径失败，已跳过该路径"
            );
        }
    }

    /// 处理单个远端文件路径。
    ///
    /// 根据配置模式生成 `.strm` 内容或下载伴生文件：`RawURL` 模式会先调用
    /// `/api/fs/get` 补充原始下载地址；视频文件写入本地 `.strm`，字幕、图片、
    /// nfo 等伴生文件则从 AList 下载链接保存到本地。
    async fn process_path(&self, context: &RunContext, mut path: AlistPath) -> Result<()> {
        debug!(
            task_id = %self.config.id,
            path = %path.full_path,
            mode = ?self.config.mode,
            "正在处理 AList 路径"
        );

        if self.config.mode == Mode::RawURL && path.raw_url.is_none() {
            // RawURL 只有 `/api/fs/get` 才会返回，遍历列表时按需补取详情。
            debug!(
                task_id = %self.config.id,
                path = %path.full_path,
                "正在获取 AList 路径详情以生成 RawURL"
            );
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
        debug!(
            task_id = %self.config.id,
            path = %path.full_path,
            local_path = %local_path.display(),
            "已计算本地目标路径"
        );
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if local_path.extension().and_then(|ext| ext.to_str()) == Some("strm") {
            let existed_before = fs::try_exists(&local_path).await?;
            // 视频文件写入 .strm 内容；伴生文件则直接下载到本地。
            let Some(content) = self.strm_content(&path) else {
                RunStats::inc(&context.stats.failed_path_count);
                warn!(path = %path.full_path, "生成 .strm 的内容为空，跳过");
                return Ok(());
            };
            debug!(
                task_id = %self.config.id,
                path = %path.full_path,
                local_path = %local_path.display(),
                "正在写入 strm 文件"
            );
            fs::write(local_path, content).await?;
            if existed_before {
                RunStats::inc(&context.stats.strm_updated_count);
            } else {
                RunStats::inc(&context.stats.strm_created_count);
            }
            info!(path = %path.full_path, "strm 文件创建成功");
        } else {
            let existed_before = fs::try_exists(&local_path).await?;
            debug!(
                task_id = %self.config.id,
                path = %path.full_path,
                local_path = %local_path.display(),
                "正在下载 AList 伴生文件"
            );
            self.download_file(context, &path.download_url(), &local_path)
                .await?;
            if existed_before {
                RunStats::inc(&context.stats.attachment_updated_count);
            } else {
                RunStats::inc(&context.stats.attachment_downloaded_count);
            }
            info!(path = %path.full_path, local_path = %local_path.display(), "伴生文件下载成功");
        }

        Ok(())
    }

    /// 生成 `.strm` 文件内容。
    ///
    /// `AlistURL` 返回 AList `/d` 下载链接并可替换为 `public_url`；
    /// `RawURL` 返回上游原始下载地址；`AlistPath` 返回远端路径本身。
    /// 如果所需信息缺失会返回 `None`，调用方会跳过该文件。
    fn strm_content(&self, path: &AlistPath) -> Option<String> {
        // 三种模式对应 Python 版本：AList 下载链接、后端原始链接、AList 路径。
        match self.config.mode {
            Mode::AlistURL => Some(path.download_url()),
            Mode::RawURL => path.raw_url.clone(),
            Mode::AlistPath => Some(path.full_path.clone()),
        }
    }

    /// 下载远端伴生文件到本地路径。
    ///
    /// 下载会使用独立信号量限流，避免字幕、图片、nfo 等伴生文件批量同步时
    /// 占满连接；只有 HTTP 200 会被视为成功，其它状态会作为下载错误返回。
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

    /// 清理本地已经不在远端扫描结果中的文件。
    ///
    /// 函数会收集目标目录里的本地文件，与本次扫描确认存在的路径做差集；
    /// 启用智能保护时，大量 `.strm` 删除会先进入宽限确认。删除文件后会尝试
    /// 清理空父目录，保持目标目录整洁。
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
                RunStats::inc(&context.stats.local_delete_ignored_count);
                continue;
            }

            if fs::try_exists(&file_path).await? {
                fs::remove_file(&file_path).await?;
                RunStats::inc(&context.stats.local_deleted_count);
                info!(path = %file_path.display(), "删除本地过期文件");
                remove_empty_parents(file_path.parent(), &self.config.target_dir).await?;
            }
        }

        Ok(())
    }

    /// 从 BDMV 候选集合中选出每个原盘目录的主片。
    ///
    /// 同一个 BDMV 根目录下会选择体积最大的 `.m2ts` 作为主片，最终只生成一个
    /// 电影标题 `.strm`，避免媒体库识别到多个分段文件。
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

    /// 计算远端路径对应的本地输出路径。
    ///
    /// 该函数把任务级别的 `source_dir`、`target_dir`、`flatten_mode` 和 BDMV
    /// 特殊规则传给路径模型，得到最终要写入或下载的本地文件位置。
    fn local_path(&self, path: &AlistPath) -> PathBuf {
        let bdmv_root = is_bdmv_file(path).then(|| bdmv_root(path)).flatten();
        path.local_path(
            &self.config.source_dir,
            &self.config.target_dir,
            self.config.flatten_mode,
            bdmv_root.as_deref(),
        )
    }

    /// 根据下载配置计算需要作为伴生文件保存的扩展名集合。
    ///
    /// 平铺模式下不会下载伴生文件；非平铺模式会根据 subtitle/image/nfo 开关和
    /// `other_ext` 合并出需要额外下载的扩展名。
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

    /// 判断是否启用了本地同步清理。
    ///
    /// 只有配置中存在 `sync` 且 `enabled=true` 时，任务结束后才会删除本地过期
    /// 文件。
    fn sync_enabled(&self) -> bool {
        self.config.sync.as_ref().is_some_and(|sync| sync.enabled)
    }

    /// 返回启用状态下的智能删除保护配置。
    ///
    /// 该配置只作用于 `.strm` 的大规模删除场景，用于防止远端扫描失败导致本地
    /// 媒体库被误清空。
    fn sync_smart_protection(&self) -> Option<&SmartProtectionConfig> {
        self.config
            .sync
            .as_ref()
            .and_then(|sync| sync.smart_protection.as_ref())
            .filter(|config| config.enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_config(target_dir: PathBuf) -> Config {
        Config {
            id: "test".to_string(),
            cron: None,
            alist: "test-alist".to_string(),
            source_dir: "/source".to_string(),
            target_dir,
            mode: Mode::AlistURL,
            flatten_mode: true,
            overwrite: true,
            download: Default::default(),
            sync: None,
            max_workers: 1,
            max_downloaders: 1,
        }
    }

    fn test_context() -> RunContext {
        RunContext {
            client: Arc::new(Client::with_token("http://127.0.0.1:5244", "token").unwrap()),
            http: reqwest::Client::new(),
            server_url: "http://127.0.0.1:5244".to_string(),
            base_path: String::new(),
            download_semaphore: Arc::new(Semaphore::new(1)),
            download_exts: HashSet::new(),
            process_exts: HashSet::new(),
            sync_ignore_pattern: None,
            stats: Arc::new(RunStats::new()),
        }
    }

    #[tokio::test]
    async fn process_paths_logs_item_errors_and_continues() {
        let unique = format!(
            "autofilm-process-error-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let target_dir = std::env::temp_dir().join(unique);
        fs::write(&target_dir, b"not a directory").await.unwrap();

        let runner = Alist2Strm::new(
            test_config(target_dir.clone()),
            Arc::new(Client::with_token("http://127.0.0.1:5244", "token").unwrap()),
            "http://127.0.0.1:5244".to_string(),
        );
        let context = Arc::new(test_context());
        let paths = vec![AlistPath {
            server_url: "http://127.0.0.1:5244".to_string(),
            base_path: String::new(),
            full_path: "/source/movie.mkv".to_string(),
            name: "movie.mkv".to_string(),
            size: 1,
            is_dir: false,
            modified_timestamp: 0,
            sign: String::new(),
            raw_url: None,
        }];

        let result = runner.process_paths(context, paths).await;
        let _ = fs::remove_file(&target_dir).await;

        assert!(result.is_ok());
    }
}
