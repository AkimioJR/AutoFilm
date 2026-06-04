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
    /// 创建 `.strm` 智能删除保护管理器。
    ///
    /// 管理器会在目标目录下使用任务 ID 对应的状态文件保存候选删除计数。
    /// 如果存在上次扫描留下的状态，会尽量加载并延续计数；加载失败则从空状态
    /// 开始，避免损坏的保护文件阻止同步任务继续运行。
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

    /// 根据本次扫描结果决定哪些 `.strm` 可以真正删除。
    ///
    /// 当待删除数量低于阈值时，认为是正常同步差异并立即返回待删除集合；
    /// 当数量达到阈值时，进入宽限计数，只有连续多次扫描都确认缺失的文件才会
    /// 返回给清理流程。仍然存在于远端扫描结果中的文件会从保护状态中移除。
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

    /// 将当前保护状态写入磁盘。
    ///
    /// 状态记录每个相对路径已连续缺失的次数。写入时先生成临时文件再 rename，
    /// 尽量避免程序中断或崩溃时留下半截 JSON。
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

    /// 将本地绝对路径转换为目标目录下的相对路径字符串。
    ///
    /// 保护状态使用相对路径持久化，避免目标目录移动后状态文件中记录的绝对路径
    /// 失效；Windows 路径分隔符也会统一转换为 `/`。
    fn to_relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.target_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

/// 从保护状态文件读取上次扫描留下的计数。
///
/// 返回值包含每个候选 `.strm` 的连续缺失次数。调用方会在读取失败时回退为空
/// 状态，因此该函数只负责严格解析文件内容。
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
