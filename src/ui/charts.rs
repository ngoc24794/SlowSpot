//! Real-time 60-second resource sparklines and area charts.
//!
//! Features translucent area shading, faint horizontal reference grids (25/50/75%),
//! peak/current metric readouts, time axis captions, and hover inspection tooltips.

use eframe::egui::{self, Color32, Pos2, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::palette::{
    COLOR_BORDER_SUBTLE, COLOR_FOREGROUND, COLOR_MUTED, COLOR_MUTED_FOREGROUND, FONT_SIZE_BODY,
    FONT_SIZE_CAPTION, FONT_SIZE_TINY, RADIUS_MD,
};
use crate::ui::strings;

/// Configuration options for rendering a 60-second resource sparkline.
#[derive(Debug, Clone, Copy)]
pub struct SparklineConfig<'a> {
    /// Metric title shown at top-left.
    pub title: &'a str,
    /// Current metric readout string shown at top-right.
    pub current_label: &'a str,
    /// Metric measurement unit (e.g. "%" or " MB/s").
    pub unit: &'a str,
    /// Minimum expected value on the Y axis.
    pub min_val: f32,
    /// Maximum expected value on the Y axis.
    pub max_val: f32,
    /// Main plot line stroke and area tint color.
    pub line_color: Color32,
    /// Allocated widget size.
    pub size: Vec2,
}

/// Paints an area sparkline graph using `egui::Painter`.
pub fn sparkline(ui: &mut Ui, cfg: SparklineConfig<'_>, values: &[f32]) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(cfg.size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter();

    // 1. Container background and border
    painter.rect_filled(rect, RADIUS_MD, COLOR_MUTED);
    painter.rect_stroke(
        rect,
        RADIUS_MD,
        Stroke::new(1.0, COLOR_BORDER_SUBTLE),
        StrokeKind::Inside,
    );

    // 2. Header metrics
    let header_y = rect.top() + 7.0;

    // Title at top-left
    painter.text(
        Pos2::new(rect.left() + 9.0, header_y),
        egui::Align2::LEFT_TOP,
        cfg.title,
        egui::FontId::proportional(FONT_SIZE_CAPTION),
        COLOR_MUTED_FOREGROUND,
    );

    // Peak metric in header
    let peak_val = values.iter().fold(0.0f32, |acc, &v| acc.max(v));
    let peak_str = format!(
        "{} {:.1}{}",
        strings::SPARKLINE_PEAK_PREFIX,
        peak_val,
        cfg.unit
    );
    let title_offset_x = rect.left() + 105.0;
    if rect.width() > 220.0 {
        painter.text(
            Pos2::new(title_offset_x, header_y),
            egui::Align2::LEFT_TOP,
            peak_str,
            egui::FontId::proportional(FONT_SIZE_TINY),
            COLOR_MUTED_FOREGROUND.gamma_multiply(0.85),
        );
    }

    // Current value at top-right
    painter.text(
        Pos2::new(rect.right() - 9.0, header_y - 1.0),
        egui::Align2::RIGHT_TOP,
        cfg.current_label,
        egui::FontId::monospace(FONT_SIZE_BODY),
        COLOR_FOREGROUND,
    );

    // 3. Graph Plot Area
    let graph_top = rect.top() + 27.0;
    let graph_bottom = rect.bottom() - 17.0;
    let graph_left = rect.left() + 9.0;
    let graph_right = rect.right() - 9.0;

    let graph_height = (graph_bottom - graph_top).max(10.0);
    let graph_width = (graph_right - graph_left).max(10.0);

    // Faint horizontal reference grid lines (25%, 50%, 75%)
    let grid_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 8));
    for frac in [0.25f32, 0.50, 0.75] {
        let y = graph_bottom - frac * graph_height;
        painter.line_segment(
            [Pos2::new(graph_left, y), Pos2::new(graph_right, y)],
            grid_stroke,
        );
    }

    // Baseline
    painter.line_segment(
        [
            Pos2::new(graph_left, graph_bottom),
            Pos2::new(graph_right, graph_bottom),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 16)),
    );

    // 4. Time axis captions ("-60s" … "nay")
    let time_y = rect.bottom() - 4.0;
    painter.text(
        Pos2::new(graph_left, time_y),
        egui::Align2::LEFT_BOTTOM,
        strings::SPARKLINE_TIME_START,
        egui::FontId::proportional(FONT_SIZE_TINY),
        COLOR_MUTED_FOREGROUND.gamma_multiply(0.7),
    );
    painter.text(
        Pos2::new(graph_right, time_y),
        egui::Align2::RIGHT_BOTTOM,
        strings::SPARKLINE_TIME_NOW,
        egui::FontId::proportional(FONT_SIZE_TINY),
        COLOR_MUTED_FOREGROUND.gamma_multiply(0.7),
    );

    // 5. Data Points, Area Shading, and Line
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

        // Shaded area under the line
        let area_fill = cfg.line_color.gamma_multiply(0.12);
        for i in 0..(points.len().saturating_sub(1)) {
            let p1 = points[i];
            let p2 = points[i + 1];
            let quad = vec![
                Pos2::new(p1.x, graph_bottom),
                p1,
                p2,
                Pos2::new(p2.x, graph_bottom),
            ];
            painter.add(Shape::convex_polygon(quad, area_fill, Stroke::NONE));
        }

        // Crisp plot line
        if points.len() >= 2 {
            painter.add(Shape::line(
                points.clone(),
                Stroke::new(1.8, cfg.line_color),
            ));
        }

        // Current value dot at the end
        if let Some(&last_pt) = points.last() {
            painter.circle_filled(last_pt, 2.5, cfg.line_color);
        }

        // 6. Hover inspection tooltip
        if response.hovered() {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if mouse_pos.x >= graph_left && mouse_pos.x <= graph_right {
                    let rel_x = (mouse_pos.x - graph_left) / graph_width;
                    let idx = ((rel_x * (n - 1) as f32).round() as usize).clamp(0, n - 1);
                    let target_pt = points[idx];
                    let val = values[idx];
                    let secs_ago = n - 1 - idx;

                    // Vertical hairline
                    painter.line_segment(
                        [
                            Pos2::new(target_pt.x, graph_top),
                            Pos2::new(target_pt.x, graph_bottom),
                        ],
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 50)),
                    );

                    // Indicator ring
                    painter.circle_filled(target_pt, 3.5, cfg.line_color);
                    painter.circle_stroke(target_pt, 4.5, Stroke::new(1.0, Color32::WHITE));

                    // Show hover tooltip
                    let time_label = if secs_ago == 0 {
                        strings::SPARKLINE_TIME_NOW.to_string()
                    } else {
                        format!("-{secs_ago}s")
                    };
                    let tip_text = format!("{time_label}: {:.1}{}", val, cfg.unit);
                    response = response.on_hover_text_at_pointer(tip_text);
                }
            }
        }
    }

    response
}
