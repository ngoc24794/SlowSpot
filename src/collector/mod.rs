//! System metrics collection module.
//!
//! Responsible for periodically polling CPU, memory, and disk I/O metrics
//! from the operating system via `sysinfo`, calculating rate deltas, and
//! streaming metric samples.
