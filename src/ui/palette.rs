//! Design tokens, color palette, typography scale, spacing constants,
//! WCAG 2.1 contrast ratio calculations, and visual indicator mappings.

use eframe::egui::Color32;

use crate::model::{PressureState, Severity};
use crate::ui::strings;

// --- Color Tokens (Swiss Minimalism / Dark Dashboard) ---

/// Main application background (#0F172A - slate 900).
pub const COLOR_BG: Color32 = Color32::from_rgb(15, 23, 42);

/// Container & Card background (#1B2336 - deep slate navy).
pub const COLOR_CARD: Color32 = Color32::from_rgb(27, 35, 54);

/// Culprit card background (#30161A - dark tinted crimson).
pub const COLOR_BG_CARD_CULPRIT: Color32 = Color32::from_rgb(48, 22, 26);

/// Healthy smooth card background (#14261C - dark tinted forest).
pub const COLOR_BG_CARD_SMOOTH: Color32 = Color32::from_rgb(20, 38, 28);

/// Muted elements, row hover & input fields (#272F42 - slate 800).
pub const COLOR_MUTED: Color32 = Color32::from_rgb(39, 47, 66);

/// Selected row background tint (#24334D).
pub const COLOR_SELECTED_ROW: Color32 = Color32::from_rgb(36, 51, 77);

/// Standard borders (#475569 - slate 600).
pub const COLOR_BORDER: Color32 = Color32::from_rgb(71, 85, 105);

/// Subtle row dividers and guidelines (slate 600 premultiplied with alpha 60).
pub const COLOR_BORDER_SUBTLE: Color32 = Color32::from_rgba_premultiplied(16, 20, 24, 60);

/// Primary high-contrast text (#F8FAFC - slate 50).
pub const COLOR_FOREGROUND: Color32 = Color32::from_rgb(248, 250, 252);

/// Secondary and muted caption text (#94A3B8 - slate 400).
pub const COLOR_MUTED_FOREGROUND: Color32 = Color32::from_rgb(148, 163, 184);

/// Healthy status / Accent OK (#22C55E - emerald 500).
pub const COLOR_ACCENT_OK: Color32 = Color32::from_rgb(34, 197, 94);

/// Warning / Suspicious (#F59E0B - amber 500).
pub const COLOR_WARNING: Color32 = Color32::from_rgb(245, 158, 11);

/// Danger / Culprit (#EF4444 - red 500).
pub const COLOR_DESTRUCTIVE: Color32 = Color32::from_rgb(239, 68, 68);

/// High-contrast culprit text on dark surfaces (#F87171 - red 400).
pub const COLOR_DESTRUCTIVE_TEXT: Color32 = Color32::from_rgb(248, 113, 113);

/// Informational / User App / Link (#38BDF8 - sky 400).
pub const COLOR_INFO: Color32 = Color32::from_rgb(56, 189, 248);

/// Protected system process accent (#E879F9 - fuchsia 400).
pub const COLOR_PROTECTED: Color32 = Color32::from_rgb(232, 121, 249);

// --- Spacing Tokens (Scale 4 / 8 / 12 / 16) ---
pub const SPACING_XS: f32 = 4.0;
pub const SPACING_SM: f32 = 8.0;
pub const SPACING_MD: f32 = 12.0;
pub const SPACING_LG: f32 = 16.0;

// --- Corner Radius Tokens ---
pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;

// --- Typography Scale ---
pub const FONT_SIZE_HEADING: f32 = 18.0;
pub const FONT_SIZE_SUBHEADING: f32 = 15.0;
pub const FONT_SIZE_BODY_LARGE: f32 = 14.0;
pub const FONT_SIZE_BODY: f32 = 13.0;
pub const FONT_SIZE_CAPTION: f32 = 11.5;
pub const FONT_SIZE_TINY: f32 = 10.0;

// --- Geometric Shape Indicators for WCAG Color-Independence ---

/// Geometric shape drawn alongside text labels to guarantee accessible identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeverityShape {
    /// Circle indicator for Normal / Healthy.
    Circle,
    /// Triangle indicator for Warning / Suspicious.
    Triangle,
    /// Diamond indicator for Danger / Culprit.
    Diamond,
}

/// Maps process severity to localized Vietnamese label, geometric shape, and color.
pub fn severity_indicator(severity: Severity) -> (&'static str, SeverityShape, Color32) {
    match severity {
        Severity::Normal => (
            strings::SEVERITY_NORMAL,
            SeverityShape::Circle,
            COLOR_ACCENT_OK,
        ),
        Severity::Suspicious => (
            strings::SEVERITY_SUSPICIOUS,
            SeverityShape::Triangle,
            COLOR_WARNING,
        ),
        Severity::Culprit => (
            strings::SEVERITY_CULPRIT,
            SeverityShape::Diamond,
            COLOR_DESTRUCTIVE,
        ),
    }
}

/// Geometric shape drawn for system pressure states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureShape {
    /// Smooth circle.
    Circle,
    /// Warning triangle.
    Triangle,
    /// Hazard octagon.
    Octagon,
}

/// Maps system pressure state to localized Vietnamese label, geometric shape, and color.
pub fn pressure_indicator(
    pressure: Option<PressureState>,
) -> (&'static str, PressureShape, Color32) {
    match pressure {
        Some(PressureState::Smooth) => (
            strings::PRESSURE_SMOOTH,
            PressureShape::Circle,
            COLOR_ACCENT_OK,
        ),
        Some(PressureState::Strained) => (
            strings::PRESSURE_STRAINED,
            PressureShape::Triangle,
            COLOR_WARNING,
        ),
        Some(PressureState::Overloaded) => (
            strings::PRESSURE_OVERLOADED,
            PressureShape::Octagon,
            COLOR_DESTRUCTIVE,
        ),
        None => (
            strings::PRESSURE_WAITING,
            PressureShape::Circle,
            COLOR_MUTED_FOREGROUND,
        ),
    }
}

// --- WCAG 2.1 Relative Luminance & Contrast Calculation ---

/// Converts an sRGB color component in range 0..255 to linear RGB luminance.
pub fn srgb_to_linear(val: u8) -> f64 {
    let s = val as f64 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// Calculates the relative luminance of a color according to WCAG 2.1 specs.
pub fn relative_luminance(color: Color32) -> f64 {
    let r = srgb_to_linear(color.r());
    let g = srgb_to_linear(color.g());
    let b = srgb_to_linear(color.b());
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Calculates the contrast ratio between two colors (ranging from 1.0 to 21.0).
pub fn contrast_ratio(color_a: Color32, color_b: Color32) -> f64 {
    let l1 = relative_luminance(color_a);
    let l2 = relative_luminance(color_b);
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wcag_contrast_ratios_for_normal_text() {
        // WCAG AA requires contrast ratio >= 4.5:1 for normal body text

        // 1. Primary foreground on background (#0F172A)
        let fg_on_bg = contrast_ratio(COLOR_FOREGROUND, COLOR_BG);
        assert!(
            fg_on_bg >= 4.5,
            "Foreground on background must be >= 4.5 (got {fg_on_bg:.2})"
        );

        // 2. Primary foreground on card (#1B2336)
        let fg_on_card = contrast_ratio(COLOR_FOREGROUND, COLOR_CARD);
        assert!(
            fg_on_card >= 4.5,
            "Foreground on card must be >= 4.5 (got {fg_on_card:.2})"
        );

        // 3. Primary foreground on muted panel (#272F42)
        let fg_on_muted = contrast_ratio(COLOR_FOREGROUND, COLOR_MUTED);
        assert!(
            fg_on_muted >= 4.5,
            "Foreground on muted must be >= 4.5 (got {fg_on_muted:.2})"
        );

        // 4. Muted foreground on background (#0F172A)
        let muted_on_bg = contrast_ratio(COLOR_MUTED_FOREGROUND, COLOR_BG);
        assert!(
            muted_on_bg >= 4.5,
            "Muted foreground on background must be >= 4.5 (got {muted_on_bg:.2})"
        );

        // 5. Muted foreground on card (#1B2336)
        let muted_on_card = contrast_ratio(COLOR_MUTED_FOREGROUND, COLOR_CARD);
        assert!(
            muted_on_card >= 4.5,
            "Muted foreground on card must be >= 4.5 (got {muted_on_card:.2})"
        );

        // 6. Accent OK (green) on background & card
        let ok_on_bg = contrast_ratio(COLOR_ACCENT_OK, COLOR_BG);
        assert!(
            ok_on_bg >= 4.5,
            "Accent OK on background must be >= 4.5 (got {ok_on_bg:.2})"
        );

        // 7. Warning (amber) on background & card
        let warn_on_bg = contrast_ratio(COLOR_WARNING, COLOR_BG);
        assert!(
            warn_on_bg >= 4.5,
            "Warning on background must be >= 4.5 (got {warn_on_bg:.2})"
        );

        // 8. Info (sky blue) on background & card
        let info_on_bg = contrast_ratio(COLOR_INFO, COLOR_BG);
        assert!(
            info_on_bg >= 4.5,
            "Info on background must be >= 4.5 (got {info_on_bg:.2})"
        );

        // 9. Destructive text (#F87171) on card (#1B2336)
        let culprit_text_on_card = contrast_ratio(COLOR_DESTRUCTIVE_TEXT, COLOR_CARD);
        assert!(
            culprit_text_on_card >= 4.5,
            "Culprit text on card must be >= 4.5 (got {culprit_text_on_card:.2})"
        );
    }

    #[test]
    fn test_severity_indicator_mapping() {
        let (label_norm, shape_norm, color_norm) = severity_indicator(Severity::Normal);
        assert_eq!(label_norm, strings::SEVERITY_NORMAL);
        assert_eq!(shape_norm, SeverityShape::Circle);
        assert_eq!(color_norm, COLOR_ACCENT_OK);

        let (label_susp, shape_susp, color_susp) = severity_indicator(Severity::Suspicious);
        assert_eq!(label_susp, strings::SEVERITY_SUSPICIOUS);
        assert_eq!(shape_susp, SeverityShape::Triangle);
        assert_eq!(color_susp, COLOR_WARNING);

        let (label_culp, shape_culp, color_culp) = severity_indicator(Severity::Culprit);
        assert_eq!(label_culp, strings::SEVERITY_CULPRIT);
        assert_eq!(shape_culp, SeverityShape::Diamond);
        assert_eq!(color_culp, COLOR_DESTRUCTIVE);
    }

    #[test]
    fn test_pressure_indicator_mapping() {
        let (label_smooth, shape_smooth, color_smooth) =
            pressure_indicator(Some(PressureState::Smooth));
        assert_eq!(label_smooth, strings::PRESSURE_SMOOTH);
        assert_eq!(shape_smooth, PressureShape::Circle);
        assert_eq!(color_smooth, COLOR_ACCENT_OK);

        let (label_strained, shape_strained, color_strained) =
            pressure_indicator(Some(PressureState::Strained));
        assert_eq!(label_strained, strings::PRESSURE_STRAINED);
        assert_eq!(shape_strained, PressureShape::Triangle);
        assert_eq!(color_strained, COLOR_WARNING);

        let (label_overloaded, shape_overloaded, color_overloaded) =
            pressure_indicator(Some(PressureState::Overloaded));
        assert_eq!(label_overloaded, strings::PRESSURE_OVERLOADED);
        assert_eq!(shape_overloaded, PressureShape::Octagon);
        assert_eq!(color_overloaded, COLOR_DESTRUCTIVE);

        let (label_none, shape_none, color_none) = pressure_indicator(None);
        assert_eq!(label_none, strings::PRESSURE_WAITING);
        assert_eq!(shape_none, PressureShape::Circle);
        assert_eq!(color_none, COLOR_MUTED_FOREGROUND);
    }
}
