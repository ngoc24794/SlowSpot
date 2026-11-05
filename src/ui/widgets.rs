//! Custom reusable GUI widgets and drawing helpers.
//!
//! Renders geometric shapes (WCAG compliant), score bars with separate labels,
//! status badges, icon painters, and mini metric chips.

use eframe::egui::{
    self, Color32, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

use crate::model::{ProcessKind, Severity};
use crate::ui::palette::{
    severity_indicator, PressureShape, SeverityShape, COLOR_BORDER_SUBTLE, COLOR_DESTRUCTIVE,
    COLOR_FOREGROUND, COLOR_MUTED, COLOR_MUTED_FOREGROUND, COLOR_PROTECTED, COLOR_WARNING,
    FONT_SIZE_CAPTION, RADIUS_SM,
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
    let desired_size = Vec2::new(138.0, 26.0);
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
    p.text(
        Pos2::new(rect.left() + 7.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(FONT_SIZE_CAPTION),
        COLOR_MUTED_FOREGROUND,
    );

    // Right value (e.g. 45% or 12.5 MB/s)
    p.text(
        Pos2::new(rect.right() - 7.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        val_str,
        egui::FontId::monospace(FONT_SIZE_CAPTION),
        COLOR_FOREGROUND,
    );

    // Mini bar in the middle
    let bar_left = rect.left() + 38.0;
    let bar_right = rect.right() - 56.0;
    let bar_w = (bar_right - bar_left).max(20.0);
    let bar_h = 4.0;
    let bar_y = rect.center().y - (bar_h / 2.0);

    let track = Rect::from_min_size(Pos2::new(bar_left, bar_y), Vec2::new(bar_w, bar_h));
    p.rect_filled(track, 2.0, Color32::from_rgb(22, 28, 40));

    let fill_ratio = (pct / 100.0).clamp(0.0, 1.0);
    if fill_ratio > 0.0 {
        let filled = Rect::from_min_size(track.min, Vec2::new(bar_w * fill_ratio, bar_h));
        p.rect_filled(filled, 2.0, color);
    }

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
