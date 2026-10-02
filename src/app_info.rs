use std::sync::LazyLock;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApplicationInfo {
    pub app_name: String,
    pub app_version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub repository: String,
    pub rustc_version: String,
    pub git_commit: String,
    pub git_branch: String,
    pub build_time: String,
    pub build_target: String,
    pub build_profile: String,
}

pub static APPLICATION_INFO: LazyLock<ApplicationInfo> = LazyLock::new(|| ApplicationInfo {
    app_name: "AutoFilm".into(),
    app_version: format!("v{}", env!("CARGO_PKG_VERSION")),
    description: env!("CARGO_PKG_DESCRIPTION").into(),
    authors: env!("CARGO_PKG_AUTHORS")
        .split(':')
        .map(|s| s.trim().into())
        .collect(),
    repository: env!("CARGO_PKG_REPOSITORY").into(),
    rustc_version: env!("AUTOFILM_RUSTC_VERSION").into(),
    git_commit: env!("AUTOFILM_GIT_COMMIT").into(),
    git_branch: env!("AUTOFILM_GIT_BRANCH").into(),
    build_time: env!("AUTOFILM_BUILD_TIME").into(),
    build_target: env!("AUTOFILM_BUILD_TARGET").into(),
    build_profile: env!("AUTOFILM_BUILD_PROFILE").into(),
});

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
    let title = format!(
        " {} {} ",
        APPLICATION_INFO.app_name, APPLICATION_INFO.app_version
    );
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
