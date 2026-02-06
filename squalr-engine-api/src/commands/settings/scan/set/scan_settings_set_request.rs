use crate::commands::privileged_command_request::PrivilegedCommandRequest;
use crate::commands::settings::scan::scan_settings_command::ScanSettingsCommand;
use crate::commands::settings::scan::scan_settings_response::ScanSettingsResponse;
use crate::commands::settings::scan::set::scan_settings_set_response::ScanSettingsSetResponse;
use crate::commands::settings::settings_command::SettingsCommand;
use crate::structures::data_types::floating_point_tolerance::FloatingPointTolerance;
use crate::structures::scanning::memory_read_mode::MemoryReadMode;
use crate::structures::settings::scan_thread_priority::ScanThreadPriority;
use crate::{commands::privileged_command::PrivilegedCommand, structures::memory::memory_alignment::MemoryAlignment};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Clone, StructOpt, Debug, Default, Serialize, Deserialize)]
pub struct ScanSettingsSetRequest {
    #[structopt(short = "sb_kb", long)]
    pub scan_buffer_kb: Option<u32>,
    #[structopt(short = "tp", long)]
    pub thread_priority: Option<ScanThreadPriority>,
    #[structopt(short = "fast", long)]
    pub fast_scan_enabled: Option<bool>,
    #[structopt(short = "fast_align", long)]
    pub fast_scan_alignment: Option<MemoryAlignment>,
    #[structopt(short = "fast_last", long)]
    pub fast_scan_last_digits: Option<u8>,
    #[structopt(long)]
    pub clear_fast_scan_alignment: Option<bool>,
    #[structopt(long)]
    pub clear_fast_scan_last_digits: Option<bool>,
    #[structopt(long)]
    pub clear_memory_alignment: Option<bool>,
    #[structopt(short = "pause", long)]
    pub pause_while_scanning: Option<bool>,
    #[structopt(short = "repeat_delay", long)]
    pub repeat_scan_delay_ms: Option<u64>,
    #[structopt(long)]
    pub results_page_size_auto: Option<bool>,
    #[structopt(long)]
    pub results_page_size_max: Option<u32>,
    #[structopt(short = "psize", long)]
    pub results_page_size: Option<u32>,
    #[structopt(short = "r_read_interval", long)]
    pub results_read_interval_ms: Option<u64>,
    #[structopt(short = "p_read_interval", long)]
    pub project_read_interval_ms: Option<u64>,
    #[structopt(short = "f_interval", long)]
    pub freeze_interval_ms: Option<u64>,
    #[structopt(short = "m_align", long)]
    pub memory_alignment: Option<MemoryAlignment>,
    #[structopt(short = "m_read", long)]
    pub memory_read_mode: Option<MemoryReadMode>,
    #[structopt(short = "f_tol", long)]
    pub floating_point_tolerance: Option<FloatingPointTolerance>,
    #[structopt(short = "st", long)]
    pub is_single_threaded_scan: Option<bool>,
    #[structopt(short = "dbg", long)]
    pub debug_perform_validation_scan: Option<bool>,
}

impl PrivilegedCommandRequest for ScanSettingsSetRequest {
    type ResponseType = ScanSettingsSetResponse;

    fn to_engine_command(&self) -> PrivilegedCommand {
        PrivilegedCommand::Settings(SettingsCommand::Scan {
            scan_settings_command: ScanSettingsCommand::Set {
                scan_settings_set_request: self.clone(),
            },
        })
    }
}

impl From<ScanSettingsSetResponse> for ScanSettingsResponse {
    fn from(scan_settings_set_response: ScanSettingsSetResponse) -> Self {
        ScanSettingsResponse::Set { scan_settings_set_response }
    }
}
