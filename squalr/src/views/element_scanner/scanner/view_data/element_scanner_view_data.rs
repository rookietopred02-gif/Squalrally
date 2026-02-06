use crate::views::element_scanner::scanner::{
    element_scanner_view_state::ElementScannerViewState, view_data::element_scanner_value_view_data::ElementScannerValueViewData,
};
use squalr_engine_api::{
    commands::{
        privileged_command_request::PrivilegedCommandRequest,
        scan::{
            collect_values::scan_collect_values_request::ScanCollectValuesRequest, element_scan::element_scan_request::ElementScanRequest,
            new::scan_new_request::ScanNewRequest,
        },
        trackable_tasks::cancel::trackable_tasks_cancel_request::TrackableTasksCancelRequest,
    },
    dependency_injection::dependency::Dependency,
    engine::engine_unprivileged_state::EngineUnprivilegedState,
    events::scan_results::updated::scan_results_updated_event::ScanResultsUpdatedEvent,
    events::trackable_task::progress_changed::trackable_task_progress_changed_event::TrackableTaskProgressChangedEvent,
    registries::symbols::symbol_registry::SymbolRegistry,
    structures::{
        data_types::{built_in_types::i32::data_type_i32::DataTypeI32, data_type_ref::DataTypeRef},
        data_values::anonymous_value_string_format::AnonymousValueStringFormat,
        scanning::{
            comparisons::{scan_compare_type::ScanCompareType, scan_compare_type_immediate::ScanCompareTypeImmediate},
            constraints::anonymous_scan_constraint::AnonymousScanConstraint,
        },
    },
};
use std::{
    sync::{Arc, OnceLock},
    thread,
    time::Duration,
};

#[derive(Clone)]
pub struct ElementScannerViewData {
    pub selected_data_type: DataTypeRef,
    pub active_display_format: AnonymousValueStringFormat,
    pub view_state: ElementScannerViewState,
    pub scan_values_and_constraints: Vec<ElementScannerValueViewData>,
    pub scan_progress: f32,
    pub scan_task_id: Option<String>,
    pub last_error_message: Option<String>,
}

impl ElementScannerViewData {
    const MAX_CONSTRAINTS: usize = 5;
    const SCAN_TIMEOUT_MS: u64 = 30000;

    pub fn new() -> Self {
        Self {
            selected_data_type: DataTypeRef::new(DataTypeI32::get_data_type_id()),
            active_display_format: AnonymousValueStringFormat::Decimal,
            view_state: ElementScannerViewState::NoResults,
            scan_values_and_constraints: vec![ElementScannerValueViewData::new(Self::create_menu_id(0))],
            scan_progress: 0.0,
            scan_task_id: None,
            last_error_message: None,
        }
    }

    pub fn reset_scan(
        element_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let element_scanner_view_data_view_state = {
            match element_scanner_view_data.read("Element scanner view data reset scan") {
                Some(element_scanner_view_data) => element_scanner_view_data.view_state,
                None => return,
            }
        };

        match element_scanner_view_data_view_state {
            ElementScannerViewState::ScanInProgress => {
                return;
            }
            ElementScannerViewState::NoResults | ElementScannerViewState::HasResults => {}
        }

        // CE-style "New Scan": establish a fresh snapshot baseline instead of clearing to an empty snapshot.
        // This prevents follow-up operations (Collect Values / First Scan) from reading 0 bytes.
        let scan_new_request = ScanNewRequest {};
        scan_new_request.send(&engine_unprivileged_state, move |_scan_new_response| {
            if let Some(mut view_data) = element_scanner_view_data.write("Element scanner view data reset scan response") {
                view_data.view_state = ElementScannerViewState::NoResults;
                view_data.scan_progress = 0.0;
                view_data.scan_task_id = None;
                view_data.last_error_message = None;
            }
        });
    }

    pub fn collect_values(engine_unprivileged_state: Arc<EngineUnprivilegedState>) {
        // Ensure a snapshot baseline exists before collecting values.
        let engine_unprivileged_state_clone = engine_unprivileged_state.clone();
        let scan_new_request = ScanNewRequest {};
        scan_new_request.send(&engine_unprivileged_state, move |_scan_new_response| {
            let collect_values_request = ScanCollectValuesRequest {};
            collect_values_request.send(&engine_unprivileged_state_clone, |_scan_collect_values_response| {});
        });
    }

    pub fn poll_scan_state(
        element_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        static POLL_STARTED: OnceLock<()> = OnceLock::new();
        if POLL_STARTED.set(()).is_err() {
            return;
        }

        let element_scanner_view_data_clone = element_scanner_view_data.clone();
        engine_unprivileged_state.listen_for_engine_event::<TrackableTaskProgressChangedEvent>(move |event| {
            if let Some(mut view_data) = element_scanner_view_data_clone.write("Element scanner progress update") {
                let should_update = view_data
                    .scan_task_id
                    .as_ref()
                    .map(|task_id| task_id == &event.task_id)
                    .unwrap_or(false);

                if should_update {
                    view_data.scan_progress = event.progress;
                }
            }
        });

        engine_unprivileged_state.listen_for_engine_event::<ScanResultsUpdatedEvent>(move |scan_results_updated_event| {
            if scan_results_updated_event.is_new_scan {
                return;
            }

            if let Some(mut element_scanner_view_data) = element_scanner_view_data.write("Element scanner scan state update") {
                element_scanner_view_data.view_state = ElementScannerViewState::HasResults;
                element_scanner_view_data.scan_progress = 1.0;
                element_scanner_view_data.scan_task_id = None;
                element_scanner_view_data.last_error_message = None;
            }
        });
    }

    pub fn start_scan(
        element_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let element_scanner_view_data_view_state = {
            match element_scanner_view_data.read("Element scanner view data start scan") {
                Some(element_scanner_view_data) => element_scanner_view_data.view_state,
                None => return,
            }
        };

        match element_scanner_view_data_view_state {
            ElementScannerViewState::HasResults => {
                Self::start_next_scan(element_scanner_view_data, engine_unprivileged_state);
            }
            ElementScannerViewState::NoResults => {
                Self::new_scan(element_scanner_view_data, engine_unprivileged_state);
            }
            ElementScannerViewState::ScanInProgress => {
                log::error!("Cannot start a new scan while a scan is in progress.");
            }
        };
    }

    pub fn cancel_scan(
        element_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let task_id = match element_scanner_view_data.read("Element scanner cancel scan") {
            Some(view_data) => view_data.scan_task_id.clone(),
            None => None,
        };

        let Some(task_id) = task_id else {
            return;
        };

        let cancel_request = TrackableTasksCancelRequest { task_id };
        cancel_request.send(&engine_unprivileged_state, move |_response| {});

        if let Some(mut view_data) = element_scanner_view_data.try_write("Element scanner cancel scan update") {
            view_data.view_state = ElementScannerViewState::NoResults;
            view_data.scan_task_id = None;
            view_data.scan_progress = 0.0;
            view_data.last_error_message = Some("Scan canceled.".to_string());
        }
    }

    fn new_scan(
        element_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let engine_unprivileged_state_clone = engine_unprivileged_state.clone();
        let element_scanner_view_data = element_scanner_view_data.clone();
        let scan_new_request = ScanNewRequest {};

        // Start a new scan, and recurse to start the scan once the new scan is made.
        scan_new_request.send(&engine_unprivileged_state, move |_scan_new_response| {
            Self::start_next_scan(element_scanner_view_data, engine_unprivileged_state_clone);
        });
    }

    fn start_next_scan(
        element_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let element_scanner_view_data_clone = element_scanner_view_data.clone();
        let mut element_scanner_view_data = {
            match element_scanner_view_data.write("Element scanner view data start next scan") {
                Some(element_scanner_view_data) => element_scanner_view_data,
                None => return,
            }
        };
        let symbol_registry = SymbolRegistry::get_instance();
        let supported_formats = symbol_registry.get_supported_anonymous_value_string_formats(&element_scanner_view_data.selected_data_type);
        let default_format = symbol_registry.get_default_anonymous_value_string_format(&element_scanner_view_data.selected_data_type);

        if !supported_formats.contains(&element_scanner_view_data.active_display_format) {
            element_scanner_view_data.active_display_format = default_format;
        }

        for scan_value_and_constraint in element_scanner_view_data.scan_values_and_constraints.iter_mut() {
            if !supported_formats.contains(&scan_value_and_constraint.current_scan_value.get_anonymous_value_string_format()) {
                scan_value_and_constraint
                    .current_scan_value
                    .set_anonymous_value_string_format(default_format);
            }
        }

        let data_type_refs = vec![element_scanner_view_data.selected_data_type.clone()];
        let scan_constraints: Vec<AnonymousScanConstraint> = element_scanner_view_data
            .scan_values_and_constraints
            .iter_mut()
            .filter_map(|scan_value_and_constraint| {
                // Ensure the value format always matches the currently selected data type.
                if !supported_formats.contains(&scan_value_and_constraint.current_scan_value.get_anonymous_value_string_format()) {
                    scan_value_and_constraint
                        .current_scan_value
                        .set_anonymous_value_string_format(default_format);
                }

                match scan_value_and_constraint.selected_scan_compare_type {
                    ScanCompareType::Relative(_) => Some(AnonymousScanConstraint::new(scan_value_and_constraint.selected_scan_compare_type, None)),
                    _ => {
                        if scan_value_and_constraint
                            .current_scan_value
                            .get_anonymous_value_string()
                            .trim()
                            .is_empty()
                        {
                            None
                        } else {
                            Some(AnonymousScanConstraint::new(
                                scan_value_and_constraint.selected_scan_compare_type,
                                Some(scan_value_and_constraint.current_scan_value.clone()),
                            ))
                        }
                    }
                }
            })
            .collect();

        if scan_constraints.is_empty() {
            log::error!("No valid scan constraints provided.");
            if let Some(mut view_data) = element_scanner_view_data_clone.write("Element scanner view data scan constraint error") {
                view_data.last_error_message = Some("No valid scan constraints provided.".to_string());
            }
            return;
        }
        let element_scan_request = ElementScanRequest {
            scan_constraints,
            data_type_refs,
        };

        element_scanner_view_data.view_state = ElementScannerViewState::ScanInProgress;
        Self::schedule_scan_timeout(element_scanner_view_data_clone.clone(), engine_unprivileged_state.clone());
        element_scanner_view_data.scan_progress = 0.0;
        element_scanner_view_data.scan_task_id = None;
        element_scanner_view_data.last_error_message = None;

        drop(element_scanner_view_data);

        element_scan_request.send(&engine_unprivileged_state, move |scan_execute_response| {
            if let Some(task_handle) = scan_execute_response.trackable_task_handle.as_ref() {
                if let Some(mut view_data) = element_scanner_view_data_clone.write("Element scanner task handle") {
                    view_data.scan_task_id = Some(task_handle.task_identifier.clone());
                    view_data.scan_progress = task_handle.progress;
                }
            }

            if scan_execute_response.trackable_task_handle.is_none() {
                if let Some(mut element_scanner_view_data) =
                    element_scanner_view_data_clone.write("Element scanner view data start next scan response")
                {
                    element_scanner_view_data.view_state = ElementScannerViewState::NoResults;
                    element_scanner_view_data.scan_progress = 0.0;
                    element_scanner_view_data.scan_task_id = None;
                    element_scanner_view_data.last_error_message = Some("Scan failed (no process opened or invalid constraints).".to_string());
                }
            }
        });
    }

    fn schedule_scan_timeout(
        element_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(Self::SCAN_TIMEOUT_MS));
            let task_id = {
                let view_data = match element_scanner_view_data.read("Element scanner scan timeout read") {
                    Some(view_data) => view_data,
                    None => return,
                };

                if !matches!(view_data.view_state, ElementScannerViewState::ScanInProgress) {
                    return;
                }

                view_data.scan_task_id.clone()
            };

            if let Some(task_id) = task_id {
                let cancel_request = TrackableTasksCancelRequest { task_id };
                cancel_request.send(&engine_unprivileged_state, move |_response| {});
            }

            if let Some(mut view_data) = element_scanner_view_data.write("Element scanner scan timeout") {
                if matches!(view_data.view_state, ElementScannerViewState::ScanInProgress) {
                    view_data.view_state = ElementScannerViewState::NoResults;
                    view_data.scan_progress = 0.0;
                    view_data.scan_task_id = None;
                    view_data.last_error_message = Some("Scan timed out.".to_string());
                    log::warn!("Scan timed out. Resetting scan state.");
                }
            }
        });
    }

    pub fn add_constraint(element_scanner_view_data: Dependency<Self>) {
        let mut element_scanner_view_data = match element_scanner_view_data.write("Element scanner view data add constraint") {
            Some(element_scanner_view_data) => element_scanner_view_data,
            None => return,
        };

        let next_index = element_scanner_view_data.scan_values_and_constraints.len();
        let desired_format = element_scanner_view_data.active_display_format;

        if next_index >= Self::MAX_CONSTRAINTS {
            return;
        }

        // If creating the 2nd constraint, <= is the most common constraint, so default to that for a better UX.
        if next_index == 1 {
            let mut value_view_data = ElementScannerValueViewData {
                selected_scan_compare_type: ScanCompareType::Immediate(ScanCompareTypeImmediate::LessThanOrEqual),
                ..ElementScannerValueViewData::new(Self::create_menu_id(next_index))
            };
            value_view_data.current_scan_value.set_anonymous_value_string_format(desired_format);
            element_scanner_view_data.scan_values_and_constraints.push(value_view_data);
        } else {
            let mut value_view_data = ElementScannerValueViewData::new(Self::create_menu_id(next_index));
            value_view_data.current_scan_value.set_anonymous_value_string_format(desired_format);
            element_scanner_view_data.scan_values_and_constraints.push(value_view_data);
        }
    }

    pub fn remove_constraint(
        element_scanner_view_data: Dependency<Self>,
        index: usize,
    ) {
        let mut element_scanner_view_data = match element_scanner_view_data.write("Element scanner view data remove constraint") {
            Some(element_scanner_view_data) => element_scanner_view_data,
            None => return,
        };

        if index <= 0 {
            return;
        }

        element_scanner_view_data
            .scan_values_and_constraints
            .remove(index);
    }

    fn create_menu_id(index: usize) -> String {
        format!("element_scanner_data_type_selector_{}", index)
    }
}
