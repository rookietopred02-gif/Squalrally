#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, LUID};
#[cfg(windows)]
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY, SE_PRIVILEGE_ENABLED,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

#[cfg(windows)]
pub fn enable_debug_privilege() {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        let opened = OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token);
        if opened == 0 {
            log::warn!("Failed to open process token for SeDebugPrivilege (error={}).", GetLastError());
            return;
        }

        let mut luid = LUID { LowPart: 0, HighPart: 0 };
        let mut name: Vec<u16> = "SeDebugPrivilege".encode_utf16().collect();
        name.push(0);

        if LookupPrivilegeValueW(std::ptr::null(), name.as_ptr(), &mut luid) == 0 {
            log::warn!("Failed to lookup SeDebugPrivilege LUID (error={}).", GetLastError());
            CloseHandle(token);
            return;
        }

        let mut privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        if AdjustTokenPrivileges(token, 0, &mut privileges, 0, std::ptr::null_mut(), std::ptr::null_mut()) == 0 {
            log::warn!("Failed to enable SeDebugPrivilege (error={}).", GetLastError());
        } else {
            let error = GetLastError();
            if error != 0 {
                log::warn!("SeDebugPrivilege not fully assigned (error={}).", error);
            } else {
                log::info!("SeDebugPrivilege enabled.");
            }
        }

        CloseHandle(token);
    }
}
