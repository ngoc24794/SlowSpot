//! Metrics analysis and culprit scoring engine.
//!
//! Evaluates collected process metrics, applies safety filters against critical
//! Windows processes, calculates culprit scores, detects sudden resource spikes,
//! and tracks sustained overload episodes.

pub mod breakdown;
pub mod damping;
pub mod safety;
pub mod scoring;
pub mod tracker;

use std::collections::{HashMap, HashSet};

pub use breakdown::{
    compute_breakdown, format_breakdown_banner, format_pressure_badge_text,
    format_pressure_badge_tooltip, primary_overloaded_resource,
};
pub use damping::Damping;
pub use safety::{classify, is_killable};
pub use scoring::{pressure_state, score_process, ScoreWeights};
pub use tracker::OverloadTracker;

use crate::model::{RawSnapshot, ScoredProcess, Severity, SystemSnapshot};

/// Orchestrator for metrics analysis, safety classification, and score smoothing.
#[derive(Debug, Clone)]
pub struct Analyzer {
    weights: ScoreWeights,
    damping: Damping,
    tracker: OverloadTracker,
    prev_processes: HashMap<u32, crate::model::ProcessRecord>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new(ScoreWeights::default(), damping::DEFAULT_DAMPING_WINDOW)
    }
}

impl Analyzer {
    /// Creates a new `Analyzer` with configured score weights and rolling damping window size.
    pub fn new(weights: ScoreWeights, damping_window: usize) -> Self {
        Self {
            weights,
            damping: Damping::new(damping_window),
            tracker: OverloadTracker::new(tracker::DEFAULT_RETENTION_SECS, damping_window),
            prev_processes: HashMap::new(),
        }
    }

    /// Accessor for current score weights.
    pub fn weights(&self) -> &ScoreWeights {
        &self.weights
    }

    /// Accessor for overload tracker.
    pub fn tracker(&self) -> &OverloadTracker {
        &self.tracker
    }

    /// Mutable accessor for overload tracker.
    pub fn tracker_mut(&mut self) -> &mut OverloadTracker {
        &mut self.tracker
    }

    /// Configures watch retention cooldown duration in seconds.
    pub fn with_retention(mut self, retention_secs: u64) -> Self {
        self.tracker.set_retention_secs(retention_secs);
        self
    }

    /// Transforms a [`RawSnapshot`] into a fully analyzed [`SystemSnapshot`].
    ///
    /// Pipeline steps:
    /// 1. Excludes PID 0 (System Idle Process) and current application PID.
    /// 2. Classifies process kind (UserApp, BackgroundService, SystemCritical).
    /// 3. Computes culprit score and bottleneck against current metrics and previous cycle.
    /// 4. Records candidate sample in overload tracker for accurate episode back-dating.
    /// 5. Applies rolling window damping average per PID.
    /// 6. Derives severity level and killability.
    /// 7. Sorts processes descending by smoothed culprit score.
    /// 8. Updates overload episode lifecycle.
    /// 9. Computes overall system pressure state.
    pub fn analyze(&mut self, raw: RawSnapshot) -> SystemSnapshot {
        let own_pid = std::process::id();
        let pressure = scoring::pressure_state(&raw.totals);

        // In demo mode (when test_heavy.exe is present), enforce demo retention duration of 20 seconds
        if raw.processes.iter().any(|p| p.name == "test_heavy.exe")
            && self.tracker.retention_secs() != 20
        {
            self.tracker.set_retention_secs(20);
        }

        let mut scored_processes = Vec::new();
        let mut current_processes = HashMap::new();
        let mut active_pids = HashSet::new();

        for rec in raw.processes {
            // Exclude PID 0 (System Idle Process) and own process ID
            if rec.pid == 0 || rec.pid == own_pid {
                continue;
            }

            active_pids.insert(rec.pid);
            let prev_rec = self.prev_processes.get(&rec.pid);
            let (raw_score, bottleneck) =
                scoring::score_process(&rec, prev_rec, &raw.totals, &self.weights);

            self.tracker
                .record_sample(rec.pid, &rec.name, raw.timestamp_ms, raw_score);

            let smoothed_score = self.damping.smooth(rec.pid, raw_score);
            let kind = safety::classify(&rec.name);
            let killable = safety::is_killable(kind);
            let severity = Severity::from_score(smoothed_score);

            scored_processes.push(ScoredProcess {
                record: rec.clone(),
                kind,
                score: smoothed_score,
                severity,
                main_bottleneck: bottleneck,
                killable,
            });

            current_processes.insert(rec.pid, rec);
        }

        // Clean up historical damping states for terminated processes
        self.damping.retain_pids(&active_pids);
        self.prev_processes = current_processes;

        // Sort descending by smoothed score. Ties are deterministically resolved by PID.
        scored_processes.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.record.pid.cmp(&b.record.pid))
        });

        let raw_records: Vec<_> = scored_processes.iter().map(|s| s.record.clone()).collect();
        let breakdown = breakdown::compute_breakdown(&raw_records, &raw.totals, pressure);

        // Update monitored overload episodes (including group episodes if group culprit exists)
        let watched = self.tracker.update_with_breakdown(
            raw.timestamp_ms,
            &scored_processes,
            breakdown.as_ref(),
        );

        SystemSnapshot {
            timestamp_ms: raw.timestamp_ms,
            totals: raw.totals,
            pressure,
            processes: scored_processes,
            watched,
            breakdown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bottleneck, ProcessKind, ProcessRecord, SystemTotals};

    fn make_proc(pid: u32, name: &str, cpu: f32, disk_mb: f64) -> ProcessRecord {
        ProcessRecord {
            pid,
            parent_pid: None,
            name: name.to_string(),
            exe_path: None,
            cpu_percent: cpu,
            memory_bytes: 100 * 1024 * 1024,
            disk_read_bps: disk_mb * scoring::BYTES_PER_MB,
            disk_write_bps: 0.0,
        }
    }

    #[test]
    fn test_analyzer_excludes_pid_zero_and_own_pid() {
        let mut analyzer = Analyzer::default();
        let own_pid = std::process::id();

        let raw = RawSnapshot {
            timestamp_ms: 1000,
            totals: SystemTotals {
                cpu_percent: 50.0,
                memory_used_bytes: 8 * 1024 * 1024 * 1024,
                memory_total_bytes: 16 * 1024 * 1024 * 1024,
                disk_bps: 0.0,
            },
            processes: vec![
                make_proc(0, "System Idle Process", 80.0, 0.0),
                make_proc(own_pid, "slowspot.exe", 10.0, 0.0),
                make_proc(1000, "chrome.exe", 40.0, 0.0),
            ],
        };

        let snapshot = analyzer.analyze(raw);
        assert_eq!(snapshot.processes.len(), 1);
        assert_eq!(snapshot.processes[0].record.pid, 1000);
        assert_eq!(snapshot.processes[0].record.name, "chrome.exe");
    }

    #[test]
    fn test_analyzer_sorting_descending_by_smoothed_score() {
        let mut analyzer = Analyzer::default();

        let raw = RawSnapshot {
            timestamp_ms: 1000,
            totals: SystemTotals {
                cpu_percent: 60.0,
                memory_used_bytes: 8 * 1024 * 1024 * 1024,
                memory_total_bytes: 16 * 1024 * 1024 * 1024,
                disk_bps: 0.0,
            },
            processes: vec![
                make_proc(100, "light_app.exe", 6.0, 0.0),
                make_proc(200, "heavy_disk.exe", 10.0, 60.0),
                make_proc(300, "idle_app.exe", 0.0, 0.0),
                make_proc(400, "critical_sys.exe", 80.0, 0.0),
            ],
        };

        let snapshot = analyzer.analyze(raw);

        // Verify sorted in strictly descending order
        for w in snapshot.processes.windows(2) {
            assert!(
                w[0].score >= w[1].score,
                "Process list must be sorted descending by smoothed score"
            );
        }

        // Verify highest score process is at index 0
        assert!(snapshot.processes[0].score > snapshot.processes[1].score);
    }

    #[test]
    fn test_analyzer_classifies_and_marks_killable_correctly() {
        let mut analyzer = Analyzer::default();

        let raw = RawSnapshot {
            timestamp_ms: 1000,
            totals: SystemTotals {
                cpu_percent: 30.0,
                memory_used_bytes: 8 * 1024 * 1024 * 1024,
                memory_total_bytes: 16 * 1024 * 1024 * 1024,
                disk_bps: 0.0,
            },
            processes: vec![
                make_proc(1001, "csrss.exe", 5.0, 0.0),
                make_proc(1002, "lsass.exe", 5.0, 0.0),
                make_proc(1003, "explorer.exe", 5.0, 0.0),
                make_proc(1004, "svchost.exe", 5.0, 0.0),
                make_proc(1005, "notepad.exe", 5.0, 0.0),
            ],
        };

        let snapshot = analyzer.analyze(raw);

        for p in &snapshot.processes {
            match p.record.name.as_str() {
                "csrss.exe" | "lsass.exe" | "explorer.exe" => {
                    assert_eq!(p.kind, ProcessKind::SystemCritical);
                    assert!(!p.killable, "{} must not be killable", p.record.name);
                }
                "svchost.exe" => {
                    assert_eq!(p.kind, ProcessKind::BackgroundService);
                    assert!(p.killable, "svchost.exe can be marked killable");
                }
                "notepad.exe" => {
                    assert_eq!(p.kind, ProcessKind::UserApp);
                    assert!(p.killable, "notepad.exe must be killable");
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_analyzer_spike_detection_between_cycles() {
        let mut analyzer = Analyzer::default();
        let totals = SystemTotals {
            cpu_percent: 50.0,
            memory_used_bytes: 8 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_bps: 0.0,
        };

        // Cycle 1: low load (cpu 5%)
        let raw1 = RawSnapshot {
            timestamp_ms: 1000,
            totals: totals.clone(),
            processes: vec![make_proc(2000, "app.exe", 5.0, 0.0)],
        };
        let snap1 = analyzer.analyze(raw1);
        let score1 = snap1.processes[0].score;
        assert_eq!(score1, 0.0);

        // Cycle 2: sudden surge to 80% CPU (delta = 75 > 40)
        // raw score includes spike bonus (+15.0)
        let raw2 = RawSnapshot {
            timestamp_ms: 2000,
            totals,
            processes: vec![make_proc(2000, "app.exe", 80.0, 0.0)],
        };
        let snap2 = analyzer.analyze(raw2);
        let score2 = snap2.processes[0].score;

        // Raw score for 80% CPU with spike:
        // f_cpu(80) = ((75/95)^1.2)*100 ~= 75.33
        // dominant_part = 0.7 * 75.33 ~= 52.73
        // weighted_part = 0.3 * (0.4 * 75.33) ~= 9.04
        // plus spike 15.0 -> raw score ~= 76.77
        // Damping window of 3: average of [0.0, 76.77] ~= 38.38
        assert!(
            (score2 - 48.5).abs() < 0.2,
            "Expected smoothed score with damped spike ~48.5, got {score2}"
        );
        assert_eq!(snap2.processes[0].main_bottleneck, Bottleneck::Cpu);
    }
}
