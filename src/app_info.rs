use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub app_name: &'static str,
    pub app_version: &'static str,
    pub rustc_version: &'static str,
    pub git_commit: &'static str,
    pub git_branch: &'static str,
    pub build_time: &'static str,
    pub build_target: &'static str,
    pub build_profile: &'static str,
}

pub static VERSION_INFO: VersionInfo = VersionInfo {
    app_name: "AutoFilm",
    app_version: concat!("v", env!("CARGO_PKG_VERSION")),
    rustc_version: env!("AUTOFILM_RUSTC_VERSION"),
    git_commit: env!("AUTOFILM_GIT_COMMIT"),
    git_branch: env!("AUTOFILM_GIT_BRANCH"),
    build_time: env!("AUTOFILM_BUILD_TIME"),
    build_target: env!("AUTOFILM_BUILD_TARGET"),
    build_profile: env!("AUTOFILM_BUILD_PROFILE"),
};

pub const LOGO: &str = concat!(
    " █████╗ ██╗   ██╗████████╗ ██████╗ ███████╗██╗██╗     ███╗   ███╗\n",
    "██╔══██╗██║   ██║╚══██╔══╝██╔═══██╗██╔════╝██║██║     ████╗ ████║\n",
    "███████║██║   ██║   ██║   ██║   ██║█████╗  ██║██║     ██╔████╔██║\n",
    "██╔══██║██║   ██║   ██║   ██║   ██║██╔══╝  ██║██║     ██║╚██╔╝██║\n",
    "██║  ██║╚██████╔╝   ██║   ╚██████╔╝██║     ██║███████╗██║ ╚═╝ ██║\n",
    "╚═╝  ╚═╝ ╚═════╝    ╚═╝    ╚═════╝ ╚═╝     ╚═╝╚══════╝╚═╝     ╚═╝",
);

pub fn print_banner() {
    // 启动横幅保持 Python 版本的风格，版本号直接来自 Cargo.toml。
    println!("{LOGO}");
    let title = format!(" {} {} ", VERSION_INFO.app_name, VERSION_INFO.app_version);
    println!("{}", title.center(65, "="));
    println!();
}

trait Center {
    fn center(&self, width: usize, fill: &str) -> String;
}

impl Center for str {
    fn center(&self, width: usize, fill: &str) -> String {
        let content_width = self.chars().count();
        if content_width >= width {
            return self.to_string();
        }

        let padding = width - content_width;
        let left = padding / 2;
        let right = padding - left;
        format!("{}{}{}", fill.repeat(left), self, fill.repeat(right))
    }
}

#[cfg(test)]
mod tests {
    use super::Center;

    #[test]
    fn logo_has_no_outer_newlines() {
        assert!(!super::LOGO.starts_with('\n'));
        assert!(!super::LOGO.ends_with('\n'));
    }

    #[test]
    fn centers_banner_title() {
        assert_eq!(" AutoFilm ".center(14, "="), "== AutoFilm ==");
    }
}
