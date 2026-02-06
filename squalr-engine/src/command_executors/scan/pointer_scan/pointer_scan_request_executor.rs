use crate::command_executors::privileged_request_executor::PrivilegedCommandRequestExecutor;
use crate::engine_privileged_state::EnginePrivilegedState;
use squalr_engine_api::commands::scan::pointer_scan::pointer_scan_request::PointerScanRequest;
use squalr_engine_api::commands::scan::pointer_scan::pointer_scan_response::PointerScanResponse;
use squalr_engine_api::events::pointer_scan_results::updated::pointer_scan_results_updated_event::PointerScanResultsUpdatedEvent;
use squalr_engine_api::events::trackable_task::progress_changed::trackable_task_progress_changed_event::TrackableTaskProgressChangedEvent;
use squalr_engine_api::structures::pointer_scan::pointer_scan_results::PointerScanResults;
use squalr_engine_api::structures::scanning::plans::pointer_scan::pointer_scan_parameters::PointerScanParameters;
use squalr_engine_api::structures::snapshots::snapshot::Snapshot;
use squalr_engine_api::structures::snapshots::snapshot_region::SnapshotRegion;
use squalr_engine_scanning::pointer_scans::pointer_scan_executor_task::PointerScanExecutorTask;
use squalr_engine_scanning::scan_settings_config::ScanSettingsConfig;
use squalr_engine_memory::memory_queryer::memory_queryer::MemoryQueryer;
use squalr_engine_memory::memory_queryer::page_retrieval_mode::PageRetrievalMode;
use std::sync::{Arc, RwLock};
use std::thread;

impl PrivilegedCommandRequestExecutor for PointerScanRequest {
    type ResponseType = PointerScanResponse;

    fn execute(
        &self,
        engine_privileged_state: &Arc<EnginePrivilegedState>,
    ) -> <Self as PrivilegedCommandRequestExecutor>::ResponseType {
        let Some(process_info) = engine_privileged_state.get_process_manager().get_opened_process() else {
            log::error!("No opened process");
            return PointerScanResponse { trackable_task_handle: None };
        };

        let symbol_registry = engine_privileged_state.get_symbol_registry();
        let symbol_registry_guard = match symbol_registry.read() {
            Ok(registry) => registry,
            Err(error) => {
                log::error!("Failed to acquire read lock on SymbolRegistry: {}", error);
                return PointerScanResponse::default();
            }
        };

        let target_address = match symbol_registry_guard.deanonymize_value_string(&self.pointer_data_type_ref, &self.target_address) {
            Ok(data_value) => data_value,
            Err(error) => {
                log::error!("Failed to deanonimize pointer target address: {}", error);
                return PointerScanResponse::default();
            }
        };

        let scan_parameters = PointerScanParameters::new(
            target_address,
            self.pointer_data_type_ref.clone(),
            self.offset_size,
            self.max_depth,
            self.scan_statics,
            self.scan_heaps,
            ScanSettingsConfig::get_is_single_threaded_scan(),
            ScanSettingsConfig::get_debug_perform_validation_scan(),
        );

        if let Ok(mut results) = engine_privileged_state.get_pointer_scan_results().write() {
            results.set_results(Vec::new());
        }

        let statics_snapshot = Arc::new(RwLock::new(build_snapshot(&process_info, PageRetrievalMode::FromModules)));
        let heaps_snapshot = Arc::new(RwLock::new(build_snapshot(&process_info, PageRetrievalMode::FromNonModules)));
        let results_sink: Arc<RwLock<Vec<squalr_engine_api::structures::pointer_scan::pointer_scan_result::PointerScanResult>>> =
            Arc::new(RwLock::new(Vec::new()));

        let task = PointerScanExecutorTask::start_task(
            process_info,
            statics_snapshot,
            heaps_snapshot,
            scan_parameters,
            results_sink.clone(),
            true,
        );

        let task_handle = task.get_task_handle();
        let engine_privileged_state = engine_privileged_state.clone();
        let progress_receiver = task.subscribe_to_progress_updates();

        engine_privileged_state.get_trackable_task_manager().register_task(task.clone());

        let task_id = task.get_task_identifier();
        let progress_engine_state = engine_privileged_state.clone();
        thread::spawn(move || {
            while let Ok(progress) = progress_receiver.recv() {
                progress_engine_state.emit_event(TrackableTaskProgressChangedEvent { task_id: task_id.clone(), progress });
            }
        });

        thread::spawn(move || {
            task.wait_for_completion();
            engine_privileged_state.get_trackable_task_manager().unregister_task(&task.get_task_identifier());

            if let Ok(results_guard) = results_sink.read() {
                let page_size = ScanSettingsConfig::get_results_page_size() as u64;
                if let Ok(mut pointer_scan_results) = engine_privileged_state.get_pointer_scan_results().write() {
                    *pointer_scan_results = PointerScanResults::new(results_guard.clone(), page_size.max(1));
                }
            }

            engine_privileged_state.emit_event(PointerScanResultsUpdatedEvent {});
        });

        PointerScanResponse { trackable_task_handle: Some(task_handle) }
    }
}

fn build_snapshot(
    process_info: &squalr_engine_api::structures::processes::opened_process_info::OpenedProcessInfo,
    page_retrieval_mode: PageRetrievalMode,
) -> Snapshot {
    let memory_pages = MemoryQueryer::get_memory_page_bounds(process_info, page_retrieval_mode);
    let mut merged_snapshot_regions = Vec::new();
    let mut page_boundaries = Vec::new();
    let mut iter = memory_pages.into_iter();
    let current_region = iter.next();

    if let Some(mut current_region) = current_region {
        loop {
            let Some(region) = iter.next() else {
                break;
            };

            if current_region.get_end_address() == region.get_base_address() {
                current_region.set_end_address(region.get_end_address());
                page_boundaries.push(region.get_base_address());
            } else {
                merged_snapshot_regions.push(SnapshotRegion::new(current_region, std::mem::take(&mut page_boundaries)));
                current_region = region;
            }
        }

        merged_snapshot_regions.push(SnapshotRegion::new(current_region, page_boundaries));
    }

    let mut snapshot = Snapshot::new();
    snapshot.set_snapshot_regions(merged_snapshot_regions);
    snapshot
}
