//! Theming and styling for SlowSpot UI.
//!
//! Provides a modern dark palette, accessible contrast ratios,
//! custom font installation (embedded Noto Sans), and consistent layout spacing.

use eframe::egui::{self, Color32, Margin, Vec2};
use std::sync::Arc;

pub const COLOR_BG_PANEL: Color32 = Color32::from_rgb(20, 23, 30);
pub const COLOR_BG_CARD: Color32 = Color32::from_rgb(28, 32, 42);
pub const COLOR_BG_CARD_CULPRIT: Color32 = Color32::from_rgb(48, 22, 26);
pub const COLOR_BG_CARD_SMOOTH: Color32 = Color32::from_rgb(20, 38, 28);
pub const COLOR_BG_INPUT: Color32 = Color32::from_rgb(15, 17, 23);

pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 243, 248);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(148, 158, 172);

// Severity & Status Colors
pub const COLOR_CULPRIT: Color32 = Color32::from_rgb(235, 87, 87);
pub const COLOR_SUSPICIOUS: Color32 = Color32::from_rgb(242, 175, 74);
pub const COLOR_NORMAL: Color32 = Color32::from_rgb(46, 204, 113);
pub const COLOR_INFO: Color32 = Color32::from_rgb(74, 144, 226);

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

    // 2. Set dark visuals and style
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = COLOR_BG_PANEL;
    style.visuals.window_fill = COLOR_BG_CARD;
    style.visuals.extreme_bg_color = COLOR_BG_INPUT;

    // 3. Set comfortable spacing
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.window_margin = Margin::same(12);

    ctx.set_style_of(egui::Theme::Dark, Arc::new(style));
    ctx.set_theme(egui::Theme::Dark);
}
