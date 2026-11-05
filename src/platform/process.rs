//! Platform process operations.
//!
//! Provides Win32 process termination, graceful closure via WM_CLOSE,
//! PID reuse validation, and File Explorer integration.

use std::fmt;
use std::path::Path;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, HANDLE, HWND, LPARAM,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, SW_SHOWNORMAL, WM_CLOSE,
};

use crate::platform::dialog::to_wide;

/// Errors returned by process manipulation and platform actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// Action is rejected due to safety or permission constraints (e.g. system critical process).
    NotAllowed(String),
    /// Process or associated window was not found, or PID reuse was detected.
    NotFound,
    /// Operating system rejected access due to insufficient privileges.
    AccessDenied,
    /// No top-level visible window found for graceful termination.
    NoWindow,
    /// Win32 API call failed with the given error code.
    Failed(u32),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAllowed(reason) => write!(f, "action not allowed: {reason}"),
            Self::NotFound => write!(f, "process or target window not found"),
            Self::AccessDenied => write!(f, "access denied by operating system"),
            Self::NoWindow => write!(f, "no visible top-level window found"),
            Self::Failed(code) => write!(f, "operating system call failed with error code {code}"),
        }
    }
}

impl std::error::Error for ActionError {}

/// RAII wrapper for Win32 `HANDLE` ensuring `CloseHandle` is invoked when dropped.
pub struct AutoHandle(HANDLE);

impl AutoHandle {
    /// Creates a new `AutoHandle` if the handle is valid.
    pub fn new(handle: HANDLE) -> Option<Self> {
        if handle.is_null() || handle == -1isize as HANDLE {
            None
        } else {
            Some(Self(handle))
        }
    }

    /// Returns the raw underlying `HANDLE`.
    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for AutoHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != -1isize as HANDLE {
            // SAFETY: The handle is non-null, not INVALID_HANDLE_VALUE, and owned exclusively.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

/// Helper function to extract filename from a path or name string, handling both slash types.
fn extract_filename(s: &str) -> &str {
    let trimmed = s.trim();
    let after_bs = trimmed.rsplit('\\').next().unwrap_or(trimmed);
    after_bs.rsplit('/').next().unwrap_or(after_bs)
}

/// Compares two executable or path strings for equality, ignoring path components and case.
pub fn names_match(a: &str, b: &str) -> bool {
    let a_file = extract_filename(a);
    let b_file = extract_filename(b);

    if a_file.eq_ignore_ascii_case(b_file) {
        return true;
    }

    let a_strip = a_file.strip_suffix(".exe").unwrap_or(a_file);
    let b_strip = b_file.strip_suffix(".exe").unwrap_or(b_file);
    a_strip.eq_ignore_ascii_case(b_strip)
}

/// Pure validation checking whether a process action is permitted according to safety rules.
pub fn check_allowed(pid: u32, expected_name: &str, own_pid: u32) -> Result<(), ActionError> {
    if pid == 0 || pid == 4 {
        return Err(ActionError::NotAllowed(
            "System process (PID 0 or 4) cannot be targeted".to_string(),
        ));
    }

    if pid == own_pid {
        return Err(ActionError::NotAllowed(format!(
            "Cannot target SlowSpot process (PID {pid})"
        )));
    }

    let base_name = extract_filename(expected_name);
    let kind = crate::analyzer::safety::classify(base_name);
    if !crate::analyzer::safety::is_killable(kind) {
        return Err(ActionError::NotAllowed(format!(
            "Process '{base_name}' is protected by system safety rules"
        )));
    }

    Ok(())
}

/// Retrieves the executable image path of a process by PID.
fn get_process_image_path(pid: u32) -> Result<String, ActionError> {
    // SAFETY: OpenProcess is called with valid PID and limited information query flags.
    let handle_raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    let handle = AutoHandle::new(handle_raw).ok_or_else(|| {
        // SAFETY: GetLastError retrieves the Win32 error code for the immediately preceding failure.
        let err = unsafe { GetLastError() };
        if err == ERROR_ACCESS_DENIED {
            ActionError::AccessDenied
        } else {
            ActionError::NotFound
        }
    })?;

    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;

    // SAFETY: handle is valid; buffer pointer and size are valid and initialized.
    let success =
        unsafe { QueryFullProcessImageNameW(handle.raw(), 0, buffer.as_mut_ptr(), &mut size) };

    if success == 0 {
        // SAFETY: GetLastError retrieves the Win32 error code for QueryFullProcessImageNameW.
        let err = unsafe { GetLastError() };
        if err == ERROR_ACCESS_DENIED {
            return Err(ActionError::AccessDenied);
        }
        return Err(ActionError::NotFound);
    }

    Ok(String::from_utf16_lossy(&buffer[..size as usize]))
}

struct EnumContext {
    target_pid: u32,
    found_hwnds: Vec<HWND>,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
    // SAFETY: lparam is a valid raw pointer to EnumContext passed from graceful_close.
    let ctx = &mut *(lparam as *mut EnumContext);
    let mut window_pid = 0u32;

    // SAFETY: hwnd is provided by the OS; window_pid is a valid out-pointer.
    GetWindowThreadProcessId(hwnd, &mut window_pid);

    if window_pid == ctx.target_pid && IsWindowVisible(hwnd) != 0 {
        ctx.found_hwnds.push(hwnd);
    }
    1
}

/// Requests a process to close gracefully by posting `WM_CLOSE` to its visible top-level windows.
pub fn graceful_close(pid: u32, expected_name: &str) -> Result<(), ActionError> {
    check_allowed(pid, expected_name, std::process::id())?;

    let actual_image_path = get_process_image_path(pid)?;
    if !names_match(&actual_image_path, expected_name) {
        return Err(ActionError::NotFound);
    }

    let mut ctx = EnumContext {
        target_pid: pid,
        found_hwnds: Vec::new(),
    };

    // SAFETY: EnumWindows is invoked with a valid callback and a valid pointer to ctx.
    unsafe {
        EnumWindows(Some(enum_windows_proc), &mut ctx as *mut _ as LPARAM);
    }

    if ctx.found_hwnds.is_empty() {
        return Err(ActionError::NoWindow);
    }

    let mut posted_any = false;
    for &hwnd in &ctx.found_hwnds {
        // SAFETY: hwnd is a visible top-level window handle of the target process.
        let ret = unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
        if ret != 0 {
            posted_any = true;
        }
    }

    if posted_any {
        Ok(())
    } else {
        // SAFETY: GetLastError gets the error code for PostMessageW failure.
        let err = unsafe { GetLastError() };
        Err(ActionError::Failed(err))
    }
}

/// Forcibly terminates a process by PID after verifying safety rules and matching image name.
pub fn force_kill(pid: u32, expected_name: &str) -> Result<(), ActionError> {
    check_allowed(pid, expected_name, std::process::id())?;

    // SAFETY: OpenProcess is called with PROCESS_TERMINATE and PROCESS_QUERY_LIMITED_INFORMATION.
    let handle_raw = unsafe {
        OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };

    let handle = AutoHandle::new(handle_raw).ok_or_else(|| {
        // SAFETY: GetLastError retrieves the Win32 error code for OpenProcess failure.
        let err = unsafe { GetLastError() };
        if err == ERROR_ACCESS_DENIED {
            ActionError::AccessDenied
        } else {
            ActionError::NotFound
        }
    })?;

    // Verify process image name to prevent PID reuse hazards
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;

    // SAFETY: handle is valid; buffer pointer and size are valid.
    let success =
        unsafe { QueryFullProcessImageNameW(handle.raw(), 0, buffer.as_mut_ptr(), &mut size) };

    if success == 0 {
        // SAFETY: GetLastError retrieves the Win32 error code for QueryFullProcessImageNameW failure.
        let err = unsafe { GetLastError() };
        if err == ERROR_ACCESS_DENIED {
            return Err(ActionError::AccessDenied);
        }
        return Err(ActionError::NotFound);
    }

    let actual_image_path = String::from_utf16_lossy(&buffer[..size as usize]);
    if !names_match(&actual_image_path, expected_name) {
        return Err(ActionError::NotFound);
    }

    // SAFETY: handle is valid with PROCESS_TERMINATE rights; exit code 1 is specified.
    let term_ret = unsafe { TerminateProcess(handle.raw(), 1) };
    if term_ret == 0 {
        // SAFETY: GetLastError retrieves the Win32 error code for TerminateProcess failure.
        let err = unsafe { GetLastError() };
        if err == ERROR_ACCESS_DENIED {
            return Err(ActionError::AccessDenied);
        }
        return Err(ActionError::Failed(err));
    }

    Ok(())
}

/// Opens File Explorer with the specified executable file selected.
pub fn open_location(exe_path: &str) -> Result<(), ActionError> {
    if exe_path.contains('\"') || exe_path.contains('\0') {
        return Err(ActionError::NotAllowed(
            "Executable path contains invalid or dangerous characters".to_string(),
        ));
    }

    let path = Path::new(exe_path);
    if !path.is_absolute() {
        return Err(ActionError::NotAllowed(
            "Executable path is not absolute".to_string(),
        ));
    }

    if !path.exists() {
        return Err(ActionError::NotFound);
    }

    let verb = to_wide("open");
    let explorer = to_wide("explorer.exe");
    let param = to_wide(&format!("/select,\"{exe_path}\""));

    // SAFETY: ShellExecuteW is invoked with valid nul-terminated UTF-16 wide strings and null hwnd.
    let ret = unsafe {
        ShellExecuteW(
            core::ptr::null_mut(),
            verb.as_ptr(),
            explorer.as_ptr(),
            param.as_ptr(),
            core::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    if (ret as isize) <= 32 {
        return Err(ActionError::Failed(ret as u32));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ChildGuard(Option<std::process::Child>);

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    #[test]
    fn test_check_allowed_pure_rules() {
        let own_pid = 9999;

        // System critical processes must be rejected
        let crit_processes = [
            "csrss.exe",
            "lsass.exe",
            "explorer.exe",
            "smss.exe",
            "winlogon.exe",
        ];
        for proc in crit_processes {
            let res = check_allowed(1234, proc, own_pid);
            assert!(
                matches!(res, Err(ActionError::NotAllowed(_))),
                "Expected '{proc}' to be rejected"
            );
        }

        // With paths
        assert!(matches!(
            check_allowed(1234, "C:\\Windows\\System32\\csrss.exe", own_pid),
            Err(ActionError::NotAllowed(_))
        ));
        assert!(matches!(
            check_allowed(1234, "C:/Windows/explorer.exe", own_pid),
            Err(ActionError::NotAllowed(_))
        ));

        // System PIDs 0 and 4 must be rejected
        assert!(matches!(
            check_allowed(0, "notepad.exe", own_pid),
            Err(ActionError::NotAllowed(_))
        ));
        assert!(matches!(
            check_allowed(4, "notepad.exe", own_pid),
            Err(ActionError::NotAllowed(_))
        ));

        // Own PID must be rejected
        assert!(matches!(
            check_allowed(own_pid, "notepad.exe", own_pid),
            Err(ActionError::NotAllowed(_))
        ));

        // Normal user apps must be allowed
        assert_eq!(check_allowed(1234, "notepad.exe", own_pid), Ok(()));
        assert_eq!(check_allowed(5678, "chrome.exe", own_pid), Ok(()));
    }

    #[test]
    fn test_names_match_pure() {
        assert!(names_match("notepad.exe", "notepad.exe"));
        assert!(names_match("notepad.exe", "NOTEPAD.EXE"));
        assert!(names_match("NOTEPAD.EXE", "notepad.exe"));
        assert!(names_match(
            "C:\\Windows\\System32\\notepad.exe",
            "notepad.exe"
        ));
        assert!(names_match(
            "C:/Windows/System32/NOTEPAD.EXE",
            "notepad.exe"
        ));
        assert!(names_match("notepad", "notepad.exe"));
        assert!(names_match("notepad.exe", "notepad"));

        assert!(!names_match("notepad.exe", "calc.exe"));
        assert!(!names_match("C:\\Windows\\explorer.exe", "notepad.exe"));
        assert!(!names_match("foo.exe", "bar.exe"));
    }

    #[test]
    fn test_open_location_validation() {
        // Path with double quotes
        assert!(matches!(
            open_location("C:\\test\\\"bad\".exe"),
            Err(ActionError::NotAllowed(_))
        ));

        // Path with null character
        assert!(matches!(
            open_location("C:\\test\0bad.exe"),
            Err(ActionError::NotAllowed(_))
        ));

        // Relative path
        assert!(matches!(
            open_location("notepad.exe"),
            Err(ActionError::NotAllowed(_))
        ));

        // Non-existent absolute path
        assert_eq!(
            open_location("C:\\SlowSpot_NonExistent_Directory_12345\\file.exe"),
            Err(ActionError::NotFound)
        );
    }

    #[test]
    fn test_os_live_force_kill_and_pid_reuse_protection() {
        // Spawn harmless ping command with 60 second delay
        let child = std::process::Command::new("ping")
            .args(["-n", "60", "127.0.0.1"])
            .spawn()
            .expect("Failed to spawn harmless child process");
        let child_pid = child.id();
        let mut guard = ChildGuard(Some(child));

        // 1. Mismatched expected_name should return NotFound (PID reuse protection)
        // and child process must remain alive
        let wrong_res = force_kill(child_pid, "definitely_wrong_process_name.exe");
        assert_eq!(
            wrong_res,
            Err(ActionError::NotFound),
            "Expected NotFound on mismatched expected_name"
        );

        // Verify child is still alive
        let child_ref = guard.0.as_mut().expect("child exists");
        assert!(
            child_ref.try_wait().expect("try_wait failed").is_none(),
            "Child should still be running after failed name check"
        );

        // 2. Correct expected_name should successfully terminate the child
        let kill_res = force_kill(child_pid, "ping.exe");
        assert!(kill_res.is_ok(), "force_kill failed: {:?}", kill_res);

        // Confirm child process exited quickly with exit code 1
        let exit_status = child_ref.wait().expect("child.wait() failed");
        assert_eq!(
            exit_status.code(),
            Some(1),
            "Expected exit code 1 from TerminateProcess"
        );

        // Disarm guard since child already exited
        guard.0.take();
    }

    #[test]
    fn test_os_live_graceful_close_headless_returns_no_window() {
        // Spawn harmless ping command which has no GUI window
        let child = std::process::Command::new("ping")
            .args(["-n", "60", "127.0.0.1"])
            .spawn()
            .expect("Failed to spawn harmless child process");
        let child_pid = child.id();
        let mut guard = ChildGuard(Some(child));

        let res = graceful_close(child_pid, "ping.exe");
        assert_eq!(res, Err(ActionError::NoWindow));

        // Child process should still be running
        let child_ref = guard.0.as_mut().expect("child exists");
        assert!(
            child_ref.try_wait().expect("try_wait failed").is_none(),
            "Child should still be running after graceful_close returned NoWindow"
        );
    }
}
