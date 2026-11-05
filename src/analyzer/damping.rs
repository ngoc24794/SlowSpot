//! Rolling window damping filter for culprit scores.
//!
//! Prevents UI jitter and transient score jumping when background tasks
//! burst CPU or Disk I/O for brief milliseconds.

use std::collections::{HashMap, HashSet, VecDeque};

/// Default number of consecutive sampling cycles retained for rolling score average.
pub const DEFAULT_DAMPING_WINDOW: usize = 3;

/// Rolling window smoother maintaining historical culprit scores per PID.
#[derive(Debug, Clone)]
pub struct Damping {
    window: usize,
    history: HashMap<u32, VecDeque<f32>>,
}

impl Default for Damping {
    fn default() -> Self {
        Self::new(DEFAULT_DAMPING_WINDOW)
    }
}

impl Damping {
    /// Creates a new `Damping` smoother with the specified window size.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            history: HashMap::new(),
        }
    }

    /// Configured window size in sampling cycles.
    pub fn window(&self) -> usize {
        self.window
    }

    /// Appends a new score for the given `pid` and returns the smoothed rolling average.
    pub fn smooth(&mut self, pid: u32, score: f32) -> f32 {
        let entry = self.history.entry(pid).or_default();
        if entry.len() >= self.window {
            entry.pop_front();
        }
        entry.push_back(score);

        let sum: f32 = entry.iter().copied().sum();
        sum / (entry.len() as f32)
    }

    /// Evicts historical records for PIDs not in the active set.
    pub fn retain_pids(&mut self, active_pids: &HashSet<u32>) {
        self.history.retain(|pid, _| active_pids.contains(pid));
    }

    /// Checks if state exists for the given PID.
    pub fn contains_pid(&self, pid: u32) -> bool {
        self.history.contains_key(&pid)
    }

    /// Clears all historical damping state.
    pub fn clear(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_cycle_spike_damping() {
        let mut damping = Damping::new(3);
        let pid = 200;

        // Baseline cycle: low score 10.0
        let s1 = damping.smooth(pid, 10.0);
        assert_eq!(s1, 10.0);

        // Second cycle: sudden spike to 100.0 -> average of [10, 100] = 55.0
        let s2 = damping.smooth(pid, 100.0);
        assert_eq!(s2, 55.0);

        // Third cycle: returns to 10.0 -> average of [10, 100, 10] = 40.0
        let s3 = damping.smooth(pid, 10.0);
        assert_eq!(s3, 40.0);

        // Fourth cycle: remains 10.0 -> drops first 10 -> [100, 10, 10] -> average = 40.0
        let s4 = damping.smooth(pid, 10.0);
        assert_eq!(s4, 40.0);

        // Fifth cycle: remains 10.0 -> drops 100 -> [10, 10, 10] -> average = 10.0
        let s5 = damping.smooth(pid, 10.0);
        assert_eq!(s5, 10.0);
    }

    #[test]
    fn test_pid_disappearance_drops_state() {
        let mut damping = Damping::new(3);
        let pid_a = 10;
        let pid_b = 20;

        damping.smooth(pid_a, 50.0);
        damping.smooth(pid_b, 80.0);

        assert!(damping.contains_pid(pid_a));
        assert!(damping.contains_pid(pid_b));

        // In the next cycle, only pid_b is active
        let mut active = HashSet::new();
        active.insert(pid_b);
        damping.retain_pids(&active);

        assert!(
            !damping.contains_pid(pid_a),
            "pid_a state should be dropped"
        );
        assert!(
            damping.contains_pid(pid_b),
            "pid_b state should be preserved"
        );

        // When pid_a reappears later, it starts with fresh history
        let s_fresh = damping.smooth(pid_a, 20.0);
        assert_eq!(s_fresh, 20.0);
    }
}
