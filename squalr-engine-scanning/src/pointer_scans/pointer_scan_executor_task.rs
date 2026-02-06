use crate::scanners::value_collector_task::ValueCollectorTask;
use squalr_engine_api::structures::data_types::built_in_types::u32::data_type_u32::DataTypeU32;
use squalr_engine_api::structures::pointer_scan::pointer_scan_result::PointerScanResult;
use squalr_engine_api::structures::scanning::plans::pointer_scan::pointer_scan_parameters::PointerScanParameters;
use squalr_engine_api::structures::snapshots::snapshot::Snapshot;
use squalr_engine_api::structures::tasks::trackable_task::TrackableTask;
use squalr_engine_api::structures::processes::opened_process_info::OpenedProcessInfo;
use squalr_engine_memory::memory_queryer::memory_queryer::MemoryQueryer;
use squalr_engine_memory::memory_queryer::memory_queryer_trait::IMemoryQueryer;
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread;

pub struct PointerScanExecutorTask {}

const TASK_NAME: &'static str = "Pointer Scan Executor";
const MAX_RESULTS: usize = 250_000;

impl PointerScanExecutorTask {
    pub fn start_task(
        process_info: OpenedProcessInfo,
        statics_snapshot: Arc<RwLock<Snapshot>>,
        heaps_snapshot: Arc<RwLock<Snapshot>>,
        pointer_scan_parameters: PointerScanParameters,
        results_sink: Arc<RwLock<Vec<PointerScanResult>>>,
        with_logging: bool,
    ) -> Arc<TrackableTask> {
        let task = TrackableTask::create(TASK_NAME.to_string(), None);
        let task_clone = task.clone();

        thread::spawn(move || {
            Self::scan_task(
                &task_clone,
                process_info,
                statics_snapshot,
                heaps_snapshot,
                pointer_scan_parameters,
                results_sink,
                with_logging,
            );

            task_clone.complete();
        });

        task
    }

    fn scan_task(
        trackable_task: &Arc<TrackableTask>,
        process_info: OpenedProcessInfo,
        statics_snapshot: Arc<RwLock<Snapshot>>,
        heaps_snapshot: Arc<RwLock<Snapshot>>,
        pointer_scan_parameters: PointerScanParameters,
        results_sink: Arc<RwLock<Vec<PointerScanResult>>>,
        with_logging: bool,
    ) {
        if with_logging {
            log::info!("Performing pointer scan...");
        }

        ValueCollectorTask::start_task(process_info.clone(), statics_snapshot.clone(), with_logging).wait_for_completion();
        ValueCollectorTask::start_task(process_info.clone(), heaps_snapshot.clone(), with_logging).wait_for_completion();

        let pointer_size = if pointer_scan_parameters.get_pointer_data_type_ref().get_data_type_id() == DataTypeU32::get_data_type_id() {
            4usize
        } else {
            8usize
        };

        let target_bytes = pointer_scan_parameters.get_target_address().get_value_bytes();
        let target_address = match target_bytes.len() {
            4 => u32::from_le_bytes([target_bytes[0], target_bytes[1], target_bytes[2], target_bytes[3]]) as u64,
            8 => u64::from_le_bytes([
                target_bytes[0], target_bytes[1], target_bytes[2], target_bytes[3],
                target_bytes[4], target_bytes[5], target_bytes[6], target_bytes[7],
            ]),
            _ => 0,
        };

        let max_offset = pointer_scan_parameters.get_offset_size();
        let max_depth = pointer_scan_parameters.get_max_depth().max(1);

        let mut pointer_map: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        let min_user_addr = 0u64;
        let max_user_addr = MemoryQueryer::get_instance().get_max_usermode_address(&process_info);

        if pointer_scan_parameters.get_scan_statics() {
            collect_pointer_values(&statics_snapshot, pointer_size, min_user_addr, max_user_addr, &mut pointer_map);
        }

        if pointer_scan_parameters.get_scan_heaps() {
            collect_pointer_values(&heaps_snapshot, pointer_size, min_user_addr, max_user_addr, &mut pointer_map);
        }

        let modules = MemoryQueryer::get_instance().get_modules(&process_info);
        let mut results: Vec<PointerScanResult> = Vec::new();
        let mut visited: HashSet<(u64, usize)> = HashSet::new();

        let mut frontier: Vec<(u64, Vec<u64>)> = vec![(target_address, Vec::new())];

        for depth in 0..max_depth {
            if trackable_task.get_cancellation_token().load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            let mut next_frontier = Vec::new();

            for (target, offsets) in frontier.iter() {
                let start = target.saturating_sub(max_offset);
                let end = target.saturating_add(max_offset);

                for (value, pointer_addresses) in pointer_map.range(start..=end) {
                    let offset = target.saturating_sub(*value);
                    for pointer_address in pointer_addresses {
                        let mut new_offsets = offsets.clone();
                        new_offsets.insert(0, offset);

                        if results.len() < MAX_RESULTS {
                            let mut module_name = String::new();
                            let mut module_offset = *pointer_address;
                            let mut is_module = false;

                            if let Some((found_module_name, offset_addr)) =
                                MemoryQueryer::get_instance().address_to_module(*pointer_address, &modules)
                            {
                                module_name = found_module_name;
                                module_offset = offset_addr;
                                is_module = true;
                            }

                            results.push(PointerScanResult::new(
                                *pointer_address,
                                module_name,
                                module_offset,
                                new_offsets.clone(),
                                is_module,
                            ));
                        }

                        if results.len() >= MAX_RESULTS {
                            break;
                        }

                        let key = (*pointer_address, depth as usize + 1);
                        if visited.insert(key) {
                            next_frontier.push((*pointer_address, new_offsets));
                        }
                    }

                    if results.len() >= MAX_RESULTS {
                        break;
                    }
                }

                if results.len() >= MAX_RESULTS {
                    break;
                }
            }

            let progress = ((depth + 1) as f32 / max_depth as f32) * 100.0;
            trackable_task.set_progress(progress);

            if next_frontier.is_empty() {
                break;
            }

            frontier = next_frontier;
        }

        if let Ok(mut sink) = results_sink.write() {
            *sink = results;
        }
    }
}

fn collect_pointer_values(
    snapshot: &Arc<RwLock<Snapshot>>,
    pointer_size: usize,
    min_addr: u64,
    max_addr: u64,
    pointer_map: &mut BTreeMap<u64, Vec<u64>>,
) {
    let snapshot = match snapshot.read() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            log::error!("Failed to acquire snapshot read lock: {}", error);
            return;
        }
    };

    for region in snapshot.get_snapshot_regions() {
        let base_address = region.get_base_address();
        let bytes = region.get_current_values();
        if bytes.len() < pointer_size {
            continue;
        }

        let mut offset = 0usize;
        while offset + pointer_size <= bytes.len() {
            let value = if pointer_size == 4 {
                let raw = u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]) as u64;
                raw
            } else {
                u64::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ])
            };

            if value >= min_addr && value <= max_addr {
                let pointer_address = base_address.saturating_add(offset as u64);
                pointer_map.entry(value).or_insert_with(Vec::new).push(pointer_address);
            }

            offset += pointer_size;
        }
    }
}
