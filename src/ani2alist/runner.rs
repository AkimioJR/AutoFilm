use std::sync::Arc;
use std::time::Duration;

use alist::Client;
use alist::models::admin::AdminPageQuery;
use alist::models::admin::storage::{Storage, StorageReq};
use chrono::Local;
use reqwest::StatusCode;
use serde_json::{Value, json};
use thiserror::Error;
use tracing::{debug, info, warn};

use super::config::{Config, UpdateConfig};
use super::url_tree::{FileEntry, Tree};
use super::utils::{
    AniDirectoryResp, current_season, file_url, is_directory_mime, is_supported_file_mime,
    join_url, parse_ani_timestamp, rss_items, season_key, season_key_from_parts,
    template_path_segments,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("alist client error: {0}")]
    Alist(#[from] alist::ClientError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("request failed with status {status}: {url}")]
    HttpStatus { status: StatusCode, url: String },
}

#[derive(Debug)]
pub struct Ani2Alist {
    config: Config,
    client: Arc<Client>,
    http: reqwest::Client,
}

impl Ani2Alist {
    pub fn new(config: Config, client: Arc<Client>) -> Self {
        Self {
            config,
            client,
            http: reqwest::Client::builder()
                .user_agent(format!("AutoFilm/{}", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client should build"),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let storage = self.storage_for_target().await?;
        let mut tree = Tree::parse(&url_structure_from_addition(&storage.addition));

        match &self.config.update {
            UpdateConfig::Rss => {
                info!(task_id = %self.config.id, rss_url = %self.config.source.rss_url, "开始解析 ANI RSS");
                self.update_from_rss(&mut tree).await?;
            }
            UpdateConfig::Latest { template } => {
                let (year, month) = current_season(Local::now());
                let key = season_key_from_parts(year, month);
                let path_prefix = latest_path_prefix(template.as_deref(), year, month);
                info!(
                    task_id = %self.config.id,
                    key = %key,
                    path_prefix = ?path_prefix,
                    "开始解析 ANI 当前季度目录"
                );
                self.update_from_directory_key(&mut tree, &key, path_prefix)
                    .await?;
            }
            UpdateConfig::Season { year, month } => {
                let key = season_key(*year, *month, Local::now());
                info!(task_id = %self.config.id, key = %key, "开始解析 ANI 季度目录");
                self.update_from_directory_key(&mut tree, &key, vec![key.clone()])
                    .await?;
            }
            UpdateConfig::Keyword { keyword } => {
                info!(task_id = %self.config.id, keyword = %keyword, "开始解析 ANI 关键字目录");
                self.update_from_directory_key(&mut tree, keyword, vec![keyword.clone()])
                    .await?;
            }
        }

        let addition = addition_with_structure(&storage.addition, &tree.to_structure())?;
        self.client
            .admin_storage_update(storage_req_from_storage(&storage, addition))
            .await?;
        info!(
            task_id = %self.config.id,
            target_dir = %self.config.target_dir,
            "AList UrlTree 存储器更新完成"
        );
        Ok(())
    }

    async fn update_from_rss(&self, tree: &mut Tree) -> Result<()> {
        let xml = self.get_text(&self.config.source.rss_url).await?;
        for remote_file in rss_items(&xml) {
            tree.upsert_path(&remote_file.path, remote_file.file);
        }
        Ok(())
    }

    async fn update_from_directory_key(
        &self,
        tree: &mut Tree,
        key: &str,
        path_prefix: Vec<String>,
    ) -> Result<()> {
        let root_url = join_url(&self.config.source.source_url, &[key]);
        let mut stack = vec![(root_url, path_prefix)];

        while let Some((url, path)) = stack.pop() {
            debug!(url = %url, "请求 ANI 目录数据");
            let directory = self.get_json::<AniDirectoryResp>(&url).await?;
            for item in directory.files {
                if is_supported_file_mime(&item.mime_type) {
                    let Some(size) = item
                        .size
                        .as_deref()
                        .and_then(|value| value.parse::<u64>().ok())
                    else {
                        warn!(name = %item.name, "ANI 文件缺少有效大小，已跳过");
                        continue;
                    };
                    let modified = item
                        .created_time
                        .as_deref()
                        .and_then(parse_ani_timestamp)
                        .unwrap_or_default();
                    tree.upsert_path(
                        &path,
                        FileEntry {
                            name: item.name.clone(),
                            size,
                            modified,
                            url: file_url(&url, &item.name),
                        },
                    );
                } else if is_directory_mime(&item.mime_type) {
                    let child_url = join_url(&url, &[&item.name]);
                    let mut child_path = path.clone();
                    child_path.push(item.name);
                    stack.push((child_url, child_path));
                } else {
                    warn!(name = %item.name, mime_type = %item.mime_type, "无法识别 ANI 文件类型，已跳过");
                }
            }
        }

        Ok(())
    }

    async fn storage_for_target(&self) -> Result<Storage> {
        let target_dir = normalize_mount_path(&self.config.target_dir);
        let storage_list = self
            .client
            .admin_storage_list(AdminPageQuery::default())
            .await?;
        if let Some(storage) = storage_list
            .content
            .into_iter()
            .find(|storage| storage.mount_path == target_dir)
        {
            return Ok(storage);
        }

        info!(target_dir = %target_dir, "未找到 UrlTree 存储器，开始自动创建");
        let addition = serde_json::to_string(&json!({
            "url_structure": "",
            "head_size": false,
            "writable": false
        }))?;
        let created = self
            .client
            .admin_storage_create(default_url_tree_storage_req(&target_dir, addition))
            .await?;
        Ok(self.client.admin_storage_get(created.id).await?)
    }

    async fn get_text(&self, url: &str) -> Result<String> {
        let response = self.http.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::HttpStatus {
                status,
                url: url.to_string(),
            });
        }
        Ok(response.text().await?)
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let response = self.http.post(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::HttpStatus {
                status,
                url: url.to_string(),
            });
        }
        let body = response.text().await?;
        Ok(serde_json::from_str(&body)?)
    }
}

fn normalize_mount_path(path: &str) -> String {
    format!("/{}", path.trim_matches('/'))
}

fn latest_path_prefix(template: Option<&str>, year: i32, month: u32) -> Vec<String> {
    template
        .filter(|template| !template.trim().is_empty())
        .map(|template| template_path_segments(template, year, month))
        .unwrap_or_default()
}

fn url_structure_from_addition(addition: &str) -> String {
    serde_json::from_str::<Value>(addition)
        .ok()
        .and_then(|value| {
            value
                .get("url_structure")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_default()
}

fn addition_with_structure(addition: &str, url_structure: &str) -> Result<String> {
    let mut value = serde_json::from_str::<Value>(addition).unwrap_or_else(|_| json!({}));
    if !value.is_object() {
        value = json!({});
    }
    value["url_structure"] = Value::String(url_structure.to_string());
    Ok(serde_json::to_string(&value)?)
}

fn default_url_tree_storage_req(mount_path: &str, addition: String) -> StorageReq {
    StorageReq {
        id: None,
        mount_path: mount_path.to_string(),
        order: Some(0),
        driver: "UrlTree".to_string(),
        remark: Some(String::new()),
        cache_expiration: Some(30),
        status: None,
        web_proxy: false,
        webdav_policy: Some("native_proxy".to_string()),
        down_proxy_url: Some(String::new()),
        order_by: "name".to_string(),
        extract_folder: "front".to_string(),
        order_direction: "asc".to_string(),
        addition,
        enable_sign: false,
    }
}

fn storage_req_from_storage(storage: &Storage, addition: String) -> StorageReq {
    StorageReq {
        id: Some(storage.id.to_string()),
        mount_path: storage.mount_path.clone(),
        order: Some(storage.order),
        driver: storage.driver.clone(),
        remark: Some(storage.remark.clone()),
        cache_expiration: Some(storage.cache_expiration),
        status: Some(storage.status.clone()),
        web_proxy: storage.web_proxy,
        webdav_policy: Some(storage.webdav_policy.clone()),
        down_proxy_url: Some(storage.down_proxy_url.clone()),
        order_by: storage.order_by.clone(),
        extract_folder: storage.extract_folder.clone(),
        order_direction: storage.order_direction.clone(),
        addition,
        enable_sign: storage.enable_sign,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage() -> Storage {
        Storage {
            id: 7,
            mount_path: "/Anime".to_string(),
            order: 2,
            driver: "UrlTree".to_string(),
            cache_expiration: 60,
            status: "work".to_string(),
            addition: r#"{"url_structure":"old","head_size":false,"writable":false}"#.to_string(),
            remark: "remark".to_string(),
            modified: String::new(),
            disabled: false,
            enable_sign: true,
            order_by: "name".to_string(),
            order_direction: "asc".to_string(),
            extract_folder: "front".to_string(),
            web_proxy: false,
            webdav_policy: "native_proxy".to_string(),
            down_proxy_url: String::new(),
        }
    }

    #[test]
    fn updates_only_url_structure_in_addition() {
        let addition = addition_with_structure(
            r#"{"url_structure":"old","head_size":true,"writable":false}"#,
            "new",
        )
        .unwrap();
        let value: Value = serde_json::from_str(&addition).unwrap();

        assert_eq!(value["url_structure"], "new");
        assert_eq!(value["head_size"], true);
        assert_eq!(value["writable"], false);
    }

    #[test]
    fn builds_storage_update_request_from_existing_storage() {
        let req = storage_req_from_storage(&storage(), r#"{"url_structure":"new"}"#.to_string());

        assert_eq!(req.id.as_deref(), Some("7"));
        assert_eq!(req.mount_path, "/Anime");
        assert_eq!(req.driver, "UrlTree");
        assert!(req.enable_sign);
        assert_eq!(req.addition, r#"{"url_structure":"new"}"#);
    }

    #[test]
    fn creates_default_url_tree_storage_request() {
        let req = default_url_tree_storage_req("/Anime", "{}".to_string());

        assert_eq!(req.mount_path, "/Anime");
        assert_eq!(req.driver, "UrlTree");
        assert_eq!(req.order_by, "name");
        assert!(!req.web_proxy);
    }

    #[test]
    fn builds_latest_path_prefix_from_template() {
        assert!(latest_path_prefix(None, 2026, 4).is_empty());
        assert!(latest_path_prefix(Some("   "), 2026, 4).is_empty());
        assert_eq!(
            latest_path_prefix(Some("{{ year }}年/{{ month }}月"), 2026, 4),
            ["2026年", "4月"]
        );
    }
}
