//! Custom reusable GUI widgets and drawing helpers.
//!
//! Renders geometric shapes (WCAG compliant), score bars with separate labels,
//! status badges, icon painters, mini metric chips, and overload episode indicators.

use eframe::egui::{
    self, Color32, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

use crate::model::{EpisodeState, ProcessKind, Severity};
use crate::ui::palette::{
    severity_indicator, PressureShape, SeverityShape, COLOR_BORDER_SUBTLE, COLOR_DESTRUCTIVE,
    COLOR_FOREGROUND, COLOR_INFO, COLOR_MUTED, COLOR_MUTED_FOREGROUND, COLOR_PROTECTED,
    COLOR_WARNING, FONT_SIZE_CAPTION, FONT_SIZE_TINY, RADIUS_SM,
};
use crate::ui::strings;

/// Renders a score progress bar with the numeric value cleanly outside the track.
///
/// Prevents illegible overlapping text on small gauges.
pub fn score_bar(ui: &mut Ui, score: f32) {
    let desired_size = Vec2::new(65.0, 14.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, Sense::hover());
    let painter = ui.painter();

    let color = if score >= 70.0 {
        COLOR_DESTRUCTIVE
    } else if score >= 40.0 {
        COLOR_WARNING
    } else {
        crate::ui::palette::COLOR_ACCENT_OK
    };

    // Split horizontally: mini track (38px) + gap (5px) + score text (22px)
    let bar_width = 38.0;
    let bar_height = 5.0;
    let bar_y = rect.center().y - (bar_height / 2.0);

    let track_rect = Rect::from_min_size(
        Pos2::new(rect.left(), bar_y),
        Vec2::new(bar_width, bar_height),
    );

    // Background track
    painter.rect_filled(track_rect, 2.5, COLOR_MUTED);
    painter.rect_stroke(
        track_rect,
        2.5,
        Stroke::new(1.0, COLOR_BORDER_SUBTLE),
        StrokeKind::Inside,
    );

    // Filled portion
    let fill_ratio = (score / 100.0).clamp(0.0, 1.0);
    if fill_ratio > 0.0 {
        let filled_width = (bar_width * fill_ratio).max(3.0);
        let filled_rect = Rect::from_min_size(
            track_rect.min,
            Vec2::new(filled_width.min(bar_width), bar_height),
        );
        painter.rect_filled(filled_rect, 2.5, color);
    }

    // Number text aligned to the right, outside the bar
    let text_pos = Pos2::new(rect.right(), rect.center().y);
    painter.text(
        text_pos,
        egui::Align2::RIGHT_CENTER,
        format!("{:.0}", score),
        egui::FontId::monospace(FONT_SIZE_CAPTION),
        COLOR_FOREGROUND,
    );
}

/// Renders a badge indicating process severity with shape + text.
pub fn severity_badge(ui: &mut Ui, severity: Severity) {
    let (text, shape, color) = severity_indicator(severity);
    let bg_color = color.gamma_multiply(0.18);

    let font_id = egui::FontId::proportional(FONT_SIZE_CAPTION);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font_id, color);
    let size = Vec2::new(galley.size().x + 22.0, 20.0);

    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();

    // Background pill
    painter.rect_filled(rect, RADIUS_SM, bg_color);
    painter.rect_stroke(
        rect,
        RADIUS_SM,
        Stroke::new(1.0, color.gamma_multiply(0.4)),
        StrokeKind::Inside,
    );

    // Shape icon on left
    let shape_center = Pos2::new(rect.left() + 9.0, rect.center().y);
    draw_severity_shape_at(painter, shape, shape_center, 8.0, color);

    // Text label
    painter.galley(
        Pos2::new(rect.left() + 17.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}

/// Renders a badge or label indicating process kind (Ứng dụng / Dịch vụ nền / Hệ thống).
///
/// Plain user apps are rendered as unbordered muted text to prevent visual noise and
/// avoid looking like clickable buttons, while background services and critical system
/// processes retain distinguishing badges.
pub fn process_kind_badge(ui: &mut Ui, kind: ProcessKind) {
    let font_id = egui::FontId::proportional(FONT_SIZE_CAPTION);

    if kind == ProcessKind::UserApp {
        let galley = ui.painter().layout_no_wrap(
            strings::KIND_USER_APP.to_string(),
            font_id,
            COLOR_MUTED_FOREGROUND,
        );
        let (rect, _) = ui.allocate_exact_size(Vec2::new(galley.size().x, 20.0), Sense::hover());
        let pos = Pos2::new(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, COLOR_MUTED_FOREGROUND);
        return;
    }

    let (text, bg_color, fg_color, has_lock) = match kind {
        ProcessKind::UserApp => unreachable!(),
        ProcessKind::BackgroundService => (
            strings::KIND_BACKGROUND,
            COLOR_MUTED,
            COLOR_MUTED_FOREGROUND,
            false,
        ),
        ProcessKind::SystemCritical => (
            strings::KIND_SYSTEM_CRITICAL,
            COLOR_PROTECTED.gamma_multiply(0.18),
            COLOR_PROTECTED,
            true,
        ),
    };

    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font_id, fg_color);
    let extra_x = if has_lock { 22.0 } else { 12.0 };
    let size = Vec2::new(galley.size().x + extra_x, 20.0);

    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, RADIUS_SM, bg_color);
    painter.rect_stroke(
        rect,
        RADIUS_SM,
        Stroke::new(1.0, fg_color.gamma_multiply(0.35)),
        StrokeKind::Inside,
    );

    if has_lock {
        let lock_rect = Rect::from_center_size(
            Pos2::new(rect.left() + 9.0, rect.center().y),
            Vec2::splat(11.0),
        );
        paint_lock_icon(painter, lock_rect, fg_color);
        painter.galley(
            Pos2::new(rect.left() + 17.0, rect.center().y - galley.size().y / 2.0),
            galley,
            fg_color,
        );
    } else {
        painter.galley(
            Pos2::new(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0),
            galley,
            fg_color,
        );
    }
}

/// Paints a geometric severity shape using Painter primitives.
fn draw_severity_shape_at(
    p: &egui::Painter,
    shape: SeverityShape,
    center: Pos2,
    size: f32,
    color: Color32,
) {
    let half = size / 2.0;
    match shape {
        SeverityShape::Circle => {
            p.circle_filled(center, half, color);
        }
        SeverityShape::Triangle => {
            let top = Pos2::new(center.x, center.y - half);
            let left = Pos2::new(center.x - half * 1.15, center.y + half * 0.9);
            let right = Pos2::new(center.x + half * 1.15, center.y + half * 0.9);
            p.add(Shape::convex_polygon(
                vec![top, right, left],
                color,
                Stroke::NONE,
            ));
        }
        SeverityShape::Diamond => {
            let top = Pos2::new(center.x, center.y - half * 1.1);
            let right = Pos2::new(center.x + half * 1.1, center.y);
            let bottom = Pos2::new(center.x, center.y + half * 1.1);
            let left = Pos2::new(center.x - half * 1.1, center.y);
            p.add(Shape::convex_polygon(
                vec![top, right, bottom, left],
                color,
                Stroke::NONE,
            ));
        }
    }
}

/// Paints a system pressure shape (Circle, Triangle, Octagon).
pub fn draw_pressure_shape_indicator(
    ui: &mut Ui,
    shape: PressureShape,
    color: Color32,
    size: f32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let center = rect.center();
        let half = size / 2.0;
        match shape {
            PressureShape::Circle => {
                p.circle_filled(center, half * 0.85, color);
            }
            PressureShape::Triangle => {
                let top = Pos2::new(center.x, center.y - half);
                let left = Pos2::new(center.x - half, center.y + half);
                let right = Pos2::new(center.x + half, center.y + half);
                p.add(Shape::convex_polygon(
                    vec![top, right, left],
                    color,
                    Stroke::NONE,
                ));
            }
            PressureShape::Octagon => {
                let d1 = half;
                let d2 = half * 0.414;
                let points = vec![
                    Pos2::new(center.x - d2, center.y - d1),
                    Pos2::new(center.x + d2, center.y - d1),
                    Pos2::new(center.x + d1, center.y - d2),
                    Pos2::new(center.x + d1, center.y + d2),
                    Pos2::new(center.x + d2, center.y + d1),
                    Pos2::new(center.x - d2, center.y + d1),
                    Pos2::new(center.x - d1, center.y + d2),
                    Pos2::new(center.x - d1, center.y - d2),
                ];
                p.add(Shape::convex_polygon(points, color, Stroke::NONE));
            }
        }
    }
    resp
}

/// Renders a compact chip for system resource status (CPU/RAM/Disk) with a mini progress bar.
pub fn mini_metric_chip(
    ui: &mut Ui,
    label: &str,
    val_str: &str,
    pct: f32,
    color: Color32,
) -> Response {
    mini_metric_chip_width(ui, label, val_str, pct, color, 126.0)
}

/// Renders a compact chip for system resource status (CPU/RAM/Disk) with an explicit width.
pub fn mini_metric_chip_width(
    ui: &mut Ui,
    label: &str,
    val_str: &str,
    pct: f32,
    color: Color32,
    width: f32,
) -> Response {
    let desired_size = Vec2::new(width, 24.0);
    let (rect, resp) = ui.allocate_exact_size(desired_size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return resp;
    }

    let p = ui.painter();
    p.rect_filled(rect, RADIUS_SM, COLOR_MUTED);
    p.rect_stroke(
        rect,
        RADIUS_SM,
        Stroke::new(1.0, COLOR_BORDER_SUBTLE),
        StrokeKind::Inside,
    );

    // Left label (CPU/RAM/Đĩa)
    let pad_left = 4.0;
    let pad_right = 4.0;
    let font_id = egui::FontId::proportional(FONT_SIZE_CAPTION);
    let label_galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), font_id, COLOR_MUTED_FOREGROUND);
    let label_w = label_galley.size().x;
    let gap = 6.0;
    let bar_w = 12.0;

    p.galley(
        Pos2::new(
            rect.left() + pad_left,
            rect.center().y - label_galley.size().y / 2.0,
        ),
        label_galley,
        COLOR_MUTED_FOREGROUND,
    );

    // Mini bar in the middle with 6px gap from label and non-overlapping bounds
    let bar_left = rect.left() + pad_left + label_w + gap;
    let bar_h = 4.0;
    let bar_y = rect.center().y - (bar_h / 2.0);

    let track = Rect::from_min_size(Pos2::new(bar_left, bar_y), Vec2::new(bar_w, bar_h));
    p.rect_filled(track, 2.0, Color32::from_rgb(22, 28, 40));

    let fill_ratio = (pct / 100.0).clamp(0.0, 1.0);
    if fill_ratio > 0.0 {
        let filled = Rect::from_min_size(track.min, Vec2::new(bar_w * fill_ratio, bar_h));
        p.rect_filled(filled, 2.0, color);
    }

    // Right-aligned value text (strictly right of the bar track, guaranteed non-overlapping)
    p.text(
        Pos2::new(rect.right() - pad_right, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        val_str,
        egui::FontId::monospace(FONT_SIZE_CAPTION),
        COLOR_FOREGROUND,
    );

    resp
}

/// Renders a crisp checkmark icon using `egui::Painter`.
pub fn draw_checkmark(ui: &mut Ui, color: Color32, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let stroke = Stroke::new((size * 0.12).max(1.5), color);
        let p1 = Pos2::new(
            rect.left() + rect.width() * 0.18,
            rect.top() + rect.height() * 0.52,
        );
        let p2 = Pos2::new(
            rect.left() + rect.width() * 0.42,
            rect.top() + rect.height() * 0.78,
        );
        let p3 = Pos2::new(
            rect.left() + rect.width() * 0.85,
            rect.top() + rect.height() * 0.25,
        );
        p.line_segment([p1, p2], stroke);
        p.line_segment([p2, p3], stroke);
    }
    resp
}

/// Renders a warning triangle icon using `egui::Painter`.
pub fn draw_warning_icon(ui: &mut Ui, color: Color32, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let stroke = Stroke::new((size * 0.10).max(1.2), color);
        let top = Pos2::new(rect.center().x, rect.top() + rect.height() * 0.10);
        let left = Pos2::new(
            rect.left() + rect.width() * 0.10,
            rect.bottom() - rect.height() * 0.10,
        );
        let right = Pos2::new(
            rect.right() - rect.width() * 0.10,
            rect.bottom() - rect.height() * 0.10,
        );
        p.line_segment([top, left], stroke);
        p.line_segment([left, right], stroke);
        p.line_segment([right, top], stroke);

        let mid_x = rect.center().x;
        let line_top = rect.top() + rect.height() * 0.38;
        let line_bot = rect.top() + rect.height() * 0.62;
        p.line_segment(
            [Pos2::new(mid_x, line_top), Pos2::new(mid_x, line_bot)],
            stroke,
        );
        let dot_y = rect.top() + rect.height() * 0.75;
        p.circle_filled(Pos2::new(mid_x, dot_y), stroke.width * 0.6, color);
    }
    resp
}

/// Renders a simple lock icon (ổ khoá) using `egui::Painter`.
pub fn draw_lock_icon(ui: &mut Ui, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_lock_icon(ui.painter(), rect, color);
    }
    resp
}

fn paint_lock_icon(p: &egui::Painter, rect: Rect, color: Color32) {
    let shackle = Rect::from_min_size(
        Pos2::new(rect.left() + 3.0, rect.top() + 1.0),
        Vec2::new(8.0, 6.0),
    );
    p.rect_stroke(shackle, 4.0, Stroke::new(1.4, color), StrokeKind::Inside);
    let body = Rect::from_min_size(
        Pos2::new(rect.left() + 1.5, rect.top() + 5.5),
        Vec2::new(11.0, 7.5),
    );
    p.rect_filled(body, 1.5, color);
}

/// Renders a pause icon (two vertical bars) using `egui::Painter`.
pub fn draw_pause_icon(ui: &mut Ui, color: Color32, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let bar_w = (size * 0.22).max(2.0);
        let bar_h = size * 0.65;
        let y = rect.center().y - (bar_h / 2.0);

        let left_bar = Rect::from_min_size(
            Pos2::new(rect.center().x - bar_w * 1.3, y),
            Vec2::new(bar_w, bar_h),
        );
        let right_bar = Rect::from_min_size(
            Pos2::new(rect.center().x + bar_w * 0.3, y),
            Vec2::new(bar_w, bar_h),
        );

        p.rect_filled(left_bar, 1.0, color);
        p.rect_filled(right_bar, 1.0, color);
    }
    resp
}

/// Renders a sharp sort indicator arrow (pointing up for Ascending, down for Descending).
pub fn draw_sort_arrow(ui: &mut Ui, ascending: bool, color: Color32) -> Response {
    let size = 9.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let center = rect.center();
        let h = size * 0.45;
        let w = size * 0.45;

        if ascending {
            let top = Pos2::new(center.x, center.y - h);
            let left = Pos2::new(center.x - w, center.y + h);
            let right = Pos2::new(center.x + w, center.y + h);
            p.add(Shape::convex_polygon(
                vec![top, right, left],
                color,
                Stroke::NONE,
            ));
        } else {
            let bottom = Pos2::new(center.x, center.y + h);
            let left = Pos2::new(center.x - w, center.y - h);
            let right = Pos2::new(center.x + w, center.y - h);
            p.add(Shape::convex_polygon(
                vec![bottom, left, right],
                color,
                Stroke::NONE,
            ));
        }
    }
    resp
}

/// Renders a state badge for a watched episode with WCAG-compliant shape + text.
pub fn watched_state_badge(ui: &mut Ui, state: EpisodeState, score: f32, remaining_secs: u64) {
    let (text, shape, color) = match state {
        EpisodeState::Overloaded => {
            let severity = Severity::from_score(score);
            let (label, shp, clr) = severity_indicator(severity);
            (label.to_string(), shp, clr)
        }
        EpisodeState::Cooling => (
            format!(
                "{}{}{}",
                strings::WATCHED_BADGE_COOLING_PREFIX,
                remaining_secs,
                strings::WATCHED_BADGE_COOLING_SUFFIX
            ),
            SeverityShape::Circle,
            COLOR_WARNING,
        ),
        EpisodeState::Exited => (
            strings::WATCHED_BADGE_EXITED.to_string(),
            SeverityShape::Diamond,
            COLOR_MUTED_FOREGROUND,
        ),
        EpisodeState::Recovered => (
            "Đã hồi phục".to_string(),
            SeverityShape::Circle,
            crate::ui::palette::COLOR_ACCENT_OK,
        ),
    };

    let bg_color = color.gamma_multiply(0.18);
    let font_id = egui::FontId::proportional(FONT_SIZE_CAPTION);
    let galley = ui.painter().layout_no_wrap(text, font_id, color);
    let size = Vec2::new(galley.size().x + 22.0, 20.0);

    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, RADIUS_SM, bg_color);
    painter.rect_stroke(
        rect,
        RADIUS_SM,
        Stroke::new(1.0, color.gamma_multiply(0.4)),
        StrokeKind::Inside,
    );

    let shape_center = Pos2::new(rect.left() + 9.0, rect.center().y);
    draw_severity_shape_at(painter, shape, shape_center, 8.0, color);

    painter.galley(
        Pos2::new(rect.left() + 17.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}

/// Renders a tiny sparkline trend for watched process scores.
pub fn mini_sparkline(ui: &mut Ui, values: &[f32], width: f32, height: f32) -> Rect {
    let size = Vec2::new(width, height);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) || values.len() < 2 {
        return rect;
    }

    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, COLOR_MUTED);

    let max_val = values.iter().copied().fold(100.0f32, f32::max);
    let min_val = 0.0f32;
    let range = (max_val - min_val).max(1.0);

    let points: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, &val)| {
            let x = rect.left() + (i as f32 / (values.len() - 1) as f32) * rect.width();
            let norm = ((val - min_val) / range).clamp(0.0, 1.0);
            let y = rect.bottom() - 1.0 - norm * (rect.height() - 2.0);
            Pos2::new(x, y)
        })
        .collect();

    let last_val = *values.last().unwrap_or(&0.0);
    let stroke_color = if last_val >= 70.0 {
        COLOR_DESTRUCTIVE
    } else if last_val >= 40.0 {
        COLOR_WARNING
    } else {
        crate::ui::palette::COLOR_ACCENT_OK
    };

    painter.add(Shape::line(points, Stroke::new(1.5, stroke_color)));
    rect
}

/// Renders a compact badge/tag marking a process row as actively monitored.
pub fn watched_tag(ui: &mut Ui, severity: Severity) {
    let font_id = egui::FontId::proportional(FONT_SIZE_CAPTION);
    let text = strings::BADGE_WATCHED_TAG;
    let color = match severity {
        Severity::Culprit => COLOR_DESTRUCTIVE,
        Severity::Suspicious => COLOR_WARNING,
        Severity::Normal => COLOR_WARNING,
    };
    let bg_color = color.gamma_multiply(0.18);

    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font_id, color);
    let size = Vec2::new(galley.size().x + 8.0, 16.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, RADIUS_SM, bg_color);
    painter.rect_stroke(
        rect,
        RADIUS_SM,
        Stroke::new(1.0, color.gamma_multiply(0.4)),
        StrokeKind::Inside,
    );

    painter.galley(
        Pos2::new(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}

/// Renders a small badge showing process group count (e.g. "×3").
/// Only rendered when group has >= 2 processes.
pub fn group_tag(ui: &mut Ui, count: usize) {
    if count < 2 {
        return;
    }
    let font_id = egui::FontId::proportional(FONT_SIZE_TINY);
    let text = format!("×{count}");
    let color = COLOR_INFO;
    let bg_color = color.gamma_multiply(0.2);

    let galley = ui.painter().layout_no_wrap(text, font_id, color);
    let size = Vec2::new(galley.size().x + 8.0, 16.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, RADIUS_SM, bg_color);
    painter.rect_stroke(
        rect,
        RADIUS_SM,
        Stroke::new(1.0, color.gamma_multiply(0.5)),
        StrokeKind::Inside,
    );

    painter.galley(
        Pos2::new(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::palette::{COLOR_ACCENT_OK, COLOR_DESTRUCTIVE, COLOR_WARNING};

    #[test]
    fn test_mini_metric_chip_no_overlap() {
        let ctx = egui::Context::default();
        let raw_input = egui::RawInput::default();
        let mut output = ctx.run_ui(raw_input, |ui| {
            mini_metric_chip(ui, "Đĩa", "999.9 MB/s", 100.0, COLOR_ACCENT_OK);
            mini_metric_chip(ui, "Đĩa", "98.6 KB/s", 50.0, COLOR_ACCENT_OK);
            mini_metric_chip(ui, "RAM", "100%", 100.0, COLOR_WARNING);
            mini_metric_chip(ui, "CPU", "100.0%", 100.0, COLOR_DESTRUCTIVE);
        });

        for shape in &output.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                let text = text_shape.galley.text();
                if text == "999.9 MB/s" || text == "98.6 KB/s" || text == "100%" || text == "100.0%"
                {
                    let text_min_x = text_shape.pos.x;
                    let text_max_x = text_shape.pos.x + text_shape.galley.size().x;
                    let chip_right = text_max_x + 4.0;
                    let chip_left = chip_right - 126.0;
                    // Max bar right edge across all label types:
                    // pad_left(4) + max_label_w(~27) + gap(6) + bar_w(12) = 49.0
                    let max_bar_right = chip_left + 49.0;
                    assert!(
                        text_min_x >= max_bar_right,
                        "Value text '{text}' min x ({text_min_x}) must be >= bar max x ({max_bar_right})"
                    );
                }
            }
        }
        output.textures_delta.clear();
    }

    #[test]
    fn test_mini_metric_chip_label_bar_gap_at_least_6px() {
        let ctx = egui::Context::default();
        let raw_input = egui::RawInput::default();
        let mut output = ctx.run_ui(raw_input, |ui| {
            mini_metric_chip(ui, "Đĩa", "10 MB/s", 50.0, COLOR_ACCENT_OK);
            mini_metric_chip(ui, "RAM", "50%", 50.0, COLOR_WARNING);
            mini_metric_chip(ui, "CPU", "50.0%", 50.0, COLOR_DESTRUCTIVE);
        });

        let mut labels = Vec::new();
        let mut rects = Vec::new();

        for shape in &output.shapes {
            match &shape.shape {
                egui::epaint::Shape::Text(text_shape) => {
                    let text = text_shape.galley.text();
                    if text == "Đĩa" || text == "RAM" || text == "CPU" {
                        let max_x = text_shape.pos.x + text_shape.galley.size().x;
                        labels.push((text.to_string(), max_x));
                    }
                }
                egui::epaint::Shape::Rect(rect_shape)
                    if rect_shape.fill == Color32::from_rgb(22, 28, 40) =>
                {
                    rects.push(rect_shape.rect.min.x);
                }
                _ => {}
            }
        }

        assert_eq!(labels.len(), 3);
        assert_eq!(rects.len(), 3);
        for ((label, label_max_x), bar_min_x) in labels.into_iter().zip(rects) {
            let gap = bar_min_x - label_max_x;
            assert!(
                gap >= 5.99,
                "Gap between label '{label}' and mini bar must be at least 6px, got {gap}px"
            );
        }
        output.textures_delta.clear();
    }

    #[test]
    fn test_group_tag_requires_at_least_two_processes() {
        let ctx = egui::Context::default();
        let raw_input = egui::RawInput::default();
        let mut output = ctx.run_ui(raw_input, |ui| {
            group_tag(ui, 1);
            group_tag(ui, 0);
            group_tag(ui, 2);
            group_tag(ui, 5);
        });

        let mut rendered_texts = Vec::new();
        for shape in &output.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                rendered_texts.push(text_shape.galley.text().to_string());
            }
        }
        assert!(!rendered_texts.contains(&"×1".to_string()));
        assert!(!rendered_texts.contains(&"×0".to_string()));
        assert!(rendered_texts.contains(&"×2".to_string()));
        assert!(rendered_texts.contains(&"×5".to_string()));
        output.textures_delta.clear();
    }
}
