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

/// Sustained lifecycle state of an overloaded process episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EpisodeState {
    /// Actively exceeding overload threshold (score >= 40.0).
    Overloaded,
    /// Dropped below threshold, currently in cooling period.
    Cooling,
    /// Cooldown expired peacefully without re-overloading.
    Recovered,
    /// Process terminated or exited while being watched.
    Exited,
}

/// Key identifying a watched process across PID reuse.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProcessKey {
    /// Process identifier (0 for group episodes).
    pub pid: u32,
    /// Process executable name.
    pub name: String,
}

impl ProcessKey {
    /// Creates a new `ProcessKey` for an individual process.
    pub fn new(pid: u32, name: impl Into<String>) -> Self {
        Self {
            pid,
            name: name.into(),
        }
    }

    /// Creates a new `ProcessKey` for a process group.
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            pid: 0,
            name: name.into().to_lowercase(),
        }
    }

    /// Returns whether this key identifies a group episode.
    pub fn is_group(&self) -> bool {
        self.pid == 0
    }
}

/// Historical and active record of a sustained process overload episode.
#[derive(Debug, Clone, PartialEq)]
pub struct Episode {
    /// Process identifier (0 for group episodes).
    pub pid: u32,
    /// Process executable name (e.g., "chrome.exe").
    pub name: String,
    /// Number of processes represented in this episode (1 for individual, > 1 for group).
    pub process_count: usize,
    /// Timestamp in Unix epoch milliseconds when overload began (back-dated to first damping tick).
    pub started_at: u64,
    /// Highest smoothed culprit score reached during this episode.
    pub peak_score: f32,
    /// Highest CPU utilization observed during this episode (0.0 to 100.0).
    pub peak_cpu: f32,
    /// Highest memory footprint observed during this episode in bytes.
    pub peak_ram_bytes: u64,
    /// Highest disk throughput observed during this episode in bytes per second.
    pub peak_disk_bps: f64,
    /// Accumulated duration spent in active overload in seconds.
    pub overloaded_seconds: u64,
    /// Timestamp in milliseconds of the latest tick in active overload.
    pub last_overloaded_at: u64,
    /// Current lifecycle state of this episode.
    pub state: EpisodeState,
    /// Process classification kind.
    pub kind: ProcessKind,
    /// Whether this process can be safely terminated.
    pub killable: bool,
    /// Rolling recent culprit score history for mini sparkline visualization.
    pub score_history: Vec<f32>,
}

impl Episode {
    /// Key representing this episode's process or group.
    pub fn key(&self) -> ProcessKey {
        if self.process_count > 1 || self.pid == 0 {
            ProcessKey::group(&self.name)
        } else {
            ProcessKey::new(self.pid, &self.name)
        }
    }

    /// Returns whether this episode represents a group of processes.
    pub fn is_group(&self) -> bool {
        self.process_count > 1 || self.pid == 0
    }

    /// Remaining cooling retention in seconds, or 0 if expired or not in cooling.
    pub fn remaining_cooling_secs(&self, now_ms: u64, retention_secs: u64) -> u64 {
        if self.state != EpisodeState::Cooling {
            return 0;
        }
        let expire_at = self.last_overloaded_at + retention_secs * 1000;
        expire_at.saturating_sub(now_ms) / 1000
    }

    /// Helper alias for remaining cooling retention seconds.
    pub fn cooldown_remaining_secs(&self, now_ms: u64, retention_secs: u64) -> u64 {
        self.remaining_cooling_secs(now_ms, retention_secs)
    }
}

/// Hardware resource experiencing elevated utilization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverloadedResource {
    Cpu,
    Memory,
    Disk,
}

/// Aggregated metrics for a group of processes sharing the same executable name.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessGroupShare {
    /// Executable image name.
    pub name: String,
    /// Count of running process instances in this group.
    pub process_count: usize,
    /// Total consumption of the overloaded resource.
    pub total_metric: f64,
    /// Percentage share of the system total (0.0 to 100.0).
    pub share_percent: f32,
    /// PIDs belonging to this group.
    pub pids: Vec<u32>,
}

/// System-wide breakdown of resource consumption when pressure is elevated.
#[derive(Debug, Clone, PartialEq)]
pub struct PressureBreakdown {
    /// Primary resource experiencing stress.
    pub resource: OverloadedResource,
    /// System-wide value of the overloaded resource.
    pub system_value: f64,
    /// Unit string ("%", " MB/s").
    pub resource_unit: &'static str,
    /// Top process groups by consumption (up to 3).
    pub top_groups: Vec<ProcessGroupShare>,
    /// Unattributed remainder percentage ("Hệ thống/khác").
    pub remainder_share_percent: f32,
    /// Group culprit if a single group accounts for >= 40% of the overloaded resource.
    pub group_culprit: Option<ProcessGroupShare>,
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
    /// Currently watched process episodes (active, cooling, and recently finished).
    pub watched: Vec<Episode>,
    /// Resource consumption breakdown when system pressure is elevated.
    pub breakdown: Option<PressureBreakdown>,
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
