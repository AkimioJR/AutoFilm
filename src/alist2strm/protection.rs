use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

use crate::alist2strm::SmartProtection;
use crate::alist2strm::runner::Result;

#[derive(Debug)]
pub struct ProtectionManager {
    target_dir: PathBuf,
    state_file: PathBuf,
    threshold: usize,
    grace_scans: usize,
    protected: HashMap<String, usize>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ProtectionState {
    updated: String,
    protected: HashMap<String, usize>,
}

impl ProtectionManager {
    /// 创建 .strm 保护管理器，并尝试读取上次扫描留下的计数状态。
    pub async fn new(target_dir: PathBuf, task_id: &str, config: &SmartProtection) -> Result<Self> {
        let state_file = target_dir.join(format!(".autofilm_strm_{task_id}.json"));
        let protected = load_state(&state_file).await.unwrap_or_default().protected;
        Ok(Self {
            target_dir,
            state_file,
            threshold: config.threshold,
            grace_scans: config.grace_scans,
            protected,
        })
    }

    pub async fn process(
        &mut self,
        strm_to_delete: HashSet<PathBuf>,
        strm_present: &HashSet<PathBuf>,
    ) -> Result<HashSet<PathBuf>> {
        // 如果文件重新出现在远端扫描结果中，说明不是误删候选，清除保护计数。
        self.protected.retain(|relative_path, _| {
            let absolute_path = self.target_dir.join(relative_path);
            !strm_present.contains(&absolute_path)
        });

        let ready = if strm_to_delete.len() < self.threshold {
            // 删除量低于阈值时认为是正常同步，立即删除。
            strm_to_delete
        } else {
            // 删除量超过阈值时进入宽限计数，连续多次确认后才真正删除。
            for file_path in strm_to_delete {
                let relative_path = self.to_relative(&file_path);
                *self.protected.entry(relative_path).or_insert(0) += 1;
            }

            let ready_relative_paths = self
                .protected
                .iter()
                .filter_map(|(path, count)| (*count >= self.grace_scans).then_some(path.clone()))
                .collect::<Vec<_>>();

            let ready = ready_relative_paths
                .iter()
                .map(|path| self.target_dir.join(path))
                .collect();

            for path in ready_relative_paths {
                self.protected.remove(&path);
            }
            ready
        };

        self.save().await?;
        Ok(ready)
    }

    async fn save(&self) -> Result<()> {
        // 使用临时文件 + rename，尽量避免程序中断时写出半截 JSON。
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        let temp_file = self.state_file.with_extension("tmp");
        let state = ProtectionState {
            updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs().to_string())
                .unwrap_or_else(|_| "0".to_string()),
            protected: self.protected.clone(),
        };
        fs::write(&temp_file, serde_json::to_vec_pretty(&state)?).await?;
        fs::rename(temp_file, &self.state_file).await?;
        Ok(())
    }

    fn to_relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.target_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

async fn load_state(path: &Path) -> Result<ProtectionState> {
    let content = fs::read(path).await?;
    Ok(serde_json::from_slice(&content)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn delays_large_deletions_until_grace_count() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let target_dir = std::env::temp_dir().join(format!("autofilm-protection-{unique}"));
        fs::create_dir_all(&target_dir).await.unwrap();

        let config = SmartProtection {
            enabled: true,
            threshold: 1,
            grace_scans: 2,
        };
        let mut manager = ProtectionManager::new(target_dir.clone(), "test", &config)
            .await
            .unwrap();
        let file = target_dir.join("movie.strm");

        let first = manager
            .process(HashSet::from([file.clone()]), &HashSet::new())
            .await
            .unwrap();
        assert!(first.is_empty());

        let second = manager
            .process(HashSet::from([file.clone()]), &HashSet::new())
            .await
            .unwrap();
        assert_eq!(second, HashSet::from([file]));
    }
}
