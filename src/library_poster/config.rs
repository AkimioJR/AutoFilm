mod resolution;

use std::path::PathBuf;

pub use resolution::Resolution;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Card,
    Split,
    #[default]
    Collage,
    Blur,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    Random,
    DateCreated,
    #[default]
    DateLastContentAdded,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderConfig {
    #[serde(default)]
    pub style: Style,
    #[serde(default)]
    pub resolution: Resolution,
    #[serde(default = "default_blur_radius")]
    pub blur_radius: f32,
    #[serde(default = "default_color_strength")]
    pub color_strength: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            style: Style::default(),
            resolution: Resolution::default(),
            blur_radius: default_blur_radius(),
            color_strength: default_color_strength(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryConfig {
    pub name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub sort: Sort,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    // 任务 ID 用于调度和日志识别。
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub cron: Option<String>,
    pub server: String,
    #[serde(default)]
    pub upload: bool,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    pub title_font: PathBuf,
    pub subtitle_font: PathBuf,
    #[serde(default)]
    pub render: RenderConfig,
    #[serde(default)]
    pub libraries: Vec<LibraryConfig>,
}

fn default_blur_radius() -> f32 {
    50.0
}

fn default_color_strength() -> f32 {
    0.8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_poster_task() {
        let config: Config = serde_yaml::from_str(
            r#"
id: 媒体库海报
server: 我的Jellyfin
upload: false
output_dir: /media/posters
title_font: /fonts/ch.ttf
subtitle_font: /fonts/en.otf
render:
  style: collage
  resolution: 1080p
libraries:
  - name: 电影
    title: 电影
    subtitle: MOVIE
    sort: random
"#,
        )
        .unwrap();

        assert_eq!(config.render.style, Style::Collage);
        assert_eq!(config.libraries[0].sort, Sort::Random);
    }
}
