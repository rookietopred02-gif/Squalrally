use crate::scanners::element_scan_dispatcher::ElementScanDispatcher;
use crate::scanners::snapshot_region_memory_reader::SnapshotRegionMemoryReader;
use crate::scanners::value_collector_task::ValueCollectorTask;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use squalr_engine_api::conversions::storage_size_conversions::StorageSizeConversions;
use squalr_engine_api::structures::processes::opened_process_info::OpenedProcessInfo;
use squalr_engine_api::structures::results::snapshot_region_scan_results::SnapshotRegionScanResults;
use squalr_engine_api::structures::scanning::memory_read_mode::MemoryReadMode;
use squalr_engine_api::structures::scanning::plans::element_scan::element_scan_plan::ElementScanPlan;
use squalr_engine_api::structures::snapshots::snapshot::Snapshot;
use squalr_engine_api::structures::snapshots::snapshot_region::SnapshotRegion;
use squalr_engine_api::structures::tasks::trackable_task::TrackableTask;
use crate::scan_settings_config::ScanSettingsConfig;
use squalr_engine_api::structures::settings::scan_thread_priority::ScanThreadPriority;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

pub struct ElementScanExecutorTask {}

const TASK_NAME: &'static str = "Element Scan Executor";

/// Implementation of a task that performs a scan against the provided snapshot. Does not collect new values.
/// Caller is assumed to have already done this if desired.
impl ElementScanExecutorTask {
    pub fn start_task(
        process_info: OpenedProcessInfo,
        snapshot: Arc<RwLock<Snapshot>>,
        element_scan_plan: ElementScanPlan,
        with_logging: bool,
    ) -> Arc<TrackableTask> {
        let task = TrackableTask::create(TASK_NAME.to_string(), None);
        let task_clone = task.clone();

        thread::spawn(move || {
            Self::apply_thread_priority(ScanSettingsConfig::get_thread_priority());
            Self::scan_task(&task_clone, process_info, snapshot, element_scan_plan, with_logging);

            task_clone.complete();
        });

        task
    }

    fn scan_task(
        trackable_task: &Arc<TrackableTask>,
        process_info: OpenedProcessInfo,
        snapshot: Arc<RwLock<Snapshot>>,
        element_scan_plan: ElementScanPlan,
        with_logging: bool,
    ) {
        let total_start_time = Instant::now();

        // If the parameter is set, first collect values before the scan.
        // This is slower overall than interleaving the reads, but better for capturing values that may soon change.
        if element_scan_plan.get_memory_read_mode() == MemoryReadMode::ReadBeforeScan {
            ValueCollectorTask::start_task(process_info.clone(), snapshot.clone(), with_logging).wait_for_completion();
        }

        if with_logging {
            log::info!("Performing manual scan...");
        }

        // Avoid holding the snapshot write-lock for the entire scan. Long write-locks freeze the UI and block result
        // queries. We take the regions out, scan them off-lock, then write them back.
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
        let cancellation_token = trackable_task.get_cancellation_token();

        // Create a function that processes every snapshot region, from which we will grab the existing snapshot filters (previous results) to perform our next scan.
        let snapshot_iterator = |snapshot_region: &mut SnapshotRegion| {
            if cancellation_token.load(Ordering::SeqCst) {
                return;
            }

            // Creates initial results if none exist yet.
            snapshot_region.initialize_scan_results(element_scan_plan.get_data_type_refs_iterator(), element_scan_plan.get_memory_alignment());

            // Attempt to read new (or initial) memory values. Ignore failures as they usually indicate deallocated pages. // JIRA: Remove failures somehow.
            if element_scan_plan.get_memory_read_mode() == MemoryReadMode::ReadInterleavedWithScan {
                if snapshot_region.read_all_memory_chunked(&process_info).is_err() {
                    snapshot_region.mark_unreadable();
                    processed_region_count.fetch_add(1, Ordering::SeqCst);
                    return;
                }
            }

            /*
            // JIRA: Fixme? Early exit gains?
            if !element_scan_plan.is_valid_for_snapshot_region(snapshot_region) {
                processed_region_count.fetch_add(1, Ordering::SeqCst);
                return;
            }*/

            // Create a function to dispatch our element scan to the best scanner implementation for the current region.
            let element_scan_dispatcher = |snapshot_region_filter_collection| {
                ElementScanDispatcher::dispatch_scan(snapshot_region, snapshot_region_filter_collection, &element_scan_plan)
            };

            // Again, select the parallel or sequential iterator to iterate over each data type in the scan. Generally there is only 1, but multi-type scans are supported.
            let scan_results_collection = snapshot_region.get_scan_results().get_filter_collections();
            let single_thread_scan = element_scan_plan.get_is_single_thread_scan() || scan_results_collection.len() == 1;
            let scan_results = SnapshotRegionScanResults::new(if single_thread_scan {
                scan_results_collection
                    .iter()
                    .map(element_scan_dispatcher)
                    .collect()
            } else {
                scan_results_collection
                    .par_iter()
                    .map(element_scan_dispatcher)
                    .collect()
            });

            snapshot_region.set_scan_results(scan_results);

            let processed = processed_region_count.fetch_add(1, Ordering::SeqCst);

            // To reduce performance impact, only periodically send progress updates.
            if processed % 32 == 0 {
                let progress = (processed as f32 / total_region_count as f32) * 100.0;
                trackable_task.set_progress(progress);
            }

            if ScanSettingsConfig::get_pause_while_scanning() {
                thread::sleep(Duration::from_millis(1));
            }
        };

        // Select either the parallel or sequential iterator. Single-thread is not advised unless debugging.
        let single_thread_scan = element_scan_plan.get_is_single_thread_scan() || snapshot_regions.len() == 1;
        if single_thread_scan {
            snapshot_regions.iter_mut().for_each(snapshot_iterator);
        } else {
            snapshot_regions.par_iter_mut().for_each(snapshot_iterator);
        };

        // Finalize: write the scanned regions back into the snapshot.
        let result_count: u64 = snapshot_regions
            .iter()
            .map(|region| region.get_scan_results().get_number_of_results())
            .sum();
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
            let byte_count = snapshot
                .read()
                .map(|s| s.get_byte_count())
                .unwrap_or(0);
            let duration = start_time.elapsed();
            let total_duration = total_start_time.elapsed();

            log::info!("Results: {} bytes", StorageSizeConversions::value_to_metric_size(byte_count as u128));
            log::info!("Result count: {}", result_count);
            log::info!("Scan complete in: {:?}", duration);
            log::info!("Total scan time: {:?}", total_duration);

            if result_count == 0 {
                log::warn!(
                    "Scan produced 0 results. If you expected matches, try loosening Memory settings (e.g., disable 'Writable' requirement for strings, or enable additional memory types), or ensure the target stores the value in the selected encoding/type."
                );
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
