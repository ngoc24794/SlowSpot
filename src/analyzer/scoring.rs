//! Culprit scoring engine and system pressure calculation.
//!
//! Implements the dominant-resource scoring equations specified in `docs/kien-truc.md` Section 3.2:
//! - Dominant-resource model: 70% share from highest saturated resource component,
//!   combined with 30% share from the weighted load across CPU, Disk, and Memory.
//! - Linear saturation CPU scaling with a 5% baseline floor and 65% saturation threshold.
//! - Disk I/O saturation threshold based on standard HDD / SATA SSD speeds (50 MB/s).
//! - Memory pressure active only under elevated system-wide RAM consumption (> 80%).
//! - Spike penalty bonus for rapid resource surges within a 1-second sample interval.

use crate::model::{Bottleneck, PressureState, ProcessRecord, SystemTotals};

/// Standard conversion constant: bytes per megabyte (10^6 bytes per MB in storage metrics).
pub const BYTES_PER_MB: f64 = 1_000_000.0;

/// Disk I/O saturation threshold in MB/s (default 50 MB/s for HDD / SATA SSD).
pub const DISK_SATURATION_THRESHOLD_MB_S: f64 = 50.0;

/// Threshold delta for CPU spike detection (+40% points increase vs previous sample).
pub const SPIKE_CPU_DELTA_THRESHOLD: f32 = 40.0;

/// Threshold delta for Disk I/O spike detection (+30 MB/s increase vs previous sample).
pub const SPIKE_DISK_DELTA_THRESHOLD_MB_S: f64 = 30.0;

/// System CPU utilization threshold for Overloaded state (>= 90%).
pub const CPU_OVERLOAD_PERCENT: f32 = 90.0;

/// System RAM utilization threshold for Overloaded state (>= 95%).
pub const RAM_OVERLOAD_PERCENT: f64 = 95.0;

/// System CPU utilization threshold for Strained state (>= 70%).
pub const CPU_STRAINED_PERCENT: f32 = 70.0;

/// System RAM utilization threshold for Strained state (>= 85%).
pub const RAM_STRAINED_PERCENT: f64 = 85.0;

/// System RAM utilization threshold above which individual process RAM pressure is evaluated (> 80%).
pub const RAM_PRESSURE_SYSTEM_THRESHOLD_PERCENT: f64 = 80.0;

/// Minimum CPU percent floor below which CPU load is considered negligible (5%).
pub const CPU_FLOOR_PERCENT: f32 = 5.0;

/// CPU percent at or above which CPU load component reaches full saturation 100.0 (65%).
pub const CPU_SATURATION_PERCENT: f32 = 65.0;

/// Weight configuration for the culprit score calculation.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreWeights {
    /// Relative weight of CPU load (default 0.40).
    pub cpu: f32,
    /// Relative weight of Disk I/O throughput (default 0.35).
    pub disk: f32,
    /// Relative weight of RAM footprint under system pressure (default 0.25).
    pub ram: f32,
    /// Flat penalty points awarded on sudden spikes (default 15.0).
    pub spike_bonus: f32,
    /// Share allocated to the single dominant saturated resource (default 0.70).
    pub dominant_share: f32,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            cpu: 0.40,
            disk: 0.35,
            ram: 0.25,
            spike_bonus: 15.0,
            dominant_share: 0.70,
        }
    }
}

/// Evaluates the normalized CPU component `f_cpu(C)` in range `0.0..=100.0`.
///
/// Implements linear saturation:
/// - If `C < CPU_FLOOR_PERCENT` (5%), returns `0.0`.
/// - If `C >= CPU_FLOOR_PERCENT`, returns `min(100.0, (C - 5.0) / (65.0 - 5.0) * 100.0)`.
pub fn calculate_f_cpu(cpu_percent: f32) -> f32 {
    if cpu_percent < CPU_FLOOR_PERCENT {
        0.0
    } else {
        let normalized =
            (cpu_percent - CPU_FLOOR_PERCENT) / (CPU_SATURATION_PERCENT - CPU_FLOOR_PERCENT);
        (normalized * 100.0).clamp(0.0, 100.0)
    }
}

/// Evaluates the normalized Disk I/O component `f_disk(D)` in range `0.0..=100.0`.
///
/// Total throughput `D = ReadRate + WriteRate` (in MB/s).
/// Returns `min(100.0, (D / 50.0) * 100.0)`.
pub fn calculate_f_disk(disk_read_bps: f64, disk_write_bps: f64) -> f32 {
    let total_bps = disk_read_bps + disk_write_bps;
    if total_bps <= 0.0 {
        0.0
    } else {
        let mb_s = total_bps / BYTES_PER_MB;
        ((mb_s / DISK_SATURATION_THRESHOLD_MB_S) * 100.0).clamp(0.0, 100.0) as f32
    }
}

/// Evaluates the normalized RAM component `f_ram(M)` in range `0.0..=100.0`.
///
/// Only active when system RAM utilization exceeds 80%.
/// Returns `(M_process / M_system_total) * 100 * 1.5`, clamped to 100.0.
pub fn calculate_f_ram(
    process_memory_bytes: u64,
    memory_used_bytes: u64,
    memory_total_bytes: u64,
) -> f32 {
    if memory_total_bytes == 0 {
        return 0.0;
    }
    let system_ram_percent = (memory_used_bytes as f64 / memory_total_bytes as f64) * 100.0;
    if system_ram_percent <= RAM_PRESSURE_SYSTEM_THRESHOLD_PERCENT {
        0.0
    } else {
        let proc_ram_ratio = process_memory_bytes as f64 / memory_total_bytes as f64;
        (proc_ram_ratio * 100.0 * 1.5).clamp(0.0, 100.0) as f32
    }
}

/// Computes the spike bonus penalty `B_spike`.
///
/// Awards `spike_bonus` if delta CPU > 40% points or delta Disk > 30 MB/s compared to `prev`.
pub fn calculate_spike_bonus(
    current: &ProcessRecord,
    prev: Option<&ProcessRecord>,
    spike_bonus: f32,
) -> f32 {
    let Some(prev) = prev else {
        return 0.0;
    };

    let delta_cpu = current.cpu_percent - prev.cpu_percent;
    let curr_disk_mb_s = (current.disk_read_bps + current.disk_write_bps) / BYTES_PER_MB;
    let prev_disk_mb_s = (prev.disk_read_bps + prev.disk_write_bps) / BYTES_PER_MB;
    let delta_disk = curr_disk_mb_s - prev_disk_mb_s;

    if delta_cpu > SPIKE_CPU_DELTA_THRESHOLD || delta_disk > SPIKE_DISK_DELTA_THRESHOLD_MB_S {
        spike_bonus
    } else {
        0.0
    }
}

/// Calculates the culprit score (0.0 to 100.0) and dominant bottleneck for a process.
///
/// Uses the dominant-resource formula:
/// `S = min(100, dominant_share * max(f) + (1 - dominant_share) * weighted_sum + B_spike)`.
/// `main_bottleneck` corresponds to the largest normalized component `f` (or `None` if `max(f) < 5.0`).
pub fn score_process(
    rec: &ProcessRecord,
    prev_rec: Option<&ProcessRecord>,
    totals: &SystemTotals,
    w: &ScoreWeights,
) -> (f32, Bottleneck) {
    let f_cpu = calculate_f_cpu(rec.cpu_percent);
    let f_disk = calculate_f_disk(rec.disk_read_bps, rec.disk_write_bps);
    let f_ram = calculate_f_ram(
        rec.memory_bytes,
        totals.memory_used_bytes,
        totals.memory_total_bytes,
    );
    let spike = calculate_spike_bonus(rec, prev_rec, w.spike_bonus);

    let max_f = f_cpu.max(f_disk).max(f_ram);
    let weighted_sum = w.cpu * f_cpu + w.disk * f_disk + w.ram * f_ram;
    let dominant_part = w.dominant_share * max_f;
    let weighted_share = (1.0 - w.dominant_share).max(0.0);
    let weighted_part = weighted_share * weighted_sum;

    let raw_score = dominant_part + weighted_part + spike;
    let score = raw_score.clamp(0.0, 100.0);

    // Dominant bottleneck is component with largest f; None if max_f < 5.0
    let bottleneck = if max_f < 5.0 {
        Bottleneck::None
    } else if f_cpu >= f_disk && f_cpu >= f_ram {
        Bottleneck::Cpu
    } else if f_disk >= f_ram {
        Bottleneck::Disk
    } else {
        Bottleneck::Memory
    };

    (score, bottleneck)
}

/// Evaluates the system-wide pressure state based on total CPU and RAM utilization.
pub fn pressure_state(totals: &SystemTotals) -> PressureState {
    let ram_percent = if totals.memory_total_bytes > 0 {
        (totals.memory_used_bytes as f64 / totals.memory_total_bytes as f64) * 100.0
    } else {
        0.0
    };

    if totals.cpu_percent >= CPU_OVERLOAD_PERCENT || ram_percent >= RAM_OVERLOAD_PERCENT {
        PressureState::Overloaded
    } else if totals.cpu_percent >= CPU_STRAINED_PERCENT || ram_percent >= RAM_STRAINED_PERCENT {
        PressureState::Strained
    } else {
        PressureState::Smooth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Severity;

    fn dummy_record(
        pid: u32,
        cpu_percent: f32,
        memory_bytes: u64,
        disk_read_bps: f64,
        disk_write_bps: f64,
    ) -> ProcessRecord {
        ProcessRecord {
            pid,
            parent_pid: None,
            name: "test.exe".to_string(),
            exe_path: None,
            cpu_percent,
            memory_bytes,
            disk_read_bps,
            disk_write_bps,
        }
    }

    fn dummy_totals(cpu_percent: f32, used_gb: u64, total_gb: u64) -> SystemTotals {
        SystemTotals {
            cpu_percent,
            memory_used_bytes: used_gb * 1024 * 1024 * 1024,
            memory_total_bytes: total_gb * 1024 * 1024 * 1024,
            disk_bps: 0.0,
        }
    }

    #[test]
    fn test_idle_process_score_and_bottleneck() {
        let rec = dummy_record(100, 0.2, 50 * 1024 * 1024, 0.0, 0.0);
        let totals = dummy_totals(10.0, 4, 16); // RAM 25% (normal)
        let weights = ScoreWeights::default();

        let (score, bottleneck) = score_process(&rec, None, &totals, &weights);
        assert!(
            score <= 5.0,
            "Idle process score should be <= 5.0, got {score}"
        );
        assert_eq!(score, 0.0);
        assert_eq!(bottleneck, Bottleneck::None);
    }

    #[test]
    fn test_cpu_95_dominant_resource_and_spike() {
        // CPU 95%, Disk 0, RAM normal
        // With linear saturation:
        // C >= 65%: f_cpu(95) = 100.0
        // dominant_share = 0.7 -> dominant_part = 0.7 * 100.0 = 70.0
        // weighted_part = 0.3 * (0.40 * 100.0) = 12.0
        // Total score without spike = 70.0 + 12.0 = 82.0 (>= 70, Severity::Culprit)
        let rec = dummy_record(101, 95.0, 100 * 1024 * 1024, 0.0, 0.0);
        let totals = dummy_totals(95.0, 8, 16); // RAM 50%
        let weights = ScoreWeights::default();

        // 1. Without spike
        let (score_no_spike, bottleneck) = score_process(&rec, None, &totals, &weights);
        assert_eq!(bottleneck, Bottleneck::Cpu);
        assert!(
            score_no_spike >= 70.0,
            "Expected score >= 70 for CPU 95%, got {score_no_spike}"
        );
        assert!(
            (score_no_spike - 82.0).abs() < 0.2,
            "Expected score ~82.0, got {score_no_spike}"
        );
        assert_eq!(
            Severity::from_score(score_no_spike),
            Severity::Culprit,
            "Severity must be Culprit"
        );

        // 2. With spike bonus (surging from 0% to 95% CPU, delta = 95 > 40)
        let prev = dummy_record(101, 0.0, 100 * 1024 * 1024, 0.0, 0.0);
        let (score_with_spike, bottleneck_spike) =
            score_process(&rec, Some(&prev), &totals, &weights);
        assert_eq!(bottleneck_spike, Bottleneck::Cpu);
        assert!(
            (score_with_spike - 97.0).abs() < 0.2,
            "Expected score with spike bonus ~97.0, got {score_with_spike}"
        );
        assert_eq!(Severity::from_score(score_with_spike), Severity::Culprit);
    }

    #[test]
    fn test_disk_heavy_case_bottleneck() {
        // CPU 15%, Disk 80 MB/s
        // f_cpu(15) = (10 / 60) * 100 = 16.6667
        // f_disk(80) = 100.0 (threshold 50 MB/s)
        // max_f = 100.0 -> dominant_part = 0.7 * 100 = 70.0
        // weighted_part = 0.3 * (0.40 * 16.6667 + 0.35 * 100) = 0.3 * 41.6667 = 12.50
        // Total score = 70.0 + 12.50 = 82.50 (>= 75, Bottleneck::Disk)
        let disk_bytes = 80.0 * BYTES_PER_MB;
        let rec = dummy_record(102, 15.0, 100 * 1024 * 1024, disk_bytes, 0.0);
        let totals = dummy_totals(20.0, 6, 16);
        let weights = ScoreWeights::default();

        let (score, bottleneck) = score_process(&rec, None, &totals, &weights);
        assert_eq!(
            bottleneck,
            Bottleneck::Disk,
            "Dominant bottleneck should be Disk"
        );
        assert!(
            score >= 75.0,
            "Expected score >= 75 for Disk 80 MB/s, got {score}"
        );
        assert!(
            (score - 82.50).abs() < 0.2,
            "Expected score ~82.50, got {score}"
        );
        assert_eq!(Severity::from_score(score), Severity::Culprit);

        // With spike bonus from 0 disk to 80 MB/s (delta = 80 > 30)
        let prev = dummy_record(102, 15.0, 100 * 1024 * 1024, 0.0, 0.0);
        let (score_spike, _) = score_process(&rec, Some(&prev), &totals, &weights);
        assert!(score_spike >= 75.0);
        assert!(
            (score_spike - 97.50).abs() < 0.2,
            "Expected score with spike ~97.50, got {score_spike}"
        );
    }

    #[test]
    fn test_cpu_calibration_targets() {
        let totals = dummy_totals(50.0, 8, 16);
        let weights = ScoreWeights::default();

        // 1. CPU 67% => ~82 (Culprit)
        let rec_67 = dummy_record(201, 67.0, 100 * 1024 * 1024, 0.0, 0.0);
        let (score_67, b_67) = score_process(&rec_67, None, &totals, &weights);
        assert_eq!(b_67, Bottleneck::Cpu);
        assert!(
            (score_67 - 82.0).abs() < 1.0,
            "Expected CPU 67% score ~82, got {score_67}"
        );
        assert_eq!(Severity::from_score(score_67), Severity::Culprit);

        // 2. CPU 50% => ~61 (Suspicious)
        let rec_50 = dummy_record(202, 50.0, 100 * 1024 * 1024, 0.0, 0.0);
        let (score_50, b_50) = score_process(&rec_50, None, &totals, &weights);
        assert_eq!(b_50, Bottleneck::Cpu);
        assert!(
            (score_50 - 61.5).abs() < 1.0,
            "Expected CPU 50% score ~61.5, got {score_50}"
        );
        assert_eq!(Severity::from_score(score_50), Severity::Suspicious);

        // 3. CPU 25% => ~27 (Normal)
        let rec_25 = dummy_record(203, 25.0, 100 * 1024 * 1024, 0.0, 0.0);
        let (score_25, b_25) = score_process(&rec_25, None, &totals, &weights);
        assert_eq!(b_25, Bottleneck::Cpu);
        assert!(
            (score_25 - 27.33).abs() < 1.0,
            "Expected CPU 25% score ~27.33, got {score_25}"
        );
        assert_eq!(Severity::from_score(score_25), Severity::Normal);

        // 4. CPU 8% => < 10
        let rec_8 = dummy_record(204, 8.0, 100 * 1024 * 1024, 0.0, 0.0);
        let (score_8, _) = score_process(&rec_8, None, &totals, &weights);
        assert!(score_8 < 10.0, "Expected CPU 8% score < 10, got {score_8}");
        assert_eq!(Severity::from_score(score_8), Severity::Normal);

        // 5. CPU 95% => >= 85 (or >= 82 without spike, >= 85 with spike)
        let rec_95 = dummy_record(205, 95.0, 100 * 1024 * 1024, 0.0, 0.0);
        let (score_95, _) = score_process(&rec_95, None, &totals, &weights);
        assert!(
            score_95 >= 80.0,
            "Expected CPU 95% score >= 80, got {score_95}"
        );
        let prev_0 = dummy_record(205, 0.0, 100 * 1024 * 1024, 0.0, 0.0);
        let (score_95_spike, _) = score_process(&rec_95, Some(&prev_0), &totals, &weights);
        assert!(
            score_95_spike >= 85.0,
            "Expected CPU 95% with spike score >= 85, got {score_95_spike}"
        );
    }

    #[test]
    fn test_ram_pressure_activation() {
        let weights = ScoreWeights::default();
        let rec = dummy_record(103, 0.0, 8 * 1024 * 1024 * 1024, 0.0, 0.0); // 8 GB

        // Case 1: System RAM <= 80% (12 / 16 = 75%) -> f_ram should be 0
        let totals_normal = dummy_totals(10.0, 12, 16);
        let (score_normal, b_normal) = score_process(&rec, None, &totals_normal, &weights);
        assert_eq!(score_normal, 0.0);
        assert_eq!(b_normal, Bottleneck::None);

        // Case 2: System RAM > 80% (14 / 16 = 87.5%) -> f_ram active
        // proc_ratio = 8 / 16 = 0.5 -> f_ram = 0.5 * 100 * 1.5 = 75.0
        // dominant_part = 0.7 * 75.0 = 52.5
        // weighted_part = 0.3 * (0.25 * 75.0) = 5.625
        // Total score = 52.5 + 5.625 = 58.125
        let totals_high = dummy_totals(10.0, 14, 16);
        let (score_high, b_high) = score_process(&rec, None, &totals_high, &weights);
        assert!(
            (score_high - 58.13).abs() < 0.2,
            "Expected score ~58.13, got {score_high}"
        );
        assert_eq!(b_high, Bottleneck::Memory);
        assert_eq!(Severity::from_score(score_high), Severity::Suspicious);
    }

    #[test]
    fn test_pressure_state_transitions() {
        // Smooth
        let s1 = dummy_totals(50.0, 8, 16); // CPU 50%, RAM 50%
        assert_eq!(pressure_state(&s1), PressureState::Smooth);

        // Strained by CPU >= 70%
        let s2 = dummy_totals(75.0, 8, 16); // CPU 75%, RAM 50%
        assert_eq!(pressure_state(&s2), PressureState::Strained);

        // Strained by RAM >= 85%
        let s3 = dummy_totals(30.0, 14, 16); // CPU 30%, RAM 87.5%
        assert_eq!(pressure_state(&s3), PressureState::Strained);

        // Overloaded by CPU >= 90%
        let s4 = dummy_totals(92.0, 8, 16);
        assert_eq!(pressure_state(&s4), PressureState::Overloaded);

        // Overloaded by RAM >= 95%
        let s5 = dummy_totals(30.0, 96, 100);
        assert_eq!(pressure_state(&s5), PressureState::Overloaded);
    }
}
