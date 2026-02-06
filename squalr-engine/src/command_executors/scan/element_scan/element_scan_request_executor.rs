use crate::command_executors::privileged_request_executor::PrivilegedCommandRequestExecutor;
use crate::engine_privileged_state::EnginePrivilegedState;
use squalr_engine_api::commands::scan::element_scan::element_scan_request::ElementScanRequest;
use squalr_engine_api::commands::scan::element_scan::element_scan_response::ElementScanResponse;
use squalr_engine_api::events::scan_results::updated::scan_results_updated_event::ScanResultsUpdatedEvent;
use squalr_engine_api::registries::scan_rules::element_scan_rule_registry::ElementScanRuleRegistry;
use squalr_engine_api::registries::symbols::symbol_registry::SymbolRegistry;
use squalr_engine_api::structures::memory::memory_alignment::MemoryAlignment;
use squalr_engine_api::structures::scanning::constraints::scan_constraint_finalized::ScanConstraintFinalized;
use squalr_engine_api::structures::scanning::plans::element_scan::element_scan_plan::ElementScanPlan;
use squalr_engine_scanning::scan_settings_config::ScanSettingsConfig;
use squalr_engine_scanning::scanners::element_scan_executor_task::ElementScanExecutorTask;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

impl PrivilegedCommandRequestExecutor for ElementScanRequest {
    type ResponseType = ElementScanResponse;

    fn execute(
        &self,
        engine_privileged_state: &Arc<EnginePrivilegedState>,
    ) -> <Self as PrivilegedCommandRequestExecutor>::ResponseType {
        if let Some(process_info) = engine_privileged_state
            .get_process_manager()
            .get_opened_process()
        {
            let snapshot = engine_privileged_state.get_snapshot();
            let region_count = snapshot
                .read()
                .map(|guard| guard.get_region_count())
                .unwrap_or(0);
            if region_count == 0 {
                log::error!("Snapshot is empty. Run a New Scan (build snapshot) before scanning.");
                return ElementScanResponse { trackable_task_handle: None };
            }

            let repeat_delay_ms = ScanSettingsConfig::get_repeat_scan_delay_ms();
            if repeat_delay_ms > 0 {
                thread::sleep(std::time::Duration::from_millis(repeat_delay_ms));
            }

            let fast_scan_enabled = ScanSettingsConfig::get_fast_scan_enabled();
            let fast_scan_alignment = ScanSettingsConfig::get_fast_scan_alignment();
            let fast_scan_last_digits = ScanSettingsConfig::get_fast_scan_last_digits();
            let explicit_alignment = ScanSettingsConfig::get_memory_alignment();
            let symbol_registry = SymbolRegistry::get_instance();
            let alignment = match explicit_alignment {
                Some(alignment) => alignment,
                None => {
                    if fast_scan_enabled {
                        if let Some(fast_alignment) = fast_scan_alignment {
                            fast_alignment
                        } else if fast_scan_last_digits.is_some() {
                            MemoryAlignment::Alignment16
                        } else {
                            let mut size: Option<i32> = None;
                            let mut mixed_sizes = false;
                            for data_type_ref in &self.data_type_refs {
                                if let Some(unit_size) = symbol_registry.get_unit_size_in_bytes(data_type_ref).try_into().ok() {
                                    if let Some(existing) = size {
                                        if existing != unit_size {
                                            mixed_sizes = true;
                                            break;
                                        }
                                    } else {
                                        size = Some(unit_size);
                                    }
                                }
                            }

                            if mixed_sizes {
                                MemoryAlignment::Alignment1
                            } else {
                                size.map(MemoryAlignment::from).unwrap_or(MemoryAlignment::Alignment1)
                            }
                        }
                    } else {
                        MemoryAlignment::Alignment1
                    }
                }
            };
            let floating_point_tolerance = ScanSettingsConfig::get_floating_point_tolerance();
            let memory_read_mode = ScanSettingsConfig::get_memory_read_mode();
            let is_single_thread_scan = ScanSettingsConfig::get_is_single_threaded_scan();
            let debug_perform_validation_scan = ScanSettingsConfig::get_debug_perform_validation_scan();

            // Deanonymize all scan constraints against all data types.
            // For example, an immediate comparison of >= 23 could end up being a byte, float, etc.
            let scan_constraints_by_data_type: HashMap<_, _> = self
                .data_type_refs
                .iter()
                .map(|data_type_ref| {
                    // Deanonymize the initial anonymous scan constraints against the current data type.
                    let scan_constraints = self
                        .scan_constraints
                        .iter()
                        .filter_map(|anonymous_scan_constraint| anonymous_scan_constraint.deanonymize_constraint(data_type_ref, floating_point_tolerance))
                        .collect();

                    // Optimize the scan constraints by running them through each parameter rule sequentially.
                    let scan_constraints_finalized: Vec<ScanConstraintFinalized> = ElementScanRuleRegistry::get_instance()
                        .get_scan_parameters_rule_registry()
                        .iter()
                        .fold(scan_constraints, |mut scan_constraint, (_id, scan_parameter_rule)| {
                            scan_parameter_rule.map_parameters(&mut scan_constraint);
                            scan_constraint
                        })
                        .into_iter()
                        .map(|scan_constraint| ScanConstraintFinalized::new(scan_constraint))
                        .collect();

                    (data_type_ref.clone(), scan_constraints_finalized)
                })
                .collect();

            if scan_constraints_by_data_type
                .values()
                .all(|constraints| constraints.is_empty())
            {
                log::error!("No valid scan constraints after parsing; aborting scan.");
                return ElementScanResponse { trackable_task_handle: None };
            }

            let element_scan_plan = ElementScanPlan::new(
                scan_constraints_by_data_type,
                alignment,
                floating_point_tolerance,
                memory_read_mode,
                is_single_thread_scan,
                debug_perform_validation_scan,
            );

            // Start the task to perform the scan.
            let task = ElementScanExecutorTask::start_task(process_info, snapshot, element_scan_plan, true);
            let task_handle = task.get_task_handle();
            let engine_privileged_state = engine_privileged_state.clone();

            engine_privileged_state
                .get_trackable_task_manager()
                .register_task(task.clone());

            thread::spawn(move || {
                task.wait_for_completion();
                engine_privileged_state
                    .get_trackable_task_manager()
                    .unregister_task(&task.get_task_identifier());
                engine_privileged_state.emit_event(ScanResultsUpdatedEvent { is_new_scan: false });
            });

            ElementScanResponse {
                trackable_task_handle: Some(task_handle),
            }
        } else {
            log::error!("No opened process");
            ElementScanResponse { trackable_task_handle: None }
        }
    }
}
