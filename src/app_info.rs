pub const APP_NAME: &str = "AutoFilm";
pub const APP_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

pub const LOGO: &str = r#"
 █████╗ ██╗   ██╗████████╗ ██████╗ ███████╗██╗██╗     ███╗   ███╗    
██╔══██╗██║   ██║╚══██╔══╝██╔═══██╗██╔════╝██║██║     ████╗ ████║    
███████║██║   ██║   ██║   ██║   ██║█████╗  ██║██║     ██╔████╔██║    
██╔══██║██║   ██║   ██║   ██║   ██║██╔══╝  ██║██║     ██║╚██╔╝██║    
██║  ██║╚██████╔╝   ██║   ╚██████╔╝██║     ██║███████╗██║ ╚═╝ ██║    
╚═╝  ╚═╝ ╚═════╝    ╚═╝    ╚═════╝ ╚═╝     ╚═╝╚══════╝╚═╝     ╚═╝
"#;

pub fn print_banner() {
    // 启动横幅保持 Python 版本的风格，版本号直接来自 Cargo.toml。
    println!("{LOGO}");
    let title = format!(" {APP_NAME} {APP_VERSION} ");
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
    use super::{APP_VERSION, Center};

    #[test]
    fn version_comes_from_cargo_package() {
        assert_eq!(APP_VERSION, concat!("v", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn centers_banner_title() {
        assert_eq!(" AutoFilm ".center(14, "="), "== AutoFilm ==");
    }
}
