use crate::scanners::snapshot_region_memory_reader::SnapshotRegionMemoryReader;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;
use squalr_engine_api::conversions::storage_size_conversions::StorageSizeConversions;
use squalr_engine_api::structures::processes::opened_process_info::OpenedProcessInfo;
use squalr_engine_api::structures::snapshots::snapshot::Snapshot;
use squalr_engine_api::structures::snapshots::snapshot_region::SnapshotRegion;
use squalr_engine_api::structures::tasks::trackable_task::TrackableTask;
use crate::scan_settings_config::ScanSettingsConfig;
use squalr_engine_api::structures::settings::scan_thread_priority::ScanThreadPriority;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use std::time::Duration;

const TASK_NAME: &'static str = "Value Collector";

pub struct ValueCollectorTask;

/// Implementation of a task that collects new or initial values for the provided snapshot.
impl ValueCollectorTask {
    pub fn start_task(
        process_info: OpenedProcessInfo,
        snapshot: Arc<RwLock<Snapshot>>,
        with_logging: bool,
    ) -> Arc<TrackableTask> {
        let task = TrackableTask::create(TASK_NAME.to_string(), None);
        let task_clone = task.clone();
        let process_info = Arc::new(process_info);
        let process_info_clone = process_info.clone();
        let snapshot = snapshot.clone();

        std::thread::spawn(move || {
            Self::apply_thread_priority(ScanSettingsConfig::get_thread_priority());
            Self::collect_values_task(&task_clone, process_info_clone, snapshot, with_logging);

            task_clone.complete();
        });

        task
    }

    fn collect_values_task(
        trackable_task: &Arc<TrackableTask>,
        process_info: Arc<OpenedProcessInfo>,
        snapshot: Arc<RwLock<Snapshot>>,
        with_logging: bool,
    ) {
        if with_logging {
            log::info!("Reading values from memory (process {})...", process_info.get_process_id_raw());
        }

        // Avoid holding the snapshot write-lock for the entire read, which can freeze the UI and block result queries.
        // We "take" the regions out, process them off-lock, then write them back.
        let (mut snapshot_regions, total_region_count) = {
            let mut snapshot_guard = match snapshot.write() {
                Ok(guard) => guard,
                Err(error) => {
                    if with_logging {
                        log::error!("Failed to acquire write lock on snapshot: {}", error);
                    }

                    return;
                }
            };

            let regions = std::mem::take(snapshot_guard.get_snapshot_regions_mut());
            let count = regions.len();
            (regions, count)
        };

        let start_time = Instant::now();
        let processed_region_count = Arc::new(AtomicUsize::new(0));

        if with_logging && total_region_count == 0 {
            log::warn!(
                "Scan snapshot contains 0 regions for process {}. This usually means no memory pages matched the current Memory settings, or the process could not be queried.",
                process_info.get_process_id_raw()
            );
        }

        let cancellation_token = trackable_task.get_cancellation_token();

        let read_memory_iterator = |snapshot_region: &mut SnapshotRegion| {
            if cancellation_token.load(Ordering::SeqCst) {
                return;
            }

            // Attempt to read new (or initial) memory values. Ignore failed regions, as these are generally just deallocated pages.
            // JIRA: We probably want some way of tombstoning deallocated pages.
            if snapshot_region.read_all_memory_chunked(&process_info).is_err() {
                snapshot_region.mark_unreadable();
            }

            // Report progress periodically (not every time for performance)
            let processed = processed_region_count.fetch_add(1, Ordering::SeqCst);

            if processed % 32 == 0 {
                let progress = (processed as f32 / total_region_count as f32) * 100.0;
                trackable_task.set_progress(progress);
            }

            if ScanSettingsConfig::get_pause_while_scanning() {
                std::thread::sleep(Duration::from_millis(1));
            }
        };

        // Collect values for each snapshot region in parallel.
        snapshot_regions.par_iter_mut().for_each(read_memory_iterator);

        // Capture pre-finalization stats (note: set_snapshot_regions discards size==0 regions).
        let unreadable_region_count = snapshot_regions
            .iter()
            .filter(|region| region.get_region_size() == 0)
            .count();
        let final_byte_count: u64 = snapshot_regions.iter().map(|r| r.get_region_size()).sum();

        // Write the regions back into the snapshot.
        {
            let mut snapshot_guard = match snapshot.write() {
                Ok(guard) => guard,
                Err(error) => {
                    if with_logging {
                        log::error!("Failed to acquire write lock on snapshot to finalize: {}", error);
                    }
                    return;
                }
            };

            snapshot_guard.set_snapshot_regions(snapshot_regions);
        }

        if with_logging {
            let duration = start_time.elapsed();
            let byte_count = final_byte_count;

            log::info!("Values collected in: {:?}", duration);
            log::info!(
                "{} bytes read ({})",
                byte_count,
                StorageSizeConversions::value_to_metric_size(byte_count as u128)
            );

            if byte_count == 0 {
                if total_region_count > 0 && unreadable_region_count == total_region_count {
                    log::warn!(
                        "All snapshot regions became unreadable while reading process {}. This often indicates insufficient access rights or a protected process.",
                        process_info.get_process_id_raw()
                    );
                } else if total_region_count > 0 {
                    log::warn!(
                        "Snapshot read yielded 0 bytes for process {} (regions={}, unreadable_regions={}).",
                        process_info.get_process_id_raw(),
                        total_region_count,
                        unreadable_region_count
                    );
                }
            }
        }
    }

    fn apply_thread_priority(priority: ScanThreadPriority) {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::Threading::{
                GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL, THREAD_PRIORITY_HIGHEST, THREAD_PRIORITY_NORMAL,
            };

            let value = match priority {
                ScanThreadPriority::Normal => THREAD_PRIORITY_NORMAL,
                ScanThreadPriority::AboveNormal => THREAD_PRIORITY_ABOVE_NORMAL,
                ScanThreadPriority::Highest => THREAD_PRIORITY_HIGHEST,
            };

            let _ = SetThreadPriority(GetCurrentThread(), value);
        }
    }
}
