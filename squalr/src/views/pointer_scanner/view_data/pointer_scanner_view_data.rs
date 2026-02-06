use squalr_engine_api::commands::pointer_scan_results::query::pointer_scan_results_query_request::PointerScanResultsQueryRequest;
use squalr_engine_api::commands::scan::pointer_scan::pointer_scan_request::PointerScanRequest;
use squalr_engine_api::commands::privileged_command_request::PrivilegedCommandRequest;
use squalr_engine_api::commands::trackable_tasks::cancel::trackable_tasks_cancel_request::TrackableTasksCancelRequest;
use squalr_engine_api::dependency_injection::dependency::Dependency;
use squalr_engine_api::engine::engine_unprivileged_state::EngineUnprivilegedState;
use squalr_engine_api::events::pointer_scan_results::updated::pointer_scan_results_updated_event::PointerScanResultsUpdatedEvent;
use squalr_engine_api::events::trackable_task::progress_changed::trackable_task_progress_changed_event::TrackableTaskProgressChangedEvent;
use squalr_engine_api::structures::data_types::built_in_types::u64::data_type_u64::DataTypeU64;
use squalr_engine_api::structures::data_types::data_type_ref::DataTypeRef;
use squalr_engine_api::structures::data_values::anonymous_value_string::AnonymousValueString;
use squalr_engine_api::structures::data_values::anonymous_value_string_format::AnonymousValueStringFormat;
use squalr_engine_api::structures::data_values::container_type::ContainerType;
use squalr_engine_api::structures::pointer_scan::pointer_scan_result::PointerScanResult;
use std::ops::RangeInclusive;
use std::sync::Arc;

#[derive(Clone)]
pub struct PointerScannerViewData {
    pub target_address: String,
    pub pointer_data_type: DataTypeRef,
    pub max_depth_text: String,
    pub offset_size_text: String,
    pub scan_statics: bool,
    pub scan_heaps: bool,
    pub current_results: Vec<PointerScanResult>,
    pub current_page_index: u64,
    pub last_page_index: u64,
    pub page_size: u64,
    pub result_count: u64,
    pub stats_string: String,
    pub is_querying_results: bool,
    pub is_scanning: bool,
    pub progress: f32,
    pub current_task_id: Option<String>,
    pub selection_index_start: Option<i32>,
    pub selection_index_end: Option<i32>,
}

impl PointerScannerViewData {
    pub fn new() -> Self {
        Self {
            target_address: String::new(),
            pointer_data_type: DataTypeRef::new(DataTypeU64::get_data_type_id()),
            max_depth_text: "3".to_string(),
            offset_size_text: "512".to_string(),
            scan_statics: true,
            scan_heaps: true,
            current_results: Vec::new(),
            current_page_index: 0,
            last_page_index: 0,
            page_size: 0,
            result_count: 0,
            stats_string: String::new(),
            is_querying_results: false,
            is_scanning: false,
            progress: 0.0,
            current_task_id: None,
            selection_index_start: None,
            selection_index_end: None,
        }
    }

    pub fn poll_results(
        pointer_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let engine_unprivileged_state_clone = engine_unprivileged_state.clone();
        let pointer_scanner_view_data_clone = pointer_scanner_view_data.clone();

        engine_unprivileged_state.listen_for_engine_event::<PointerScanResultsUpdatedEvent>(move |_event| {
            Self::query_results(pointer_scanner_view_data_clone.clone(), engine_unprivileged_state_clone.clone());
        });

        let _engine_unprivileged_state_clone = engine_unprivileged_state.clone();
        let pointer_scanner_view_data_clone = pointer_scanner_view_data.clone();

        engine_unprivileged_state.listen_for_engine_event::<TrackableTaskProgressChangedEvent>(move |event| {
            if let Some(mut view_data) = pointer_scanner_view_data_clone.write("Pointer scan progress event") {
                let should_update = view_data
                    .current_task_id
                    .as_ref()
                    .map(|task_id| task_id == &event.task_id)
                    .unwrap_or(false);

                if should_update {
                    view_data.progress = event.progress;
                    if event.progress >= 1.0 {
                        view_data.is_scanning = false;
                    }
                }
            }
        });
    }

    pub fn start_scan(
        pointer_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let (target_address, pointer_data_type, max_depth, offset_size, scan_statics, scan_heaps) = {
            let mut view_data = match pointer_scanner_view_data.write("Pointer scanner start scan") {
                Some(view_data) => view_data,
                None => return,
            };

            let max_depth = view_data.max_depth_text.parse::<u64>().unwrap_or(3);
            let offset_size = view_data.offset_size_text.parse::<u64>().unwrap_or(512);

            view_data.is_scanning = true;
            view_data.progress = 0.0;
            view_data.current_results.clear();
            view_data.current_page_index = 0;
            view_data.last_page_index = 0;
            view_data.result_count = 0;
            view_data.stats_string.clear();
            view_data.selection_index_start = None;
            view_data.selection_index_end = None;

            (
                view_data.target_address.clone(),
                view_data.pointer_data_type.clone(),
                max_depth,
                offset_size,
                view_data.scan_statics,
                view_data.scan_heaps,
            )
        };

        let format = if target_address.trim_start().starts_with("0x") {
            AnonymousValueStringFormat::Hexadecimal
        } else {
            AnonymousValueStringFormat::Decimal
        };

        let pointer_scan_request = PointerScanRequest {
            target_address: AnonymousValueString::new(target_address, format, ContainerType::None),
            pointer_data_type_ref: pointer_data_type,
            max_depth,
            offset_size,
            scan_statics,
            scan_heaps,
        };

        let pointer_scanner_view_data_clone = pointer_scanner_view_data.clone();
        pointer_scan_request.send(&engine_unprivileged_state, move |response| {
            if let Some(mut view_data) = pointer_scanner_view_data_clone.write("Pointer scan start response") {
                view_data.current_task_id = response
                    .trackable_task_handle
                    .as_ref()
                    .map(|handle| handle.task_identifier.clone());
            }
        });
    }

    pub fn cancel_scan(
        pointer_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let task_id = match pointer_scanner_view_data.read("Pointer scan cancel read") {
            Some(view_data) => view_data.current_task_id.clone(),
            None => None,
        };

        let Some(task_id) = task_id else {
            return;
        };

        let cancel_request = TrackableTasksCancelRequest { task_id };
        let pointer_scanner_view_data_clone = pointer_scanner_view_data.clone();

        cancel_request.send(&engine_unprivileged_state, move |_response| {
            if let Some(mut view_data) = pointer_scanner_view_data_clone.write("Pointer scan cancel response") {
                view_data.is_scanning = false;
                view_data.progress = 0.0;
                view_data.current_task_id = None;
            }
        });
    }

    pub fn navigate_first_page(
        pointer_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        Self::set_page_index(pointer_scanner_view_data, engine_unprivileged_state, 0);
    }

    pub fn navigate_last_page(
        pointer_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let last_page_index = match pointer_scanner_view_data.read("Pointer scan last page") {
            Some(view_data) => view_data.last_page_index,
            None => return,
        };

        Self::set_page_index(pointer_scanner_view_data, engine_unprivileged_state, last_page_index);
    }

    pub fn navigate_previous_page(
        pointer_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let current_page_index = match pointer_scanner_view_data.read("Pointer scan previous page") {
            Some(view_data) => view_data.current_page_index,
            None => return,
        };

        Self::set_page_index(pointer_scanner_view_data, engine_unprivileged_state, current_page_index.saturating_sub(1));
    }

    pub fn navigate_next_page(
        pointer_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let current_page_index = match pointer_scanner_view_data.read("Pointer scan next page") {
            Some(view_data) => view_data.current_page_index,
            None => return,
        };

        Self::set_page_index(pointer_scanner_view_data, engine_unprivileged_state, current_page_index.saturating_add(1));
    }

    pub fn set_page_index(
        pointer_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        new_page_index: u64,
    ) {
        let mut view_data = match pointer_scanner_view_data.write("Pointer scan set page index") {
            Some(view_data) => view_data,
            None => return,
        };

        let bounded_page_index = new_page_index.clamp(0, view_data.last_page_index);
        view_data.current_page_index = bounded_page_index;
        view_data.selection_index_start = None;
        view_data.selection_index_end = None;

        drop(view_data);

        Self::query_results(pointer_scanner_view_data, engine_unprivileged_state);
    }

    pub fn query_results(
        pointer_scanner_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        if pointer_scanner_view_data
            .read("Pointer scan query check")
            .map(|view_data| view_data.is_querying_results)
            .unwrap_or(false)
        {
            return;
        }

        let (page_index, pointer_scanner_view_data_clone) = {
            let mut view_data = match pointer_scanner_view_data.write("Pointer scan query") {
                Some(view_data) => view_data,
                None => return,
            };
            view_data.is_querying_results = true;
            (view_data.current_page_index, pointer_scanner_view_data.clone())
        };

        let pointer_scan_results_query_request = PointerScanResultsQueryRequest { page_index };

        pointer_scan_results_query_request.send(&engine_unprivileged_state, move |response| {
            if let Some(mut view_data) = pointer_scanner_view_data_clone.write("Pointer scan query response") {
                view_data.is_querying_results = false;
                view_data.current_results = response.results;
                view_data.page_size = response.page_size;
                view_data.result_count = response.result_count;
                view_data.last_page_index = response.last_page_index;
                view_data.stats_string = format!(
                    "Results: {} (Page {}/{})",
                    response.result_count,
                    response.page_index + 1,
                    response.last_page_index + 1
                );
            }
        });
    }

    pub fn set_selection_start(
        pointer_scanner_view_data: Dependency<Self>,
        index: Option<i32>,
    ) {
        if let Some(mut view_data) = pointer_scanner_view_data.write("Pointer scan select start") {
            view_data.selection_index_start = index;
            view_data.selection_index_end = None;
        }
    }

    pub fn set_selection_end(
        pointer_scanner_view_data: Dependency<Self>,
        index: Option<i32>,
    ) {
        if let Some(mut view_data) = pointer_scanner_view_data.write("Pointer scan select end") {
            view_data.selection_index_end = index;
        }
    }

    pub fn select_all(pointer_scanner_view_data: Dependency<Self>) {
        if let Some(mut view_data) = pointer_scanner_view_data.write("Pointer scan select all") {
            if view_data.current_results.is_empty() {
                view_data.selection_index_start = None;
                view_data.selection_index_end = None;
                return;
            }

            view_data.selection_index_start = Some(0);
            view_data.selection_index_end = Some(view_data.current_results.len().saturating_sub(1) as i32);
        }
    }

    pub fn copy_selected_results(pointer_scanner_view_data: Dependency<Self>) -> String {
        let view_data = match pointer_scanner_view_data.read("Pointer scan copy selection") {
            Some(view_data) => view_data,
            None => return String::new(),
        };

        let Some(range) = Self::get_selected_results_range(&view_data) else {
            return String::new();
        };

        view_data
            .current_results
            .iter()
            .enumerate()
            .filter(|(index, _)| range.contains(index))
            .map(|(_, result)| {
                let base = if result.is_module() {
                    format!("{}+{:X}", result.get_module_name(), result.get_module_offset())
                } else {
                    format!("{:016X}", result.get_base_address())
                };
                let offsets = result
                    .get_offsets()
                    .iter()
                    .map(|offset| format!("{:X}", offset))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} -> [{}]", base, offsets)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn get_selected_results_range(view_data: &PointerScannerViewData) -> Option<RangeInclusive<usize>> {
        let start = view_data
            .selection_index_start
            .or(view_data.selection_index_end)?;
        let end = view_data
            .selection_index_end
            .or(view_data.selection_index_start)?;
        let (range_low, range_high) = (start.min(end), start.max(end));

        Some(range_low.max(0) as usize..=range_high.max(0) as usize)
    }
}
