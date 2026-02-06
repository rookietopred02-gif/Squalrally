// Diagnostics: read the engine MemorySettingsConfig and show how many pages are returned by PageRetrievalMode::FromSettings.
//
// Usage:
//   cargo run -p squalr-engine-memory --example diag_settings --release <pid>
//
// Note: MemorySettingsConfig is resolved relative to the running executable's directory.

#[cfg(windows)]
fn main() {
    use squalr_engine_api::structures::memory::bitness::Bitness;
    use squalr_engine_api::structures::processes::opened_process_info::OpenedProcessInfo;
    use squalr_engine_memory::memory_queryer::memory_queryer::MemoryQueryer;
    use squalr_engine_memory::memory_queryer::page_retrieval_mode::PageRetrievalMode;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE};

    let pid: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or_else(|| std::process::id());

    unsafe {
        let access = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION;
        let handle = OpenProcess(access, 0, pid);
        if handle.is_null() {
            eprintln!("OpenProcess failed for pid={} (error={})", pid, GetLastError());
            return;
        }

        let process = OpenedProcessInfo::new(pid, "target".to_string(), handle as u64, Bitness::Bit64, None);

        let regions = MemoryQueryer::get_memory_page_bounds(&process, PageRetrievalMode::FromSettings);
        let total: u64 = regions.iter().map(|r| r.get_region_size()).sum();

        println!("regions={} total_bytes={}", regions.len(), total);

        let _ = CloseHandle(handle);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("diag_settings example is windows-only");
}