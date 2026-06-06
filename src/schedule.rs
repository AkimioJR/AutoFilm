use chrono_tz::Tz;
use std::collections::HashMap;
use std::sync::Arc;

use alist::Client;

use crate::alist::build_client;
use crate::alist2strm::Alist2Strm;
use crate::config::Config;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use tracing::{error, info, warn};

pub async fn create_scheduler(
    config: Config,
    tz: Tz,
) -> Result<(JobScheduler, usize), JobSchedulerError> {
    let scheduler = JobScheduler::new().await?;
    let mut scheduled_count = 0usize;
    let mut alist_clients: HashMap<String, Arc<Client>> = HashMap::new();
    let alist_configs = config
        .alist
        .into_iter()
        .map(|alist| (alist.id.clone(), alist))
        .collect::<HashMap<_, _>>();

    for task in config.alist2strm_tasks {
        let task_id = task.id.clone();
        let Some(cron) = task.cron.clone() else {
            warn!(task_id = %task_id, "Alist2Strm 任务缺少 cron，已跳过");
            continue;
        };

        info!(task_id = %task_id, cron = %cron, "添加 Alist2Strm 定时任务");
        let Some(alist_config) = alist_configs.get(&task.alist).cloned() else {
            error!(
                task_id = %task_id,
                alist = %task.alist,
                "Alist2Strm 任务引用的 AList 客户端不存在，已跳过任务"
            );
            continue;
        };

        let client = match alist_clients.get(&alist_config.id) {
            Some(client) => client.clone(),
            None => match build_client(&alist_config) {
                Ok(client) => {
                    let client = Arc::new(client);
                    alist_clients.insert(alist_config.id.clone(), client.clone());
                    client
                }
                Err(err) => {
                    error!(task_id = %task_id, error = %err, "创建 AList 客户端失败，已跳过任务");
                    continue;
                }
            },
        };
        let server_url = alist_config
            .public_url
            .unwrap_or_else(|| alist_config.base_url.clone());
        let runner = Arc::new(Alist2Strm::new(task, client, server_url));
        scheduler
            .add(Job::new_async_tz(cron, tz, move |_uuid, _lock| {
                let runner = runner.clone();
                let task_id = task_id.clone();
                Box::pin(async move {
                    info!(task_id = %task_id, "开始执行 Alist2Strm 任务");
                    match runner.run().await {
                        Err(err) => {
                            error!(task_id = %task_id, error = %err, "Alist2Strm 任务失败");
                        }
                        Ok(summary) => {
                            info!(
                               task_id = %summary.task_id,
                               source_dir = %summary.source_dir,
                               target_dir = %summary.target_dir.display(),
                               start_time = %&summary.start_time.with_timezone(&tz),
                               end_time = %&summary.end_time.with_timezone(&tz),
                               duration_millis = summary.duration_millis,
                               scanned_dir_count = summary.scanned_dir_count,
                               skipped_dir_count = summary.skipped_dir_count,
                               discovered_file_count = summary.discovered_file_count,
                               matched_file_count = summary.matched_file_count,
                               filtered_file_count = summary.filtered_file_count,
                               bdmv_collection_count = summary.bdmv_collection_count,
                               bdmv_selected_count = summary.bdmv_selected_count,
                               strm_created_count = summary.strm_created_count,
                               strm_updated_count = summary.strm_updated_count,
                               strm_skipped_count = summary.strm_skipped_count,
                               attachment_downloaded_count = summary.attachment_downloaded_count,
                               attachment_updated_count = summary.attachment_updated_count,
                               attachment_skipped_count = summary.attachment_skipped_count,
                               local_deleted_count = summary.local_deleted_count,
                               local_delete_ignored_count = summary.local_delete_ignored_count,
                               failed_path_count = summary.failed_path_count,
                               "Alist2Strm 任务完成"
                            );
                        }
                    }
                })
            })?)
            .await?;
        scheduled_count += 1;
    }

    Ok((scheduler, scheduled_count))
}
