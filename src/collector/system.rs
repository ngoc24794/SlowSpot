//! System metrics collection backed by `sysinfo`.
//!
//! Queries operating system CPU, memory, and disk I/O metrics using the `sysinfo` crate.

use std::time::{Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use crate::collector::SampleSource;
use crate::model::{ProcessRecord, RawSnapshot, SystemTotals};

/// System metrics collector implementation using `sysinfo`.
///
/// Keeps an internal `System` instance to retain state between sampling intervals,
/// enabling rate delta calculations for CPU utilization and disk throughput.
pub struct SysinfoSource {
    system: System,
    last_sample_instant: Option<Instant>,
}

impl SysinfoSource {
    /// Creates a new `SysinfoSource` with an initialized `sysinfo::System`.
    pub fn new() -> Self {
        let mut system = System::new().expect("failed to initialize sysinfo System");
        system.refresh_cpu_usage();
        Self {
            system,
            last_sample_instant: None,
        }
    }
}

impl Default for SysinfoSource {
    fn default() -> Self {
        Self::new()
    }
}

impl SampleSource for SysinfoSource {
    /// Collects a raw snapshot of system and process metrics.
    ///
    /// # Baseline Notice
    /// The first invocation of `sample()` establishes the baseline for rate delta calculations.
    /// Because CPU and Disk rates require elapsed time between two sampling ticks, the baseline
    /// snapshot returns `cpu_percent = 0.0` and `disk_bps = 0.0` for both system totals and
    /// individual process records.
    fn sample(&mut self) -> RawSnapshot {
        let now = Instant::now();
        let is_baseline = self.last_sample_instant.is_none();
        let elapsed_sec = self
            .last_sample_instant
            .map(|prev| now.duration_since(prev).as_secs_f64())
            .unwrap_or(0.0);
        self.last_sample_instant = Some(now);

        // Only refresh CPU, memory, and targeted process specifics (cpu, memory, disk).
        // Excludes user, environment variables, and command-line arguments for efficiency.
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        let refresh_kind = ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_disk_usage();
        self.system
            .refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind);

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let memory_used_bytes = self.system.used_memory();
        let memory_total_bytes = self.system.total_memory();

        if is_baseline {
            let processes: Vec<ProcessRecord> = self
                .system
                .processes()
                .iter()
                .map(|(pid, proc_)| ProcessRecord {
                    pid: pid.as_u32(),
                    parent_pid: proc_.parent().map(|p| p.as_u32()),
                    name: proc_.name().to_string_lossy().to_string(),
                    exe_path: proc_.exe().map(|p| p.to_string_lossy().to_string()),
                    cpu_percent: 0.0,
                    memory_bytes: proc_.memory(),
                    disk_read_bps: 0.0,
                    disk_write_bps: 0.0,
                })
                .collect();

            return RawSnapshot {
                timestamp_ms,
                totals: SystemTotals {
                    cpu_percent: 0.0,
                    memory_used_bytes,
                    memory_total_bytes,
                    disk_bps: 0.0,
                },
                processes,
            };
        }

        let logical_cores = (self.system.cpus().len().max(1)) as f32;
        let mut processes = Vec::with_capacity(self.system.processes().len());
        let mut total_disk_bps = 0.0;

        for (pid, proc_) in self.system.processes() {
            // Normalize process CPU percent across all logical cores (0.0 to 100.0)
            let cpu_percent = (proc_.cpu_usage() / logical_cores).clamp(0.0, 100.0);

            // Compute real-time throughput from disk delta bytes and actual elapsed time
            let disk = proc_.disk_usage();
            let disk_read_bps = if elapsed_sec > 0.0 {
                disk.read_bytes as f64 / elapsed_sec
            } else {
                0.0
            };
            let disk_write_bps = if elapsed_sec > 0.0 {
                disk.written_bytes as f64 / elapsed_sec
            } else {
                0.0
            };

            total_disk_bps += disk_read_bps + disk_write_bps;

            processes.push(ProcessRecord {
                pid: pid.as_u32(),
                parent_pid: proc_.parent().map(|p| p.as_u32()),
                name: proc_.name().to_string_lossy().to_string(),
                exe_path: proc_.exe().map(|p| p.to_string_lossy().to_string()),
                cpu_percent,
                memory_bytes: proc_.memory(),
                disk_read_bps,
                disk_write_bps,
            });
        }

        let totals = SystemTotals {
            cpu_percent: self.system.global_cpu_usage().clamp(0.0, 100.0),
            memory_used_bytes,
            memory_total_bytes,
            disk_bps: total_disk_bps,
        };

        RawSnapshot {
            timestamp_ms,
            totals,
            processes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_sysinfo_source_live_sampling() {
        let mut source = SysinfoSource::new();

        // First sample establishes baseline
        let baseline = source.sample();
        assert_eq!(baseline.totals.cpu_percent, 0.0);
        assert_eq!(baseline.totals.disk_bps, 0.0);
        assert!(baseline.totals.memory_total_bytes > 0);
        for p in &baseline.processes {
            assert_eq!(p.cpu_percent, 0.0);
            assert_eq!(p.disk_read_bps, 0.0);
            assert_eq!(p.disk_write_bps, 0.0);
        }

        // Wait ~200ms and take second sample
        std::thread::sleep(Duration::from_millis(200));
        let second = source.sample();

        assert!(
            !second.processes.is_empty(),
            "Processes should be discovered"
        );

        // The current test process must exist and have memory > 0
        let own_pid = std::process::id();
        let own_process = second.processes.iter().find(|p| p.pid == own_pid);
        assert!(
            own_process.is_some(),
            "Current test runner PID {own_pid} should be present in process list"
        );
        let own_rec = own_process.unwrap();
        assert!(
            own_rec.memory_bytes > 0,
            "Own process memory footprint should be > 0 bytes"
        );
    }
}
