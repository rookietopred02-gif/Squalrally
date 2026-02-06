use crate::command_executors::privileged_request_executor::PrivilegedCommandRequestExecutor;
use crate::engine_privileged_state::EnginePrivilegedState;
use squalr_engine_api::commands::settings::scan::set::scan_settings_set_request::ScanSettingsSetRequest;
use squalr_engine_api::commands::settings::scan::set::scan_settings_set_response::ScanSettingsSetResponse;
use squalr_engine_scanning::scan_settings_config::ScanSettingsConfig;
use std::sync::Arc;

impl PrivilegedCommandRequestExecutor for ScanSettingsSetRequest {
    type ResponseType = ScanSettingsSetResponse;

    fn execute(
        &self,
        _engine_privileged_state: &Arc<EnginePrivilegedState>,
    ) -> <Self as PrivilegedCommandRequestExecutor>::ResponseType {
        if let Some(scan_buffer_kb) = self.scan_buffer_kb {
            ScanSettingsConfig::set_scan_buffer_kb(scan_buffer_kb);
        }

        if let Some(thread_priority) = self.thread_priority {
            ScanSettingsConfig::set_thread_priority(thread_priority);
        }

        if let Some(fast_scan_enabled) = self.fast_scan_enabled {
            ScanSettingsConfig::set_fast_scan_enabled(fast_scan_enabled);
        }

        if let Some(fast_scan_alignment) = self.fast_scan_alignment {
            ScanSettingsConfig::set_fast_scan_alignment(Some(fast_scan_alignment));
        }

        if let Some(clear_fast_scan_alignment) = self.clear_fast_scan_alignment {
            if clear_fast_scan_alignment {
                ScanSettingsConfig::set_fast_scan_alignment(None);
            }
        }

        if let Some(fast_scan_last_digits) = self.fast_scan_last_digits {
            ScanSettingsConfig::set_fast_scan_last_digits(Some(fast_scan_last_digits));
        }

        if let Some(clear_fast_scan_last_digits) = self.clear_fast_scan_last_digits {
            if clear_fast_scan_last_digits {
                ScanSettingsConfig::set_fast_scan_last_digits(None);
            }
        }

        if let Some(clear_memory_alignment) = self.clear_memory_alignment {
            if clear_memory_alignment {
                ScanSettingsConfig::set_memory_alignment(None);
            }
        }

        if let Some(pause_while_scanning) = self.pause_while_scanning {
            ScanSettingsConfig::set_pause_while_scanning(pause_while_scanning);
        }

        if let Some(repeat_scan_delay_ms) = self.repeat_scan_delay_ms {
            ScanSettingsConfig::set_repeat_scan_delay_ms(repeat_scan_delay_ms);
        }

        if let Some(results_page_size_auto) = self.results_page_size_auto {
            ScanSettingsConfig::set_results_page_size_auto(results_page_size_auto);
        }

        if let Some(results_page_size_max) = self.results_page_size_max {
            ScanSettingsConfig::set_results_page_size_max(results_page_size_max);
        }

        if let Some(results_page_size) = self.results_page_size {
            ScanSettingsConfig::set_results_page_size(results_page_size);
        }

        if let Some(results_read_interval_ms) = self.results_read_interval_ms {
            ScanSettingsConfig::set_results_read_interval_ms(results_read_interval_ms);
        }

        if let Some(project_read_interval_ms) = self.project_read_interval_ms {
            ScanSettingsConfig::set_project_read_interval_ms(project_read_interval_ms);
        }

        if let Some(freeze_interval_ms) = self.freeze_interval_ms {
            ScanSettingsConfig::set_freeze_interval_ms(freeze_interval_ms);
        }

        if let Some(memory_alignment) = self.memory_alignment {
            ScanSettingsConfig::set_memory_alignment(Some(memory_alignment));
        }

        if let Some(memory_read_mode) = self.memory_read_mode {
            ScanSettingsConfig::set_memory_read_mode(memory_read_mode);
        }

        if let Some(floating_point_tolerance) = self.floating_point_tolerance {
            ScanSettingsConfig::set_floating_point_tolerance(floating_point_tolerance);
        }

        if let Some(is_single_threaded_scan) = self.is_single_threaded_scan {
            ScanSettingsConfig::set_is_single_threaded_scan(is_single_threaded_scan);
        }

        if let Some(debug_perform_validation_scan) = self.debug_perform_validation_scan {
            ScanSettingsConfig::set_debug_perform_validation_scan(debug_perform_validation_scan);
        }

        ScanSettingsSetResponse {}
    }
}
