use crate::memory_writer::memory_writer_trait::IMemoryWriter;
use squalr_engine_api::structures::processes::opened_process_info::OpenedProcessInfo;
use std::os::raw::c_void;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows_sys::Win32::System::Memory::{PAGE_READWRITE, VirtualProtectEx};

pub struct WindowsMemoryWriter;

impl WindowsMemoryWriter {
    pub fn new() -> Self {
        WindowsMemoryWriter
    }

    fn write_memory(
        process_handle: u64,
        address: u64,
        data: &[u8],
    ) -> bool {
        let mut old_protection = 0u32;
        let mut did_protect = false;

        let success = unsafe {
            // Best-effort: attempt to make the region writable to match Cheat Engine behavior, but do not
            // treat VirtualProtectEx failure as fatal (WriteProcessMemory may still succeed).
            if VirtualProtectEx(
                process_handle as *mut c_void,
                address as *mut _,
                data.len(),
                PAGE_READWRITE,
                &mut old_protection,
            ) != 0
            {
                did_protect = true;
            } else {
                log::debug!(
                    "VirtualProtectEx failed (addr=0x{:X}, size={}, last_error={})",
                    address,
                    data.len(),
                    GetLastError()
                );
            }

            let mut bytes_written = 0usize;
            let write_ok = WriteProcessMemory(
                process_handle as *mut c_void,
                address as *mut _,
                data.as_ptr() as *const _,
                data.len(),
                &mut bytes_written,
            ) != 0
                && bytes_written == data.len();

            if !write_ok {
                log::debug!(
                    "WriteProcessMemory failed (addr=0x{:X}, size={}, bytes_written={}, last_error={})",
                    address,
                    data.len(),
                    bytes_written,
                    GetLastError()
                );
            }

            if did_protect {
                let mut _unused_old_protection = 0u32;
                if VirtualProtectEx(
                    process_handle as *mut c_void,
                    address as *mut _,
                    data.len(),
                    old_protection,
                    &mut _unused_old_protection,
                ) == 0
                {
                    log::debug!(
                        "VirtualProtectEx restore failed (addr=0x{:X}, size={}, last_error={})",
                        address,
                        data.len(),
                        GetLastError()
                    );
                }
            }

            write_ok
        };

        return success;
    }
}

impl IMemoryWriter for WindowsMemoryWriter {
    fn write_bytes(
        &self,
        process_info: &OpenedProcessInfo,
        address: u64,
        values: &[u8],
    ) -> bool {
        Self::write_memory(process_info.get_handle(), address, values)
    }
}
