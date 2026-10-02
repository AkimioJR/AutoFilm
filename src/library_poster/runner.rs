use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use ab_glyph::FontArc;
use base64::Engine;
use image::{DynamicImage, ImageFormat};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use super::{
    Config, LibraryConfig, Sort, Style,
    renderer::{self, Fonts},
};
use crate::media_server::{Client, ImageKind, Library, MediaItem};

#[derive(Debug, Default)]
pub struct Summary {
    pub task_id: String,
    pub library_count: usize,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub downloaded_image_count: usize,
    pub generated_count: usize,
    pub saved_count: usize,
    pub uploaded_count: usize,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("读取字体文件失败: {path}")]
    ReadFont {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("字体文件无效: {0}")]
    InvalidFont(PathBuf),

    #[error("任务必须至少启用 output_dir 或 upload")]
    MissingOutput,

    #[error(transparent)]
    MediaServer(#[from] crate::media_server::Error),

    #[error(transparent)]
    Render(#[from] renderer::Error),

    #[error("编码 PNG 失败: {0}")]
    EncodeImage(image::ImageError),

    #[error("创建海报输出目录失败: {path}")]
    CreateOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("写入海报文件失败: {path}")]
    WriteOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct LibraryPoster {
    config: Config,
    client: Arc<Client>,
    fonts: Fonts,
}

impl LibraryPoster {
    /// 初始化任务并提前验证字体，避免定时任务触发后才发现静态资源无效。
    pub fn new(config: Config, client: Arc<Client>) -> Result<Self> {
        if !config.upload && config.output_dir.is_none() {
            return Err(Error::MissingOutput);
        }

        let fonts = Fonts {
            title: load_font(&config.title_font)?,
            subtitle: load_font(&config.subtitle_font)?,
        };
        Ok(Self {
            config,
            client,
            fonts,
        })
    }

    /// 依次处理任务中明确配置的媒体库，并隔离单个媒体库的失败。
    pub async fn run(&self) -> Result<Summary> {
        let user_id = self.client.resolve_user_id().await?;
        let libraries = self
            .client
            .libraries()
            .await?
            .into_iter()
            .map(|library| (library.name.clone(), library))
            .collect::<HashMap<_, _>>();
        let mut summary = Summary {
            task_id: self.config.id.clone(),
            library_count: self.config.libraries.len(),
            ..Summary::default()
        };

        for library_config in &self.config.libraries {
            let Some(library) = libraries.get(&library_config.name) else {
                warn!(
                    task_id = %self.config.id,
                    library = %library_config.name,
                    "媒体库不存在，已跳过"
                );
                summary.failed_count += 1;
                continue;
            };

            match self
                .process_library(&user_id, library, library_config)
                .await
            {
                Ok(result) => {
                    summary.succeeded_count += 1;
                    summary.downloaded_image_count += result.downloaded_images;
                    summary.generated_count += 1;
                    summary.saved_count += usize::from(result.saved);
                    summary.uploaded_count += usize::from(result.uploaded);
                }
                Err(err) => {
                    summary.failed_count += 1;
                    error!(
                        task_id = %self.config.id,
                        library = %library.name,
                        error = %err,
                        "生成媒体库海报失败"
                    );
                }
            }
        }

        Ok(summary)
    }

    async fn process_library(
        &self,
        user_id: &str,
        library: &Library,
        library_config: &LibraryConfig,
    ) -> Result<LibraryResult> {
        let render_config = &self.config.render;
        let sort_by = sort_name(library_config.sort);
        let random_seed = matches!(library_config.sort, Sort::Random).then(rand::random);
        debug!(
            task_id = %self.config.id,
            library = %library.name,
            collection_type = ?library.collection_type,
            style = ?render_config.style,
            resolution = ?render_config.resolution,
            sort = ?library_config.sort,
            "开始筛选媒体库素材"
        );
        let items = self
            .client
            .items(user_id, &library.id, sort_by, random_seed, 50)
            .await?;
        let references = select_image_references(&items, render_config.style);
        if references.is_empty() {
            return Err(renderer::Error::MissingImage.into());
        }

        let required = match render_config.style {
            Style::Collage => 9,
            Style::Card => 7,
            Style::Split | Style::Blur => 1,
        };
        let mut images = Vec::new();
        for reference in references {
            if images.len() >= required {
                break;
            }
            match self
                .client
                .download_image(&reference.item_id, reference.kind, reference.tag.as_deref())
                .await
            {
                Ok(bytes) => match image::load_from_memory(&bytes) {
                    Ok(image) => images.push(image),
                    Err(err) => warn!(
                        task_id = %self.config.id,
                        library = %library.name,
                        item_id = %reference.item_id,
                        error = %err,
                        "媒体图片解码失败，尝试下一张素材"
                    ),
                },
                Err(err) => warn!(
                    task_id = %self.config.id,
                    library = %library.name,
                    item_id = %reference.item_id,
                    error = %err,
                    "媒体图片下载失败，尝试下一张素材"
                ),
            }
        }
        if images.is_empty() {
            return Err(renderer::Error::MissingImage.into());
        }

        let title = if library_config.title.trim().is_empty() {
            library.name.as_str()
        } else {
            library_config.title.as_str()
        };
        let poster = renderer::render(
            &images,
            title,
            &library_config.subtitle,
            &self.fonts,
            render_config,
        )?;
        let png = encode_png(poster)?;

        let saved = if let Some(output_dir) = &self.config.output_dir {
            let output_path = output_path(output_dir, &self.config.id, &library.name);
            save_output(&output_path, &png).await?;
            info!(
                task_id = %self.config.id,
                library = %library.name,
                output = %output_path.display(),
                "媒体库海报已保存"
            );
            true
        } else {
            false
        };

        let uploaded = if self.config.upload {
            let encoded = base64::engine::general_purpose::STANDARD.encode(&png);
            self.client
                .upload_primary_image(&library.id, &encoded)
                .await?;
            info!(
                task_id = %self.config.id,
                library = %library.name,
                "媒体库海报已上传"
            );
            true
        } else {
            false
        };

        Ok(LibraryResult {
            downloaded_images: images.len(),
            saved,
            uploaded,
        })
    }
}

struct LibraryResult {
    downloaded_images: usize,
    saved: bool,
    uploaded: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ImageReference {
    item_id: String,
    kind: ImageKind,
    tag: Option<String>,
}

fn load_font(path: &Path) -> Result<FontArc> {
    let data = std::fs::read(path).map_err(|source| Error::ReadFont {
        path: path.to_path_buf(),
        source,
    })?;
    FontArc::try_from_vec(data).map_err(|_| Error::InvalidFont(path.to_path_buf()))
}

fn sort_name(sort: Sort) -> &'static str {
    match sort {
        Sort::Random => "Random",
        Sort::DateCreated => "DateCreated",
        Sort::DateLastContentAdded => "DateLastContentAdded",
    }
}

/// 根据渲染风格选择素材类型，并对同一剧集或同一图片做去重。
fn select_image_references(items: &[MediaItem], style: Style) -> Vec<ImageReference> {
    let mut references = Vec::new();
    let mut seen_content = HashSet::new();
    let mut seen_images = HashSet::new();

    for item in items {
        let content_key = if item.item_type == "Episode" {
            item.series_id
                .as_deref()
                .map(|id| format!("series:{id}"))
                .unwrap_or_else(|| format!("item:{}", item.id))
        } else {
            format!("item:{}", item.id)
        };
        if seen_content.contains(&content_key) {
            continue;
        }

        let reference = if matches!(style, Style::Card | Style::Collage) {
            primary_reference(item).or_else(|| backdrop_reference(item))
        } else {
            backdrop_reference(item).or_else(|| primary_reference(item))
        };
        let Some(reference) = reference else {
            continue;
        };
        let image_key = format!(
            "{}:{:?}:{}",
            reference.item_id,
            reference.kind,
            reference.tag.as_deref().unwrap_or_default()
        );
        if seen_images.insert(image_key) {
            seen_content.insert(content_key);
            references.push(reference);
        }
    }

    references
}

fn primary_reference(item: &MediaItem) -> Option<ImageReference> {
    if item.item_type == "Episode"
        && let (Some(series_id), Some(tag)) = (
            item.series_id.as_deref(),
            item.series_primary_image_tag.as_deref(),
        )
    {
        return Some(ImageReference {
            item_id: series_id.to_string(),
            kind: ImageKind::Primary,
            tag: Some(tag.to_string()),
        });
    }

    item.image_tags.get("Primary").map(|tag| ImageReference {
        item_id: item.id.clone(),
        kind: ImageKind::Primary,
        tag: Some(tag.clone()),
    })
}

fn backdrop_reference(item: &MediaItem) -> Option<ImageReference> {
    if let (Some(parent_id), Some(tag)) = (
        item.parent_backdrop_item_id.as_deref(),
        item.parent_backdrop_image_tags.first(),
    ) {
        return Some(ImageReference {
            item_id: parent_id.to_string(),
            kind: ImageKind::Backdrop,
            tag: Some(tag.clone()),
        });
    }

    item.backdrop_image_tags.first().map(|tag| ImageReference {
        item_id: item.id.clone(),
        kind: ImageKind::Backdrop,
        tag: Some(tag.clone()),
    })
}

fn encode_png(image: image::RgbaImage) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut output, ImageFormat::Png)
        .map_err(Error::EncodeImage)?;
    Ok(output.into_inner())
}

fn output_path(output_dir: &Path, task_id: &str, library_name: &str) -> PathBuf {
    output_dir
        .join(sanitize_filename(task_id))
        .join(format!("{}.png", sanitize_filename(library_name)))
}

fn sanitize_filename(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            character => character,
        })
        .collect::<String>();
    let sanitized = sanitized.trim().trim_matches('.').trim();
    if sanitized.is_empty() {
        "未命名".to_string()
    } else {
        sanitized.to_string()
    }
}

async fn save_output(path: &Path, png: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|source| Error::CreateOutput {
            path: parent.to_path_buf(),
            source,
        })?;
    tokio::fs::write(path, png)
        .await
        .map_err(|source| Error::WriteOutput {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use image::RgbaImage;

    use super::*;
    fn item(
        id: &str,
        item_type: &str,
        series_id: Option<&str>,
        primary: Option<&str>,
        backdrop: Option<&str>,
    ) -> MediaItem {
        MediaItem {
            id: id.to_string(),
            item_type: item_type.to_string(),
            series_id: series_id.map(str::to_string),
            series_primary_image_tag: primary.map(str::to_string),
            parent_backdrop_item_id: series_id.map(str::to_string),
            parent_backdrop_image_tags: backdrop.into_iter().map(str::to_string).collect(),
            backdrop_image_tags: backdrop.into_iter().map(str::to_string).collect(),
            image_tags: primary
                .map(|tag| HashMap::from([("Primary".to_string(), tag.to_string())]))
                .unwrap_or_default(),
        }
    }

    #[test]
    fn selects_images_by_style_and_deduplicates_series() {
        let items = vec![
            item(
                "episode-1",
                "Episode",
                Some("series"),
                Some("primary"),
                Some("backdrop"),
            ),
            item(
                "episode-2",
                "Episode",
                Some("series"),
                Some("primary"),
                Some("backdrop"),
            ),
        ];

        let collage = select_image_references(&items, Style::Collage);
        let card = select_image_references(&items, Style::Card);

        assert_eq!(collage.len(), 1);
        assert_eq!(collage[0].kind, ImageKind::Primary);
        assert_eq!(card.len(), 1);
        assert_eq!(card[0].kind, ImageKind::Primary);
    }

    #[test]
    fn encodes_valid_png_and_sanitizes_output_path() {
        let png = encode_png(RgbaImage::new(32, 18)).unwrap();
        let decoded = image::load_from_memory_with_format(&png, ImageFormat::Png).unwrap();
        let path = output_path(Path::new("/tmp/posters"), "任务/一", "电影:精选");

        assert_eq!(decoded.width(), 32);
        assert_eq!(
            path,
            Path::new("/tmp/posters")
                .join("任务_一")
                .join("电影_精选.png")
        );
    }
}
