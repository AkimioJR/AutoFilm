use std::time::Duration;

use alist::{Authentication, Client};
use serde::{Deserialize, Serialize};

use crate::alist2strm::{Error, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlistConfig {
    // AList 连接 ID，用于任务配置引用并复用客户端对象。
    pub id: String,
    // AList 服务地址，允许不写协议；运行时会默认补 https://。
    pub base_url: String,
    #[serde(default)]
    pub public_url: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub otp_code: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub wait_time: f64,
}

/// 根据配置构建 AList API 客户端。
///
/// 优先使用永久 token；未配置 token 时使用用户名、密码和可选 OTP 登录。
/// 同时会应用请求间隔配置，用于降低对 AList 或上游存储的请求压力。
pub(crate) fn build_client(config: &AlistConfig) -> Result<Client> {
    let request_interval =
        (config.wait_time > 0.0).then(|| Duration::from_secs_f64(config.wait_time));
    if let Some(token) = config
        .token
        .as_deref()
        .filter(|token| !token.trim().is_empty())
    {
        return Ok(Client::with_token(&config.base_url, token.to_string())?
            .with_api_request_interval(request_interval));
    }

    let username = config.username.as_deref().filter(|value| !value.is_empty());
    let password = config.password.as_deref().filter(|value| !value.is_empty());
    match (username, password) {
        (Some(username), Some(password)) => Ok(Client::with_authentication(
            &config.base_url,
            Authentication::username_password(
                username.to_string(),
                password.to_string(),
                config.otp_code.clone(),
            ),
        )?
        .with_api_request_interval(request_interval)),
        _ => Err(Error::MissingAuthentication),
    }
}
