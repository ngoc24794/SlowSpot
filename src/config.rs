//! Application configuration management.
//!
//! Stores in-memory runtime parameters such as collector sampling tick rate,
//! score damping window size, and sparkline history depth.

/// Application runtime configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppConfig {
    /// Collector sampling interval in milliseconds (default: 1000).
    pub tick_ms: u64,
    /// Number of rolling cycles used for score damping (default: 3).
    pub damping_window: usize,
    /// Number of historical snapshots retained for sparkline charts (default: 60).
    pub history_len: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tick_ms: 1000,
            damping_window: 3,
            history_len: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.tick_ms, 1000);
        assert_eq!(config.damping_window, 3);
        assert_eq!(config.history_len, 60);
    }
}
