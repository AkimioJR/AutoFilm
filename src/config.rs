use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct AlistConfig {
    base_url: Url,
    username: Option<String>,
    password: Option<String>,
    otp_code: Option<String>,
    token: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
struct DownloadOption {
    subtitle: Bool,
    image: Bool,
    nfo: Bool,
    other_ext: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
enum Alist2StrmMode {
    AlistURL,
    RawURL,
    AlistPath,
}

#[derive(Debug, Deserialize, Serialize)]
struct Alist2StrmSartProtection {
    enabled: bool,
    threshold: u16,
    grace_scans: u16,
}

#[derive(Debug, Deserialize, Serialize)]
struct Alist2StrmConfig {
    id: String,
    cron: String,
    alist: AlistConfig,
    source_dir: String,
    target_dir: String,
    mode: Alist2StrmMode,
    flatten_mode: Bool,
    smart_protection: Alist2StrmSartProtection,
    max_workers: u16,
    max_downloaders: u16,
    wait_time: u16,
}
