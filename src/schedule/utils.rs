use std::{collections::HashMap, sync::Arc};

use alist::Client;
use tracing::{debug, error, warn};

use crate::{
    alist::{AlistConfig, build_client},
    media_server,
};

pub async fn create_alist_clients(
    alist_configs: &Vec<AlistConfig>,
) -> HashMap<String, (Arc<Client>, String)> {
    let mut alist_clients = HashMap::new();

    for alist_config in alist_configs {
        if alist_clients.contains_key(&alist_config.id) {
            warn!(
                alist = %alist_config.id,
                "AList 客户端 ID 重复，已跳过后续重复配置"
            );
            continue;
        }

        let server_url = alist_config
            .public_url
            .clone()
            .unwrap_or_else(|| alist_config.base_url.clone());
        match build_client(&alist_config).await {
            Ok(client) => {
                debug!(
                    id = %alist_config.id,
                    base_url = %alist_config.base_url,
                    public_url = ?alist_config.public_url,
                    server_url = %server_url,
                    "成功创建 AList 客户端",
                );
                alist_clients.insert(alist_config.id.clone(), (Arc::new(client), server_url));
            }
            Err(err) => {
                error!(
                    id = %alist_config.id,
                    error = %err,
                    "创建 AList 客户端失败，引用该客户端的任务将被跳过"
                );
            }
        }
    }

    alist_clients
}

/// 创建可复用的媒体服务器客户端，调用方直接获得不可变映射。
pub fn create_media_server_clients(
    configs: &[media_server::Config],
) -> HashMap<String, Arc<media_server::Client>> {
    configs.iter().fold(HashMap::new(), |mut clients, config| {
        if clients.contains_key(&config.id) {
            warn!(
                server = %config.id,
                "媒体服务器 ID 重复，已跳过后续重复配置"
            );
            return clients;
        }

        match media_server::Client::new(config.clone()) {
            Ok(client) => {
                debug!(
                    id = %client.id(),
                    kind = ?client.kind(),
                    base_url = %config.base_url,
                    "成功创建媒体服务器客户端"
                );
                clients.insert(config.id.clone(), Arc::new(client));
            }
            Err(err) => {
                error!(
                    id = %config.id,
                    error = %err,
                    "创建媒体服务器客户端失败，引用该服务器的任务将被跳过"
                );
            }
        }
        clients
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media_server::Kind;

    #[test]
    fn creates_unique_media_server_clients() {
        let configs = vec![
            media_server::Config {
                id: "server".to_string(),
                kind: Kind::Jellyfin,
                base_url: "http://localhost:8096".to_string(),
                api_key: "secret".to_string(),
                user_id: None,
                timeout: 30,
            },
            media_server::Config {
                id: "server".to_string(),
                kind: Kind::Emby,
                base_url: "http://localhost:8097".to_string(),
                api_key: "secret".to_string(),
                user_id: None,
                timeout: 30,
            },
        ];

        let clients = create_media_server_clients(&configs);

        assert_eq!(clients.len(), 1);
        assert_eq!(clients["server"].kind(), Kind::Jellyfin);
    }
}
