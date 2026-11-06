//! Pressure breakdown and process group aggregation.
//!
//! When system pressure is high (Strained or Overloaded), this module aggregates
//! processes by executable image name (case-insensitive) per overloaded resource.
//! It computes the top 3 process groups, their share of the system total, and the
//! unattributed remainder ("Hệ thống/khác").
//! If a single group accounts for >= 40% of the overloaded resource, it is identified
//! as a GROUP culprit.

use std::collections::HashMap;

use crate::analyzer::scoring::{
    BYTES_PER_MB, CPU_OVERLOAD_PERCENT, CPU_STRAINED_PERCENT, DISK_SATURATION_THRESHOLD_MB_S,
    RAM_OVERLOAD_PERCENT, RAM_STRAINED_PERCENT,
};
use crate::model::{
    OverloadedResource, PressureBreakdown, PressureState, ProcessGroupShare, ProcessRecord,
    SystemTotals,
};

/// Threshold share of the overloaded resource (>= 40%) required to designate a group as a culprit.
pub const GROUP_CULPRIT_THRESHOLD_PERCENT: f32 = 40.0;

/// Determines which hardware resource is currently experiencing the primary overload/strain.
pub fn primary_overloaded_resource(totals: &SystemTotals) -> OverloadedResource {
    let ram_percent = if totals.memory_total_bytes > 0 {
        (totals.memory_used_bytes as f64 / totals.memory_total_bytes as f64) * 100.0
    } else {
        0.0
    };
    let disk_mb_s = totals.disk_bps / BYTES_PER_MB;

    let cpu_ratio = totals.cpu_percent as f64 / CPU_OVERLOAD_PERCENT as f64;
    let ram_ratio = ram_percent / RAM_OVERLOAD_PERCENT;
    let disk_ratio = disk_mb_s / DISK_SATURATION_THRESHOLD_MB_S;

    if cpu_ratio >= ram_ratio && cpu_ratio >= disk_ratio {
        OverloadedResource::Cpu
    } else if ram_ratio >= disk_ratio {
        OverloadedResource::Memory
    } else {
        OverloadedResource::Disk
    }
}

/// Evaluates all resources experiencing elevated pressure (Overloaded or Strained).
pub fn active_overloaded_resources(
    totals: &SystemTotals,
    pressure: PressureState,
) -> Vec<OverloadedResource> {
    let mut list = Vec::new();
    let ram_percent = if totals.memory_total_bytes > 0 {
        (totals.memory_used_bytes as f64 / totals.memory_total_bytes as f64) * 100.0
    } else {
        0.0
    };
    let disk_mb_s = totals.disk_bps / BYTES_PER_MB;

    match pressure {
        PressureState::Overloaded => {
            if totals.cpu_percent >= CPU_OVERLOAD_PERCENT {
                list.push(OverloadedResource::Cpu);
            }
            if ram_percent >= RAM_OVERLOAD_PERCENT {
                list.push(OverloadedResource::Memory);
            }
            if disk_mb_s >= DISK_SATURATION_THRESHOLD_MB_S {
                list.push(OverloadedResource::Disk);
            }
            // Fallback if none reached the strict overload threshold but state is Overloaded
            if list.is_empty() {
                list.push(primary_overloaded_resource(totals));
            }
        }
        PressureState::Strained => {
            if totals.cpu_percent >= CPU_STRAINED_PERCENT {
                list.push(OverloadedResource::Cpu);
            }
            if ram_percent >= RAM_STRAINED_PERCENT {
                list.push(OverloadedResource::Memory);
            }
            if disk_mb_s >= 30.0 {
                list.push(OverloadedResource::Disk);
            }
            if list.is_empty() {
                list.push(primary_overloaded_resource(totals));
            }
        }
        PressureState::Smooth => {}
    }

    list
}

/// Formats the header badge string naming the overloaded resources.
/// Examples: "Quá tải CPU", "Quá tải RAM", "Quá tải đĩa", "Quá tải CPU + RAM".
pub fn format_pressure_badge_text(totals: &SystemTotals, pressure: PressureState) -> String {
    match pressure {
        PressureState::Smooth => "Mượt mà".to_string(),
        PressureState::Overloaded | PressureState::Strained => {
            let prefix = if pressure == PressureState::Overloaded {
                "Quá tải"
            } else {
                "Căng thẳng"
            };

            let resources = active_overloaded_resources(totals, pressure);
            let names: Vec<&'static str> = resources
                .iter()
                .map(|r| match r {
                    OverloadedResource::Cpu => "CPU",
                    OverloadedResource::Memory => "RAM",
                    OverloadedResource::Disk => "đĩa",
                })
                .collect();

            if names.is_empty() {
                format!("{prefix} tài nguyên")
            } else {
                format!("{prefix} {}", names.join(" + "))
            }
        }
    }
}

/// Formats the header badge tooltip detailing actual metric values versus configured thresholds.
pub fn format_pressure_badge_tooltip(totals: &SystemTotals) -> String {
    let ram_percent = if totals.memory_total_bytes > 0 {
        (totals.memory_used_bytes as f64 / totals.memory_total_bytes as f64) * 100.0
    } else {
        0.0
    };
    let disk_mb_s = totals.disk_bps / BYTES_PER_MB;

    format!(
        "CPU: {:.1}% (ngưỡng {:.0}%)\nRAM: {:.1}% (ngưỡng {:.0}%)\nĐĩa: {:.1} MB/s (ngưỡng {:.0} MB/s)",
        totals.cpu_percent,
        CPU_OVERLOAD_PERCENT,
        ram_percent,
        RAM_OVERLOAD_PERCENT,
        disk_mb_s,
        DISK_SATURATION_THRESHOLD_MB_S,
    )
}

/// Aggregates processes by executable image name (case-insensitive) per overloaded resource.
///
/// Returns top 3 groups with process count and share of the system total, plus the
/// unattributed remainder ("Hệ thống/khác").
pub fn compute_breakdown(
    processes: &[ProcessRecord],
    totals: &SystemTotals,
    pressure: PressureState,
) -> Option<PressureBreakdown> {
    if pressure == PressureState::Smooth {
        return None;
    }

    let resource = primary_overloaded_resource(totals);

    let (system_val, unit) = match resource {
        OverloadedResource::Cpu => (totals.cpu_percent as f64, "%"),
        OverloadedResource::Memory => {
            let ram_percent = if totals.memory_total_bytes > 0 {
                (totals.memory_used_bytes as f64 / totals.memory_total_bytes as f64) * 100.0
            } else {
                0.0
            };
            (ram_percent, "%")
        }
        OverloadedResource::Disk => (totals.disk_bps / BYTES_PER_MB, " MB/s"),
    };

    // Group accumulator: normalized_lowercase_name -> (display_name, count, total_metric, pids)
    let mut groups: HashMap<String, (String, usize, f64, Vec<u32>)> = HashMap::new();

    for proc in processes {
        let key = proc.name.to_lowercase();
        let metric = match resource {
            OverloadedResource::Cpu => proc.cpu_percent as f64,
            OverloadedResource::Memory => {
                if totals.memory_total_bytes > 0 {
                    (proc.memory_bytes as f64 / totals.memory_total_bytes as f64) * 100.0
                } else {
                    0.0
                }
            }
            OverloadedResource::Disk => (proc.disk_read_bps + proc.disk_write_bps) / BYTES_PER_MB,
        };

        let entry = groups
            .entry(key)
            .or_insert_with(|| (proc.name.clone(), 0, 0.0, Vec::new()));
        entry.1 += 1;
        entry.2 += metric;
        entry.3.push(proc.pid);
    }

    let mut sorted_groups: Vec<_> = groups.into_values().collect();
    // Sort descending by total_metric
    sorted_groups.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut top_groups = Vec::new();
    let mut sum_shares: f32 = 0.0;

    for (name, count, metric, pids) in sorted_groups.into_iter().take(3) {
        let share_percent = if system_val > 0.0 {
            ((metric / system_val) * 100.0).clamp(0.0, 100.0) as f32
        } else {
            0.0
        };
        sum_shares += share_percent;
        top_groups.push(ProcessGroupShare {
            name,
            process_count: count,
            total_metric: metric,
            share_percent,
            pids,
        });
    }

    let remainder_share_percent = (100.0 - sum_shares).clamp(0.0, 100.0);

    let group_culprit = top_groups
        .first()
        .filter(|g| g.share_percent >= GROUP_CULPRIT_THRESHOLD_PERCENT)
        .cloned();

    Some(PressureBreakdown {
        resource,
        system_value: system_val,
        resource_unit: unit,
        top_groups,
        remainder_share_percent,
        group_culprit,
    })
}

/// Formats a specific banner explanation string from a pressure breakdown.
/// Example: "CPU đang 99.7% — rustc.exe (3 tiến trình) chiếm 52%, chrome.exe (4) 8%, còn lại 40% từ hệ thống/tiến trình khác"
pub fn format_breakdown_banner(bd: &PressureBreakdown) -> String {
    let res_name = match bd.resource {
        OverloadedResource::Cpu => "CPU",
        OverloadedResource::Memory => "RAM",
        OverloadedResource::Disk => "Đĩa",
    };

    let mut parts = Vec::new();
    for (i, g) in bd.top_groups.iter().enumerate() {
        if i == 0 {
            parts.push(format!(
                "{} ({} tiến trình) chiếm {:.0}%",
                g.name, g.process_count, g.share_percent
            ));
        } else {
            parts.push(format!(
                "{} ({}) {:.0}%",
                g.name, g.process_count, g.share_percent
            ));
        }
    }

    parts.push(format!(
        "còn lại {:.0}% từ hệ thống/tiến trình khác",
        bd.remainder_share_percent
    ));

    format!(
        "{res_name} đang {:.1}{} — {}",
        bd.system_value,
        bd.resource_unit,
        parts.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proc(pid: u32, name: &str, cpu: f32, ram_gb: u64, disk_mb: f64) -> ProcessRecord {
        ProcessRecord {
            pid,
            parent_pid: None,
            name: name.to_string(),
            exe_path: None,
            cpu_percent: cpu,
            memory_bytes: ram_gb * 1024 * 1024 * 1024,
            disk_read_bps: disk_mb * BYTES_PER_MB,
            disk_write_bps: 0.0,
        }
    }

    #[test]
    fn test_empty_input() {
        let totals = SystemTotals {
            cpu_percent: 95.0,
            memory_used_bytes: 10 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_bps: 0.0,
        };
        let bd = compute_breakdown(&[], &totals, PressureState::Overloaded)
            .expect("Should compute breakdown");
        assert_eq!(bd.top_groups.len(), 0);
        assert_eq!(bd.remainder_share_percent, 100.0);
        assert!(bd.group_culprit.is_none());
    }

    #[test]
    fn test_aggregation_and_case_insensitivity() {
        let procs = vec![
            make_proc(1, "rustc.exe", 24.0, 1, 0.0),
            make_proc(2, "RUSTC.EXE", 21.0, 1, 0.0),
            make_proc(3, "Rustc.exe", 7.0, 1, 0.0),
            make_proc(4, "chrome.exe", 2.0, 1, 0.0),
            make_proc(5, "Chrome.exe", 2.0, 1, 0.0),
            make_proc(6, "CHROME.EXE", 2.0, 1, 0.0),
            make_proc(7, "chrome.exe", 2.0, 1, 0.0),
        ];

        let totals = SystemTotals {
            cpu_percent: 100.0,
            memory_used_bytes: 8 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_bps: 0.0,
        };

        let bd = compute_breakdown(&procs, &totals, PressureState::Overloaded).unwrap();
        assert_eq!(bd.top_groups.len(), 2);

        // Group 1: rustc.exe with 3 processes, 52% share
        let g1 = &bd.top_groups[0];
        assert_eq!(g1.name.to_lowercase(), "rustc.exe");
        assert_eq!(g1.process_count, 3);
        assert_eq!(g1.total_metric, 52.0);
        assert!((g1.share_percent - 52.0).abs() < 0.1);

        // Group 2: chrome.exe with 4 processes, 8% share
        let g2 = &bd.top_groups[1];
        assert_eq!(g2.name.to_lowercase(), "chrome.exe");
        assert_eq!(g2.process_count, 4);
        assert_eq!(g2.total_metric, 8.0);
        assert!((g2.share_percent - 8.0).abs() < 0.1);

        // Remainder: 100 - (52 + 8) = 40%
        assert!((bd.remainder_share_percent - 40.0).abs() < 0.1);

        // Group culprit exists because 52% >= 40%
        assert!(bd.group_culprit.is_some());
        assert_eq!(bd.group_culprit.unwrap().name.to_lowercase(), "rustc.exe");
    }

    #[test]
    fn test_40_percent_threshold() {
        let totals = SystemTotals {
            cpu_percent: 100.0,
            memory_used_bytes: 8 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_bps: 0.0,
        };

        // Case 1: 39.9% share -> NOT a group culprit
        let procs1 = vec![make_proc(1, "app.exe", 39.9, 1, 0.0)];
        let bd1 = compute_breakdown(&procs1, &totals, PressureState::Overloaded).unwrap();
        assert!(bd1.group_culprit.is_none());

        // Case 2: 40.0% share -> IS a group culprit
        let procs2 = vec![make_proc(2, "app.exe", 40.0, 1, 0.0)];
        let bd2 = compute_breakdown(&procs2, &totals, PressureState::Overloaded).unwrap();
        assert!(bd2.group_culprit.is_some());
        assert_eq!(bd2.group_culprit.unwrap().name, "app.exe");
    }

    #[test]
    fn test_remainder_calc_and_banner_format() {
        let procs = vec![
            make_proc(1, "rustc.exe", 24.0, 1, 0.0),
            make_proc(2, "rustc.exe", 21.0, 1, 0.0),
            make_proc(3, "rustc.exe", 7.0, 1, 0.0),
            make_proc(4, "chrome.exe", 8.0, 1, 0.0),
        ];

        let totals = SystemTotals {
            cpu_percent: 99.7,
            memory_used_bytes: 8 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_bps: 0.0,
        };

        let bd = compute_breakdown(&procs, &totals, PressureState::Overloaded).unwrap();
        let banner = format_breakdown_banner(&bd);
        assert!(banner.contains("CPU đang 99.7%"));
        assert!(banner.contains("rustc.exe (3 tiến trình) chiếm 52%"));
        assert!(banner.contains("chrome.exe (1) 8%"));
        assert!(banner.contains("còn lại 40% từ hệ thống/tiến trình khác"));
    }

    #[test]
    fn test_badge_text_and_tooltip() {
        let t1 = SystemTotals {
            cpu_percent: 95.0,
            memory_used_bytes: 8 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_bps: 0.0,
        };
        assert_eq!(
            format_pressure_badge_text(&t1, PressureState::Overloaded),
            "Quá tải CPU"
        );

        let t2 = SystemTotals {
            cpu_percent: 95.0,
            memory_used_bytes: 96 * 1024 * 1024 * 1024,
            memory_total_bytes: 100 * 1024 * 1024 * 1024,
            disk_bps: 0.0,
        };
        assert_eq!(
            format_pressure_badge_text(&t2, PressureState::Overloaded),
            "Quá tải CPU + RAM"
        );

        let tooltip = format_pressure_badge_tooltip(&t2);
        assert!(tooltip.contains("CPU: 95.0% (ngưỡng 90%)"));
        assert!(tooltip.contains("RAM: 96.0% (ngưỡng 95%)"));
    }
}
