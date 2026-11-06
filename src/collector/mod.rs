//! System metrics collection module.
//!
//! Responsible for periodically polling CPU, memory, and disk I/O metrics
//! from the operating system via `sysinfo` or simulated sources, calculating rate deltas,
//! and streaming metric samples to the analyzer.

pub mod mock;
pub mod system;
pub mod worker;

use crate::model::RawSnapshot;

pub use mock::{MockScenario, MockSource, Scenario};
pub use system::SysinfoSource;
pub use worker::{spawn, CollectorHandle};

/// Trait abstracting a source of periodic raw system metric snapshots.
pub trait SampleSource: Send {
    /// Collects and returns a snapshot of the current system and process metrics.
    fn sample(&mut self) -> RawSnapshot;
}
