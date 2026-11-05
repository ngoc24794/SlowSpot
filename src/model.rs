//! Data models and schemas for SlowSpot.
//!
//! Defines shared data structures, system state enums, process records,
//! metric snapshots, and action commands used across modules.

/// Represents a single process metric observation at a given sampling tick.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRecord {
    /// Process identifier assigned by Windows.
    pub pid: u32,
    /// Parent process identifier if available.
    pub parent_pid: Option<u32>,
    /// Process executable name (e.g., "chrome.exe").
    pub name: String,
    /// Absolute path to the executable image if accessible.
    pub exe_path: Option<String>,
    /// CPU utilization normalized across all logical CPU cores (0.0 to 100.0).
    pub cpu_percent: f32,
    /// Memory working set footprint in bytes.
    pub memory_bytes: u64,
    /// Disk read throughput in bytes per second.
    pub disk_read_bps: f64,
    /// Disk write throughput in bytes per second.
    pub disk_write_bps: f64,
}

/// Global system resource consumption metrics at a sampling tick.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemTotals {
    /// Total system-wide CPU utilization (0.0 to 100.0).
    pub cpu_percent: f32,
    /// Total memory used across system in bytes.
    pub memory_used_bytes: u64,
    /// Total physical installed memory in bytes.
    pub memory_total_bytes: u64,
    /// Total system disk throughput in bytes per second.
    pub disk_bps: f64,
}

/// Raw metrics snapshot produced by the collector layer before domain analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSnapshot {
    /// Timestamp in Unix epoch milliseconds.
    pub timestamp_ms: u64,
    /// System-wide total metrics.
    pub totals: SystemTotals,
    /// List of sampled process records.
    pub processes: Vec<ProcessRecord>,
}

/// Classification category of a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessKind {
    /// Standard user-facing application (e.g., notepad.exe, chrome.exe).
    UserApp,
    /// Windows or 3rd-party background daemon/service.
    BackgroundService,
    /// Essential Windows kernel or critical OS process that must never be terminated.
    SystemCritical,
}

/// Culprit severity ranking for a scored process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Process resource usage is within normal operational parameters (score < 40).
    Normal,
    /// Process exhibits elevated resource consumption requiring observation (40 <= score < 70).
    Suspicious,
    /// Process is an active culprit causing PC lag or thrashing (score >= 70).
    Culprit,
}

impl Severity {
    /// Determines the severity ranking from a numerical culprit score (0.0 to 100.0).
    pub fn from_score(score: f32) -> Self {
        if score >= 70.0 {
            Self::Culprit
        } else if score >= 40.0 {
            Self::Suspicious
        } else {
            Self::Normal
        }
    }
}

/// Overall system resource stress state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PressureState {
    /// Resources are healthy and responsive.
    Smooth,
    /// System is under notable load (CPU >= 70% or RAM >= 85%).
    Strained,
    /// System is experiencing critical stress (CPU >= 90% or RAM >= 95%).
    Overloaded,
}

/// Dominant hardware bottleneck resource consumed by a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bottleneck {
    /// CPU processing capacity.
    Cpu,
    /// Disk I/O throughput / storage bandwidth.
    Disk,
    /// Physical memory footprint under system memory pressure.
    Memory,
    /// No significant bottleneck identified.
    None,
}

/// Fully analyzed and scored process record ready for presentation.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredProcess {
    /// Original process metrics.
    pub record: ProcessRecord,
    /// Categorical classification.
    pub kind: ProcessKind,
    /// Smoothed culprit score (0.0 to 100.0).
    pub score: f32,
    /// Culprit severity classification.
    pub severity: Severity,
    /// Primary hardware resource bottleneck.
    pub main_bottleneck: Bottleneck,
    /// Whether the user is permitted to terminate this process safely.
    pub killable: bool,
}

/// Final system snapshot packaged by the analyzer and sent to the UI.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemSnapshot {
    /// Snapshot capture timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// System-wide total metrics.
    pub totals: SystemTotals,
    /// Evaluated system pressure state.
    pub pressure: PressureState,
    /// Process list sorted by smoothed culprit score descending.
    pub processes: Vec<ScoredProcess>,
}

/// User action requested from the UI to be executed by the platform layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionCommand {
    /// Gracefully request the process window to close (WM_CLOSE).
    GracefulClose(u32),
    /// Forcibly terminate the process immediately (TerminateProcess).
    ForceKill(u32),
    /// Open the directory containing the process executable in File Explorer.
    OpenLocation(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_from_score() {
        assert_eq!(Severity::from_score(0.0), Severity::Normal);
        assert_eq!(Severity::from_score(39.99), Severity::Normal);
        assert_eq!(Severity::from_score(40.0), Severity::Suspicious);
        assert_eq!(Severity::from_score(69.99), Severity::Suspicious);
        assert_eq!(Severity::from_score(70.0), Severity::Culprit);
        assert_eq!(Severity::from_score(100.0), Severity::Culprit);
    }
}
