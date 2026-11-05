//! User action executor abstractions and platform implementations.
//!
//! Bridges UI interaction buttons (graceful close, force kill, open location)
//! to native OS platform commands with localized Vietnamese feedback.

use crate::model::{ActionCommand, ScoredProcess};
use crate::platform::process::{force_kill, graceful_close, open_location, ActionError};
use crate::ui::strings;

/// Runner trait executing user-requested process actions.
pub trait ActionRunner: Send {
    /// Executes the specified action command against the target process.
    ///
    /// Returns `Ok(Vietnamese success message)` or `Err(Vietnamese friendly error)`.
    fn run(&mut self, cmd: ActionCommand, target: &ScoredProcess) -> Result<String, String>;
}

/// Real operating system action runner invoking Win32 APIs.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealActions;

impl ActionRunner for RealActions {
    fn run(&mut self, cmd: ActionCommand, target: &ScoredProcess) -> Result<String, String> {
        let pid = target.record.pid;
        let name = &target.record.name;

        match cmd {
            ActionCommand::GracefulClose(_) => graceful_close(pid, name)
                .map(|_| format!("Đã gửi yêu cầu đóng an toàn tới '{name}' (PID {pid})"))
                .map_err(format_action_error),
            ActionCommand::ForceKill(_) => force_kill(pid, name)
                .map(|_| format!("Đã buộc tắt thành công tiến trình '{name}' (PID {pid})"))
                .map_err(format_action_error),
            ActionCommand::OpenLocation(_) => match target.record.exe_path.as_deref() {
                Some(path) => open_location(path)
                    .map(|_| format!("Đã mở thư mục chứa '{name}'"))
                    .map_err(format_action_error),
                None => Err(strings::MSG_NO_EXE_PATH.to_string()),
            },
        }
    }
}

/// Simulated demo action runner that returns dummy success messages without modifying OS state.
#[derive(Debug, Default, Clone, Copy)]
pub struct DemoActions;

impl ActionRunner for DemoActions {
    fn run(&mut self, cmd: ActionCommand, target: &ScoredProcess) -> Result<String, String> {
        let pid = target.record.pid;
        let name = &target.record.name;

        match cmd {
            ActionCommand::GracefulClose(_) => Ok(format!(
                "[Demo] Đã gửi yêu cầu đóng an toàn tới '{name}' (PID {pid})"
            )),
            ActionCommand::ForceKill(_) => Ok(format!(
                "[Demo] Đã buộc tắt tiến trình '{name}' (PID {pid})"
            )),
            ActionCommand::OpenLocation(_) => Ok(format!("[Demo] Đã mở thư mục chứa '{name}'")),
        }
    }
}

/// Converts platform action errors into user-friendly Vietnamese messages.
fn format_action_error(err: ActionError) -> String {
    match err {
        ActionError::AccessDenied => strings::MSG_ACCESS_DENIED.to_string(),
        ActionError::NotAllowed(_) => strings::MSG_SYSTEM_PROTECTED.to_string(),
        ActionError::NoWindow => strings::MSG_NO_WINDOW.to_string(),
        ActionError::NotFound => strings::MSG_NOT_FOUND.to_string(),
        ActionError::Failed(code) => format!("Thao tác thất bại từ Windows (Mã lỗi: {code})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bottleneck, ProcessKind, ProcessRecord, Severity};

    fn make_test_process(pid: u32, name: &str, exe_path: Option<&str>) -> ScoredProcess {
        ScoredProcess {
            record: ProcessRecord {
                pid,
                parent_pid: None,
                name: name.to_string(),
                exe_path: exe_path.map(String::from),
                cpu_percent: 25.0,
                memory_bytes: 1024 * 1024 * 200,
                disk_read_bps: 1000.0,
                disk_write_bps: 2000.0,
            },
            kind: ProcessKind::UserApp,
            score: 75.0,
            severity: Severity::Culprit,
            main_bottleneck: Bottleneck::Cpu,
            killable: true,
        }
    }

    #[test]
    fn test_demo_actions() {
        let mut runner = DemoActions;
        let proc = make_test_process(1234, "test.exe", Some("C:\\test.exe"));

        let res_close = runner.run(ActionCommand::GracefulClose(1234), &proc);
        assert!(res_close.is_ok());
        assert!(res_close.unwrap().contains("[Demo]"));

        let res_kill = runner.run(ActionCommand::ForceKill(1234), &proc);
        assert!(res_kill.is_ok());
        assert!(res_kill.unwrap().contains("[Demo]"));

        let res_open = runner.run(ActionCommand::OpenLocation(1234), &proc);
        assert!(res_open.is_ok());
        assert!(res_open.unwrap().contains("[Demo]"));
    }

    #[test]
    fn test_real_actions_missing_path() {
        let mut runner = RealActions;
        let proc = make_test_process(9999, "nopath.exe", None);

        let res = runner.run(ActionCommand::OpenLocation(9999), &proc);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), strings::MSG_NO_EXE_PATH);
    }

    #[test]
    fn test_format_action_error_variants() {
        assert_eq!(
            format_action_error(ActionError::AccessDenied),
            strings::MSG_ACCESS_DENIED
        );
        assert_eq!(
            format_action_error(ActionError::NotAllowed("sys".to_string())),
            strings::MSG_SYSTEM_PROTECTED
        );
        assert_eq!(
            format_action_error(ActionError::NoWindow),
            strings::MSG_NO_WINDOW
        );
        assert_eq!(
            format_action_error(ActionError::NotFound),
            strings::MSG_NOT_FOUND
        );
        assert_eq!(
            format_action_error(ActionError::Failed(5)),
            "Thao tác thất bại từ Windows (Mã lỗi: 5)"
        );
    }
}
