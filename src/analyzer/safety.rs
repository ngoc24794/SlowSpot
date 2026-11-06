//! Process classification and safety filtering against critical OS processes.
//!
//! Protects the Windows operating system from stability hazards (such as Blue Screens
//! of Death) by forbidding termination of essential system processes.

use crate::model::ProcessKind;

/// Hardcoded whitelist of Windows critical processes that must never be terminated.
pub const CRITICAL_PROCESS_NAMES: &[&str] = &[
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "services.exe",
    "lsass.exe",
    "winlogon.exe",
    "fontdrvhost.exe",
    "system",
    "registry",
    "system idle process",
    "dwm.exe",
    "explorer.exe",
];

/// Known background services and system helper processes.
pub const BACKGROUND_SERVICE_NAMES: &[&str] = &[
    "svchost.exe",
    "spoolsv.exe",
    "searchindexer.exe",
    "sihost.exe",
    "taskhostw.exe",
    "runtimebroker.exe",
    "conhost.exe",
    "wmiprvse.exe",
    "dllhost.exe",
    "msmpeng.exe",
    "securityhealthservice.exe",
    "searchhost.exe",
    "smartscreen.exe",
    "ctfmon.exe",
];

/// Classifies a process into [`ProcessKind`] based on its executable name.
///
/// Matching is case-insensitive and normalizes filenames by stripping `.exe` if present.
pub fn classify(name: &str) -> ProcessKind {
    let lower = name.trim().to_ascii_lowercase();
    let lower_no_ext = lower.strip_suffix(".exe").unwrap_or(&lower);

    // Check system critical processes
    for &crit in CRITICAL_PROCESS_NAMES {
        let crit_no_ext = crit.strip_suffix(".exe").unwrap_or(crit);
        if lower == crit || lower_no_ext == crit_no_ext {
            return ProcessKind::SystemCritical;
        }
    }

    // Check known background service processes
    for &svc in BACKGROUND_SERVICE_NAMES {
        let svc_no_ext = svc.strip_suffix(".exe").unwrap_or(svc);
        if lower == svc || lower_no_ext == svc_no_ext {
            return ProcessKind::BackgroundService;
        }
    }

    ProcessKind::UserApp
}

/// Evaluates whether a process of the given [`ProcessKind`] is permitted to be terminated.
///
/// System critical processes are never killable.
pub fn is_killable(kind: ProcessKind) -> bool {
    match kind {
        ProcessKind::SystemCritical => false,
        ProcessKind::UserApp | ProcessKind::BackgroundService => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_critical_processes_not_killable() {
        let critical_samples = [
            "smss.exe",
            "SMSS.EXE",
            "csrss.exe",
            "wininit.exe",
            "services.exe",
            "lsass.exe",
            "winlogon.exe",
            "fontdrvhost.exe",
            "System",
            "system",
            "Registry",
            "registry",
            "System Idle Process",
            "system idle process",
            "dwm.exe",
            "DWM.exe",
            "explorer.exe",
            "Explorer.EXE",
        ];

        for name in critical_samples {
            let kind = classify(name);
            assert_eq!(
                kind,
                ProcessKind::SystemCritical,
                "Expected '{name}' to be classified as SystemCritical"
            );
            assert!(!is_killable(kind), "Expected '{name}' to not be killable");
        }
    }

    #[test]
    fn test_user_apps_killable() {
        let user_apps = [
            "notepad.exe",
            "NOTEPAD.EXE",
            "chrome.exe",
            "code.exe",
            "game.exe",
        ];

        for name in user_apps {
            let kind = classify(name);
            assert_eq!(
                kind,
                ProcessKind::UserApp,
                "Expected '{name}' to be classified as UserApp"
            );
            assert!(is_killable(kind), "Expected '{name}' to be killable");
        }
    }

    #[test]
    fn test_background_services_killable() {
        let services = ["svchost.exe", "spoolsv.exe", "msmpeng.exe"];

        for name in services {
            let kind = classify(name);
            assert_eq!(
                kind,
                ProcessKind::BackgroundService,
                "Expected '{name}' to be classified as BackgroundService"
            );
            assert!(
                is_killable(kind),
                "Expected background service '{name}' to be killable"
            );
        }
    }
}
