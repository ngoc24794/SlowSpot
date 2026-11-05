//! User interface presentation layer built with `eframe` and `egui`.
//!
//! Renders reactive dashboard views, including system pressure status banners,
//! top culprit cards, sparkline resource graphs, and interactive process tables.

pub mod actions;
pub mod app;
pub mod charts;
pub mod palette;
pub mod state;
pub mod strings;
pub mod table;
pub mod theme;
pub mod widgets;

pub use actions::{ActionRunner, DemoActions, RealActions};
pub use app::SlowSpotApp;
pub use state::ViewState;
