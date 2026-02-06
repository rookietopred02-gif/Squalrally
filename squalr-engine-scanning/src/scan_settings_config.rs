use serde_json::to_string_pretty;
use squalr_engine_api::structures::data_types::floating_point_tolerance::FloatingPointTolerance;
use squalr_engine_api::structures::memory::memory_alignment::MemoryAlignment;
use squalr_engine_api::structures::scanning::memory_read_mode::MemoryReadMode;
use squalr_engine_api::structures::settings::scan_settings::ScanSettings;
use squalr_engine_api::structures::settings::scan_thread_priority::ScanThreadPriority;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::sync::{Arc, RwLock};

pub struct ScanSettingsConfig {
    config: Arc<RwLock<ScanSettings>>,
    config_file: PathBuf,
}

impl ScanSettingsConfig {
    fn new() -> Self {
        let config_file = Self::default_config_path();
        let config = if config_file.exists() {
            match fs::read_to_string(&config_file) {
                Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
                Err(_) => ScanSettings::default(),
            }
        } else {
            ScanSettings::default()
        };

        Self {
            config: Arc::new(RwLock::new(config)),
            config_file,
        }
    }

    fn get_instance() -> &'static ScanSettingsConfig {
        static mut INSTANCE: Option<ScanSettingsConfig> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                let instance = ScanSettingsConfig::new();
                INSTANCE = Some(instance);
            });

            #[allow(static_mut_refs)]
            INSTANCE.as_ref().unwrap_unchecked()
        }
    }

    fn default_config_path() -> PathBuf {
        std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(&Path::new(""))
            .join("scan_settings.json")
    }

    fn save_config() {
        if let Ok(config) = Self::get_instance().config.read() {
            if let Ok(json) = to_string_pretty(&*config) {
                let _ = fs::write(&Self::get_instance().config_file, json);
            }
        }
    }

    pub fn get_full_config() -> &'static Arc<RwLock<ScanSettings>> {
        &Self::get_instance().config
    }

    pub fn get_results_page_size() -> u32 {
        if let Ok(config) = Self::get_instance().config.read() {
            config.results_page_size_max.max(1)
        } else {
            ScanSettings::default().results_page_size_max.max(1)
        }
    }

    pub fn get_results_page_size_max() -> u32 {
        if let Ok(config) = Self::get_instance().config.read() {
            config.results_page_size_max.max(1)
        } else {
            ScanSettings::default().results_page_size_max.max(1)
        }
    }

    pub fn get_results_page_size_auto() -> bool {
        if let Ok(config) = Self::get_instance().config.read() {
            config.results_page_size_auto
        } else {
            ScanSettings::default().results_page_size_auto
        }
    }

    pub fn get_scan_buffer_kb() -> u32 {
        if let Ok(config) = Self::get_instance().config.read() {
            config.scan_buffer_kb
        } else {
            ScanSettings::default().scan_buffer_kb
        }
    }

    pub fn set_scan_buffer_kb(value: u32) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.scan_buffer_kb = value.max(64);
        }

        Self::save_config();
    }

    pub fn get_thread_priority() -> ScanThreadPriority {
        if let Ok(config) = Self::get_instance().config.read() {
            config.thread_priority
        } else {
            ScanSettings::default().thread_priority
        }
    }

    pub fn set_thread_priority(value: ScanThreadPriority) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.thread_priority = value;
        }

        Self::save_config();
    }

    pub fn get_fast_scan_enabled() -> bool {
        if let Ok(config) = Self::get_instance().config.read() {
            config.fast_scan_enabled
        } else {
            ScanSettings::default().fast_scan_enabled
        }
    }

    pub fn set_fast_scan_enabled(value: bool) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.fast_scan_enabled = value;
        }

        Self::save_config();
    }

    pub fn get_fast_scan_alignment() -> Option<MemoryAlignment> {
        if let Ok(config) = Self::get_instance().config.read() {
            config.fast_scan_alignment
        } else {
            ScanSettings::default().fast_scan_alignment
        }
    }

    pub fn set_fast_scan_alignment(value: Option<MemoryAlignment>) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.fast_scan_alignment = value;
        }

        Self::save_config();
    }

    pub fn get_fast_scan_last_digits() -> Option<u8> {
        if let Ok(config) = Self::get_instance().config.read() {
            config.fast_scan_last_digits
        } else {
            ScanSettings::default().fast_scan_last_digits
        }
    }

    pub fn set_fast_scan_last_digits(value: Option<u8>) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.fast_scan_last_digits = value.map(|digit| digit.min(15));
        }

        Self::save_config();
    }

    pub fn get_pause_while_scanning() -> bool {
        if let Ok(config) = Self::get_instance().config.read() {
            config.pause_while_scanning
        } else {
            ScanSettings::default().pause_while_scanning
        }
    }

    pub fn set_pause_while_scanning(value: bool) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.pause_while_scanning = value;
        }

        Self::save_config();
    }

    pub fn get_repeat_scan_delay_ms() -> u64 {
        if let Ok(config) = Self::get_instance().config.read() {
            config.repeat_scan_delay_ms
        } else {
            ScanSettings::default().repeat_scan_delay_ms
        }
    }

    pub fn set_repeat_scan_delay_ms(value: u64) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.repeat_scan_delay_ms = value;
        }

        Self::save_config();
    }

    pub fn set_results_page_size(value: u32) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            let clamped = value.max(1);
            config.results_page_size = clamped;
            config.results_page_size_max = clamped;
        }

        Self::save_config();
    }

    pub fn set_results_page_size_max(value: u32) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            let clamped = value.max(1);
            config.results_page_size_max = clamped;
            config.results_page_size = clamped;
        }

        Self::save_config();
    }

    pub fn set_results_page_size_auto(value: bool) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.results_page_size_auto = value;
        }

        Self::save_config();
    }

    pub fn get_results_read_interval_ms() -> u64 {
        if let Ok(config) = Self::get_instance().config.read() {
            config.results_read_interval_ms
        } else {
            ScanSettings::default().results_read_interval_ms
        }
    }

    pub fn set_results_read_interval_ms(value: u64) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.results_read_interval_ms = value;
        }

        Self::save_config();
    }

    pub fn get_project_read_interval_ms() -> u64 {
        if let Ok(config) = Self::get_instance().config.read() {
            config.project_read_interval_ms
        } else {
            ScanSettings::default().project_read_interval_ms
        }
    }

    pub fn set_project_read_interval_ms(value: u64) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.project_read_interval_ms = value;
        }

        Self::save_config();
    }

    pub fn get_freeze_interval_ms() -> u64 {
        if let Ok(config) = Self::get_instance().config.read() {
            config.freeze_interval_ms
        } else {
            ScanSettings::default().freeze_interval_ms
        }
    }

    pub fn set_freeze_interval_ms(value: u64) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.freeze_interval_ms = value;
        }

        Self::save_config();
    }

    pub fn get_memory_alignment() -> Option<MemoryAlignment> {
        if let Ok(config) = Self::get_instance().config.read() {
            config.memory_alignment
        } else {
            ScanSettings::default().memory_alignment
        }
    }

    pub fn set_memory_alignment(value: Option<MemoryAlignment>) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.memory_alignment = value;
        }

        Self::save_config();
    }

    pub fn get_memory_read_mode() -> MemoryReadMode {
        if let Ok(config) = Self::get_instance().config.read() {
            config.memory_read_mode
        } else {
            ScanSettings::default().memory_read_mode
        }
    }

    pub fn set_memory_read_mode(value: MemoryReadMode) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.memory_read_mode = value;
        }

        Self::save_config();
    }

    pub fn get_floating_point_tolerance() -> FloatingPointTolerance {
        if let Ok(config) = Self::get_instance().config.read() {
            config.floating_point_tolerance
        } else {
            ScanSettings::default().floating_point_tolerance
        }
    }

    pub fn set_floating_point_tolerance(value: FloatingPointTolerance) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.floating_point_tolerance = value;
        }

        Self::save_config();
    }

    pub fn get_is_single_threaded_scan() -> bool {
        if let Ok(config) = Self::get_instance().config.read() {
            config.is_single_threaded_scan
        } else {
            ScanSettings::default().is_single_threaded_scan
        }
    }

    pub fn set_is_single_threaded_scan(value: bool) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.is_single_threaded_scan = value;
        }

        Self::save_config();
    }

    pub fn get_debug_perform_validation_scan() -> bool {
        if let Ok(config) = Self::get_instance().config.read() {
            config.debug_perform_validation_scan
        } else {
            ScanSettings::default().debug_perform_validation_scan
        }
    }

    pub fn set_debug_perform_validation_scan(value: bool) {
        if let Ok(mut config) = Self::get_instance().config.write() {
            config.debug_perform_validation_scan = value;
        }

        Self::save_config();
    }
}
