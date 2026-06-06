use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use chrono::{DateTime, Utc};

use crate::alist2strm::Config;

/// 一次 Alist2Strm 运行结束后的结构化总结。
///
/// 该结构可直接用于日志，也为后续通知接口保留稳定的数据入口。
#[derive(Debug, Clone, serde::Serialize)]
pub struct RunSummary {
    pub task_id: String,
    pub source_dir: String,
    pub target_dir: PathBuf,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_millis: u128,
    pub scanned_dir_count: usize,
    pub skipped_dir_count: usize,
    pub discovered_file_count: usize,
    pub matched_file_count: usize,
    pub filtered_file_count: usize,
    pub bdmv_collection_count: usize,
    pub bdmv_selected_count: usize,
    pub strm_created_count: usize,
    pub strm_updated_count: usize,
    pub strm_skipped_count: usize,
    pub attachment_downloaded_count: usize,
    pub attachment_updated_count: usize,
    pub attachment_skipped_count: usize,
    pub local_deleted_count: usize,
    pub local_delete_ignored_count: usize,
    pub failed_path_count: usize,
}

#[derive(Debug)]
pub(super) struct RunStats {
    start_time: DateTime<Utc>,
    started_at: Instant,
    pub(super) scanned_dir_count: AtomicUsize,
    pub(super) skipped_dir_count: AtomicUsize,
    pub(super) discovered_file_count: AtomicUsize,
    pub(super) matched_file_count: AtomicUsize,
    pub(super) filtered_file_count: AtomicUsize,
    pub(super) bdmv_collection_count: AtomicUsize,
    pub(super) bdmv_selected_count: AtomicUsize,
    pub(super) strm_created_count: AtomicUsize,
    pub(super) strm_updated_count: AtomicUsize,
    pub(super) strm_skipped_count: AtomicUsize,
    pub(super) attachment_downloaded_count: AtomicUsize,
    pub(super) attachment_updated_count: AtomicUsize,
    pub(super) attachment_skipped_count: AtomicUsize,
    pub(super) local_deleted_count: AtomicUsize,
    pub(super) local_delete_ignored_count: AtomicUsize,
    pub(super) failed_path_count: AtomicUsize,
}

impl RunStats {
    pub(super) fn new() -> Self {
        Self {
            start_time: Utc::now(),
            started_at: Instant::now(),
            scanned_dir_count: AtomicUsize::new(0),
            skipped_dir_count: AtomicUsize::new(0),
            discovered_file_count: AtomicUsize::new(0),
            matched_file_count: AtomicUsize::new(0),
            filtered_file_count: AtomicUsize::new(0),
            bdmv_collection_count: AtomicUsize::new(0),
            bdmv_selected_count: AtomicUsize::new(0),
            strm_created_count: AtomicUsize::new(0),
            strm_updated_count: AtomicUsize::new(0),
            strm_skipped_count: AtomicUsize::new(0),
            attachment_downloaded_count: AtomicUsize::new(0),
            attachment_updated_count: AtomicUsize::new(0),
            attachment_skipped_count: AtomicUsize::new(0),
            local_deleted_count: AtomicUsize::new(0),
            local_delete_ignored_count: AtomicUsize::new(0),
            failed_path_count: AtomicUsize::new(0),
        }
    }

    pub(super) fn inc(counter: &AtomicUsize) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    fn load(counter: &AtomicUsize) -> usize {
        counter.load(Ordering::Relaxed)
    }

    pub(super) fn summarize(&self, config: &Config) -> RunSummary {
        let end_time = Utc::now();
        RunSummary {
            task_id: config.id.clone(),
            source_dir: config.source_dir.clone(),
            target_dir: config.target_dir.clone(),
            start_time: self.start_time,
            end_time,
            duration_millis: self.started_at.elapsed().as_millis(),
            scanned_dir_count: Self::load(&self.scanned_dir_count),
            skipped_dir_count: Self::load(&self.skipped_dir_count),
            discovered_file_count: Self::load(&self.discovered_file_count),
            matched_file_count: Self::load(&self.matched_file_count),
            filtered_file_count: Self::load(&self.filtered_file_count),
            bdmv_collection_count: Self::load(&self.bdmv_collection_count),
            bdmv_selected_count: Self::load(&self.bdmv_selected_count),
            strm_created_count: Self::load(&self.strm_created_count),
            strm_updated_count: Self::load(&self.strm_updated_count),
            strm_skipped_count: Self::load(&self.strm_skipped_count),
            attachment_downloaded_count: Self::load(&self.attachment_downloaded_count),
            attachment_updated_count: Self::load(&self.attachment_updated_count),
            attachment_skipped_count: Self::load(&self.attachment_skipped_count),
            local_deleted_count: Self::load(&self.local_deleted_count),
            local_delete_ignored_count: Self::load(&self.local_delete_ignored_count),
            failed_path_count: Self::load(&self.failed_path_count),
        }
    }
}
