//! Process overload episode tracking and lifecycle management.
//!
//! Monitored episodes track sustained resource thrashing across time, ensuring that
//! problematic processes remain visible and accountable during cooldown windows
//! rather than flickering out immediately upon a momentary dip.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::model::{Episode, EpisodeState, ScoredProcess, Severity};

/// Default retention cooldown duration in seconds.
pub const DEFAULT_RETENTION_SECS: u64 = 120;
/// Maximum number of active/recent watched episodes retained for display.
pub const MAX_WATCHED_ENTRIES: usize = 20;
/// Maximum number of completed historical episodes retained.
pub const MAX_FINISHED_EPISODES: usize = 10;
/// Maximum number of score points retained in episode sparkline history.
pub const MAX_SCORE_HISTORY: usize = 30;

/// Pure domain tracker managing the lifecycle of process overload episodes.
#[derive(Debug, Clone)]
pub struct OverloadTracker {
    retention_secs: u64,
    damping_window: usize,
    /// Recent raw score samples and timestamps per (PID, process name).
    recent_ticks: HashMap<(u32, String), VecDeque<(u64, f32)>>,
    /// Active and recent watched episodes keyed by (PID, process name).
    episodes: HashMap<(u32, String), Episode>,
    /// Last 10 finished episodes (Recovered or Exited).
    finished: VecDeque<Episode>,
    /// Timestamp in milliseconds of the previous evaluation tick.
    prev_timestamp_ms: Option<u64>,
}

impl Default for OverloadTracker {
    fn default() -> Self {
        Self::new(DEFAULT_RETENTION_SECS, 3)
    }
}

impl OverloadTracker {
    /// Creates a new `OverloadTracker` with specified retention duration and damping window.
    pub fn new(retention_secs: u64, damping_window: usize) -> Self {
        Self {
            retention_secs: retention_secs.max(1),
            damping_window: damping_window.max(1),
            recent_ticks: HashMap::new(),
            episodes: HashMap::new(),
            finished: VecDeque::with_capacity(MAX_FINISHED_EPISODES),
            prev_timestamp_ms: None,
        }
    }

    /// Configured retention cooldown duration in seconds.
    pub fn retention_secs(&self) -> u64 {
        self.retention_secs
    }

    /// Sets the retention cooldown duration in seconds.
    pub fn set_retention_secs(&mut self, secs: u64) {
        self.retention_secs = secs.max(1);
    }

    /// Records an observed raw score sample for a process before smoothing.
    ///
    /// Buffers timestamps across the damping window to allow episode start times
    /// to be accurately back-dated to the initial moment sustained overload began.
    pub fn record_sample(&mut self, pid: u32, name: &str, timestamp_ms: u64, raw_score: f32) {
        let key = (pid, name.to_string());
        let deque = self.recent_ticks.entry(key).or_default();
        if deque.len() >= self.damping_window {
            deque.pop_front();
        }
        deque.push_back((timestamp_ms, raw_score));
    }

    /// Evaluates current scored processes at injected timestamp `now_ms` and updates episodes.
    pub fn update(&mut self, now_ms: u64, scored_processes: &[ScoredProcess]) -> Vec<Episode> {
        self.update_with_breakdown(now_ms, scored_processes, None)
    }

    /// Evaluates current scored processes and optional system pressure breakdown at injected timestamp `now_ms`.
    pub fn update_with_breakdown(
        &mut self,
        now_ms: u64,
        scored_processes: &[ScoredProcess],
        breakdown: Option<&crate::model::PressureBreakdown>,
    ) -> Vec<Episode> {
        let elapsed_tick_secs = self
            .prev_timestamp_ms
            .map(|prev| (now_ms.saturating_sub(prev)) / 1000)
            .unwrap_or(1)
            .max(1);

        let mut current_keys: HashSet<(u32, String)> = HashSet::new();
        let mut current_pids: HashSet<u32> = HashSet::new();
        let mut newly_finished: Vec<Episode> = Vec::new();

        for proc in scored_processes {
            let pid = proc.record.pid;
            let name = &proc.record.name;
            let key = (pid, name.clone());
            current_keys.insert(key.clone());
            current_pids.insert(pid);

            let is_overloaded = proc.severity >= Severity::Suspicious;
            let total_disk_bps = proc.record.disk_read_bps + proc.record.disk_write_bps;

            if is_overloaded {
                let start_time = calculate_start_time(&self.recent_ticks, &key, now_ms);

                if let Some(existing) = self.episodes.get_mut(&key) {
                    // Already tracked process continues overload
                    match existing.state {
                        EpisodeState::Overloaded => {
                            existing.last_overloaded_at = now_ms;
                            existing.overloaded_seconds = existing
                                .overloaded_seconds
                                .saturating_add(elapsed_tick_secs);
                            existing.peak_score = existing.peak_score.max(proc.score);
                            existing.peak_cpu = existing.peak_cpu.max(proc.record.cpu_percent);
                            existing.peak_ram_bytes =
                                existing.peak_ram_bytes.max(proc.record.memory_bytes);
                            existing.peak_disk_bps = existing.peak_disk_bps.max(total_disk_bps);
                            push_score(&mut existing.score_history, proc.score);
                        }
                        EpisodeState::Cooling => {
                            // Process overloads again during Cooling -> continue the SAME episode without resetting started_at
                            existing.state = EpisodeState::Overloaded;
                            existing.last_overloaded_at = now_ms;
                            existing.overloaded_seconds = existing
                                .overloaded_seconds
                                .saturating_add(elapsed_tick_secs);
                            existing.peak_score = existing.peak_score.max(proc.score);
                            existing.peak_cpu = existing.peak_cpu.max(proc.record.cpu_percent);
                            existing.peak_ram_bytes =
                                existing.peak_ram_bytes.max(proc.record.memory_bytes);
                            existing.peak_disk_bps = existing.peak_disk_bps.max(total_disk_bps);
                            push_score(&mut existing.score_history, proc.score);
                        }
                        EpisodeState::Recovered | EpisodeState::Exited => {
                            // Finished episode: start fresh new episode
                            let initial_overloaded_secs =
                                (now_ms.saturating_sub(start_time) / 1000).max(1);

                            *existing = Episode {
                                pid,
                                name: name.clone(),
                                process_count: 1,
                                started_at: start_time,
                                peak_score: proc.score,
                                peak_cpu: proc.record.cpu_percent,
                                peak_ram_bytes: proc.record.memory_bytes,
                                peak_disk_bps: total_disk_bps,
                                overloaded_seconds: initial_overloaded_secs,
                                last_overloaded_at: now_ms,
                                state: EpisodeState::Overloaded,
                                kind: proc.kind,
                                killable: proc.killable,
                                score_history: vec![proc.score],
                            };
                        }
                    }
                } else {
                    // New sustained overload detected: back-date start time to first damping tick
                    let initial_overloaded_secs = (now_ms.saturating_sub(start_time) / 1000).max(1);

                    let ep = Episode {
                        pid,
                        name: name.clone(),
                        process_count: 1,
                        started_at: start_time,
                        peak_score: proc.score,
                        peak_cpu: proc.record.cpu_percent,
                        peak_ram_bytes: proc.record.memory_bytes,
                        peak_disk_bps: total_disk_bps,
                        overloaded_seconds: initial_overloaded_secs,
                        last_overloaded_at: now_ms,
                        state: EpisodeState::Overloaded,
                        kind: proc.kind,
                        killable: proc.killable,
                        score_history: vec![proc.score],
                    };
                    self.episodes.insert(key, ep);
                }
            } else if let Some(existing) = self.episodes.get_mut(&key) {
                // Process resource consumption is currently below threshold
                match existing.state {
                    EpisodeState::Overloaded => {
                        // Transitions from Overloaded to Cooling
                        existing.state = EpisodeState::Cooling;
                        push_score(&mut existing.score_history, proc.score);
                    }
                    EpisodeState::Cooling => {
                        // Check if retention window has elapsed
                        let retention_ms = self.retention_secs * 1000;
                        if now_ms > existing.last_overloaded_at + retention_ms {
                            existing.state = EpisodeState::Recovered;
                            newly_finished.push(existing.clone());
                        } else {
                            push_score(&mut existing.score_history, proc.score);
                        }
                    }
                    EpisodeState::Recovered | EpisodeState::Exited => {
                        // Already finished
                    }
                }
            }
        }

        // --- Group Episode Handling ---
        let group_culprit_key = breakdown
            .and_then(|bd| bd.group_culprit.as_ref())
            .map(|gc| (0, gc.name.to_lowercase()));

        if let Some(gc) = breakdown.and_then(|bd| bd.group_culprit.as_ref()) {
            let key = (0, gc.name.to_lowercase());
            if let Some(existing) = self.episodes.get_mut(&key) {
                existing.process_count = gc.process_count;
                match existing.state {
                    EpisodeState::Overloaded | EpisodeState::Cooling => {
                        existing.state = EpisodeState::Overloaded;
                        existing.last_overloaded_at = now_ms;
                        existing.overloaded_seconds = existing
                            .overloaded_seconds
                            .saturating_add(elapsed_tick_secs);
                        existing.peak_score = existing.peak_score.max(gc.share_percent);
                        existing.peak_cpu = existing.peak_cpu.max(gc.total_metric as f32);
                        push_score(&mut existing.score_history, gc.share_percent);
                    }
                    EpisodeState::Recovered | EpisodeState::Exited => {
                        *existing = Episode {
                            pid: 0,
                            name: gc.name.clone(),
                            process_count: gc.process_count,
                            started_at: now_ms,
                            peak_score: gc.share_percent,
                            peak_cpu: gc.total_metric as f32,
                            peak_ram_bytes: 0,
                            peak_disk_bps: 0.0,
                            overloaded_seconds: 1,
                            last_overloaded_at: now_ms,
                            state: EpisodeState::Overloaded,
                            kind: crate::model::ProcessKind::UserApp,
                            killable: false,
                            score_history: vec![gc.share_percent],
                        };
                    }
                }
            } else {
                let ep = Episode {
                    pid: 0,
                    name: gc.name.clone(),
                    process_count: gc.process_count,
                    started_at: now_ms,
                    peak_score: gc.share_percent,
                    peak_cpu: gc.total_metric as f32,
                    peak_ram_bytes: 0,
                    peak_disk_bps: 0.0,
                    overloaded_seconds: 1,
                    last_overloaded_at: now_ms,
                    state: EpisodeState::Overloaded,
                    kind: crate::model::ProcessKind::UserApp,
                    killable: false,
                    score_history: vec![gc.share_percent],
                };
                self.episodes.insert(key, ep);
            }
        }

        // Handle process terminations and group cooldowns
        let retention_ms = self.retention_secs * 1000;
        for (key, ep) in self.episodes.iter_mut() {
            if key.0 == 0 {
                // Group episode lifecycle
                let is_active_culprit = group_culprit_key.as_ref() == Some(key);
                if !is_active_culprit {
                    match ep.state {
                        EpisodeState::Overloaded => {
                            ep.state = EpisodeState::Cooling;
                            push_score(&mut ep.score_history, 0.0);
                        }
                        EpisodeState::Cooling => {
                            let any_alive = scored_processes
                                .iter()
                                .any(|p| p.record.name.eq_ignore_ascii_case(&ep.name));
                            if !any_alive {
                                ep.state = EpisodeState::Exited;
                                newly_finished.push(ep.clone());
                            } else if now_ms > ep.last_overloaded_at + retention_ms {
                                ep.state = EpisodeState::Recovered;
                                newly_finished.push(ep.clone());
                            } else {
                                push_score(&mut ep.score_history, 0.0);
                            }
                        }
                        EpisodeState::Recovered | EpisodeState::Exited => {}
                    }
                }
            } else {
                // Individual process lifecycle
                if !current_pids.contains(&key.0) {
                    if ep.state == EpisodeState::Overloaded || ep.state == EpisodeState::Cooling {
                        ep.state = EpisodeState::Exited;
                        newly_finished.push(ep.clone());
                    }
                } else if !current_keys.contains(key) && ep.state == EpisodeState::Cooling {
                    // PID exists with a different name (PID reuse!) -> previous process exited
                    ep.state = EpisodeState::Exited;
                    newly_finished.push(ep.clone());
                } else if !current_keys.contains(key)
                    && ep.state == EpisodeState::Cooling
                    && now_ms > ep.last_overloaded_at + retention_ms
                {
                    // Retention elapsed for a process that wasn't in the scored loop above
                    ep.state = EpisodeState::Recovered;
                    newly_finished.push(ep.clone());
                }
            }
        }

        for finished_ep in newly_finished {
            self.push_finished(finished_ep);
        }

        // Clean up candidate history for terminated processes
        self.recent_ticks
            .retain(|(pid, _), _| current_pids.contains(pid));

        // Enforce capacity constraint: cap watched entries at 20 (drop oldest Recovered first)
        self.enforce_capacity();

        self.prev_timestamp_ms = Some(now_ms);
        self.watched_entries()
    }

    /// Records a finished episode into the recent 10 finished episodes ring buffer.
    fn push_finished(&mut self, ep: Episode) {
        if self.finished.len() >= MAX_FINISHED_EPISODES {
            self.finished.pop_front();
        }
        self.finished.push_back(ep);
    }

    /// Enforces the maximum 20 watched entries cap, dropping oldest Recovered entries first.
    fn enforce_capacity(&mut self) {
        while self.episodes.len() > MAX_WATCHED_ENTRIES {
            // 1. Try dropping oldest Recovered entry
            if let Some(oldest_recovered) = self
                .episodes
                .iter()
                .filter(|(_, ep)| ep.state == EpisodeState::Recovered)
                .min_by_key(|(_, ep)| ep.last_overloaded_at)
                .map(|(k, _)| k.clone())
            {
                self.episodes.remove(&oldest_recovered);
                continue;
            }

            // 2. Try dropping oldest Exited entry
            if let Some(oldest_exited) = self
                .episodes
                .iter()
                .filter(|(_, ep)| ep.state == EpisodeState::Exited)
                .min_by_key(|(_, ep)| ep.last_overloaded_at)
                .map(|(k, _)| k.clone())
            {
                self.episodes.remove(&oldest_exited);
                continue;
            }

            // 3. Try dropping oldest Cooling entry
            if let Some(oldest_cooling) = self
                .episodes
                .iter()
                .filter(|(_, ep)| ep.state == EpisodeState::Cooling)
                .min_by_key(|(_, ep)| ep.last_overloaded_at)
                .map(|(k, _)| k.clone())
            {
                self.episodes.remove(&oldest_cooling);
                continue;
            }

            // 4. Fallback: drop oldest entry overall
            if let Some(oldest_any) = self
                .episodes
                .iter()
                .min_by_key(|(_, ep)| ep.started_at)
                .map(|(k, _)| k.clone())
            {
                self.episodes.remove(&oldest_any);
            } else {
                break;
            }
        }
    }

    /// Returns all watched entries sorted for UI presentation.
    ///
    /// Ordering: Overloaded (highest score first) -> Cooling -> Exited -> Recovered.
    pub fn watched_entries(&self) -> Vec<Episode> {
        let mut list: Vec<Episode> = self.episodes.values().cloned().collect();
        list.sort_by(|a, b| {
            let state_rank = |s: EpisodeState| match s {
                EpisodeState::Overloaded => 0,
                EpisodeState::Cooling => 1,
                EpisodeState::Exited => 2,
                EpisodeState::Recovered => 3,
            };

            state_rank(a.state)
                .cmp(&state_rank(b.state))
                .then_with(|| b.peak_score.total_cmp(&a.peak_score))
                .then_with(|| b.last_overloaded_at.cmp(&a.last_overloaded_at))
                .then_with(|| a.pid.cmp(&b.pid))
        });
        list
    }

    /// Returns the last 10 finished episodes (Recovered or Exited).
    pub fn finished_episodes(&self) -> &[Episode] {
        self.finished.as_slices().0
    }

    /// Checks whether a process key is currently actively watched (Overloaded or Cooling).
    pub fn is_actively_watched(&self, pid: u32, name: &str) -> bool {
        if let Some(ep) = self.episodes.get(&(pid, name.to_string())) {
            ep.state == EpisodeState::Overloaded || ep.state == EpisodeState::Cooling
        } else {
            false
        }
    }

    /// Clears all tracking state.
    pub fn clear(&mut self) {
        self.recent_ticks.clear();
        self.episodes.clear();
        self.finished.clear();
        self.prev_timestamp_ms = None;
    }
}

/// Determines the back-dated start timestamp of an episode from recent damping ticks.
fn calculate_start_time(
    recent_ticks: &HashMap<(u32, String), VecDeque<(u64, f32)>>,
    key: &(u32, String),
    now_ms: u64,
) -> u64 {
    if let Some(ticks) = recent_ticks.get(key) {
        // Pick the first tick where raw score was >= 40.0; fallback to the oldest tick in window
        ticks
            .iter()
            .find(|(_, score)| *score >= 40.0)
            .map(|(ts, _)| *ts)
            .or_else(|| ticks.front().map(|(ts, _)| *ts))
            .unwrap_or(now_ms)
    } else {
        now_ms
    }
}

fn push_score(history: &mut Vec<f32>, score: f32) {
    if history.len() >= MAX_SCORE_HISTORY {
        history.remove(0);
    }
    history.push(score);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bottleneck, ProcessKind, ProcessRecord};

    fn make_scored(pid: u32, name: &str, cpu: f32, score: f32, kind: ProcessKind) -> ScoredProcess {
        let severity = Severity::from_score(score);
        ScoredProcess {
            record: ProcessRecord {
                pid,
                parent_pid: None,
                name: name.to_string(),
                exe_path: Some(format!("C:\\{name}")),
                cpu_percent: cpu,
                memory_bytes: 200 * 1024 * 1024,
                disk_read_bps: 10_000.0,
                disk_write_bps: 0.0,
            },
            kind,
            score,
            severity,
            main_bottleneck: Bottleneck::Cpu,
            killable: kind != ProcessKind::SystemCritical,
        }
    }

    #[test]
    fn test_episode_start_back_dating() {
        let mut tracker = OverloadTracker::new(120, 3);
        let pid = 100;
        let name = "render.exe";

        // Ticks 1, 2, 3: sustained overload building over 3 ticks
        tracker.record_sample(pid, name, 10_000, 85.0);
        let _ = tracker.update(
            10_000,
            &[make_scored(pid, name, 85.0, 28.3, ProcessKind::UserApp)],
        );

        tracker.record_sample(pid, name, 11_000, 85.0);
        let _ = tracker.update(
            11_000,
            &[make_scored(pid, name, 85.0, 56.6, ProcessKind::UserApp)],
        );

        tracker.record_sample(pid, name, 12_000, 85.0);
        let watched = tracker.update(
            12_000,
            &[make_scored(pid, name, 85.0, 85.0, ProcessKind::UserApp)],
        );

        assert_eq!(watched.len(), 1);
        let ep = &watched[0];
        assert_eq!(ep.pid, pid);
        assert_eq!(ep.name, name);
        assert_eq!(ep.state, EpisodeState::Overloaded);
        // Back-dated to the first of the damping ticks (t = 10_000, NOT 12_000)
        assert_eq!(
            ep.started_at, 10_000,
            "Episode started_at must be back-dated to first damping tick"
        );
        assert_eq!(ep.last_overloaded_at, 12_000);
        assert!(ep.overloaded_seconds >= 2);
    }

    #[test]
    fn test_continuation_within_retention() {
        let mut tracker = OverloadTracker::new(120, 3);
        let pid = 200;
        let name = "node.exe";

        // Step 1: Overload begins
        tracker.record_sample(pid, name, 10_000, 90.0);
        let _ = tracker.update(
            10_000,
            &[make_scored(pid, name, 90.0, 80.0, ProcessKind::UserApp)],
        );
        let watched = tracker.watched_entries();
        assert_eq!(watched[0].state, EpisodeState::Overloaded);
        let original_start = watched[0].started_at;

        // Step 2: Resource usage cools down below threshold at t = 20_000
        let _ = tracker.update(
            20_000,
            &[make_scored(pid, name, 5.0, 10.0, ProcessKind::UserApp)],
        );
        let watched = tracker.watched_entries();
        assert_eq!(watched[0].state, EpisodeState::Cooling);
        assert_eq!(watched[0].last_overloaded_at, 10_000);

        // Step 3: At t = 50_000 (30s later, well inside 120s retention), process spikes again
        let _ = tracker.update(
            50_000,
            &[make_scored(pid, name, 95.0, 85.0, ProcessKind::UserApp)],
        );
        let watched = tracker.watched_entries();
        assert_eq!(watched.len(), 1);
        let ep = &watched[0];
        assert_eq!(
            ep.state,
            EpisodeState::Overloaded,
            "Must return to Overloaded state"
        );
        assert_eq!(
            ep.started_at, original_start,
            "started_at must NOT be reset when continuing episode"
        );
        assert_eq!(ep.last_overloaded_at, 50_000);
    }

    #[test]
    fn test_expiry_after_retention() {
        let mut tracker = OverloadTracker::new(120, 3);
        let pid = 300;
        let name = "db.exe";

        // Step 1: Overload at t = 10_000
        let _ = tracker.update(
            10_000,
            &[make_scored(pid, name, 90.0, 80.0, ProcessKind::UserApp)],
        );
        assert_eq!(tracker.watched_entries()[0].state, EpisodeState::Overloaded);

        // Step 2: Drops to normal at t = 11_000 -> enters Cooling
        let _ = tracker.update(
            11_000,
            &[make_scored(pid, name, 2.0, 10.0, ProcessKind::UserApp)],
        );
        assert_eq!(tracker.watched_entries()[0].state, EpisodeState::Cooling);

        // Step 3: At t = 130_000 (119s after last overload at 10_000) -> still Cooling
        let _ = tracker.update(
            130_000,
            &[make_scored(pid, name, 2.0, 10.0, ProcessKind::UserApp)],
        );
        assert_eq!(tracker.watched_entries()[0].state, EpisodeState::Cooling);

        // Step 4: At t = 131_000 (121s after last overload) -> retention expired -> Recovered
        let _ = tracker.update(
            131_000,
            &[make_scored(pid, name, 2.0, 10.0, ProcessKind::UserApp)],
        );
        let watched = tracker.watched_entries();
        assert_eq!(watched[0].state, EpisodeState::Recovered);
        assert!(!tracker.is_actively_watched(pid, name));
        assert_eq!(tracker.finished.len(), 1);
        assert_eq!(tracker.finished[0].state, EpisodeState::Recovered);
    }

    #[test]
    fn test_exit_handling() {
        let mut tracker = OverloadTracker::new(120, 3);
        let pid = 400;
        let name = "worker.exe";

        // Step 1: Overload at t = 10_000
        let _ = tracker.update(
            10_000,
            &[make_scored(pid, name, 90.0, 80.0, ProcessKind::UserApp)],
        );
        assert_eq!(tracker.watched_entries()[0].state, EpisodeState::Overloaded);

        // Step 2: PID 400 is not present in snapshot at t = 12_000 (process died/killed)
        let _ = tracker.update(12_000, &[]);
        let watched = tracker.watched_entries();
        assert_eq!(watched[0].state, EpisodeState::Exited);
        assert_eq!(tracker.finished.len(), 1);
        assert_eq!(tracker.finished[0].state, EpisodeState::Exited);
    }

    #[test]
    fn test_pid_reuse_with_different_name() {
        let mut tracker = OverloadTracker::new(120, 3);
        let pid = 500;

        // Step 1: PID 500 "old_app.exe" overloads at t = 10_000 and exits at t = 12_000
        let _ = tracker.update(
            10_000,
            &[make_scored(
                pid,
                "old_app.exe",
                90.0,
                80.0,
                ProcessKind::UserApp,
            )],
        );
        let _ = tracker.update(12_000, &[]); // old_app exits
        assert_eq!(tracker.watched_entries()[0].state, EpisodeState::Exited);

        // Step 2: PID 500 reappears as "new_app.exe" at t = 20_000 and overloads
        let _ = tracker.update(
            20_000,
            &[make_scored(
                pid,
                "new_app.exe",
                95.0,
                85.0,
                ProcessKind::UserApp,
            )],
        );
        let watched = tracker.watched_entries();

        // Must have two distinct episodes: the new one for new_app.exe and the exited one for old_app.exe
        let new_ep = watched
            .iter()
            .find(|e| e.name == "new_app.exe")
            .expect("new_app must exist");
        let old_ep = watched
            .iter()
            .find(|e| e.name == "old_app.exe")
            .expect("old_app must exist");

        assert_eq!(new_ep.state, EpisodeState::Overloaded);
        assert_eq!(
            new_ep.started_at, 20_000,
            "New process must start its own episode"
        );
        assert_eq!(old_ep.state, EpisodeState::Exited);
    }

    #[test]
    fn test_cap_behavior() {
        let mut tracker = OverloadTracker::new(120, 3);

        // Create 22 distinct processes with overload
        for i in 1..=22 {
            let pid = 1000 + i;
            let name = format!("proc_{i}.exe");
            let _ = tracker.update(
                (i as u64) * 1000,
                &[make_scored(pid, &name, 80.0, 75.0, ProcessKind::UserApp)],
            );
        }

        // Must be capped at MAX_WATCHED_ENTRIES (20)
        assert_eq!(tracker.watched_entries().len(), MAX_WATCHED_ENTRIES);

        // Mark some as Recovered
        let oldest_key = (1001, "proc_1.exe".to_string());
        if let Some(ep) = tracker.episodes.get_mut(&oldest_key) {
            ep.state = EpisodeState::Recovered;
        }

        // Add 23rd process: should drop oldest Recovered first
        let _ = tracker.update(
            30_000,
            &[make_scored(
                1023,
                "proc_23.exe",
                90.0,
                80.0,
                ProcessKind::UserApp,
            )],
        );
        assert_eq!(tracker.watched_entries().len(), MAX_WATCHED_ENTRIES);
        assert!(
            !tracker.episodes.contains_key(&oldest_key),
            "Oldest Recovered entry should have been dropped first"
        );
    }

    #[test]
    fn test_system_critical_watched_for_display_but_non_killable() {
        let mut tracker = OverloadTracker::new(120, 3);
        let pid = 104;
        let name = "csrss.exe";

        let _ = tracker.update(
            10_000,
            &[make_scored(
                pid,
                name,
                80.0,
                75.0,
                ProcessKind::SystemCritical,
            )],
        );

        let watched = tracker.watched_entries();
        assert_eq!(watched.len(), 1);
        let ep = &watched[0];
        assert_eq!(ep.name, "csrss.exe");
        assert_eq!(ep.kind, ProcessKind::SystemCritical);
        assert!(!ep.killable, "SystemCritical process must NOT be killable");
        assert_eq!(ep.state, EpisodeState::Overloaded);
    }

    #[test]
    fn test_demo_scenario_overloaded_cooling_recovered_within_60s() {
        // Retention of 25s for fast demonstration scenario
        let mut tracker = OverloadTracker::new(25, 3);
        let pid = 9999;
        let name = "test_heavy.exe";

        // Phase 1: Idle (0s to 14s)
        for t in 0..14 {
            let now_ms = t * 1000;
            let _ = tracker.update(
                now_ms,
                &[make_scored(pid, name, 0.1, 0.0, ProcessKind::UserApp)],
            );
        }
        assert!(tracker.watched_entries().is_empty());

        // Phase 2: Overload starts at 15s (15_000ms)
        for t in 15..30 {
            let now_ms = t * 1000;
            tracker.record_sample(pid, name, now_ms, 95.0);
            let _ = tracker.update(
                now_ms,
                &[make_scored(pid, name, 95.0, 90.0, ProcessKind::UserApp)],
            );
        }
        assert_eq!(tracker.watched_entries().len(), 1);
        assert_eq!(tracker.watched_entries()[0].state, EpisodeState::Overloaded);
        assert_eq!(tracker.watched_entries()[0].started_at, 15_000);

        // Phase 3: Cooldown begins at 30s (drops below threshold)
        for t in 30..54 {
            let now_ms = t * 1000;
            let _ = tracker.update(
                now_ms,
                &[make_scored(pid, name, 1.0, 10.0, ProcessKind::UserApp)],
            );
            assert_eq!(tracker.watched_entries()[0].state, EpisodeState::Cooling);
        }

        // Phase 4: At 55s (30s + 25s retention = 55s <= 60s), retention expires -> Recovered
        let _ = tracker.update(
            55_000,
            &[make_scored(pid, name, 1.0, 10.0, ProcessKind::UserApp)],
        );
        assert_eq!(tracker.watched_entries()[0].state, EpisodeState::Recovered);
        assert_eq!(tracker.finished.len(), 1);
        assert_eq!(tracker.finished[0].state, EpisodeState::Recovered);
    }

    #[test]
    fn test_group_episode_tracking_lifecycle() {
        let mut tracker = OverloadTracker::new(10, 3);

        let p1 = make_scored(101, "rustc.exe", 25.0, 25.0, ProcessKind::UserApp);
        let p2 = make_scored(102, "rustc.exe", 25.0, 25.0, ProcessKind::UserApp);
        let procs = vec![p1, p2];

        // Group culprit with 50% CPU
        let gc = crate::model::ProcessGroupShare {
            name: "rustc.exe".to_string(),
            process_count: 2,
            total_metric: 50.0,
            share_percent: 50.0,
            pids: vec![101, 102],
        };
        let bd = crate::model::PressureBreakdown {
            resource: crate::model::OverloadedResource::Cpu,
            system_value: 100.0,
            resource_unit: "%",
            top_groups: vec![gc.clone()],
            remainder_share_percent: 50.0,
            group_culprit: Some(gc),
        };

        // Tick 1: Group culprit active -> Overloaded group episode
        let watched = tracker.update_with_breakdown(1000, &procs, Some(&bd));
        let group_ep = watched
            .iter()
            .find(|e| e.is_group())
            .expect("Must have group episode");
        assert_eq!(group_ep.name, "rustc.exe");
        assert_eq!(group_ep.process_count, 2);
        assert_eq!(group_ep.state, EpisodeState::Overloaded);

        // Tick 2: Pressure relieved -> Group enters Cooling
        let watched2 = tracker.update_with_breakdown(2000, &procs, None);
        let group_ep2 = watched2
            .iter()
            .find(|e| e.is_group())
            .expect("Must have group episode");
        assert_eq!(group_ep2.state, EpisodeState::Cooling);

        // Tick 3: 11 seconds later -> Group recovers
        let watched3 = tracker.update_with_breakdown(13_000, &procs, None);
        let group_ep3 = watched3
            .iter()
            .find(|e| e.is_group())
            .expect("Must have group episode");
        assert_eq!(group_ep3.state, EpisodeState::Recovered);
    }
}
