use chrono::{DateTime, Datelike, Local, NaiveDateTime};
use regex::Regex;
use serde::Deserialize;

use super::url_tree::FileEntry;

const VIDEO_MIME_TYPES: &[&str] = &["video/mp4", "video/x-matroska"];
const SUBTITLE_MIME_TYPES: &[&str] = &["application/octet-stream"];
const ZIP_MIME_TYPES: &[&str] = &["application/zip"];
const ANI_SEASON_MONTHS: &[u32] = &[1, 4, 7, 10];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFile {
    pub path: Vec<String>,
    pub file: FileEntry,
}

#[derive(Debug, Deserialize)]
pub struct AniDirectoryResp {
    pub files: Vec<AniObject>,
}

#[derive(Debug, Deserialize)]
pub struct AniObject {
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default, rename = "createdTime")]
    pub created_time: Option<String>,
}

pub fn current_season(now: DateTime<Local>) -> (i32, u32) {
    let month = ANI_SEASON_MONTHS
        .iter()
        .rev()
        .find(|month| **month <= now.month())
        .copied()
        .unwrap_or(1);
    (now.year(), month)
}

pub fn default_season_key(now: DateTime<Local>) -> String {
    let (year, month) = current_season(now);
    format!("{year}-{month}")
}

pub fn season_key(year: Option<i32>, month: Option<u32>, now: DateTime<Local>) -> String {
    match (year, month) {
        (Some(year), Some(month)) => format!("{year}-{month}"),
        _ => default_season_key(now),
    }
}

pub fn season_key_from_parts(year: i32, month: u32) -> String {
    format!("{year}-{month}")
}

pub fn render_template(template: &str, year: i32, month: u32) -> String {
    template
        .replace("{{ year }}", &year.to_string())
        .replace("{{ month }}", &month.to_string())
}

pub fn template_path_segments(template: &str, year: i32, month: u32) -> Vec<String> {
    render_template(template, year, month)
        .split('/')
        .filter_map(|segment| {
            let segment = segment.trim();
            (!segment.is_empty()).then(|| segment.to_string())
        })
        .collect()
}

pub fn join_url(base_url: &str, segments: &[&str]) -> String {
    let mut url = base_url.trim_end_matches('/').to_string();
    for segment in segments {
        url.push('/');
        url.push_str(&urlencoding::encode(segment));
    }
    if !url.ends_with('/') {
        url.push('/');
    }
    url
}

pub fn file_url(parent_url: &str, file_name: &str) -> String {
    format!(
        "{}{}?d=true",
        parent_url.trim_end_matches('/').to_string() + "/",
        urlencoding::encode(file_name)
    )
}

pub fn parse_ani_timestamp(value: &str) -> Option<i64> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.fZ")
        .ok()
        .map(|date_time| date_time.and_utc().timestamp())
}

pub fn parse_rss_timestamp(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|date_time| date_time.timestamp())
}

pub fn parse_size_to_bytes(value: &str) -> Option<u64> {
    let mut parts = value.split_whitespace();
    let number = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next()?.to_ascii_uppercase();
    let multiplier = match unit.as_str() {
        "B" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024.0_f64.powi(2),
        "GB" => 1024.0_f64.powi(3),
        "TB" => 1024.0_f64.powi(4),
        _ => return None,
    };
    Some((number * multiplier) as u64)
}

pub fn is_supported_file_mime(mime_type: &str) -> bool {
    VIDEO_MIME_TYPES.contains(&mime_type)
        || SUBTITLE_MIME_TYPES.contains(&mime_type)
        || ZIP_MIME_TYPES.contains(&mime_type)
}

pub fn is_directory_mime(mime_type: &str) -> bool {
    mime_type == "application/vnd.google-apps.folder"
}

pub fn rss_items(xml: &str) -> Vec<RemoteFile> {
    let item_regex = Regex::new(r"(?s)<item\b[^>]*>(.*?)</item>").expect("valid item regex");
    item_regex
        .captures_iter(xml)
        .filter_map(|capture| rss_item(capture.get(1)?.as_str()))
        .collect()
}

fn rss_item(xml: &str) -> Option<RemoteFile> {
    let title = xml_tag(xml, "title")?;
    let link = xml_tag(xml, "link")?;
    let published = xml_tag(xml, "pubDate")?;
    let size = xml_tag(xml, "anime:size")
        .or_else(|| xml_tag(xml, "anime_size"))
        .and_then(|value| parse_size_to_bytes(&value))
        .unwrap_or_default();
    let modified = parse_rss_timestamp(&published).unwrap_or_default();
    let path = rss_parent_path(&link);

    Some(RemoteFile {
        path,
        file: FileEntry {
            name: title,
            size,
            modified,
            url: link,
        },
    })
}

fn rss_parent_path(link: &str) -> Vec<String> {
    let without_query = link.split('?').next().unwrap_or(link);
    without_query
        .split('/')
        .skip(3)
        .filter_map(|segment| {
            let decoded = urlencoding::decode(segment).ok()?;
            (!decoded.is_empty()).then(|| decoded.to_string())
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn xml_tag(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"(?s)<{tag}\b[^>]*>(.*?)</{tag}>");
    let regex = Regex::new(&pattern).ok()?;
    regex
        .captures(xml)?
        .get(1)
        .map(|value| xml_unescape(value.as_str().trim()))
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn builds_season_keys() {
        let now = Local.with_ymd_and_hms(2026, 6, 8, 0, 0, 0).unwrap();
        assert_eq!(current_season(now), (2026, 4));
        assert_eq!(default_season_key(now), "2026-4");
        assert_eq!(season_key(Some(2025), Some(10), now), "2025-10");
        assert_eq!(season_key_from_parts(2026, 4), "2026-4");
    }

    #[test]
    fn renders_latest_templates() {
        assert_eq!(render_template("{{ year }}-{{ month }}", 2026, 4), "2026-4");
        assert_eq!(
            template_path_segments("{{ year }}年/{{ month }}月", 2026, 4),
            ["2026年", "4月"]
        );
        assert_eq!(
            template_path_segments(" / {{ year }}年 // {{ month }}月 / ", 2026, 4),
            ["2026年", "4月"]
        );
        assert!(template_path_segments("   ", 2026, 4).is_empty());
    }

    #[test]
    fn joins_urls_with_encoded_segments() {
        assert_eq!(
            join_url("https://example.com/root/", &["2026-4", "动画"]),
            "https://example.com/root/2026-4/%E5%8A%A8%E7%94%BB/"
        );
        assert_eq!(
            file_url("https://example.com/2026-4/", "动画 01.mp4"),
            "https://example.com/2026-4/%E5%8A%A8%E7%94%BB%2001.mp4?d=true"
        );
    }

    #[test]
    fn parses_dates_and_sizes() {
        assert_eq!(
            parse_ani_timestamp("2024-11-10T09:01:47.000Z"),
            Some(1731229307)
        );
        assert_eq!(
            parse_rss_timestamp("Sun, 10 Nov 2024 09:01:47 GMT"),
            Some(1731229307)
        );
        assert_eq!(parse_size_to_bytes("473.0 MB"), Some(495_976_448));
    }

    #[test]
    fn parses_rss_items() {
        let items = rss_items(
            r#"
<rss><channel><item>
  <title>动画 01.mp4</title>
  <link>https://resources.ani.rip/2026-4/%E5%8A%A8%E7%94%BB%2001.mp4?d=true</link>
  <pubDate>Sun, 10 Nov 2024 09:01:47 GMT</pubDate>
  <anime:size>473.0 MB</anime:size>
</item></channel></rss>
"#,
        );

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, ["2026-4"]);
        assert_eq!(items[0].file.name, "动画 01.mp4");
        assert_eq!(items[0].file.size, 495_976_448);
    }
}
