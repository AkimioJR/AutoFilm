use crate::alist::{AlistConfig, build_client};
use alist::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, warn};

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
