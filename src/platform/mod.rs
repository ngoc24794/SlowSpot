//! Windows platform integration and native Win32 FFI bridges.
//!
//! Provides native process controls (graceful close via WM_CLOSE, forced termination),
//! process image path resolution, Explorer integration, and native message boxes.

pub mod dialog;
pub mod process;

pub use dialog::{show_error_box, to_wide};
pub use process::{
    check_allowed, force_kill, graceful_close, names_match, open_location, ActionError,
};
