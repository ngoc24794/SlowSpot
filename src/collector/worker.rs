//! Background worker thread orchestrating periodic metrics sampling and streaming.
//!
//! Manages continuous execution of sampling, analysis, channel dispatching,
//! and UI repaint notification callbacks without drift.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::analyzer::scoring::ScoreWeights;
use crate::analyzer::Analyzer;
use crate::collector::SampleSource;
use crate::config::AppConfig;
use crate::model::SystemSnapshot;

/// Active control handle to the background collector worker.
///
/// Holds the receiving end of the snapshot channel and ensures orderly termination
/// of the background thread when dropped.
pub struct CollectorHandle {
    /// Receiving end of analyzed system snapshots dispatched to the UI.
    pub rx: Receiver<SystemSnapshot>,
    stop_flag: Arc<AtomicBool>,
    notifier: Arc<(Mutex<bool>, Condvar)>,
    thread_handle: Option<JoinHandle<()>>,
}

impl CollectorHandle {
    /// Signals the collector worker thread to terminate and wakes it from sleep immediately.
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Release);
        let (lock, cvar) = &*self.notifier;
        if let Ok(mut guard) = lock.lock() {
            *guard = true;
            cvar.notify_all();
        }
    }

    /// Queries whether the worker thread is active.
    pub fn is_running(&self) -> bool {
        !self.stop_flag.load(Ordering::Acquire)
    }
}

impl Drop for CollectorHandle {
    fn drop(&mut self) {
        self.stop();
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawns a background thread that periodically samples metrics, executes analysis,
/// dispatches snapshots via channel, and triggers a repaint callback.
///
/// Sampling cadence follows wall-clock time without drift by scheduling against
/// the expected start of each tick interval.
pub fn spawn(
    mut source: Box<dyn SampleSource>,
    cfg: AppConfig,
    on_snapshot: Box<dyn Fn() + Send>,
) -> CollectorHandle {
    let (tx, rx) = channel();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let notifier = Arc::new((Mutex::new(false), Condvar::new()));

    let thread_stop = Arc::clone(&stop_flag);
    let thread_notifier = Arc::clone(&notifier);

    let tick_duration = Duration::from_millis(cfg.tick_ms.max(1));
    let mut analyzer = Analyzer::new(ScoreWeights::default(), cfg.damping_window);

    let thread_handle = thread::Builder::new()
        .name("slowspot-collector".to_string())
        .spawn(move || {
            let mut next_tick = Instant::now();

            while !thread_stop.load(Ordering::Acquire) {
                let raw = source.sample();
                let snapshot = analyzer.analyze(raw);

                // Exit cleanly if receiver is dropped
                if tx.send(snapshot).is_err() {
                    thread_stop.store(true, Ordering::Release);
                    break;
                }

                // Request UI repaint or notify listener
                on_snapshot();

                // Compute remaining sleep interval avoiding drift
                next_tick += tick_duration;
                let now = Instant::now();

                if next_tick > now {
                    let sleep_duration = next_tick - now;
                    let (lock, cvar) = &*thread_notifier;
                    if let Ok(guard) = lock.lock() {
                        let _ = cvar.wait_timeout_while(guard, sleep_duration, |stopped| !*stopped);
                    }
                } else {
                    // Loop fell behind schedule; catch up without queuing backlog
                    next_tick = now;
                }
            }
        })
        .expect("failed to spawn slowspot-collector thread");

    CollectorHandle {
        rx,
        stop_flag,
        notifier,
        thread_handle: Some(thread_handle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::mock::{MockSource, Scenario};
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_worker_lifecycle_and_streaming() {
        let callback_count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&callback_count);

        let on_snapshot = Box::new(move || {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let source = Box::new(MockSource::new(Scenario::Idle));
        let cfg = AppConfig {
            tick_ms: 20, // Fast 20ms tick rate for quick test execution
            damping_window: 3,
            history_len: 60,
            watch_retention_secs: 120,
        };

        let handle = spawn(source, cfg, on_snapshot);

        // Receive at least 3 snapshots within a generous 1000ms timeout
        for i in 0..3 {
            let snapshot = handle
                .rx
                .recv_timeout(Duration::from_millis(1000))
                .unwrap_or_else(|_| panic!("Failed to receive snapshot index {i} within timeout"));
            assert!(!snapshot.processes.is_empty());
        }

        // Verify that on_snapshot callback was triggered at least 3 times
        let calls = callback_count.load(Ordering::SeqCst);
        assert!(
            calls >= 3,
            "Expected on_snapshot callback to be called >= 3 times, got {calls}"
        );

        // Dropping handle must stop the background thread and join cleanly without hanging
        drop(handle);
    }
}
