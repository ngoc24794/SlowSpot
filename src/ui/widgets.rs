//! Custom reusable GUI widgets and drawing helpers.
//!
//! Renders 60-second resource sparklines using `egui::Painter`, score gauges,
//! and status badges according to the application visual theme.

use eframe::egui::{
    self, Color32, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

use crate::model::{ProcessKind, Severity};
use crate::ui::strings;
use crate::ui::theme::{
    COLOR_BG_INPUT, COLOR_CULPRIT, COLOR_INFO, COLOR_NORMAL, COLOR_SUSPICIOUS, COLOR_TEXT_MUTED,
    COLOR_TEXT_PRIMARY,
};

/// Configuration parameters for rendering a sparkline graph.
#[derive(Debug, Clone, Copy)]
pub struct SparklineConfig<'a> {
    /// Metric title shown at top-left.
    pub title: &'a str,
    /// Current metric reading shown at top-right.
    pub current_label: &'a str,
    /// Minimum expected value on the Y axis.
    pub min_val: f32,
    /// Maximum expected value on the Y axis.
    pub max_val: f32,
    /// Plot line stroke color.
    pub line_color: Color32,
    /// Allocated widget size.
    pub size: Vec2,
}

/// Paints a 60-second resource sparkline graph using `egui::Painter` and `egui::Shape::line`.
///
/// Designed to avoid heavy plotting dependencies like `egui_plot`.
pub fn sparkline(ui: &mut Ui, cfg: SparklineConfig<'_>, values: &[f32]) -> Response {
    let (rect, response) = ui.allocate_exact_size(cfg.size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter();

    // 1. Background container
    painter.rect_filled(rect, 6.0, COLOR_BG_INPUT);
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20)),
        StrokeKind::Inside,
    );

    // 2. Header text: Title at top-left, current metric at top-right
    let header_y = rect.top() + 6.0;
    painter.text(
        Pos2::new(rect.left() + 8.0, header_y),
        egui::Align2::LEFT_TOP,
        cfg.title,
        egui::FontId::proportional(12.0),
        COLOR_TEXT_MUTED,
    );

    painter.text(
        Pos2::new(rect.right() - 8.0, header_y),
        egui::Align2::RIGHT_TOP,
        cfg.current_label,
        egui::FontId::proportional(13.0),
        COLOR_TEXT_PRIMARY,
    );

    // 3. Graph area
    let graph_top = rect.top() + 26.0;
    let graph_bottom = rect.bottom() - 6.0;
    let graph_left = rect.left() + 8.0;
    let graph_right = rect.right() - 8.0;

    let graph_height = (graph_bottom - graph_top).max(10.0);
    let graph_width = (graph_right - graph_left).max(10.0);

    // Draw baseline
    painter.line_segment(
        [
            Pos2::new(graph_left, graph_bottom),
            Pos2::new(graph_right, graph_bottom),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 15)),
    );

    // Draw 50% guideline
    let mid_y = graph_top + graph_height * 0.5;
    painter.line_segment(
        [Pos2::new(graph_left, mid_y), Pos2::new(graph_right, mid_y)],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 10)),
    );

    // 4. Plot points
    if !values.is_empty() {
        let max_range = (cfg.max_val - cfg.min_val).max(0.001);
        let n = values.len();

        let points: Vec<Pos2> = values
            .iter()
            .enumerate()
            .map(|(idx, &v)| {
                let x = if n > 1 {
                    graph_left + (idx as f32 / (n - 1) as f32) * graph_width
                } else {
                    graph_right
                };
                let norm = ((v - cfg.min_val) / max_range).clamp(0.0, 1.0);
                let y = graph_bottom - norm * graph_height;
                Pos2::new(x, y)
            })
            .collect();

        if points.len() >= 2 {
            painter.add(Shape::line(points, Stroke::new(1.8, cfg.line_color)));
        } else if points.len() == 1 {
            painter.circle_filled(points[0], 2.5, cfg.line_color);
        }
    }

    response
}

/// Renders a compact colored score progress bar with numeric label (0 to 100).
pub fn score_bar(ui: &mut Ui, score: f32) {
    let desired_size = Vec2::new(65.0, 14.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, Sense::hover());

    let painter = ui.painter();

    // Background track
    painter.rect_filled(rect, 3.0, Color32::from_rgb(35, 39, 50));

    let color = if score >= 70.0 {
        COLOR_CULPRIT
    } else if score >= 40.0 {
        COLOR_SUSPICIOUS
    } else {
        COLOR_NORMAL
    };

    // Filled portion
    let fill_ratio = (score / 100.0).clamp(0.0, 1.0);
    let filled_rect = Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width() * fill_ratio, rect.height()),
    );
    painter.rect_filled(filled_rect, 3.0, color);

    // Text label
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{:.0}", score),
        egui::FontId::proportional(10.0),
        Color32::WHITE,
    );
}

/// Renders a badge indicating process severity (Thủ phạm / Đáng ngờ / Bình thường).
pub fn severity_badge(ui: &mut Ui, severity: Severity) {
    let (text, bg_color, fg_color) = match severity {
        Severity::Culprit => (
            strings::SEVERITY_CULPRIT,
            Color32::from_rgb(90, 25, 25),
            COLOR_CULPRIT,
        ),
        Severity::Suspicious => (
            strings::SEVERITY_SUSPICIOUS,
            Color32::from_rgb(85, 60, 20),
            COLOR_SUSPICIOUS,
        ),
        Severity::Normal => (
            strings::SEVERITY_NORMAL,
            Color32::from_rgb(25, 65, 35),
            COLOR_NORMAL,
        ),
    };

    render_badge(ui, text, bg_color, fg_color);
}

/// Renders a badge indicating process kind (Ứng dụng / Dịch vụ nền / Hệ thống).
pub fn process_kind_badge(ui: &mut Ui, kind: ProcessKind) {
    let (text, bg_color, fg_color) = match kind {
        ProcessKind::UserApp => (
            strings::KIND_USER_APP,
            Color32::from_rgb(25, 45, 65),
            COLOR_INFO,
        ),
        ProcessKind::BackgroundService => (
            strings::KIND_BACKGROUND,
            Color32::from_rgb(45, 50, 60),
            COLOR_TEXT_MUTED,
        ),
        ProcessKind::SystemCritical => (
            strings::KIND_SYSTEM_CRITICAL,
            Color32::from_rgb(60, 30, 50),
            Color32::from_rgb(220, 140, 200),
        ),
    };

    render_badge(ui, text, bg_color, fg_color);
}

fn render_badge(ui: &mut Ui, text: &str, bg: Color32, fg: Color32) {
    let font_id = egui::FontId::proportional(11.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, fg);
    let size = galley.size() + Vec2::new(10.0, 4.0);

    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, 4.0, bg);
    painter.galley(rect.min + Vec2::new(5.0, 2.0), galley, fg);
}
