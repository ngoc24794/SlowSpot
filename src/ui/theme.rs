//! Theming and styling for SlowSpot UI.
//!
//! Provides a modern dark palette, accessible contrast ratios,
//! custom font installation (embedded Noto Sans), and consistent layout spacing.

use eframe::egui::{self, Color32, Margin, Vec2};
use std::sync::Arc;

pub use crate::ui::palette::*;

// Backward-compatible color aliases
pub const COLOR_BG_PANEL: Color32 = COLOR_BG;
pub const COLOR_BG_CARD: Color32 = COLOR_CARD;
pub const COLOR_BG_INPUT: Color32 = COLOR_MUTED;
pub const COLOR_TEXT_PRIMARY: Color32 = COLOR_FOREGROUND;
pub const COLOR_TEXT_MUTED: Color32 = COLOR_MUTED_FOREGROUND;
pub const COLOR_CULPRIT: Color32 = COLOR_DESTRUCTIVE;
pub const COLOR_SUSPICIOUS: Color32 = COLOR_WARNING;
pub const COLOR_NORMAL: Color32 = COLOR_ACCENT_OK;

/// Checks whether reduced motion is requested via the environment variable `SLOWSPOT_REDUCED_MOTION=1`.
pub fn is_reduced_motion() -> bool {
    std::env::var("SLOWSPOT_REDUCED_MOTION")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Configures fonts, dark styling, and widget appearances on the given egui context.
pub fn install(ctx: &egui::Context) {
    // 1. Install embedded Noto Sans font
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "NotoSans".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/NotoSans-Regular.ttf"
        ))),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "NotoSans".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "NotoSans".to_owned());

    ctx.set_fonts(fonts);

    // 2. Set dark visuals and style adhering to Swiss minimalism & dashboard density
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = COLOR_BG;
    visuals.window_fill = COLOR_CARD;
    visuals.extreme_bg_color = COLOR_MUTED;

    visuals.widgets.noninteractive.bg_fill = COLOR_BG;
    visuals.widgets.noninteractive.fg_stroke.color = COLOR_FOREGROUND;

    visuals.widgets.inactive.bg_fill = COLOR_CARD;
    visuals.widgets.inactive.fg_stroke.color = COLOR_FOREGROUND;
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(RADIUS_SM as u8);

    visuals.widgets.hovered.bg_fill = COLOR_MUTED;
    visuals.widgets.hovered.fg_stroke.color = COLOR_FOREGROUND;
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(RADIUS_SM as u8);

    visuals.widgets.active.bg_fill = COLOR_MUTED;
    visuals.widgets.active.fg_stroke.color = COLOR_FOREGROUND;
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(RADIUS_SM as u8);

    visuals.selection.bg_fill = COLOR_INFO.gamma_multiply(0.25);
    visuals.selection.stroke = egui::Stroke::new(1.0, COLOR_INFO);

    style.visuals = visuals;

    // 3. Spacing aligned to 4 / 8 / 12 scale
    style.spacing.item_spacing = Vec2::new(SPACING_SM, SPACING_SM);
    style.spacing.window_margin = Margin::same(SPACING_MD as i8);

    ctx.set_style_of(egui::Theme::Dark, Arc::new(style));
    ctx.set_theme(egui::Theme::Dark);
}
