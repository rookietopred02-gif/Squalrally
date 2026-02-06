// Diagnostics: evaluate current filtering logic against a real process + settings file.
//
// Usage:
//   cargo run -p squalr-engine-memory --example diag_filters --release <pid> [settings.json]
//
// Example:
//   cargo run -p squalr-engine-memory --example diag_filters --release 17032 target/release/memory_settings.json

#[cfg(windows)]
fn main() {
    use squalr_engine_api::structures::memory::bitness::Bitness;
    use squalr_engine_api::structures::processes::opened_process_info::OpenedProcessInfo;
    use squalr_engine_api::structures::settings::memory_settings::MemorySettings;
    use squalr_engine_memory::memory_queryer::memory_protection_enum::MemoryProtectionEnum;
    use squalr_engine_memory::memory_queryer::memory_queryer_trait::IMemoryQueryer;
    use squalr_engine_memory::memory_queryer::memory_type_enum::MemoryTypeEnum;
    use squalr_engine_memory::memory_queryer::MemoryQueryerImpl;
    use squalr_engine_memory::memory_queryer::region_bounds_handling::RegionBoundsHandling;
    use std::fs;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
    };

    let mut args = std::env::args().skip(1);
    let pid: u32 = args
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or_else(|| {
            eprintln!("usage: diag_filters <pid> [settings.json]");
            std::process::exit(2);
        });

    let settings_path = args
        .next()
        .unwrap_or_else(|| "target/release/memory_settings.json".to_string());

    let settings: MemorySettings = match fs::read_to_string(&settings_path)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
    {
        Some(s) => s,
        None => {
            eprintln!("Failed to read/parse settings at '{}'", settings_path);
            std::process::exit(2);
        }
    };

    unsafe {
        let full_access = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION;
        let handle = OpenProcess(full_access, 0, pid);
        if handle.is_null() {
            eprintln!("OpenProcess failed for pid={} (error={})", pid, GetLastError());
            return;
        }

        let process = OpenedProcessInfo::new(pid, "target".to_string(), handle as u64, Bitness::Bit64, None);

        let mut allowed = MemoryTypeEnum::empty();
        if settings.memory_type_none {
            allowed |= MemoryTypeEnum::NONE;
        }
        if settings.memory_type_private {
            allowed |= MemoryTypeEnum::PRIVATE;
        }
        if settings.memory_type_image {
            allowed |= MemoryTypeEnum::IMAGE;
        }
        if settings.memory_type_mapped {
            allowed |= MemoryTypeEnum::MAPPED;
        }

        let mut required = MemoryProtectionEnum::empty();
        if settings.required_write {
            required |= MemoryProtectionEnum::WRITE;
        }
        if settings.required_execute {
            required |= MemoryProtectionEnum::EXECUTE;
        }
        if settings.required_copy_on_write {
            required |= MemoryProtectionEnum::COPY_ON_WRITE;
        }

        let mut excluded = MemoryProtectionEnum::empty();
        if settings.excluded_write {
            excluded |= MemoryProtectionEnum::WRITE;
        }
        if settings.excluded_execute {
            excluded |= MemoryProtectionEnum::EXECUTE;
        }
        if settings.excluded_copy_on_write {
            excluded |= MemoryProtectionEnum::COPY_ON_WRITE;
        }
        if settings.excluded_no_cache {
            excluded |= MemoryProtectionEnum::NO_CACHE;
        }
        if settings.excluded_write_combine {
            excluded |= MemoryProtectionEnum::WRITE_COMBINE;
        }

        let queryer = MemoryQueryerImpl::new();
        let (start, end) = if settings.only_query_usermode {
            (0, queryer.get_max_usermode_address(&process))
        } else {
            (settings.start_address, settings.end_address)
        };

        let regions = queryer.get_virtual_pages(&process, required, excluded, allowed, start, end, RegionBoundsHandling::Exclude);
        let total: u64 = regions.iter().map(|r| r.get_region_size()).sum::<u64>();

        println!("settings_path={}", settings_path);
        println!("required={:?} excluded={:?} allowed={:?} start=0x{:X} end=0x{:X}", required, excluded, allowed, start, end);
        println!("regions={} total_bytes={}", regions.len(), total);

        let _ = CloseHandle(handle);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("diag_filters example is windows-only");
}
