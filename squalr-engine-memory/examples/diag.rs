// Minimal diagnostics for "0 regions / 0 bytes read" issues on Windows.
// Run: cargo run -p squalr-engine-memory --example diag

#[cfg(windows)]
fn main() {
    use squalr_engine_api::structures::memory::bitness::Bitness;
    use squalr_engine_api::structures::processes::opened_process_info::OpenedProcessInfo;
    use squalr_engine_memory::memory_queryer::memory_queryer::MemoryQueryer;
    use squalr_engine_memory::memory_queryer::page_retrieval_mode::PageRetrievalMode;
    use squalr_engine_memory::memory_reader::MemoryReader;
    use squalr_engine_memory::memory_reader::memory_reader_trait::IMemoryReader;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

    let pid: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or_else(|| std::process::id());
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
        if handle.is_null() {
            eprintln!("OpenProcess failed for self pid={} (error={})", pid, GetLastError());
            return;
        }

        let name = if pid == std::process::id() { "self" } else { "target" }.to_string();
        let process = OpenedProcessInfo::new(pid, name, handle as u64, Bitness::Bit64, None);

        let regions = MemoryQueryer::get_memory_page_bounds(&process, PageRetrievalMode::FromUserMode);
        println!("regions={}", regions.len());
        let total: u64 = regions.iter().map(|r| r.get_region_size()).sum();
        println!("total_bytes={}", total);

        if let Some(first) = regions.first() {
            let addr = first.get_base_address();
            let mut buf = vec![0u8; 16];
            let ok = MemoryReader::get_instance().read_bytes(&process, addr, &mut buf);
            println!("read @0x{:X} ok={} bytes={:02X?}", addr, ok, buf);
        }

        let _ = CloseHandle(handle);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("diag example is windows-only");
}
