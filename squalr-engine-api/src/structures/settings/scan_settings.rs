use crate::structures::memory::memory_alignment::MemoryAlignment;
use crate::structures::settings::scan_thread_priority::ScanThreadPriority;
use crate::structures::{data_types::floating_point_tolerance::FloatingPointTolerance, scanning::memory_read_mode::MemoryReadMode};
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::fmt;

#[derive(Copy, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ScanSettings {
    pub scan_buffer_kb: u32,
    pub thread_priority: ScanThreadPriority,
    pub fast_scan_enabled: bool,
    pub fast_scan_alignment: Option<MemoryAlignment>,
    pub fast_scan_last_digits: Option<u8>,
    pub pause_while_scanning: bool,
    pub repeat_scan_delay_ms: u64,
    pub results_page_size_auto: bool,
    pub results_page_size_max: u32,
    pub results_page_size: u32,
    pub results_read_interval_ms: u64,
    pub project_read_interval_ms: u64,
    pub freeze_interval_ms: u64,
    pub memory_alignment: Option<MemoryAlignment>,
    pub memory_read_mode: MemoryReadMode,
    pub floating_point_tolerance: FloatingPointTolerance,
    pub is_single_threaded_scan: bool,
    pub debug_perform_validation_scan: bool,
}

impl fmt::Debug for ScanSettings {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match to_string_pretty(&self) {
            Ok(json) => write!(formatter, "Settings for scan: {}", json),
            Err(_) => write!(formatter, "Scan config {{ could not serialize to JSON }}"),
        }
    }
}

impl Default for ScanSettings {
    fn default() -> Self {
        Self {
            scan_buffer_kb: 2048,
            thread_priority: ScanThreadPriority::Normal,
            fast_scan_enabled: true,
            fast_scan_alignment: None,
            fast_scan_last_digits: None,
            pause_while_scanning: false,
            repeat_scan_delay_ms: 0,
            results_page_size_auto: true,
            results_page_size_max: 1_000_000,
            results_page_size: 1_000_000,
            results_read_interval_ms: 200,
            project_read_interval_ms: 200,
            freeze_interval_ms: 50,
            memory_alignment: None,
            floating_point_tolerance: FloatingPointTolerance::default(),
            // Reading interleaved avoids a dedicated full-pass value collection step which can stall the UI on large scans.
            memory_read_mode: MemoryReadMode::ReadInterleavedWithScan,
            is_single_threaded_scan: false,
            debug_perform_validation_scan: false,
        }
    }
}
