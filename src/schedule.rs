use chrono_tz::Tz;
use std::sync::Arc;

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

    for task in config.alist2strm_tasks {
        let task_id = task.id.clone();
        let Some(cron) = task.cron.clone() else {
            warn!(task_id = %task_id, "Alist2Strm 任务缺少 cron，已跳过");
            continue;
        };

        info!(task_id = %task_id, cron = %cron, "添加 Alist2Strm 定时任务");
        let runner = Arc::new(Alist2Strm::new(task));
        scheduler
            .add(Job::new_async_tz(cron, tz, move |_uuid, _lock| {
                let runner = runner.clone();
                let task_id = task_id.clone();
                Box::pin(async move {
                    info!(task_id = %task_id, "开始执行 Alist2Strm 任务");
                    if let Err(err) = runner.run().await {
                        error!(task_id = %task_id, error = %err, "Alist2Strm 任务失败");
                    } else {
                        info!(task_id = %task_id, "Alist2Strm 任务完成");
                    }
                })
            })?)
            .await?;
        scheduled_count += 1;
    }

    Ok((scheduler, scheduled_count))
}
