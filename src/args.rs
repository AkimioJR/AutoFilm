use crate::app_info;
use chrono_tz::Tz;
use clap::Parser;

use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = app_info::VERSION_INFO.app_name,
    disable_version_flag = true
)]
pub struct CliArgs {
    /// 是否启用调试日志
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// 指定配置文件路径
    #[arg(
        long = "config",
        value_name = "PATH",
        default_value = "config/config.yaml"
    )]
    pub config_path: PathBuf,

    /// 指定日志文件目录路径（为空表示禁用日志文件写入）
    #[arg(long = "log", value_name = "PATH", default_value = "logs")]
    pub log_path: String,

    /// 指定时区（默认为系统本地时区，或 UTC 作为后备）
    /// 支持解析 IANA 时区字符串，如 "Asia/Shanghai"、"America/New_York"、 "UTC" 等
    #[arg(long, value_name = "TZ")]
    pub timezone: Option<Tz>,

    /// 是否启用彩色日志输出
    /// `true`（默认）表示启用彩色日志
    /// `false` 表示禁用彩色日志输出
    #[arg(long, default_value_t = true)]
    pub colorful_log: bool,

    /// 显示版本、Git 与编译信息
    #[arg(short = 'v', long = "version", default_value_t = false)]
    pub show_version: bool,
}

impl CliArgs {
    pub fn log_level(&self) -> tracing::Level {
        if self.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        }
    }

    pub fn app_timezone(&self) -> Tz {
        self.timezone
            .or_else(|| iana_time_zone::get_timezone().ok()?.parse().ok())
            .unwrap_or(chrono_tz::UTC)
    }
}

#[cfg(test)]
mod tests {
    use super::CliArgs;
    use clap::Parser;

    #[test]
    fn parses_debug_flag_and_config_path() {
        let args = CliArgs::parse_from(["autofilm", "--debug", "--config", "config/demo.yaml"]);
        assert!(args.debug);
        assert_eq!(
            args.config_path,
            std::path::PathBuf::from("config/demo.yaml")
        );
    }

    #[test]
    fn rejects_positional_config_path() {
        let result = CliArgs::try_parse_from(["autofilm", "config/demo.yaml"]);
        assert!(result.is_err());
    }

    #[test]
    fn defaults_config_path() {
        let args = CliArgs::parse_from(["autofilm"]);
        assert_eq!(
            args.config_path,
            std::path::PathBuf::from("config/config.yaml")
        );
    }

    #[test]
    fn defaults_log_path() {
        let args = CliArgs::parse_from(["autofilm"]);
        assert_eq!(args.log_path, std::path::PathBuf::from("logs"));
    }

    #[test]
    fn parses_custom_log_path() {
        let args = CliArgs::parse_from(["autofilm", "--log", "/tmp/autofilm-logs"]);
        assert_eq!(
            args.log_path,
            std::path::PathBuf::from("/tmp/autofilm-logs")
        );
    }

    #[test]
    fn defaults_timezone_to_none() {
        let args = CliArgs::parse_from(["autofilm"]);
        assert_eq!(args.timezone, None);
    }

    #[test]
    fn parses_cli_timezone_shanghai() {
        let args = CliArgs::parse_from(["autofilm", "--timezone", "Asia/Shanghai"]);
        assert_eq!(args.timezone, Some(chrono_tz::Asia::Shanghai));
    }

    #[test]
    fn parses_cli_timezone_new_york() {
        let args = CliArgs::parse_from(["autofilm", "--timezone", "America/New_York"]);
        assert_eq!(args.timezone, Some(chrono_tz::America::New_York));
    }

    #[test]
    fn parses_cli_timezone_utc() {
        let args = CliArgs::parse_from(["autofilm", "--timezone", "UTC"]);
        assert_eq!(args.timezone, Some(chrono_tz::UTC));
    }

    #[test]
    fn rejects_invalid_cli_timezone() {
        let result = CliArgs::try_parse_from(["autofilm", "--timezone", "Not/AZone"]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_version_flag() {
        let args = CliArgs::parse_from(["autofilm", "--version"]);
        assert!(args.show_version);
    }
}
