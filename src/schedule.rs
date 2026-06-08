mod utils;

use chrono_tz::Tz;
use std::sync::Arc;

use crate::alist2strm::Alist2Strm;
use crate::ani2alist::Ani2Alist;
use crate::config::Config;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use tracing::{debug, error, info, warn};

pub async fn create_scheduler(
    config: Config,
    tz: Tz,
) -> Result<(JobScheduler, usize), JobSchedulerError> {
    let scheduler = JobScheduler::new().await?;
    let mut scheduled_count = 0usize;

    // key是AList客户端ID，value是(Client, 服务器URL)二元组
    let alist_clients = utils::create_alist_clients(&config.alist).await;

    let mut alist2strm_scheduled_count = 0usize;
    for task in config.alist2strm_tasks {
        let task_id = task.id.clone();
        let Some(cron) = task.cron.clone() else {
            warn!(task_id = %task_id, "Alist2Strm 任务缺少 cron，已跳过");
            continue;
        };

        info!(task_id = %task_id, cron = %cron, "添加 Alist2Strm 定时任务");
        let Some((client, server_url)) = alist_clients.get(&task.alist) else {
            error!(
                task_id = %task_id,
                alist = %task.alist,
                "Alist2Strm 任务引用的 AList 客户端不存在，已跳过任务"
            );
            continue;
        };

        debug!(
            task_id = %task_id,
            alist = %task.alist,
            source_dir = %task.source_dir,
            target_dir = %task.target_dir.display(),
            "成功解析 Alist2Strm 任务配置"
        );

        let runner = Arc::new(Alist2Strm::new(task, client.clone(), server_url.clone()));
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
                               duration_millis = summary.duration.as_millis(),
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
        alist2strm_scheduled_count += 1;
        scheduled_count += 1;
    }

    info!(
        module = "Alist2Strm",
        scheduled_count = alist2strm_scheduled_count,
        "子任务调度完成"
    );

    let mut ani2alist_scheduled_count = 0usize;
    for task in config.ani2alist_tasks {
        let task_id = task.id.clone();
        let Some(cron) = task.cron.clone() else {
            warn!(task_id = %task_id, "Ani2Alist 任务缺少 cron，已跳过");
            continue;
        };

        info!(task_id = %task_id, cron = %cron, "添加 Ani2Alist 定时任务");
        let Some((client, _server_url)) = alist_clients.get(&task.alist) else {
            error!(
                task_id = %task_id,
                alist = %task.alist,
                "Ani2Alist 任务引用的 AList 客户端不存在，已跳过任务"
            );
            continue;
        };

        debug!(
            task_id = %task_id,
            alist = %task.alist,
            target_dir = %task.target_dir,
            "成功解析 Ani2Alist 任务配置"
        );

        let runner = Arc::new(Ani2Alist::new(task, client.clone(), tz));
        scheduler
            .add(Job::new_async_tz(cron, tz, move |_uuid, _lock| {
                let runner = runner.clone();
                let task_id = task_id.clone();
                Box::pin(async move {
                    info!(task_id = %task_id, "开始执行 Ani2Alist 任务");
                    match runner.run().await {
                        Err(err) => {
                            error!(task_id = %task_id, error = %err, "Ani2Alist 任务失败");
                        }
                        Ok(()) => {
                            info!(task_id = %task_id, "Ani2Alist 任务完成");
                        }
                    }
                })
            })?)
            .await?;
        ani2alist_scheduled_count += 1;
        scheduled_count += 1;
    }

    info!(
        module = "Ani2Alist",
        scheduled_count = ani2alist_scheduled_count,
        "子任务调度完成"
    );
    info!(scheduled_count, "全部子任务调度完成");

    Ok((scheduler, scheduled_count))
}
