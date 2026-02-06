use arc_swap::Guard;
use squalr_engine_api::commands::scan_results::add_to_project::scan_results_add_to_project_request::ScanResultsAddToProjectRequest;
use squalr_engine_api::commands::scan_results::delete::scan_results_delete_request::ScanResultsDeleteRequest;
use squalr_engine_api::commands::scan_results::freeze::scan_results_freeze_request::ScanResultsFreezeRequest;
use squalr_engine_api::conversions::storage_size_conversions::StorageSizeConversions;
use squalr_engine_api::dependency_injection::dependency::Dependency;
use squalr_engine_api::dependency_injection::write_guard::WriteGuard;
use squalr_engine_api::engine::engine_unprivileged_state::EngineUnprivilegedState;
use squalr_engine_api::structures::data_values::anonymous_value_string_format::AnonymousValueStringFormat;
use squalr_engine_api::structures::data_values::container_type::ContainerType;
use squalr_engine_api::structures::scan_results::scan_result_base::ScanResultBase;
use squalr_engine_api::structures::scan_results::scan_result_ref::ScanResultRef;
use squalr_engine_api::{
    commands::{
        privileged_command_request::PrivilegedCommandRequest,
        scan_results::{
            query::scan_results_query_request::ScanResultsQueryRequest, refresh::scan_results_refresh_request::ScanResultsRefreshRequest,
            set_property::scan_results_set_property_request::ScanResultsSetPropertyRequest,
        },
    },
    events::scan_results::updated::scan_results_updated_event::ScanResultsUpdatedEvent,
    structures::{data_values::anonymous_value_string::AnonymousValueString, scan_results::scan_result::ScanResult},
};
use std::ops::RangeInclusive;
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::views::struct_viewer::view_data::struct_viewer_view_data::StructViewerViewData;
use crate::ui::converters::data_type_to_string_converter::DataTypeToStringConverter;
use crate::views::element_scanner::results::view_data::element_scanner_result_frame_action::ElementScannerResultFrameAction;

#[derive(Clone)]
pub struct ElementScannerResultsViewData {
    // audio_player: AudioPlayer,
    pub value_splitter_ratio: f32,
    pub previous_value_splitter_ratio: f32,
    pub current_scan_results: Arc<Vec<ScanResult>>,
    pub current_page_index: u64,
    pub cached_last_page_index: u64,
    pub last_page_size: u64,
    pub page_size_override: Option<u32>,
    pub last_queried_page_size_override: Option<u32>,
    pub last_page_size_override_change: Option<Instant>,
    pub selection_index_start: Option<i32>,
    pub selection_index_end: Option<i32>,
    pub result_count: u64,
    pub stats_string: String,
    pub current_display_string: AnonymousValueString,
    pub is_querying_scan_results: bool,
    pub is_refreshing_scan_results: bool,
    pub is_setting_properties: bool,
    pub is_freezing_entries: bool,
    pub show_change_value_dialog: bool,
    pub change_value_string: AnonymousValueString,
    pub pending_frame_action: ElementScannerResultFrameAction,
}

impl ElementScannerResultsViewData {
    pub const DEFAULT_VALUE_SPLITTER_RATIO: f32 = 0.35;
    pub const DEFAULT_PREVIOUS_VALUE_SPLITTER_RATIO: f32 = 0.70;
    const AUTO_REFRESH_INTERVAL_MS: u64 = 750;
    const AUTO_REFRESH_MAX_RESULTS_PER_PAGE: usize = 512;
    const PAGE_SIZE_REQUERY_DEBOUNCE_MS: u64 = 200;

    pub fn new() -> Self {
        Self {
            value_splitter_ratio: Self::DEFAULT_VALUE_SPLITTER_RATIO,
            previous_value_splitter_ratio: Self::DEFAULT_PREVIOUS_VALUE_SPLITTER_RATIO,
            current_scan_results: Arc::new(Vec::new()),
            current_page_index: 0,
            cached_last_page_index: 0,
            last_page_size: 1,
            page_size_override: None,
            last_queried_page_size_override: None,
            last_page_size_override_change: None,
            selection_index_start: None,
            selection_index_end: None,
            result_count: 0,
            stats_string: String::new(),
            current_display_string: AnonymousValueString::new(String::new(), AnonymousValueStringFormat::Decimal, ContainerType::None),
            is_querying_scan_results: false,
            is_refreshing_scan_results: false,
            is_setting_properties: false,
            is_freezing_entries: false,
            show_change_value_dialog: false,
            change_value_string: AnonymousValueString::new(String::new(), AnonymousValueStringFormat::Decimal, ContainerType::None),
            pending_frame_action: ElementScannerResultFrameAction::None,
        }
    }

    pub fn select_all(element_scanner_results_view_data: Dependency<Self>) {
        if let Some(mut element_scanner_results_view_data) = element_scanner_results_view_data.write("Element scanner select all") {
            if element_scanner_results_view_data.current_scan_results.is_empty() {
                element_scanner_results_view_data.selection_index_start = None;
                element_scanner_results_view_data.selection_index_end = None;
                return;
            }

                element_scanner_results_view_data.selection_index_start = Some(0);
                element_scanner_results_view_data.selection_index_end =
                    Some(element_scanner_results_view_data.current_scan_results.len().saturating_sub(1) as i32);
        }
    }

    pub fn copy_selected_addresses(element_scanner_results_view_data: Dependency<Self>) -> String {
        let element_scanner_results_view_data = match element_scanner_results_view_data.read("Element scanner copy selected addresses") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return String::new(),
        };

        let Some(range) = Self::get_selected_results_range(&element_scanner_results_view_data) else {
            return String::new();
        };

        range
            .filter_map(|index| element_scanner_results_view_data.current_scan_results.get(index))
            .map(|scan_result| {
                let address = scan_result.get_address();
                if scan_result.is_module() {
                    format!("{}+{:X}", scan_result.get_module(), scan_result.get_module_offset())
                } else if address <= u32::MAX as u64 {
                    format!("{:08X}", address)
                } else {
                    format!("{:016X}", address)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn copy_selected_rows_tsv(
        element_scanner_results_view_data: Dependency<Self>,
        active_display_format: AnonymousValueStringFormat,
    ) -> String {
        let element_scanner_results_view_data = match element_scanner_results_view_data.read("Element scanner copy selected rows") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return String::new(),
        };

        let Some(range) = Self::get_selected_results_range(&element_scanner_results_view_data) else {
            return String::new();
        };

        range
            .filter_map(|index| element_scanner_results_view_data.current_scan_results.get(index))
            .map(|scan_result| {
                let address = scan_result.get_address();
                let address_string = if scan_result.is_module() {
                    format!("{}+{:X}", scan_result.get_module(), scan_result.get_module_offset())
                } else if address <= u32::MAX as u64 {
                    format!("{:08X}", address)
                } else {
                    format!("{:016X}", address)
                };

                let current_value_string = scan_result
                    .get_recently_read_display_value(active_display_format)
                    .or_else(|| scan_result.get_current_display_value(active_display_format))
                    .map(|value| value.get_anonymous_value_string())
                    .unwrap_or("??");

                let previous_value_string = scan_result
                    .get_previous_display_value(active_display_format)
                    .map(|value| value.get_anonymous_value_string())
                    .unwrap_or("??");

                let type_string = DataTypeToStringConverter::convert_data_type_to_string(scan_result.get_data_type_ref().get_data_type_id());

                format!(
                    "{}\t{}\t{}\t{}",
                    address_string, current_value_string, previous_value_string, type_string
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn show_change_value_dialog(
        element_scanner_results_view_data: Dependency<Self>,
        seed_value: AnonymousValueString,
    ) {
        if let Some(mut element_scanner_results_view_data) = element_scanner_results_view_data.write("Element scanner show change value dialog") {
            element_scanner_results_view_data.change_value_string = seed_value;
            element_scanner_results_view_data.show_change_value_dialog = true;
        }
    }

    pub fn hide_change_value_dialog(element_scanner_results_view_data: Dependency<Self>) {
        if let Some(mut element_scanner_results_view_data) = element_scanner_results_view_data.write("Element scanner hide change value dialog") {
            element_scanner_results_view_data.show_change_value_dialog = false;
        }
    }

    pub fn poll_scan_results(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        static POLL_STARTED: OnceLock<()> = OnceLock::new();
        if POLL_STARTED.set(()).is_err() {
            return;
        }

        let engine_unprivileged_state_clone = engine_unprivileged_state.clone();
        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();

        // Requery all scan results if they update.
        {
            engine_unprivileged_state.listen_for_engine_event::<ScanResultsUpdatedEvent>(move |scan_results_updated_event| {
                let element_scanner_results_view_data = element_scanner_results_view_data_clone.clone();
                let engine_unprivileged_state = engine_unprivileged_state_clone.clone();
                let play_sound = !scan_results_updated_event.is_new_scan;

                Self::query_scan_results(element_scanner_results_view_data, engine_unprivileged_state, play_sound);
            });
        }

        let engine_unprivileged_state_clone = engine_unprivileged_state.clone();
        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();

        // Refresh scan values periodically (throttled).
        //
        // NOTE: This is disabled by default because background writers can block UI interactions
        // (e.g., clicking results) and cause Windows "App Hang" symptoms when the UI thread waits
        // on a contended dependency writer lock. Re-enable only for debugging:
        //   set SQUALR_ENABLE_SCAN_RESULT_AUTO_REFRESH=1
        if std::env::var_os("SQUALR_ENABLE_SCAN_RESULT_AUTO_REFRESH").is_some() {
            thread::spawn(move || {
                loop {
                    let should_refresh = element_scanner_results_view_data_clone
                        .read("Element scanner results auto refresh guard")
                        .map(|view_data| {
                            !view_data.is_querying_scan_results
                                && !view_data.is_refreshing_scan_results
                                && !view_data.current_scan_results.is_empty()
                                && view_data.current_scan_results.len() <= Self::AUTO_REFRESH_MAX_RESULTS_PER_PAGE
                        })
                        .unwrap_or(false);

                    if should_refresh {
                        let element_scanner_results_view_data = element_scanner_results_view_data_clone.clone();
                        let engine_unprivileged_state = engine_unprivileged_state_clone.clone();
                        Self::refresh_scan_results(element_scanner_results_view_data, engine_unprivileged_state);
                    }

                    thread::sleep(Duration::from_millis(Self::AUTO_REFRESH_INTERVAL_MS));
                }
            });
        }
    }

    pub fn navigate_first_page(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let new_page_index = 0;

        Self::set_page_index(element_scanner_results_view_data, engine_unprivileged_state, new_page_index);
    }

    pub fn navigate_last_page(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let cached_last_page_index = match element_scanner_results_view_data.read("Element scanner results navigation last") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data.cached_last_page_index,
            None => return,
        };
        let cached_last_page_index = cached_last_page_index;
        let new_page_index = cached_last_page_index;

        Self::set_page_index(element_scanner_results_view_data, engine_unprivileged_state, new_page_index);
    }

    pub fn navigate_previous_page(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();
        let element_scanner_results_view_data = match element_scanner_results_view_data.read("Element scanner results navigation previous") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return,
        };
        let new_page_index = Self::load_current_page_index(&element_scanner_results_view_data).saturating_sub(1);

        drop(element_scanner_results_view_data);

        Self::set_page_index(element_scanner_results_view_data_clone, engine_unprivileged_state, new_page_index);
    }

    pub fn navigate_next_page(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();
        let element_scanner_results_view_data = match element_scanner_results_view_data.read("Element scanner results navigation next") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return,
        };
        let new_page_index = Self::load_current_page_index(&element_scanner_results_view_data).saturating_add(1);

        drop(element_scanner_results_view_data);

        Self::set_page_index(element_scanner_results_view_data_clone, engine_unprivileged_state, new_page_index);
    }

    pub fn set_page_size_override(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        page_size_override: Option<u32>,
    ) {
        let now = Instant::now();
        let mut should_query = false;

        if let Some(mut view_data) = element_scanner_results_view_data.write("Set page size override") {
            let normalized_override = page_size_override.map(|value| value.max(1));
            if view_data.page_size_override != normalized_override {
                view_data.page_size_override = normalized_override;
                view_data.last_page_size_override_change = Some(now);
            }

            if view_data.is_querying_scan_results {
                return;
            }

            let debounce_elapsed = view_data
                .last_page_size_override_change
                .map(|changed_at| now.duration_since(changed_at) >= Duration::from_millis(Self::PAGE_SIZE_REQUERY_DEBOUNCE_MS))
                .unwrap_or(true);

            if debounce_elapsed && view_data.last_queried_page_size_override != view_data.page_size_override {
                view_data.last_queried_page_size_override = view_data.page_size_override;
                should_query = true;
            }
        }

        if should_query {
            Self::query_scan_results(element_scanner_results_view_data, engine_unprivileged_state, false);
        }
    }

    pub fn set_selected_scan_results_value(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        field_namespace: &str,
        anonymous_value_string: AnonymousValueString,
    ) {
        let scan_result_refs = Self::collect_selected_scan_result_refs(element_scanner_results_view_data.clone());

        if scan_result_refs.is_empty() {
            return;
        }

        let scan_results_set_property_request = ScanResultsSetPropertyRequest {
            scan_result_refs,
            field_namespace: field_namespace.to_string(),
            anonymous_value_string,
        };

        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();
        if let Some(mut element_scanner_results_view_data) = element_scanner_results_view_data.write("Set selected scan results") {
            element_scanner_results_view_data.is_setting_properties = true;
        }
        Self::schedule_flag_timeout(element_scanner_results_view_data.clone(), FlagType::SettingProperties, 5000);

        scan_results_set_property_request.send(&engine_unprivileged_state, move |_scan_results_set_property_response| {
            let mut element_scanner_results_view_data = match element_scanner_results_view_data_clone.write("Set selected scan results response") {
                Some(element_scanner_results_view_data) => element_scanner_results_view_data,
                None => return,
            };

            element_scanner_results_view_data.is_setting_properties = false;
        });
    }

    fn load_current_page_index(element_scanner_results_view_data: &Guard<Arc<ElementScannerResultsViewData>>) -> u64 {
        element_scanner_results_view_data
            .current_page_index
            .clamp(0, element_scanner_results_view_data.cached_last_page_index)
    }

    fn load_current_page_index_write(element_scanner_results_view_data: &WriteGuard<'_, ElementScannerResultsViewData>) -> u64 {
        element_scanner_results_view_data
            .current_page_index
            .clamp(0, element_scanner_results_view_data.cached_last_page_index)
    }

    fn query_scan_results(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        play_sound: bool,
    ) {
        if element_scanner_results_view_data
            .read("Query scan results")
            .map(|element_scanner_results_view_data| element_scanner_results_view_data.is_querying_scan_results)
            .unwrap_or(false)
        {
            return;
        }

        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();
        let mut element_scanner_results_view_data = match element_scanner_results_view_data.write("Query scan results") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return,
        };
        let page_index = Self::load_current_page_index_write(&element_scanner_results_view_data);
        let page_size = element_scanner_results_view_data.page_size_override;
        let scan_results_query_request = ScanResultsQueryRequest { page_index, page_size };

        element_scanner_results_view_data.is_querying_scan_results = true;
        Self::schedule_flag_timeout(element_scanner_results_view_data_clone.clone(), FlagType::QueryingResults, 5000);
        // Drop the write-guard before sending the request. The request may complete quickly and invoke the callback
        // synchronously, which would otherwise deadlock when it tries to acquire this same lock to update the UI.
        drop(element_scanner_results_view_data);

        scan_results_query_request.send(&engine_unprivileged_state, move |scan_results_query_response| {
            // let audio_player = &self.audio_player;
            let byte_size_in_metric = StorageSizeConversions::value_to_metric_size(scan_results_query_response.total_size_in_bytes as u128);
            let result_count = scan_results_query_response.result_count;

            if let Some(mut element_scanner_results_view_data) = element_scanner_results_view_data_clone.write("Query scan results response") {
                element_scanner_results_view_data.is_querying_scan_results = false;
                element_scanner_results_view_data.current_page_index = scan_results_query_response.page_index;
                element_scanner_results_view_data.cached_last_page_index = scan_results_query_response.last_page_index;
                element_scanner_results_view_data.last_page_size = scan_results_query_response.page_size.max(1);
                element_scanner_results_view_data.result_count = result_count;
                element_scanner_results_view_data.stats_string = format!("{} (Count: {})", byte_size_in_metric, result_count);
                element_scanner_results_view_data.current_scan_results = Arc::new(scan_results_query_response.scan_results);
            }

            if play_sound {
                if result_count > 0 {
                    // audio_player.play_sound(SoundType::Success);
                } else {
                    // audio_player.play_sound(SoundType::Warn);
                }
            }
        });
    }

    /// Fetches up-to-date values and module information for the current scan results, then updates the UI.
    fn refresh_scan_results(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        if element_scanner_results_view_data
            .read("Refresh scan results")
            .map(|element_scanner_results_view_data| {
                element_scanner_results_view_data.is_querying_scan_results || element_scanner_results_view_data.is_refreshing_scan_results
            })
            .unwrap_or(false)
        {
            return;
        }

        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();
        let mut element_scanner_results_view_data = match element_scanner_results_view_data.write("Refresh scan results") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return,
        };
        let engine_unprivileged_state = &engine_unprivileged_state;

        element_scanner_results_view_data.is_refreshing_scan_results = true;
        Self::schedule_flag_timeout(element_scanner_results_view_data_clone.clone(), FlagType::RefreshingResults, 5000);

        // Fire a request to get all scan result data needed for display.
        let scan_results_refresh_request = ScanResultsRefreshRequest {
            scan_result_refs: element_scanner_results_view_data
                .current_scan_results
                .iter()
                .map(|scan_result| scan_result.get_base_result().get_scan_result_ref().clone())
                .collect(),
        };

        // Drop to commit the write.
        drop(element_scanner_results_view_data);

        scan_results_refresh_request.send(engine_unprivileged_state, move |scan_results_refresh_response| {
            let mut element_scanner_results_view_data = match element_scanner_results_view_data_clone.write("Refresh scan results response") {
                Some(element_scanner_results_view_data) => element_scanner_results_view_data,
                None => return,
            };

            // Update UI with refreshed, full scan result values.
            element_scanner_results_view_data.is_refreshing_scan_results = false;
            element_scanner_results_view_data.current_scan_results = Arc::new(scan_results_refresh_response.scan_results);
        });
    }

    fn set_page_index(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        new_page_index: u64,
    ) {
        if element_scanner_results_view_data
            .read("Set page index")
            .map(|element_scanner_results_view_data| element_scanner_results_view_data.is_querying_scan_results)
            .unwrap_or(false)
        {
            return;
        }

        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();
        let mut element_scanner_results_view_data = match element_scanner_results_view_data.write("Set page index") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return,
        };
        let new_page_index = new_page_index.clamp(0, element_scanner_results_view_data.cached_last_page_index);

        // If the new index is the same as the current one, do nothing.
        if new_page_index == element_scanner_results_view_data.current_page_index {
            return;
        }

        element_scanner_results_view_data.current_page_index = new_page_index;

        // Clear out our selected items.
        element_scanner_results_view_data.selection_index_start = None;
        element_scanner_results_view_data.selection_index_end = None;

        // Drop to commit the write.
        drop(element_scanner_results_view_data);

        // Refresh scan results with the new page index. // JIRA: Should happen in the loop technically, but we need to make the MVVM bindings deadlock resistant.
        Self::query_scan_results(element_scanner_results_view_data_clone, engine_unprivileged_state, false);
    }

    pub fn set_page_index_string(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        new_page_index_text: &str,
    ) {
        // Extract numeric part from new_page_index_text and parse it to u64, defaulting to 0.
        let new_page_index = new_page_index_text
            .chars()
            .take_while(|char| char.is_digit(10))
            .collect::<String>()
            .parse::<u64>()
            .unwrap_or(0);

        Self::set_page_index(element_scanner_results_view_data, engine_unprivileged_state, new_page_index);
    }

    pub fn set_scan_result_selection_start(
        element_scanner_results_view_data: Dependency<Self>,
        _struct_viewer_view_data: Dependency<StructViewerViewData>,
        scan_result_collection_start_index: Option<i32>,
    ) -> bool {
        let mut element_scanner_results_view_data = match element_scanner_results_view_data.try_write("Set scan result selection start") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return false,
        };

        element_scanner_results_view_data.selection_index_start = scan_result_collection_start_index;
        element_scanner_results_view_data.selection_index_end = None;

        true
    }

    pub fn set_scan_result_selection_end(
        element_scanner_results_view_data: Dependency<Self>,
        _struct_viewer_view_data: Dependency<StructViewerViewData>,
        scan_result_collection_end_index: Option<i32>,
    ) -> bool {
        let mut element_scanner_results_view_data = match element_scanner_results_view_data.try_write("Set scan result selection end") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return false,
        };

        element_scanner_results_view_data.selection_index_end = scan_result_collection_end_index;

        true
    }

    pub fn add_scan_results_to_project(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let scan_result_refs = Self::collect_selected_scan_result_refs(element_scanner_results_view_data);

        if !scan_result_refs.is_empty() {
            let engine_unprivileged_state = &engine_unprivileged_state;
            let scan_results_add_to_project_request = ScanResultsAddToProjectRequest { scan_result_refs };

            scan_results_add_to_project_request.send(engine_unprivileged_state, |_response| {});
        }
    }

    pub fn delete_selected_scan_results(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let scan_result_refs = Self::collect_selected_scan_result_refs(element_scanner_results_view_data);

        if !scan_result_refs.is_empty() {
            let engine_unprivileged_state = &engine_unprivileged_state;
            let scan_results_delete_request = ScanResultsDeleteRequest { scan_result_refs };

            scan_results_delete_request.send(engine_unprivileged_state, |_response| {});
        }
    }

    pub fn set_scan_result_frozen(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        local_scan_result_index: i32,
        is_frozen: bool,
    ) {
        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();
        let local_scan_result_indices_vec = (local_scan_result_index..=local_scan_result_index).collect::<Vec<_>>();
        let scan_result_refs = Self::collect_scan_result_refs_by_indicies(element_scanner_results_view_data.clone(), &&local_scan_result_indices_vec);
        let should_send_freeze_request = !scan_result_refs.is_empty();
        let mut element_scanner_results_view_data = match element_scanner_results_view_data.write("Element scanner results view data: set scan result frozen") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return,
        };

        if element_scanner_results_view_data.is_freezing_entries {
            return;
        }

        if let Some(scan_result) =
            Arc::make_mut(&mut element_scanner_results_view_data.current_scan_results).get_mut(local_scan_result_index as usize)
        {
            scan_result.set_is_frozen_client_only(is_frozen);
        } else {
            log::warn!("Failed to find scan result to apply client side freeze at index: {}", local_scan_result_index)
        }

        if !should_send_freeze_request {
            // Nothing to toggle server-side. Avoid leaving the UI in a busy state.
            return;
        }

        element_scanner_results_view_data.is_freezing_entries = true;

        // Drop the write guard before sending the request. The request may complete quickly and invoke the callback
        // synchronously, which would otherwise deadlock when it tries to acquire this same lock to update the UI.
        drop(element_scanner_results_view_data);

        Self::schedule_flag_timeout(element_scanner_results_view_data_clone.clone(), FlagType::FreezingEntries, 5000);

        let engine_unprivileged_state = &engine_unprivileged_state;
        let scan_results_freeze_request = ScanResultsFreezeRequest { scan_result_refs, is_frozen };

        scan_results_freeze_request.send(engine_unprivileged_state, move |scan_results_freeze_response| {
            let mut element_scanner_results_view_data =
                match element_scanner_results_view_data_clone.write("Element scanner results view data: set scan result frozen response") {
                    Some(element_scanner_results_view_data) => element_scanner_results_view_data,
                    None => return,
                };

            // Revert failures by mapping global -> local, and revert to previous state.
            for failed_scan_result_ref in scan_results_freeze_response.failed_freeze_toggle_scan_result_refs {
                let global_index = failed_scan_result_ref.get_scan_result_global_index();

                if let Some(local_index) = Self::find_local_index_by_global_index(&element_scanner_results_view_data, global_index) {
                        if let Some(scan_result) = Arc::make_mut(&mut element_scanner_results_view_data.current_scan_results).get_mut(local_index) {
                            scan_result.set_is_frozen_client_only(!is_frozen);
                        }
                } else {
                    log::warn!("Failed to find scan result to revert client side freeze (global index: {})", global_index);
                }
            }

            element_scanner_results_view_data.is_freezing_entries = false;
        });
    }

    pub fn toggle_selected_scan_results_frozen(
        element_scanner_results_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        is_frozen: bool,
    ) {
        let element_scanner_results_view_data_clone = element_scanner_results_view_data.clone();
        let scan_result_refs = Self::collect_selected_scan_result_refs(element_scanner_results_view_data.clone());

        if scan_result_refs.is_empty() {
            return;
        }

        let mut element_scanner_results_view_data =
            match element_scanner_results_view_data.write("Element scanner results view data: set selected scan results frozen") {
                Some(element_scanner_results_view_data) => element_scanner_results_view_data,
                None => return,
            };

        if element_scanner_results_view_data.is_freezing_entries {
            return;
        }

        Self::for_each_selected_scan_result(&mut element_scanner_results_view_data, |scan_result| {
            scan_result.set_is_frozen_client_only(is_frozen);
        });

        element_scanner_results_view_data.is_freezing_entries = true;

        // Drop the write guard before sending the request. The request may complete quickly and invoke the callback
        // synchronously, which would otherwise deadlock when it tries to acquire this same lock to update the UI.
        drop(element_scanner_results_view_data);

        Self::schedule_flag_timeout(element_scanner_results_view_data_clone.clone(), FlagType::FreezingEntries, 5000);

        let engine_unprivileged_state = &engine_unprivileged_state;
        let scan_results_freeze_request = ScanResultsFreezeRequest { scan_result_refs, is_frozen };

        scan_results_freeze_request.send(engine_unprivileged_state, move |scan_results_freeze_response| {
            let mut element_scanner_results_view_data =
                match element_scanner_results_view_data_clone.write("Element scanner results view data: set selected scan results frozen response") {
                    Some(element_scanner_results_view_data) => element_scanner_results_view_data,
                    None => return,
                };

            // Revert failures by mapping global -> local, and revert to previous state.
            for failed_scan_result_ref in scan_results_freeze_response.failed_freeze_toggle_scan_result_refs {
                let global_index = failed_scan_result_ref.get_scan_result_global_index();

                if let Some(local_index) = Self::find_local_index_by_global_index(&element_scanner_results_view_data, global_index) {
                    if let Some(scan_result) =
                        Arc::make_mut(&mut element_scanner_results_view_data.current_scan_results).get_mut(local_index)
                    {
                        scan_result.set_is_frozen_client_only(!is_frozen);
                    }
                } else {
                    log::warn!("Failed to find scan result to revert client side freeze (global index: {})", global_index);
                }
            }

            element_scanner_results_view_data.is_freezing_entries = false;
        });
    }

    fn get_selected_results_range(element_scanner_results_view_data: &ElementScannerResultsViewData) -> Option<RangeInclusive<usize>> {
        let start = element_scanner_results_view_data
            .selection_index_start
            .or(element_scanner_results_view_data.selection_index_end)?;
        let end = element_scanner_results_view_data
            .selection_index_end
            .or(element_scanner_results_view_data.selection_index_start)?;
        let (range_low, range_high) = (start.min(end), start.max(end));

        Some(range_low.max(0) as usize..=range_high.max(0) as usize)
    }

    fn for_each_selected_scan_result(
        element_scanner_results_view_data: &mut ElementScannerResultsViewData,
        mut callback: impl FnMut(&mut ScanResult),
    ) {
        let Some(range) = Self::get_selected_results_range(element_scanner_results_view_data) else {
            return;
        };

        for index in range {
            if let Some(scan_result) = Arc::make_mut(&mut element_scanner_results_view_data.current_scan_results).get_mut(index) {
                callback(scan_result);
            }
        }
    }

    fn collect_selected_scan_result_refs(element_scanner_results_view_data: Dependency<Self>) -> Vec<ScanResultRef> {
        let element_scanner_results_view_data = match element_scanner_results_view_data.read("Collect selected scan result refs") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return Vec::new(),
        };

        let Some(range) = Self::get_selected_results_range(&element_scanner_results_view_data) else {
            return Vec::new();
        };

        range
            .filter_map(|index| {
                element_scanner_results_view_data
                    .current_scan_results
                    .get(index)
            })
            .map(|scan_result| scan_result.get_base_result().get_scan_result_ref().clone())
            .collect()
    }

    fn collect_scan_result_refs_by_indicies(
        element_scanner_results_view_data: Dependency<Self>,
        local_scan_result_indices: &[i32],
    ) -> Vec<ScanResultRef> {
        Self::collect_scan_result_bases_by_indicies(element_scanner_results_view_data, local_scan_result_indices)
            .into_iter()
            .map(|scan_result| scan_result.get_scan_result_ref().clone())
            .collect()
    }

    fn collect_scan_result_bases_by_indicies(
        element_scanner_results_view_data: Dependency<Self>,
        local_scan_result_indices: &[i32],
    ) -> Vec<ScanResultBase> {
        let element_scanner_results_view_data = match element_scanner_results_view_data.read("Collect scan result bases") {
            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
            None => return Vec::new(),
        };
        let scan_results = local_scan_result_indices
            .iter()
            .filter_map(|index| {
                element_scanner_results_view_data
                    .current_scan_results
                    .get(*index as usize)
                    .map(|scan_result| scan_result.get_base_result().clone())
            })
            .collect();

        scan_results
    }

    fn find_local_index_by_global_index(
        element_scanner_results_view_data: &ElementScannerResultsViewData,
        global_index: u64,
    ) -> Option<usize> {
        element_scanner_results_view_data
            .current_scan_results
            .iter()
            .position(|scan_result| {
                scan_result
                    .get_base_result()
                    .get_scan_result_ref()
                    .get_scan_result_global_index()
                    == global_index
            })
    }

    fn schedule_flag_timeout(
        element_scanner_results_view_data: Dependency<Self>,
        flag_type: FlagType,
        timeout_ms: u64,
    ) {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(timeout_ms));
            if let Some(mut view_data) = element_scanner_results_view_data.write("Element scanner results timeout") {
                match flag_type {
                    FlagType::QueryingResults => {
                        if view_data.is_querying_scan_results {
                            view_data.is_querying_scan_results = false;
                        }
                    }
                    FlagType::RefreshingResults => {
                        if view_data.is_refreshing_scan_results {
                            view_data.is_refreshing_scan_results = false;
                        }
                    }
                    FlagType::SettingProperties => {
                        if view_data.is_setting_properties {
                            view_data.is_setting_properties = false;
                        }
                    }
                    FlagType::FreezingEntries => {
                        if view_data.is_freezing_entries {
                            view_data.is_freezing_entries = false;
                        }
                    }
                }
            }
        });
    }
}

#[derive(Copy, Clone)]
enum FlagType {
    QueryingResults,
    RefreshingResults,
    SettingProperties,
    FreezingEntries,
}
