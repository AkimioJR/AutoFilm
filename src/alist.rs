use std::time::Duration;

use alist::{Authentication, Client};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::alist2strm::Result;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlistConfig {
    // AList 连接 ID，用于任务配置引用并复用客户端对象。
    pub id: String,
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
/// 优先使用永久 token；未配置 token 时使用用户名、密码和可选 OTP 登录；
/// 都未配置时创建无认证客户端。
/// 同时会应用请求间隔配置，用于降低对 AList 服务器的压力。
pub(crate) async fn build_client(config: &AlistConfig) -> Result<Client> {
    let request_interval =
        (config.wait_time > 0.0).then(|| Duration::from_secs_f64(config.wait_time));

    let client = Client::new(&config.base_url)?.with_api_request_interval(request_interval);

    if let Some(token) = config
        .token
        .as_deref()
        .filter(|token| !token.trim().is_empty())
    {
        return Ok(client.with_authentication(Authentication::Token(token.to_string())));
    }

    let username = config.username.as_deref().filter(|value| !value.is_empty());
    let password = config.password.as_deref().filter(|value| !value.is_empty());
    if let (Some(username), Some(password)) = (username, password) {
        return Ok(
            client.with_authentication(Authentication::UsernamePassword {
                username: username.to_string(),
                password: password.to_string(),
                otp_code: config.otp_code.clone(),
            }),
        );
    }

    match client.ping().await {
        Ok(true) => {
            debug!(id = %config.id, url = %config.base_url, "AList 服务器连通性检查成功");
        }
        Ok(false) => {
            warn!(id = %config.id, url = %config.base_url, "AList 服务器连通性检查未通过，返回了非预期响应");
        }
        Err(err) => {
            warn!(id = %config.id, url = %config.base_url, error = %err, "AList 服务器连通性检查失败");
        }
    }

    Ok(client)
}
