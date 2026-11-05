//! Simulated mock metrics source for testing and demonstration modes.
//!
//! Generates realistic Windows process telemetry across distinct operational
//! scenarios without requiring hardware stress or external dependencies.

use crate::collector::SampleSource;
use crate::model::{ProcessRecord, RawSnapshot, SystemTotals};

/// Simulation scenario defining system load and culprit behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// Normal operation with balanced low utilization across all processes.
    Idle,
    /// Severe CPU thrashing caused by a single runaway process.
    CpuHog,
    /// High disk I/O write saturation choking storage throughput.
    DiskThrash,
    /// Gradual or critical memory leak consuming nearly all physical RAM.
    MemoryLeak,
    /// Demo Phase 1: Calm operation for 8 seconds.
    Calm,
    /// Demo Phase 2: Suspicious early warning (score 40-69) for 15 seconds.
    Suspicious,
    /// Demo Phase 3: Active culprit thrashing system (score >= 70) for 15 seconds.
    Culprit,
    /// Demo Phase 4: Cooldown recovery for 20 seconds.
    Cooling,
}

/// Type alias for backward and forward naming compatibility.
pub type MockScenario = Scenario;

/// Timed operational phase for the demo simulation scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoPhase {
    /// Normal operation with balanced low utilization (8 seconds).
    Calm,
    /// Elevated resource consumption warning (score 40-69) for 15 seconds.
    Suspicious,
    /// Runaway culprit overloading the PC (score >= 70) for 15 seconds.
    Culprit,
    /// Cooldown window watched for 20 seconds.
    Cooling,
}

impl DemoPhase {
    pub const CALM_SECS: u64 = 8;
    pub const SUSPICIOUS_SECS: u64 = 15;
    pub const CULPRIT_SECS: u64 = 15;
    pub const COOLING_SECS: u64 = 20;
    pub const CYCLE_SECS: u64 =
        Self::CALM_SECS + Self::SUSPICIOUS_SECS + Self::CULPRIT_SECS + Self::COOLING_SECS;

    pub fn from_tick(tick: u64) -> (Self, u64) {
        let t = tick % Self::CYCLE_SECS;
        if t < Self::CALM_SECS {
            (Self::Calm, Self::CALM_SECS - t)
        } else if t < Self::CALM_SECS + Self::SUSPICIOUS_SECS {
            (
                Self::Suspicious,
                Self::CALM_SECS + Self::SUSPICIOUS_SECS - t,
            )
        } else if t < Self::CALM_SECS + Self::SUSPICIOUS_SECS + Self::CULPRIT_SECS {
            (
                Self::Culprit,
                Self::CALM_SECS + Self::SUSPICIOUS_SECS + Self::CULPRIT_SECS - t,
            )
        } else {
            (Self::Cooling, Self::CYCLE_SECS - t)
        }
    }

    pub fn name_vi(&self) -> &'static str {
        match self {
            Self::Calm => "Bình thường",
            Self::Suspicious => "Đáng ngờ",
            Self::Culprit => "Thủ phạm",
            Self::Cooling => "Hạ nhiệt",
        }
    }

    pub fn to_scenario(&self) -> Scenario {
        match self {
            Self::Calm => Scenario::Calm,
            Self::Suspicious => Scenario::Suspicious,
            Self::Culprit => Scenario::Culprit,
            Self::Cooling => Scenario::Cooling,
        }
    }
}

/// Linear Congruential Generator for deterministic pseudo-random variations.
#[derive(Debug, Clone)]
pub struct Lcg {
    state: u64,
}

impl Lcg {
    /// Default seed constant.
    pub const DEFAULT_SEED: u64 = 0x5103_5907_1234_5678;

    /// Creates an LCG initialized with a specified seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Generates next 32-bit unsigned integer.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    /// Generates next float in `0.0..1.0`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    /// Generates a float within `[min, max]`.
    pub fn gen_range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MockMode {
    Fixed(Scenario),
    Demo,
}

/// Mock metric generator providing synthetic [`RawSnapshot`] observations.
pub struct MockSource {
    mode: MockMode,
    tick_count: u64,
    lcg: Lcg,
    base_timestamp_ms: u64,
}

impl MockSource {
    /// Base timestamp in milliseconds for synthetic demo timestamps.
    pub const BASE_TIMESTAMP_MS: u64 = 1_700_000_000_000;

    /// Creates a mock source locked to a fixed scenario.
    pub fn new(scenario: Scenario) -> Self {
        Self::with_seed(scenario, Lcg::DEFAULT_SEED)
    }

    /// Creates a mock source with an explicit seed for reproducible variation.
    pub fn with_seed(scenario: Scenario, seed: u64) -> Self {
        Self {
            mode: MockMode::Fixed(scenario),
            tick_count: 0,
            lcg: Lcg::new(seed),
            base_timestamp_ms: Self::BASE_TIMESTAMP_MS,
        }
    }

    /// Creates a demo mock source running the timed 4-phase demo scenario.
    pub fn demo() -> Self {
        Self {
            mode: MockMode::Demo,
            tick_count: 0,
            lcg: Lcg::new(Lcg::DEFAULT_SEED),
            base_timestamp_ms: Self::BASE_TIMESTAMP_MS,
        }
    }

    /// Returns the currently active scenario for this tick.
    pub fn current_scenario(&self) -> Scenario {
        match self.mode {
            MockMode::Fixed(s) => s,
            MockMode::Demo => {
                let (phase, _) = DemoPhase::from_tick(self.tick_count);
                phase.to_scenario()
            }
        }
    }

    /// Returns the current demo phase label and remaining seconds at the given timestamp.
    pub fn demo_phase_at(timestamp_ms: u64) -> (&'static str, u64) {
        let tick = timestamp_ms.saturating_sub(Self::BASE_TIMESTAMP_MS) / 1000;
        let (phase, remaining) = DemoPhase::from_tick(tick);
        (phase.name_vi(), remaining)
    }

    /// Number of samples produced so far.
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }
}

impl Default for MockSource {
    fn default() -> Self {
        Self::demo()
    }
}

impl SampleSource for MockSource {
    fn sample(&mut self) -> RawSnapshot {
        let scenario = self.current_scenario();
        let timestamp_ms = self.base_timestamp_ms + self.tick_count * 1000;
        self.tick_count += 1;

        let total_ram_bytes: u64 = 16 * 1024 * 1024 * 1024; // 16 GB installed RAM

        // ~15 standard Windows processes with realistic metadata
        let mut processes = vec![
            ProcessRecord {
                pid: 104,
                parent_pid: None,
                name: "csrss.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\csrss.exe".to_string()),
                cpu_percent: 0.1,
                memory_bytes: 8 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 108,
                parent_pid: None,
                name: "lsass.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\lsass.exe".to_string()),
                cpu_percent: 0.1,
                memory_bytes: 25 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 112,
                parent_pid: None,
                name: "smss.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\smss.exe".to_string()),
                cpu_percent: 0.0,
                memory_bytes: 4 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 116,
                parent_pid: None,
                name: "services.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\services.exe".to_string()),
                cpu_percent: 0.1,
                memory_bytes: 18 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 120,
                parent_pid: Some(116),
                name: "svchost.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\svchost.exe".to_string()),
                cpu_percent: 0.2,
                memory_bytes: 55 * 1024 * 1024,
                disk_read_bps: 12_000.0,
                disk_write_bps: 4_000.0,
            },
            ProcessRecord {
                pid: 124,
                parent_pid: Some(116),
                name: "svchost.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\svchost.exe".to_string()),
                cpu_percent: 0.1,
                memory_bytes: 35 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 130,
                parent_pid: None,
                name: "dwm.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\dwm.exe".to_string()),
                cpu_percent: 1.2,
                memory_bytes: 140 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 200,
                parent_pid: None,
                name: "explorer.exe".to_string(),
                exe_path: Some("C:\\Windows\\explorer.exe".to_string()),
                cpu_percent: 0.6,
                memory_bytes: 220 * 1024 * 1024,
                disk_read_bps: 8_000.0,
                disk_write_bps: 15_000.0,
            },
            ProcessRecord {
                pid: 300,
                parent_pid: Some(200),
                name: "chrome.exe".to_string(),
                exe_path: Some(
                    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe".to_string(),
                ),
                cpu_percent: 1.5,
                memory_bytes: 420 * 1024 * 1024,
                disk_read_bps: 45_000.0,
                disk_write_bps: 12_000.0,
            },
            ProcessRecord {
                pid: 304,
                parent_pid: Some(300),
                name: "chrome.exe".to_string(),
                exe_path: Some(
                    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe".to_string(),
                ),
                cpu_percent: 0.8,
                memory_bytes: 280 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 400,
                parent_pid: Some(200),
                name: "code.exe".to_string(),
                exe_path: Some(
                    "C:\\Users\\User\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe"
                        .to_string(),
                ),
                cpu_percent: 1.0,
                memory_bytes: 380 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 5_000.0,
            },
            ProcessRecord {
                pid: 404,
                parent_pid: Some(400),
                name: "code.exe".to_string(),
                exe_path: Some(
                    "C:\\Users\\User\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe"
                        .to_string(),
                ),
                cpu_percent: 0.4,
                memory_bytes: 190 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 500,
                parent_pid: Some(200),
                name: "cmd.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\cmd.exe".to_string()),
                cpu_percent: 0.0,
                memory_bytes: 12 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 600,
                parent_pid: Some(200),
                name: "taskmgr.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\Taskmgr.exe".to_string()),
                cpu_percent: 0.5,
                memory_bytes: 65 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            ProcessRecord {
                pid: 700,
                parent_pid: Some(116),
                name: "RuntimeBroker.exe".to_string(),
                exe_path: Some("C:\\Windows\\System32\\RuntimeBroker.exe".to_string()),
                cpu_percent: 0.1,
                memory_bytes: 28 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
        ];

        // Culprit process behavior tailored to current scenario
        let (culprit_proc, system_cpu, system_ram_used) = match scenario {
            Scenario::Idle => {
                let rec = ProcessRecord {
                    pid: 9999,
                    parent_pid: Some(200),
                    name: "test_heavy.exe".to_string(),
                    exe_path: Some("C:\\SlowSpot\\test_heavy.exe".to_string()),
                    cpu_percent: 0.1,
                    memory_bytes: 45 * 1024 * 1024,
                    disk_read_bps: 0.0,
                    disk_write_bps: 0.0,
                };
                (Some(rec), 6.8, 5 * 1024 * 1024 * 1024)
            }
            Scenario::CpuHog => {
                let jitter = self.lcg.gen_range_f32(-0.5, 0.5);
                let rec = ProcessRecord {
                    pid: 9999,
                    parent_pid: Some(200),
                    name: "test_heavy.exe".to_string(),
                    exe_path: Some("C:\\SlowSpot\\test_heavy.exe".to_string()),
                    cpu_percent: (99.0 + jitter).clamp(95.0, 100.0),
                    memory_bytes: 350 * 1024 * 1024,
                    disk_read_bps: 0.0,
                    disk_write_bps: 0.0,
                };
                (Some(rec), 99.2, 6 * 1024 * 1024 * 1024)
            }
            Scenario::DiskThrash => {
                let jitter = self.lcg.gen_range_f32(-2_000_000.0, 2_000_000.0);
                let rec = ProcessRecord {
                    pid: 9999,
                    parent_pid: Some(200),
                    name: "test_heavy.exe".to_string(),
                    exe_path: Some("C:\\SlowSpot\\test_heavy.exe".to_string()),
                    cpu_percent: 2.0,
                    memory_bytes: 260 * 1024 * 1024,
                    disk_read_bps: 0.0,
                    disk_write_bps: (120_000_000.0 + jitter as f64).max(100_000_000.0),
                };
                (Some(rec), 18.5, 6 * 1024 * 1024 * 1024)
            }
            Scenario::MemoryLeak => {
                let rec = ProcessRecord {
                    pid: 9999,
                    parent_pid: Some(200),
                    name: "test_heavy.exe".to_string(),
                    exe_path: Some("C:\\SlowSpot\\test_heavy.exe".to_string()),
                    cpu_percent: 1.5,
                    memory_bytes: 14 * 1024 * 1024 * 1024, // 14 GB footprint
                    disk_read_bps: 0.0,
                    disk_write_bps: 0.0,
                };
                (Some(rec), 14.0, 15 * 1024 * 1024 * 1024) // 15 GB / 16 GB used = 93.75% RAM pressure
            }
            Scenario::Calm => (None, 6.8, 5 * 1024 * 1024 * 1024),
            Scenario::Suspicious => {
                let rec = ProcessRecord {
                    pid: 9999,
                    parent_pid: Some(200),
                    name: "test_heavy.exe".to_string(),
                    exe_path: Some("C:\\SlowSpot\\test_heavy.exe".to_string()),
                    cpu_percent: 47.0,
                    memory_bytes: 45 * 1024 * 1024,
                    disk_read_bps: 0.0,
                    disk_write_bps: 0.0,
                };
                (Some(rec), 48.0, 6 * 1024 * 1024 * 1024)
            }
            Scenario::Culprit => {
                let jitter = self.lcg.gen_range_f32(-0.2, 0.2);
                let rec = ProcessRecord {
                    pid: 9999,
                    parent_pid: Some(200),
                    name: "test_heavy.exe".to_string(),
                    exe_path: Some("C:\\SlowSpot\\test_heavy.exe".to_string()),
                    cpu_percent: (99.1 + jitter).clamp(95.0, 100.0),
                    memory_bytes: 350 * 1024 * 1024,
                    disk_read_bps: 0.0,
                    disk_write_bps: 0.0,
                };
                (Some(rec), 99.2, 6 * 1024 * 1024 * 1024)
            }
            Scenario::Cooling => {
                let rec = ProcessRecord {
                    pid: 9999,
                    parent_pid: Some(200),
                    name: "test_heavy.exe".to_string(),
                    exe_path: Some("C:\\SlowSpot\\test_heavy.exe".to_string()),
                    cpu_percent: 0.2,
                    memory_bytes: 45 * 1024 * 1024,
                    disk_read_bps: 0.0,
                    disk_write_bps: 0.0,
                };
                (Some(rec), 7.5, 5 * 1024 * 1024 * 1024)
            }
        };

        if let Some(proc) = culprit_proc {
            processes.push(proc);
        }

        let total_disk_bps: f64 = processes
            .iter()
            .map(|p| p.disk_read_bps + p.disk_write_bps)
            .sum();

        let totals = SystemTotals {
            cpu_percent: system_cpu,
            memory_used_bytes: system_ram_used,
            memory_total_bytes: total_ram_bytes,
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
    use crate::analyzer::Analyzer;
    use crate::model::{Bottleneck, EpisodeState, Severity};

    #[test]
    fn test_mock_cpu_hog_scenario() {
        let mut source = MockSource::new(Scenario::CpuHog);
        let mut analyzer = Analyzer::default();

        let raw = source.sample();
        let snapshot = analyzer.analyze(raw);

        let top = snapshot.processes.first().expect("must have processes");
        assert_eq!(top.record.name, "test_heavy.exe");
        assert_eq!(top.severity, Severity::Culprit);
        assert_eq!(top.main_bottleneck, Bottleneck::Cpu);
    }

    #[test]
    fn test_mock_disk_thrash_scenario() {
        let mut source = MockSource::new(Scenario::DiskThrash);
        let mut analyzer = Analyzer::default();

        let raw = source.sample();
        let snapshot = analyzer.analyze(raw);

        let top = snapshot.processes.first().expect("must have processes");
        assert_eq!(top.record.name, "test_heavy.exe");
        assert_eq!(top.severity, Severity::Culprit);
        assert_eq!(top.main_bottleneck, Bottleneck::Disk);
    }

    #[test]
    fn test_mock_idle_scenario_has_no_culprits() {
        let mut source = MockSource::new(Scenario::Idle);
        let mut analyzer = Analyzer::default();

        let raw = source.sample();
        let snapshot = analyzer.analyze(raw);

        for p in &snapshot.processes {
            assert_ne!(
                p.severity,
                Severity::Culprit,
                "Process {} should not be marked Culprit in Idle scenario",
                p.record.name
            );
        }
    }

    #[test]
    fn test_mock_system_critical_processes_never_killable() {
        let mut source = MockSource::new(Scenario::CpuHog);
        let mut analyzer = Analyzer::default();

        let raw = source.sample();
        let snapshot = analyzer.analyze(raw);

        let csrss = snapshot
            .processes
            .iter()
            .find(|p| p.record.name == "csrss.exe")
            .expect("csrss.exe must be present");
        assert!(!csrss.killable, "csrss.exe must not be killable");

        let lsass = snapshot
            .processes
            .iter()
            .find(|p| p.record.name == "lsass.exe")
            .expect("lsass.exe must be present");
        assert!(!lsass.killable, "lsass.exe must not be killable");
    }

    #[test]
    fn test_mock_demo_scenario_phases_and_durations() {
        let mut source = MockSource::demo();
        let mut analyzer = Analyzer::default().with_retention(20);

        // 1. Calm phase: 8 ticks (0..8)
        for tick in 0..8 {
            assert_eq!(
                source.current_scenario(),
                Scenario::Calm,
                "Tick {} must be Scenario::Calm",
                tick
            );
            let raw = source.sample();
            let snap = analyzer.analyze(raw);
            assert!(
                snap.watched.is_empty(),
                "Tick {}: no watched episodes expected in Calm, found {}",
                tick,
                snap.watched.len()
            );
            assert!(
                snap.processes
                    .iter()
                    .all(|p| p.severity == Severity::Normal),
                "Tick {}: all processes should be Normal in Calm",
                tick
            );
        }

        // 2. Suspicious phase: 15 ticks (8..23)
        for tick in 8..23 {
            assert_eq!(
                source.current_scenario(),
                Scenario::Suspicious,
                "Tick {} must be Scenario::Suspicious",
                tick
            );
            let raw = source.sample();
            let snap = analyzer.analyze(raw);
            let target = snap
                .processes
                .iter()
                .find(|p| p.record.name == "test_heavy.exe")
                .expect("test_heavy.exe must be present");
            assert_eq!(
                target.severity,
                Severity::Suspicious,
                "Tick {}: test_heavy.exe must be Suspicious (score: {})",
                tick,
                target.score
            );
            assert!(target.score >= 40.0 && target.score < 70.0);
            assert_eq!(snap.watched.len(), 1);
            assert_eq!(snap.watched[0].state, EpisodeState::Overloaded);
        }

        // 3. Culprit phase: 15 ticks (23..38)
        for tick in 23..38 {
            assert_eq!(
                source.current_scenario(),
                Scenario::Culprit,
                "Tick {} must be Scenario::Culprit",
                tick
            );
            let raw = source.sample();
            let snap = analyzer.analyze(raw);
            let target = snap
                .processes
                .iter()
                .find(|p| p.record.name == "test_heavy.exe")
                .expect("test_heavy.exe must be present");
            assert_eq!(
                target.severity,
                Severity::Culprit,
                "Tick {}: test_heavy.exe must be Culprit (score: {})",
                tick,
                target.score
            );
            assert!(target.score >= 70.0);
            assert_eq!(snap.watched.len(), 1);
            assert_eq!(snap.watched[0].state, EpisodeState::Overloaded);
        }

        // 4. Cooling phase: 20 ticks (38..58)
        for tick in 38..58 {
            assert_eq!(
                source.current_scenario(),
                Scenario::Cooling,
                "Tick {} must be Scenario::Cooling",
                tick
            );
            let raw = source.sample();
            let snap = analyzer.analyze(raw);
            assert_eq!(
                snap.watched.len(),
                1,
                "Tick {}: watched episode must be retained",
                tick
            );
            assert!(
                snap.watched[0].state == EpisodeState::Cooling
                    || snap.watched[0].state == EpisodeState::Overloaded,
                "Tick {}: episode state must be Cooling or transitioning",
                tick
            );
        }

        // 5. Expiration after retention
        let raw = source.sample();
        let snap = analyzer.analyze(raw);
        let watched_cooling = snap
            .watched
            .iter()
            .find(|e| e.name == "test_heavy.exe" && e.state == EpisodeState::Cooling);
        assert!(
            watched_cooling.is_none(),
            "Tick 58: episode must no longer be in Cooling (recovered or removed)"
        );
    }
}
