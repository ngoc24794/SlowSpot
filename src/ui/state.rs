//! Pure presentation state management for SlowSpot UI.
//!
//! Decoupled from `egui` and platform APIs, this module handles metric history,
//! search filtering, process list stabilization (damping), culprit selection,
//! pending action timers, and user-facing notifications.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::model::{Bottleneck, ProcessKind, ScoredProcess, Severity, SystemSnapshot};

/// Maximum number of historical metric samples retained for sparklines (60 seconds).
pub const MAX_HISTORY_POINTS: usize = 60;

/// Number of consecutive snapshot updates before the process table order is refreshed.
pub const DAMPING_TICK_THRESHOLD: usize = 3;

/// Single point in time capturing system-wide resource utilization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemMetricPoint {
    /// Overall CPU utilization percentage (0.0 to 100.0).
    pub cpu_percent: f32,
    /// Overall physical RAM utilization percentage (0.0 to 100.0).
    pub memory_percent: f32,
    /// System-wide disk throughput in bytes per second.
    pub disk_bps: f64,
}

/// Table columns available for user sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortColumn {
    #[default]
    Score,
    Name,
    Pid,
    Cpu,
    Memory,
    Disk,
    Kind,
}

/// Sort order direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

/// Ephemeral toast notification presented in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    /// Message content displayed to the user.
    pub message: String,
    /// Whether the message indicates an error or warning.
    pub is_error: bool,
    /// Time when the notification was created.
    pub created_at: Instant,
    /// Display lifetime before fading.
    pub duration: Duration,
}

impl Toast {
    /// Checks whether this notification is still within its active duration.
    pub fn is_active(&self, now: Instant) -> bool {
        now.duration_since(self.created_at) < self.duration
    }
}

/// Pure presentation state for the SlowSpot dashboard.
#[derive(Debug, Clone)]
pub struct ViewState {
    /// Most recent snapshot received from the collector/analyzer pipeline.
    pub latest_snapshot: Option<SystemSnapshot>,
    /// Rolling window of the last 60 system metric points.
    pub history: Vec<SystemMetricPoint>,
    /// Search filter string for process name or PID.
    pub search_query: String,
    /// Active table sort column.
    pub sort_column: SortColumn,
    /// Active table sort direction.
    pub sort_direction: SortDirection,
    /// Stable process list preserved between damping sort cycles.
    pub display_processes: Vec<ScoredProcess>,
    /// Number of snapshot updates processed since the last table re-sort.
    pub ticks_since_sort: usize,
    /// Set of process names explicitly ignored by the user (stored lowercase).
    pub ignored_names: HashSet<String>,
    /// Pending graceful close operations mapped by PID to their initiation timestamp.
    pub pending_graceful: HashMap<u32, Instant>,
    /// Active toast notification, if any.
    pub toast: Option<Toast>,
    /// Timestamp until which the cooldown notice ("Đã hạ nhiệt") remains visible.
    pub cooldown_until: Option<Instant>,
    /// Tracks whether the previous system state was under stress (had culprits or strained pressure).
    pub was_under_stress: bool,
    /// Whether real-time display updates are currently frozen (paused) by the user.
    pub is_paused: bool,
    /// Pending snapshot received while display was paused, to be ingested upon resume.
    pub paused_snapshot: Option<SystemSnapshot>,
    /// Currently selected process PID for details and keyboard navigation.
    pub selected_pid: Option<u32>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewState {
    /// Creates an empty initial presentation view state.
    pub fn new() -> Self {
        Self {
            latest_snapshot: None,
            history: Vec::with_capacity(MAX_HISTORY_POINTS),
            search_query: String::new(),
            sort_column: SortColumn::Score,
            sort_direction: SortDirection::Descending,
            display_processes: Vec::new(),
            ticks_since_sort: 0,
            ignored_names: HashSet::new(),
            pending_graceful: HashMap::new(),
            toast: None,
            cooldown_until: None,
            was_under_stress: false,
            is_paused: false,
            paused_snapshot: None,
            selected_pid: None,
        }
    }

    /// Ingests a new system snapshot and updates historical metrics, cooldown alerts,
    /// and the damped stable process list.
    ///
    /// # Process Order Stabilization:
    /// To prevent the process table from jittering and frustrating user interactions,
    /// the order of processes is NOT recalculated every single second. Instead:
    /// 1. Metrics for existing rows are updated in-place without shifting their position.
    /// 2. New processes are appended to the bottom; dead processes are removed.
    /// 3. A full re-sort only occurs every `N=3` snapshots, or immediately when the user
    ///    changes sort column / direction.
    pub fn push_snapshot(&mut self, snapshot: SystemSnapshot, now: Instant) {
        // If display is frozen, store incoming snapshot in buffer and return immediately
        if self.is_paused {
            self.paused_snapshot = Some(snapshot);
            return;
        }

        // 1. Record rolling history point (capped at 60)
        let mem_pct = if snapshot.totals.memory_total_bytes > 0 {
            (snapshot.totals.memory_used_bytes as f32 / snapshot.totals.memory_total_bytes as f32)
                * 100.0
        } else {
            0.0
        };

        self.history.push(SystemMetricPoint {
            cpu_percent: snapshot.totals.cpu_percent,
            memory_percent: mem_pct,
            disk_bps: snapshot.totals.disk_bps,
        });

        if self.history.len() > MAX_HISTORY_POINTS {
            self.history.remove(0);
        }

        // 2. Cooldown detection: triggers when moving from stress to smooth
        let has_culprit = snapshot
            .processes
            .iter()
            .any(|p| !self.is_ignored(&p.record.name) && p.severity != Severity::Normal);
        let is_under_stress =
            has_culprit || snapshot.pressure != crate::model::PressureState::Smooth;

        if self.was_under_stress && !is_under_stress {
            // PC was under stress and has now recovered
            self.cooldown_until = Some(now + Duration::from_secs(10));
        } else if is_under_stress {
            // Still under stress, cancel cooldown notice
            self.cooldown_until = None;
        }
        self.was_under_stress = is_under_stress;

        // 3. Process list stabilization update
        self.ticks_since_sort += 1;
        let should_resort =
            self.display_processes.is_empty() || self.ticks_since_sort >= DAMPING_TICK_THRESHOLD;

        if should_resort {
            let mut list = snapshot.processes.clone();
            sort_processes(&mut list, self.sort_column, self.sort_direction);
            self.display_processes = list;
            self.ticks_since_sort = 0;
        } else {
            // Update existing display rows in-place to avoid visual jumps
            let new_lookup: HashMap<u32, &ScoredProcess> = snapshot
                .processes
                .iter()
                .map(|p| (p.record.pid, p))
                .collect();

            // Retain living processes and update their metrics in-place
            self.display_processes
                .retain(|p| new_lookup.contains_key(&p.record.pid));
            for p in &mut self.display_processes {
                if let Some(updated) = new_lookup.get(&p.record.pid) {
                    *p = (*updated).clone();
                }
            }

            // Append newly spawned processes at the bottom
            let existing_pids: HashSet<u32> = self
                .display_processes
                .iter()
                .map(|p| p.record.pid)
                .collect();
            for p in &snapshot.processes {
                if !existing_pids.contains(&p.record.pid) {
                    self.display_processes.push(p.clone());
                }
            }
        }

        // 4. Prune completed pending actions
        let live_pids: HashSet<u32> = snapshot.processes.iter().map(|p| p.record.pid).collect();
        self.pending_graceful
            .retain(|pid, _| live_pids.contains(pid));

        self.latest_snapshot = Some(snapshot);
    }

    /// Toggles between live real-time updates and frozen display mode.
    pub fn toggle_pause(&mut self, now: Instant) {
        self.is_paused = !self.is_paused;
        if !self.is_paused {
            if let Some(snap) = self.paused_snapshot.take() {
                self.push_snapshot(snap, now);
            }
        }
    }

    /// Sets pause state directly.
    pub fn set_paused(&mut self, paused: bool, now: Instant) {
        if self.is_paused != paused {
            self.toggle_pause(now);
        }
    }

    /// Selects the specified process PID.
    pub fn select_pid(&mut self, pid: Option<u32>) {
        self.selected_pid = pid;
    }

    /// Selects the next process in the filtered list (navigating Down with arrow keys).
    pub fn select_next_process(&mut self) {
        let procs = self.filtered_processes();
        if procs.is_empty() {
            self.selected_pid = None;
            return;
        }

        let next_pid = match self.selected_pid {
            None => procs[0].record.pid,
            Some(pid) => match procs.iter().position(|p| p.record.pid == pid) {
                Some(idx) => {
                    let next_idx = (idx + 1).min(procs.len() - 1);
                    procs[next_idx].record.pid
                }
                None => procs[0].record.pid,
            },
        };
        self.selected_pid = Some(next_pid);
    }

    /// Selects the previous process in the filtered list (navigating Up with arrow keys).
    pub fn select_prev_process(&mut self) {
        let procs = self.filtered_processes();
        if procs.is_empty() {
            self.selected_pid = None;
            return;
        }

        let prev_pid = match self.selected_pid {
            None => procs.last().unwrap().record.pid,
            Some(pid) => match procs.iter().position(|p| p.record.pid == pid) {
                Some(idx) => {
                    let prev_idx = idx.saturating_sub(1);
                    procs[prev_idx].record.pid
                }
                None => procs[0].record.pid,
            },
        };
        self.selected_pid = Some(prev_pid);
    }

    /// Updates the sorting column. If the same column is chosen again, toggles direction.
    /// Immediately triggers an orderly re-sort of the process list.
    pub fn set_sort(&mut self, col: SortColumn) {
        if self.sort_column == col {
            self.sort_direction = match self.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_column = col;
            self.sort_direction = match col {
                SortColumn::Name | SortColumn::Pid => SortDirection::Ascending,
                _ => SortDirection::Descending,
            };
        }

        sort_processes(
            &mut self.display_processes,
            self.sort_column,
            self.sort_direction,
        );
        self.ticks_since_sort = 0;
    }

    /// Adds a process name to the ignore set (case-insensitive).
    pub fn ignore_process(&mut self, name: &str) {
        self.ignored_names.insert(name.to_lowercase());
    }

    /// Removes a process name from the ignore set.
    pub fn unignore_process(&mut self, name: &str) {
        self.ignored_names.remove(&name.to_lowercase());
    }

    /// Checks if a process name is currently ignored.
    pub fn is_ignored(&self, name: &str) -> bool {
        self.ignored_names.contains(&name.to_lowercase())
    }

    /// Returns the primary active culprit from the latest snapshot.
    ///
    /// The culprit is the first process ordered by score that is neither ignored
    /// nor classified as `Severity::Normal`. Returns `None` if the system is healthy.
    pub fn top_culprit(&self) -> Option<&ScoredProcess> {
        let snapshot = self.latest_snapshot.as_ref()?;
        snapshot
            .processes
            .iter()
            .find(|p| !self.is_ignored(&p.record.name) && p.severity != Severity::Normal)
    }

    /// Records that a graceful close signal (`WM_CLOSE`) was sent to the given PID.
    pub fn record_graceful_close(&mut self, pid: u32, now: Instant) {
        self.pending_graceful.insert(pid, now);
    }

    /// Checks whether the user is permitted to perform a forced kill on the given PID.
    ///
    /// Returns true only if:
    /// 1. A graceful close was initiated at least 5 seconds ago, AND
    /// 2. The process is still alive in the latest snapshot, AND
    /// 3. The process is killable (not a protected system process).
    pub fn can_force_kill(&self, pid: u32, now: Instant) -> bool {
        let Some(&sent_at) = self.pending_graceful.get(&pid) else {
            return false;
        };

        if now.duration_since(sent_at) < Duration::from_secs(5) {
            return false;
        }

        let Some(snapshot) = &self.latest_snapshot else {
            return false;
        };

        snapshot
            .processes
            .iter()
            .find(|p| p.record.pid == pid)
            .map(|p| p.killable)
            .unwrap_or(false)
    }

    /// Returns the remaining countdown seconds until forced kill becomes available.
    pub fn graceful_wait_remaining_secs(&self, pid: u32, now: Instant) -> Option<u64> {
        let sent_at = self.pending_graceful.get(&pid)?;
        let elapsed = now.duration_since(*sent_at).as_secs();
        if elapsed < 5 {
            Some(5 - elapsed)
        } else {
            Some(0)
        }
    }

    /// Displays a toast notification in the UI.
    pub fn add_toast(&mut self, message: String, is_error: bool, now: Instant, duration: Duration) {
        self.toast = Some(Toast {
            message,
            is_error,
            created_at: now,
            duration,
        });
    }

    /// Retrieves the currently active toast notification, if not expired.
    pub fn active_toast(&self, now: Instant) -> Option<&Toast> {
        self.toast.as_ref().filter(|t| t.is_active(now))
    }

    /// Checks whether the "PC đã hạ nhiệt" banner should be shown.
    pub fn is_cooldown_active(&self, now: Instant) -> bool {
        self.cooldown_until.map(|exp| now < exp).unwrap_or(false)
    }

    /// Returns the filtered process list matching the current search query.
    pub fn filtered_processes(&self) -> Vec<&ScoredProcess> {
        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            self.display_processes.iter().collect()
        } else {
            self.display_processes
                .iter()
                .filter(|p| {
                    p.record.name.to_lowercase().contains(&query)
                        || p.record.pid.to_string().contains(&query)
                })
                .collect()
        }
    }
}

/// Sorts a slice of scored processes in-place according to column and direction.
fn sort_processes(list: &mut [ScoredProcess], col: SortColumn, dir: SortDirection) {
    list.sort_by(|a, b| {
        let cmp = match col {
            SortColumn::Score => a
                .score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortColumn::Name => a
                .record
                .name
                .to_lowercase()
                .cmp(&b.record.name.to_lowercase()),
            SortColumn::Pid => a.record.pid.cmp(&b.record.pid),
            SortColumn::Cpu => a
                .record
                .cpu_percent
                .partial_cmp(&b.record.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortColumn::Memory => a.record.memory_bytes.cmp(&b.record.memory_bytes),
            SortColumn::Disk => {
                let disk_a = a.record.disk_read_bps + a.record.disk_write_bps;
                let disk_b = b.record.disk_read_bps + b.record.disk_write_bps;
                disk_a
                    .partial_cmp(&disk_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
            SortColumn::Kind => kind_rank(a.kind).cmp(&kind_rank(b.kind)),
        };

        match dir {
            SortDirection::Ascending => cmp,
            SortDirection::Descending => cmp.reverse(),
        }
    });
}

fn kind_rank(kind: ProcessKind) -> u8 {
    match kind {
        ProcessKind::SystemCritical => 0,
        ProcessKind::BackgroundService => 1,
        ProcessKind::UserApp => 2,
    }
}

/// Formats a human-readable Vietnamese explanation for a process bottleneck.
pub fn format_bottleneck_explanation(process: &ScoredProcess) -> String {
    match process.main_bottleneck {
        Bottleneck::Cpu => format!("chiếm {:.0}% CPU", process.record.cpu_percent),
        Bottleneck::Disk => {
            let total_bps = process.record.disk_read_bps + process.record.disk_write_bps;
            format!("đọc/ghi đĩa {}", format_bps(total_bps))
        }
        Bottleneck::Memory => format!("dùng {} RAM", format_bytes(process.record.memory_bytes)),
        Bottleneck::None => "tiêu thụ tài nguyên tổng hợp".to_string(),
    }
}

/// Formats byte quantities into human-readable units (B, KB, MB, GB).
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Formats byte throughput per second into human-readable units (B/s, KB/s, MB/s, GB/s).
pub fn format_bps(bps: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;

    if bps >= GB {
        format!("{:.1} GB/s", bps / GB)
    } else if bps >= MB {
        format!("{:.1} MB/s", bps / MB)
    } else if bps >= KB {
        format!("{:.1} KB/s", bps / KB)
    } else {
        format!("{:.0} B/s", bps.max(0.0))
    }
}

/// Formats a percentage value with 1 decimal place.
pub fn format_percent(pct: f32) -> String {
    format!("{:.1}%", pct.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PressureState, ProcessRecord, SystemTotals};

    fn make_dummy_process(pid: u32, name: &str, score: f32, severity: Severity) -> ScoredProcess {
        ScoredProcess {
            record: ProcessRecord {
                pid,
                parent_pid: None,
                name: name.to_string(),
                exe_path: Some(format!("C:\\{name}")),
                cpu_percent: 45.0,
                memory_bytes: 1024 * 1024 * 512,
                disk_read_bps: 1024.0 * 500.0,
                disk_write_bps: 1024.0 * 500.0,
            },
            kind: ProcessKind::UserApp,
            score,
            severity,
            main_bottleneck: Bottleneck::Cpu,
            killable: true,
        }
    }

    fn make_snapshot(processes: Vec<ScoredProcess>, pressure: PressureState) -> SystemSnapshot {
        SystemSnapshot {
            timestamp_ms: 1000,
            totals: SystemTotals {
                cpu_percent: 50.0,
                memory_used_bytes: 4 * 1024 * 1024 * 1024,
                memory_total_bytes: 16 * 1024 * 1024 * 1024,
                disk_bps: 1024.0 * 1024.0 * 10.0,
            },
            pressure,
            processes,
        }
    }

    #[test]
    fn test_keyboard_row_navigation() {
        let mut state = ViewState::new();
        let now = Instant::now();

        // 1. Empty list
        assert_eq!(state.selected_pid, None);
        state.select_next_process();
        assert_eq!(state.selected_pid, None);
        state.select_prev_process();
        assert_eq!(state.selected_pid, None);

        // 2. Populate 3 processes
        let procs = vec![
            make_dummy_process(101, "proc1.exe", 80.0, Severity::Culprit),
            make_dummy_process(202, "proc2.exe", 50.0, Severity::Suspicious),
            make_dummy_process(303, "proc3.exe", 20.0, Severity::Normal),
        ];
        state.push_snapshot(make_snapshot(procs, PressureState::Smooth), now);

        // Down from None -> selects first (101)
        state.select_next_process();
        assert_eq!(state.selected_pid, Some(101));

        // Down from 101 -> selects 202
        state.select_next_process();
        assert_eq!(state.selected_pid, Some(202));

        // Down from 202 -> selects 303
        state.select_next_process();
        assert_eq!(state.selected_pid, Some(303));

        // Down from last (303) -> clamped at 303
        state.select_next_process();
        assert_eq!(state.selected_pid, Some(303));

        // Up from 303 -> selects 202
        state.select_prev_process();
        assert_eq!(state.selected_pid, Some(202));

        // Up from 202 -> selects 101
        state.select_prev_process();
        assert_eq!(state.selected_pid, Some(101));

        // Up from first (101) -> clamped at 101
        state.select_prev_process();
        assert_eq!(state.selected_pid, Some(101));

        // Direct select_pid
        state.select_pid(Some(202));
        assert_eq!(state.selected_pid, Some(202));
        state.select_pid(None);
        assert_eq!(state.selected_pid, None);
    }

    #[test]
    fn test_pause_resume_freeze_display_snapshot() {
        let mut state = ViewState::new();
        let now = Instant::now();

        // 1. Initial push while live
        let snap1 = make_snapshot(
            vec![make_dummy_process(
                1,
                "snap1.exe",
                60.0,
                Severity::Suspicious,
            )],
            PressureState::Smooth,
        );
        state.push_snapshot(snap1, now);
        assert_eq!(
            state.latest_snapshot.as_ref().unwrap().processes[0]
                .record
                .pid,
            1
        );
        assert_eq!(state.display_processes[0].record.name, "snap1.exe");
        assert!(!state.is_paused);

        // 2. Pause display
        state.set_paused(true, now);
        assert!(state.is_paused);

        // 3. New snapshot arrives while paused
        let snap2 = make_snapshot(
            vec![make_dummy_process(2, "snap2.exe", 95.0, Severity::Culprit)],
            PressureState::Overloaded,
        );
        state.push_snapshot(snap2, now + Duration::from_secs(1));

        // Display remains FROZEN at snap1
        assert_eq!(
            state.latest_snapshot.as_ref().unwrap().processes[0]
                .record
                .pid,
            1,
            "display snapshot must be frozen while paused"
        );
        assert_eq!(
            state.display_processes[0].record.name, "snap1.exe",
            "display processes must remain frozen while paused"
        );
        assert!(state.paused_snapshot.is_some());

        // 4. Resume display
        state.set_paused(false, now + Duration::from_secs(2));
        assert!(!state.is_paused);

        // Display updates to flushed pending snapshot (snap2)
        assert_eq!(
            state.latest_snapshot.as_ref().unwrap().processes[0]
                .record
                .pid,
            2,
            "display snapshot must update after resuming"
        );
        assert_eq!(state.display_processes[0].record.name, "snap2.exe");
    }

    #[test]
    fn test_formatting_bytes_and_bps() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(1024 * 1024 * 15), "15.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 3), "3.0 GB");

        assert_eq!(format_bps(100.0), "100 B/s");
        assert_eq!(format_bps(2048.0), "2.0 KB/s");
        assert_eq!(format_bps(1024.0 * 1024.0 * 45.5), "45.5 MB/s");
        assert_eq!(format_bps(1024.0 * 1024.0 * 1024.0 * 1.5), "1.5 GB/s");

        assert_eq!(format_percent(12.34), "12.3%");
    }

    #[test]
    fn test_format_bottleneck_explanation() {
        let mut p = make_dummy_process(1, "heavy.exe", 90.0, Severity::Culprit);
        p.main_bottleneck = Bottleneck::Cpu;
        p.record.cpu_percent = 88.0;
        assert_eq!(format_bottleneck_explanation(&p), "chiếm 88% CPU");

        p.main_bottleneck = Bottleneck::Disk;
        p.record.disk_read_bps = 50.0 * 1024.0 * 1024.0;
        p.record.disk_write_bps = 70.0 * 1024.0 * 1024.0;
        assert_eq!(format_bottleneck_explanation(&p), "đọc/ghi đĩa 120.0 MB/s");

        p.main_bottleneck = Bottleneck::Memory;
        p.record.memory_bytes = (4.2 * 1024.0 * 1024.0 * 1024.0) as u64;
        assert_eq!(format_bottleneck_explanation(&p), "dùng 4.2 GB RAM");

        p.main_bottleneck = Bottleneck::None;
        assert_eq!(
            format_bottleneck_explanation(&p),
            "tiêu thụ tài nguyên tổng hợp"
        );
    }

    #[test]
    fn test_history_capped_at_60() {
        let mut state = ViewState::new();
        let now = Instant::now();

        for i in 0..100 {
            let snap = make_snapshot(vec![], PressureState::Smooth);
            state.push_snapshot(snap, now + Duration::from_secs(i));
        }

        assert_eq!(state.history.len(), 60);
    }

    #[test]
    fn test_filtering() {
        let mut state = ViewState::new();
        let now = Instant::now();

        let procs = vec![
            make_dummy_process(100, "chrome.exe", 80.0, Severity::Culprit),
            make_dummy_process(200, "code.exe", 30.0, Severity::Normal),
            make_dummy_process(300, "system_idle", 10.0, Severity::Normal),
        ];

        state.push_snapshot(make_snapshot(procs, PressureState::Smooth), now);
        assert_eq!(state.filtered_processes().len(), 3);

        state.search_query = "CHROME".to_string();
        let filtered = state.filtered_processes();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].record.name, "chrome.exe");

        state.search_query = "200".to_string();
        let filtered_pid = state.filtered_processes();
        assert_eq!(filtered_pid.len(), 1);
        assert_eq!(filtered_pid[0].record.pid, 200);

        state.search_query = "nonexistent".to_string();
        assert!(state.filtered_processes().is_empty());
    }

    #[test]
    fn test_top_culprit_and_ignore() {
        let mut state = ViewState::new();
        let now = Instant::now();

        let procs = vec![
            make_dummy_process(100, "bad_app.exe", 95.0, Severity::Culprit),
            make_dummy_process(200, "suspicious.exe", 60.0, Severity::Suspicious),
            make_dummy_process(300, "healthy.exe", 10.0, Severity::Normal),
        ];

        state.push_snapshot(make_snapshot(procs, PressureState::Strained), now);

        // Initially bad_app.exe is the top culprit
        let top = state.top_culprit().expect("expected top culprit");
        assert_eq!(top.record.name, "bad_app.exe");

        // Ignore bad_app.exe -> top culprit switches to suspicious.exe
        state.ignore_process("bad_app.exe");
        let top = state.top_culprit().expect("expected next culprit");
        assert_eq!(top.record.name, "suspicious.exe");

        // Ignore suspicious.exe -> top culprit is None because healthy.exe is Normal
        state.ignore_process("SUSPICIOUS.EXE");
        assert!(state.top_culprit().is_none());

        // Unignore bad_app.exe -> returns again
        state.unignore_process("bad_app.exe");
        assert_eq!(state.top_culprit().unwrap().record.name, "bad_app.exe");
    }

    #[test]
    fn test_ordering_stability_damping() {
        let mut state = ViewState::new();
        let now = Instant::now();

        // Initial snapshot: [pid 1 (score 90), pid 2 (score 50)]
        let s1 = make_snapshot(
            vec![
                make_dummy_process(1, "p1.exe", 90.0, Severity::Culprit),
                make_dummy_process(2, "p2.exe", 50.0, Severity::Suspicious),
            ],
            PressureState::Strained,
        );
        state.push_snapshot(s1, now);
        assert_eq!(state.display_processes[0].record.pid, 1);
        assert_eq!(state.display_processes[1].record.pid, 2);

        // Snapshot 2 (tick 1): p2 score jumps to 99, but order should NOT change yet!
        let s2 = make_snapshot(
            vec![
                make_dummy_process(2, "p2.exe", 99.0, Severity::Culprit),
                make_dummy_process(1, "p1.exe", 90.0, Severity::Culprit),
            ],
            PressureState::Strained,
        );
        state.push_snapshot(s2, now + Duration::from_secs(1));
        assert_eq!(
            state.display_processes[0].record.pid, 1,
            "order must remain damped/stable on tick 1"
        );
        assert_eq!(
            state.display_processes[1].record.pid, 2,
            "order must remain damped/stable on tick 1"
        );
        assert_eq!(state.display_processes[1].score, 99.0);

        // Snapshot 3 (tick 2): still stable
        let s3 = make_snapshot(
            vec![
                make_dummy_process(2, "p2.exe", 99.0, Severity::Culprit),
                make_dummy_process(1, "p1.exe", 90.0, Severity::Culprit),
            ],
            PressureState::Strained,
        );
        state.push_snapshot(s3, now + Duration::from_secs(2));
        assert_eq!(state.display_processes[0].record.pid, 1);

        // Snapshot 4 (tick 3): threshold reached (DAMPING_TICK_THRESHOLD = 3), order re-sorts!
        let s4 = make_snapshot(
            vec![
                make_dummy_process(2, "p2.exe", 99.0, Severity::Culprit),
                make_dummy_process(1, "p1.exe", 90.0, Severity::Culprit),
            ],
            PressureState::Strained,
        );
        state.push_snapshot(s4, now + Duration::from_secs(3));
        assert_eq!(
            state.display_processes[0].record.pid, 2,
            "resort must occur at tick 3"
        );
    }

    #[test]
    fn test_force_kill_condition_5s() {
        let mut state = ViewState::new();
        let now = Instant::now();

        let proc = make_dummy_process(42, "target.exe", 85.0, Severity::Culprit);
        state.push_snapshot(make_snapshot(vec![proc], PressureState::Strained), now);

        // Not initiated yet
        assert!(!state.can_force_kill(42, now));

        // Graceful close requested
        state.record_graceful_close(42, now);

        // At 3 seconds: not ready
        assert!(!state.can_force_kill(42, now + Duration::from_secs(3)));
        assert_eq!(
            state.graceful_wait_remaining_secs(42, now + Duration::from_secs(3)),
            Some(2)
        );

        // At exactly 5 seconds: ready
        assert!(state.can_force_kill(42, now + Duration::from_secs(5)));
        assert_eq!(
            state.graceful_wait_remaining_secs(42, now + Duration::from_secs(5)),
            Some(0)
        );

        // If target was SystemCritical (!killable): never allowed
        let mut protected = make_dummy_process(99, "csrss.exe", 85.0, Severity::Culprit);
        protected.killable = false;
        state.push_snapshot(make_snapshot(vec![protected], PressureState::Strained), now);
        state.record_graceful_close(99, now);
        assert!(!state.can_force_kill(99, now + Duration::from_secs(10)));
    }

    #[test]
    fn test_cooldown_transition() {
        let mut state = ViewState::new();
        let now = Instant::now();

        // 1. Initial smooth: no cooldown
        let s1 = make_snapshot(vec![], PressureState::Smooth);
        state.push_snapshot(s1, now);
        assert!(!state.is_cooldown_active(now));

        // 2. High stress
        let s2 = make_snapshot(
            vec![make_dummy_process(1, "hog.exe", 90.0, Severity::Culprit)],
            PressureState::Overloaded,
        );
        state.push_snapshot(s2, now + Duration::from_secs(1));
        assert!(!state.is_cooldown_active(now + Duration::from_secs(1)));

        // 3. Stress relieved (recovered to smooth) -> cooldown notice active
        let s3 = make_snapshot(vec![], PressureState::Smooth);
        state.push_snapshot(s3, now + Duration::from_secs(2));
        assert!(state.is_cooldown_active(now + Duration::from_secs(2)));
        assert!(state.is_cooldown_active(now + Duration::from_secs(11)));
        assert!(!state.is_cooldown_active(now + Duration::from_secs(13)));
    }
}
