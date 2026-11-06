//! Design tokens, color palette, typography scale, spacing constants,
//! WCAG 2.1 contrast ratio calculations, and visual indicator mappings.

use eframe::egui::Color32;

use crate::model::{PressureState, ScoredProcess, Severity, SystemTotals};
use crate::ui::strings;

// --- Color Tokens (Swiss Minimalism / Dark Dashboard) ---

/// Main application background (#0F172A - slate 900).
pub const COLOR_BG: Color32 = Color32::from_rgb(15, 23, 42);

/// Container & Card background (#1B2336 - deep slate navy).
pub const COLOR_CARD: Color32 = Color32::from_rgb(27, 35, 54);

/// Culprit card background (#30161A - dark tinted crimson).
pub const COLOR_BG_CARD_CULPRIT: Color32 = Color32::from_rgb(38, 16, 20);

/// Suspicious card background (#281808 - dark tinted amber).
pub const COLOR_BG_CARD_SUSPICIOUS: Color32 = Color32::from_rgb(40, 24, 8);

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
pub const FONT_SIZE_CAPTION: f32 = 12.0;
pub const FONT_SIZE_TINY: f32 = 12.0;

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

/// Complete visual design token set for process overload severity level presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelTokens {
    /// Border / frame stroke color (#F59E0B for Suspicious, #EF4444 for Culprit).
    pub border: Color32,
    /// Tinted card background (#281808 dark amber for Suspicious, #261014 dark crimson for Culprit).
    pub bg: Color32,
    /// High-contrast accent color matching the frame.
    pub text_accent: Color32,
    /// Localized hero card title ("ĐÁNG NGỜ" or "THỦ PHẠM CHÍNH").
    pub title: &'static str,
    /// Accompanying geometric indicator shape (Triangle or Diamond).
    pub shape: SeverityShape,
    /// Localized badge label ("Đáng ngờ" or "Thủ phạm").
    pub badge_label: &'static str,
}

/// Returns the complete visual design tokens for a given severity level.
pub fn level_tokens(severity: Severity) -> LevelTokens {
    match severity {
        Severity::Suspicious => LevelTokens {
            border: COLOR_WARNING,
            bg: COLOR_BG_CARD_SUSPICIOUS,
            text_accent: COLOR_WARNING,
            title: strings::HERO_TITLE_SUSPICIOUS,
            shape: SeverityShape::Triangle,
            badge_label: strings::SEVERITY_SUSPICIOUS,
        },
        Severity::Culprit => LevelTokens {
            border: COLOR_DESTRUCTIVE,
            bg: COLOR_BG_CARD_CULPRIT,
            text_accent: COLOR_DESTRUCTIVE,
            title: strings::HERO_TITLE_CULPRIT,
            shape: SeverityShape::Diamond,
            badge_label: strings::SEVERITY_CULPRIT,
        },
        Severity::Normal => LevelTokens {
            border: COLOR_ACCENT_OK,
            bg: COLOR_BG_CARD_SMOOTH,
            text_accent: COLOR_ACCENT_OK,
            title: strings::HERO_NO_CULPRIT,
            shape: SeverityShape::Circle,
            badge_label: strings::SEVERITY_NORMAL,
        },
    }
}

/// Helper mapping a raw or smoothed culprit score directly to level design tokens.
pub fn level_tokens_for_score(score: f32) -> LevelTokens {
    level_tokens(Severity::from_score(score))
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

/// Header status badge visual presentation info.
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderBadgeInfo {
    pub level: Severity,
    pub shape: PressureShape,
    pub color: Color32,
    pub text: String,
}

/// Evaluates the unified system status level combining culprit presence and system pressure.
///
/// Follows the same level mapping as the hero card:
/// - Normal: no culprit and Smooth pressure.
/// - Suspicious: suspicious culprit or Strained pressure.
/// - Culprit/Overloaded: culprit process or Overloaded pressure.
pub fn evaluate_system_level(culprit: Option<&ScoredProcess>, pressure: PressureState) -> Severity {
    let pressure_level = match pressure {
        PressureState::Overloaded => Severity::Culprit,
        PressureState::Strained => Severity::Suspicious,
        PressureState::Smooth => Severity::Normal,
    };
    if let Some(target) = culprit {
        std::cmp::max(target.severity, pressure_level)
    } else {
        pressure_level
    }
}

/// Evaluates the header status badge presentation (shape, color, text).
/// Single source of truth for level -> badge/color.
pub fn evaluate_header_badge(
    culprit: Option<&ScoredProcess>,
    pressure: PressureState,
    totals: Option<&SystemTotals>,
) -> HeaderBadgeInfo {
    let level = evaluate_system_level(culprit, pressure);
    match level {
        Severity::Normal => HeaderBadgeInfo {
            level,
            shape: PressureShape::Circle,
            color: COLOR_ACCENT_OK,
            text: strings::PRESSURE_SMOOTH.to_string(),
        },
        Severity::Suspicious => HeaderBadgeInfo {
            level,
            shape: PressureShape::Triangle,
            color: COLOR_WARNING,
            text: strings::STATUS_SUSPICIOUS.to_string(),
        },
        Severity::Culprit => {
            let text = if pressure == PressureState::Overloaded {
                if let Some(t) = totals {
                    crate::analyzer::format_pressure_badge_text(t, PressureState::Overloaded)
                } else {
                    format!("{} tài nguyên", strings::PRESSURE_OVERLOADED)
                }
            } else if let Some(target) = culprit {
                let res_name = match target.main_bottleneck {
                    crate::model::Bottleneck::Cpu => "CPU",
                    crate::model::Bottleneck::Memory => "RAM",
                    crate::model::Bottleneck::Disk => "đĩa",
                    crate::model::Bottleneck::None => "tài nguyên",
                };
                format!("{} {}", strings::PRESSURE_OVERLOADED, res_name)
            } else if let Some(t) = totals {
                crate::analyzer::format_pressure_badge_text(t, PressureState::Overloaded)
            } else {
                format!("{} tài nguyên", strings::PRESSURE_OVERLOADED)
            };
            HeaderBadgeInfo {
                level,
                shape: PressureShape::Octagon,
                color: COLOR_DESTRUCTIVE,
                text,
            }
        }
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

        // 5b. Muted foreground on muted container surface (#272F42)
        let muted_on_muted = contrast_ratio(COLOR_MUTED_FOREGROUND, COLOR_MUTED);
        assert!(
            muted_on_muted >= 4.5,
            "Muted foreground on muted container must be >= 4.5 (got {muted_on_muted:.2})"
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

    #[test]
    fn test_level_color_token_mapping() {
        // 1. Suspicious level token assertions (score 40-69)
        let susp = level_tokens(Severity::Suspicious);
        assert_eq!(susp.border, COLOR_WARNING);
        assert_eq!(susp.border, Color32::from_rgb(245, 158, 11));
        assert_eq!(susp.bg, COLOR_BG_CARD_SUSPICIOUS);
        assert_eq!(susp.title, "ĐÁNG NGỜ");
        assert_eq!(susp.shape, SeverityShape::Triangle);
        assert_eq!(susp.badge_label, strings::SEVERITY_SUSPICIOUS);
        assert_eq!(susp.text_accent, COLOR_WARNING);

        // WCAG AA contrast ratio >= 4.5:1 on tinted suspicious background
        let border_on_susp = contrast_ratio(susp.border, susp.bg);
        assert!(
            border_on_susp >= 4.5,
            "Border on suspicious bg must be >= 4.5 (got {border_on_susp:.2})"
        );
        let fg_on_susp = contrast_ratio(COLOR_FOREGROUND, susp.bg);
        assert!(
            fg_on_susp >= 4.5,
            "Foreground text on suspicious bg must be >= 4.5 (got {fg_on_susp:.2})"
        );
        let muted_on_susp = contrast_ratio(COLOR_MUTED_FOREGROUND, susp.bg);
        assert!(
            muted_on_susp >= 4.5,
            "Muted text on suspicious bg must be >= 4.5 (got {muted_on_susp:.2})"
        );

        // 2. Culprit level token assertions (score >= 70)
        let culp = level_tokens(Severity::Culprit);
        assert_eq!(culp.border, COLOR_DESTRUCTIVE);
        assert_eq!(culp.border, Color32::from_rgb(239, 68, 68));
        assert_eq!(culp.bg, COLOR_BG_CARD_CULPRIT);
        assert_eq!(culp.title, "THỦ PHẠM CHÍNH");
        assert_eq!(culp.shape, SeverityShape::Diamond);
        assert_eq!(culp.badge_label, strings::SEVERITY_CULPRIT);
        assert_eq!(culp.text_accent, COLOR_DESTRUCTIVE);

        // WCAG AA contrast ratio >= 4.5:1 on tinted culprit background
        let border_on_culp = contrast_ratio(culp.border, culp.bg);
        assert!(
            border_on_culp >= 4.5,
            "Border on culprit bg must be >= 4.5 (got {border_on_culp:.2})"
        );
        let fg_on_culp = contrast_ratio(COLOR_FOREGROUND, culp.bg);
        assert!(
            fg_on_culp >= 4.5,
            "Foreground text on culprit bg must be >= 4.5 (got {fg_on_culp:.2})"
        );
        let muted_on_culp = contrast_ratio(COLOR_MUTED_FOREGROUND, culp.bg);
        assert!(
            muted_on_culp >= 4.5,
            "Muted text on culprit bg must be >= 4.5 (got {muted_on_culp:.2})"
        );

        // 3. Score-based mapping verification
        assert_eq!(level_tokens_for_score(40.0), susp);
        assert_eq!(level_tokens_for_score(57.4), susp);
        assert_eq!(level_tokens_for_score(69.9), susp);
        assert_eq!(level_tokens_for_score(70.0), culp);
        assert_eq!(level_tokens_for_score(99.1), culp);
    }

    #[test]
    fn test_evaluate_header_badge_levels() {
        use crate::model::{Bottleneck, ProcessKind, ProcessRecord};

        let make_proc = |severity: Severity, bottleneck: Bottleneck| ScoredProcess {
            record: ProcessRecord {
                pid: 1234,
                parent_pid: None,
                name: "heavy.exe".to_string(),
                exe_path: None,
                cpu_percent: 50.0,
                memory_bytes: 1024 * 1024 * 1024,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            kind: ProcessKind::UserApp,
            score: match severity {
                Severity::Normal => 20.0,
                Severity::Suspicious => 55.0,
                Severity::Culprit => 85.0,
            },
            severity,
            main_bottleneck: bottleneck,
            killable: true,
        };

        let totals = SystemTotals {
            cpu_percent: 30.0,
            memory_used_bytes: 4 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_bps: 100.0,
        };

        // 1. Normal: Smooth pressure and no culprit
        let b_normal = evaluate_header_badge(None, PressureState::Smooth, Some(&totals));
        assert_eq!(b_normal.level, Severity::Normal);
        assert_eq!(b_normal.color, COLOR_ACCENT_OK);
        assert_eq!(b_normal.shape, PressureShape::Circle);
        assert_eq!(b_normal.text, "Mượt mà");

        // 2. Suspicious: Suspicious culprit with Smooth pressure
        let susp_proc = make_proc(Severity::Suspicious, Bottleneck::Cpu);
        let b_susp_culprit =
            evaluate_header_badge(Some(&susp_proc), PressureState::Smooth, Some(&totals));
        assert_eq!(b_susp_culprit.level, Severity::Suspicious);
        assert_eq!(b_susp_culprit.color, COLOR_WARNING);
        assert_eq!(b_susp_culprit.shape, PressureShape::Triangle);
        assert_eq!(b_susp_culprit.text, "Có dấu hiệu chậm");

        // 3. Suspicious: Strained pressure without culprit
        let b_strained = evaluate_header_badge(None, PressureState::Strained, Some(&totals));
        assert_eq!(b_strained.level, Severity::Suspicious);
        assert_eq!(b_strained.color, COLOR_WARNING);
        assert_eq!(b_strained.shape, PressureShape::Triangle);
        assert_eq!(b_strained.text, "Có dấu hiệu chậm");

        // 4. Culprit: Culprit process with Smooth pressure (names culprit resource)
        let culp_proc = make_proc(Severity::Culprit, Bottleneck::Cpu);
        let b_culp = evaluate_header_badge(Some(&culp_proc), PressureState::Smooth, Some(&totals));
        assert_eq!(b_culp.level, Severity::Culprit);
        assert_eq!(b_culp.color, COLOR_DESTRUCTIVE);
        assert_eq!(b_culp.shape, PressureShape::Octagon);
        assert_eq!(b_culp.text, "Quá tải CPU");

        // 5. Culprit/Overload: Overloaded pressure with high CPU
        let overloaded_totals = SystemTotals {
            cpu_percent: 95.0,
            memory_used_bytes: 4 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_bps: 100.0,
        };
        let b_overload =
            evaluate_header_badge(None, PressureState::Overloaded, Some(&overloaded_totals));
        assert_eq!(b_overload.level, Severity::Culprit);
        assert_eq!(b_overload.color, COLOR_DESTRUCTIVE);
        assert_eq!(b_overload.shape, PressureShape::Octagon);
        assert_eq!(b_overload.text, "Quá tải CPU");
    }
}
