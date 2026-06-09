use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    // 任务 ID 用于日志输出和调度识别。
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub cron: Option<String>,
    pub alist: String,
    pub target_dir: String,
    #[serde(default)]
    pub source: SourceConfig,
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SourceConfig {
    #[serde(default = "default_source_url")]
    pub source_url: String,
    #[serde(default = "default_rss_url")]
    pub rss_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum UpdateConfig {
    Rss,
    Latest {
        #[serde(default)]
        template: Option<String>,
    },
    Keyword {
        keyword: String,
    },
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            source_url: default_source_url(),
            rss_url: default_rss_url(),
        }
    }
}

fn default_source_url() -> String {
    "https://aniopen.an-i.workers.dev".to_string()
}

fn default_rss_url() -> String {
    "https://api.ani.rip/ani-download.xml".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rss_update_config() {
        let config: Config = serde_yaml::from_str(
            r#"
id: 新番追更
cron: 0 20 12 * * *
alist: 我的Alist
target_dir: /Anime
source:
  source_url: https://aniopen.an-i.workers.dev
  rss_url: https://api.ani.rip/ani-download.xml
update:
  mode: rss
"#,
        )
        .unwrap();

        assert_eq!(config.alist, "我的Alist");
        assert_eq!(config.source.source_url, "https://aniopen.an-i.workers.dev");
        assert!(matches!(config.update, UpdateConfig::Rss));
    }

    #[test]
    fn parses_season_and_keyword_update_configs() {
        let latest: Config = serde_yaml::from_str(
            r#"
id: 当前季度
alist: 我的Alist
target_dir: /Anime
update:
  mode: latest
  template: "{{ year }}年/{{ month }}月"
"#,
        )
        .unwrap();
        assert!(matches!(
            latest.update,
            UpdateConfig::Latest { ref template }
                if template.as_deref() == Some("{{ year }}年/{{ month }}月")
        ));

        let latest_without_template: Config = serde_yaml::from_str(
            r#"
id: 当前季度直写
alist: 我的Alist
target_dir: /Anime
update:
  mode: latest
"#,
        )
        .unwrap();
        assert!(matches!(
            latest_without_template.update,
            UpdateConfig::Latest { template: None }
        ));

        let keyword: Config = serde_yaml::from_str(
            r#"
id: 自定义关键字
alist: 我的Alist
target_dir: /Anime
update:
  mode: keyword
  keyword: "2026-4"
"#,
        )
        .unwrap();
        assert!(matches!(
            keyword.update,
            UpdateConfig::Keyword { ref keyword } if keyword == "2026-4"
        ));
    }
}
