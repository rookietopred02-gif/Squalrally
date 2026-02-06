#[cfg(windows)]
mod windows {
    use rustc_demangle::demangle;
    use std::ffi::CStr;
    use std::io::Write;
    use std::os::windows::io::AsRawHandle;
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::Diagnostics::Debug::{
        AddVectoredExceptionHandler, EXCEPTION_POINTERS, IMAGEHLP_LINE64, MINIDUMP_EXCEPTION_INFORMATION,
        MiniDumpWriteDump, SYMOPT_DEFERRED_LOADS, SYMOPT_LOAD_LINES, SYMOPT_UNDNAME, SYMBOL_INFO, SetUnhandledExceptionFilter, SymFromAddr,
        SymGetLineFromAddr64, SymInitialize, SymSetOptions, RtlCaptureStackBackTrace,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    const MAX_SYMBOL_NAME_LEN: usize = 512;
    const EXCEPTION_EXECUTE_HANDLER: i32 = 1;
    const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
    const MAX_BACKTRACE_FRAMES: u32 = 128;

    static IN_HANDLER: AtomicBool = AtomicBool::new(false);

    pub fn install() {
        unsafe {
            // Prefer a vectored exception handler: other libraries can overwrite SetUnhandledExceptionFilter,
            // but vectored handlers are additive and generally fire reliably for access violations.
            let _ = AddVectoredExceptionHandler(1, Some(vectored_exception_handler));

            // Best-effort: register a top-level unhandled exception filter so access violations can be diagnosed even
            // in "windows_subsystem=windows" builds where no console is present.
            SetUnhandledExceptionFilter(Some(unhandled_exception_filter));
        }
    }

    unsafe extern "system" fn vectored_exception_handler(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
        if IN_HANDLER
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return EXCEPTION_CONTINUE_SEARCH;
        }

        let _ = unsafe { write_crash_report(exception_info) };
        IN_HANDLER.store(false, Ordering::SeqCst);

        EXCEPTION_CONTINUE_SEARCH
    }

    unsafe extern "system" fn unhandled_exception_filter(exception_info: *const EXCEPTION_POINTERS) -> i32 {
        // Must not panic from inside the exception filter.
        let _ = unsafe { write_crash_report(exception_info) };
        EXCEPTION_EXECUTE_HANDLER
    }

    fn make_crash_paths() -> (std::path::PathBuf, std::path::PathBuf) {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let pid = unsafe { GetCurrentProcessId() };
        let tid = unsafe { GetCurrentThreadId() };

        let base = format!("squalr_crash_{timestamp_ms}_pid{pid}_tid{tid}");
        let dir = std::env::temp_dir();
        (dir.join(format!("{base}.log")), dir.join(format!("{base}.dmp")))
    }

    unsafe fn write_minidump(
        dump_path: &std::path::Path,
        exception_info: *const EXCEPTION_POINTERS,
    ) -> std::io::Result<()> {
        let process = unsafe { GetCurrentProcess() };
        if process == std::ptr::null_mut() {
            return Ok(());
        }

        let dump_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(dump_path)?;

        let dump_handle = dump_file.as_raw_handle() as HANDLE;
        if dump_handle.is_null() {
            return Ok(());
        }

        let mut exception = MINIDUMP_EXCEPTION_INFORMATION {
            ThreadId: unsafe { GetCurrentThreadId() },
            ExceptionPointers: exception_info as *mut _,
            ClientPointers: 0,
        };

        // Use a conservative dump type: enough to debug crashes without generating huge dumps.
        // 0x00000000 == MiniDumpNormal
        let dump_type = 0i32;

        let ok = unsafe {
            MiniDumpWriteDump(
                process,
                GetCurrentProcessId(),
                dump_handle,
                dump_type,
                &mut exception,
                std::ptr::null(),
                std::ptr::null(),
            )
        };

        if ok == 0 {
            // Best-effort; ignore error details to avoid additional Win32 calls in exception context.
            return Ok(());
        }

        Ok(())
    }

    unsafe fn write_crash_report(exception_info: *const EXCEPTION_POINTERS) -> std::io::Result<()> {
        if exception_info.is_null() {
            return Ok(());
        }

        let exception_record = unsafe { (*exception_info).ExceptionRecord };
        if exception_record.is_null() {
            return Ok(());
        }

        let exception_code = unsafe { (*exception_record).ExceptionCode };
        let exception_address = unsafe { (*exception_record).ExceptionAddress as u64 };

        let (crash_log_path, crash_dump_path) = make_crash_paths();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_log_path)?;

        let _ = writeln!(file, "================ Squalr crash ================");
        let _ = writeln!(file, "ExceptionCode: 0x{exception_code:08X}");
        let _ = writeln!(file, "ExceptionAddress: 0x{exception_address:016X}");
        let _ = writeln!(file, "PID: {}", unsafe { GetCurrentProcessId() });
        let _ = writeln!(file, "TID: {}", unsafe { GetCurrentThreadId() });
        let _ = writeln!(file, "CrashLog: {}", crash_log_path.display());

        let process = unsafe { GetCurrentProcess() };
        if process == std::ptr::null_mut() {
            let _ = writeln!(file, "GetCurrentProcess failed.");
            return Ok(());
        }

        let _ = unsafe { write_minidump(&crash_dump_path, exception_info) };
        let _ = writeln!(file, "Minidump: {}", crash_dump_path.display());

        // Initialize symbol handler (best-effort).
        unsafe { SymSetOptions(SYMOPT_UNDNAME | SYMOPT_DEFERRED_LOADS | SYMOPT_LOAD_LINES) };
        if unsafe { SymInitialize(process, std::ptr::null(), 1) } == 0 {
            let _ = writeln!(file, "SymInitialize failed.");
            return Ok(());
        }

        // Resolve symbol name.
        //
        // NOTE: SYMBOL_INFO has alignment requirements. Do not use Vec<u8> here because its allocation
        // alignment is 1 and casting to SYMBOL_INFO would be UB on some allocators/builds.
        let symbol_buf_size = std::mem::size_of::<SYMBOL_INFO>() + MAX_SYMBOL_NAME_LEN;
        let symbol_layout = match std::alloc::Layout::from_size_align(symbol_buf_size, std::mem::align_of::<SYMBOL_INFO>()) {
            Ok(layout) => layout,
            Err(_) => {
                let _ = writeln!(file, "Failed to compute SYMBOL_INFO layout.");
                let _ = file.flush();
                return Ok(());
            }
        };

        let symbol_buf = unsafe { std::alloc::alloc_zeroed(symbol_layout) };
        if symbol_buf.is_null() {
            let _ = writeln!(file, "Failed to allocate SYMBOL_INFO buffer.");
            let _ = file.flush();
            return Ok(());
        }

        let symbol = symbol_buf as *mut SYMBOL_INFO;
        unsafe {
            (*symbol).SizeOfStruct = std::mem::size_of::<SYMBOL_INFO>() as u32;
            (*symbol).MaxNameLen = MAX_SYMBOL_NAME_LEN as u32;
        }

        let mut displacement64: u64 = 0;
        if unsafe { SymFromAddr(process, exception_address, &mut displacement64, symbol) } != 0 {
            let raw_name = unsafe { CStr::from_ptr((*symbol).Name.as_ptr() as *const i8) }
                .to_string_lossy()
                .into_owned();
            let demangled = demangle(&raw_name).to_string();
            let _ = writeln!(file, "Symbol: {demangled}");
            let _ = writeln!(file, "SymbolDisplacement: 0x{displacement64:X}");
        } else {
            let _ = writeln!(file, "SymFromAddr failed.");
        }

        unsafe { std::alloc::dealloc(symbol_buf, symbol_layout) };

        // Resolve source line.
        let mut line: IMAGEHLP_LINE64 = unsafe { std::mem::zeroed() };
        line.SizeOfStruct = std::mem::size_of::<IMAGEHLP_LINE64>() as u32;

        let mut displacement32: u32 = 0;
        if unsafe { SymGetLineFromAddr64(process, exception_address, &mut displacement32, &mut line) } != 0 {
            if !line.FileName.is_null() {
                let file_name = unsafe { CStr::from_ptr(line.FileName as *const i8) }.to_string_lossy();
                let _ = writeln!(file, "File: {}:{}", file_name, line.LineNumber);
                let _ = writeln!(file, "LineDisplacement: 0x{displacement32:X}");
            }
        } else {
            let _ = writeln!(file, "SymGetLineFromAddr64 failed.");
        }

        // Capture and print a best-effort stack trace for the current thread.
        let mut frames: [*mut core::ffi::c_void; MAX_BACKTRACE_FRAMES as usize] = [std::ptr::null_mut(); MAX_BACKTRACE_FRAMES as usize];
        let mut hash: u32 = 0;
        let captured = unsafe { RtlCaptureStackBackTrace(0, MAX_BACKTRACE_FRAMES, frames.as_mut_ptr(), &mut hash as *mut u32) } as u32;
        if captured > 0 {
            let _ = writeln!(file, "StackBackTrace (CaptureStackBackTrace) frames={captured}:");
            for (i, frame) in frames.iter().take(captured as usize).enumerate() {
                let addr = *frame as u64;
                let mut displacement64: u64 = 0;

                // Allocate SYMBOL_INFO with correct alignment.
                let symbol_buf_size = std::mem::size_of::<SYMBOL_INFO>() + MAX_SYMBOL_NAME_LEN;
                let symbol_layout = match std::alloc::Layout::from_size_align(symbol_buf_size, std::mem::align_of::<SYMBOL_INFO>()) {
                    Ok(layout) => layout,
                    Err(_) => {
                        let _ = writeln!(file, "  #{i}: 0x{addr:016X} (symbol layout failed)");
                        continue;
                    }
                };
                let symbol_buf = unsafe { std::alloc::alloc_zeroed(symbol_layout) };
                if symbol_buf.is_null() {
                    let _ = writeln!(file, "  #{i}: 0x{addr:016X} (symbol alloc failed)");
                    continue;
                }

                let symbol = symbol_buf as *mut SYMBOL_INFO;
                unsafe {
                    (*symbol).SizeOfStruct = std::mem::size_of::<SYMBOL_INFO>() as u32;
                    (*symbol).MaxNameLen = MAX_SYMBOL_NAME_LEN as u32;
                }

                if unsafe { SymFromAddr(process, addr, &mut displacement64, symbol) } != 0 {
                    let raw_name = unsafe { CStr::from_ptr((*symbol).Name.as_ptr() as *const i8) }
                        .to_string_lossy()
                        .into_owned();
                    let demangled = demangle(&raw_name).to_string();
                    let _ = writeln!(file, "  #{i}: 0x{addr:016X} {demangled}+0x{displacement64:X}");
                } else {
                    let _ = writeln!(file, "  #{i}: 0x{addr:016X} (SymFromAddr failed)");
                }

                unsafe { std::alloc::dealloc(symbol_buf, symbol_layout) };
            }
        }

        let _ = file.flush();
        Ok(())
    }

    // Compile-time assertions for windows-sys types.
    #[allow(dead_code)]
    fn _assert_win_types() {
        fn _assert_handle(_: HANDLE) {}
    }
}

#[cfg(windows)]
pub fn install() {
    windows::install();
}

#[cfg(not(windows))]
pub fn install() {}
