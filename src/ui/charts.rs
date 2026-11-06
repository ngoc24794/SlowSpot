//! Real-time 60-second resource sparklines and area charts.
//!
//! Features translucent area shading, faint horizontal reference grids (25/50/75%),
//! peak/current metric readouts, time axis captions, and hover inspection tooltips.

use eframe::egui::{self, Color32, Pos2, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::palette::{
    COLOR_BORDER_SUBTLE, COLOR_FOREGROUND, COLOR_MUTED, COLOR_MUTED_FOREGROUND, FONT_SIZE_BODY,
    FONT_SIZE_CAPTION, RADIUS_MD,
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

/// Computes equal widths for 3 resource sparkline cards distributed across available width.
///
/// Ensures the sum of the 3 cards plus spacing between them precisely equals `available_width`
/// without causing rounding overflows or fractional pixel jitter.
pub fn calculate_sparkline_widths(available_width: f32, spacing: f32) -> [f32; 3] {
    let total_spacing = spacing * 2.0;
    let usable_width = (available_width - total_spacing).max(0.0);
    let card_width = (usable_width / 3.0).floor().max(1.0);
    let last_card_width = (usable_width - card_width * 2.0).max(1.0);
    [card_width, card_width, last_card_width]
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
    let header_y = rect.top() + 3.0;

    // Current value at top-right
    let current_font = egui::FontId::monospace(FONT_SIZE_BODY);
    let current_galley = painter.layout_no_wrap(
        cfg.current_label.to_string(),
        current_font,
        COLOR_FOREGROUND,
    );
    let current_w = current_galley.size().x;
    let current_pos = Pos2::new(rect.right() - 8.0 - current_w, header_y);
    painter.galley(current_pos, current_galley, COLOR_FOREGROUND);

    // Title at top-left, dynamically constrained to prevent overlap with current readout
    let caption_font = egui::FontId::proportional(FONT_SIZE_CAPTION);
    let max_title_w = (current_pos.x - (rect.left() + 8.0) - 6.0).max(10.0);
    let title_galley = painter.layout(
        cfg.title.to_string(),
        caption_font.clone(),
        COLOR_MUTED_FOREGROUND,
        max_title_w,
    );
    painter.galley(
        Pos2::new(rect.left() + 8.0, header_y),
        title_galley,
        COLOR_MUTED_FOREGROUND,
    );

    // Peak metric on its own line right under the title (Defect 2: no overlap at any width >= 760)
    let peak_val = values.iter().fold(0.0f32, |acc, &v| acc.max(v));
    let peak_str = format!(
        "{} {:.1}{}",
        strings::SPARKLINE_PEAK_PREFIX,
        peak_val,
        cfg.unit
    );
    let peak_y = header_y + 11.0;
    let max_peak_w = (rect.width() - 16.0).max(10.0);
    let peak_galley = painter.layout(peak_str, caption_font, COLOR_MUTED_FOREGROUND, max_peak_w);
    painter.galley(
        Pos2::new(rect.left() + 8.0, peak_y),
        peak_galley,
        COLOR_MUTED_FOREGROUND,
    );

    // 3. Graph Plot Area
    let graph_top = rect.top() + 25.0;
    let graph_bottom = rect.bottom() - 10.0;
    let graph_left = rect.left() + 8.0;
    let graph_right = rect.right() - 8.0;

    let graph_height = (graph_bottom - graph_top).max(5.0);
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

    // 4. Time axis captions ("-60s" … "nay") (>= 12px caption, solid high-contrast muted foreground)
    let time_y = rect.bottom() - 1.0;
    painter.text(
        Pos2::new(graph_left, time_y),
        egui::Align2::LEFT_BOTTOM,
        strings::SPARKLINE_TIME_START,
        egui::FontId::proportional(FONT_SIZE_CAPTION),
        COLOR_MUTED_FOREGROUND,
    );
    painter.text(
        Pos2::new(graph_right, time_y),
        egui::Align2::RIGHT_BOTTOM,
        strings::SPARKLINE_TIME_NOW,
        egui::FontId::proportional(FONT_SIZE_CAPTION),
        COLOR_MUTED_FOREGROUND,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_three_card_widths_pure_layout() {
        // Test standard and boundary widths (1000px, 760px, and inner widths with 12px outer margins)
        for &available in &[1000.0, 976.0, 760.0, 736.0, 600.0, 500.0] {
            let spacing = 8.0;
            let [w0, w1, w2] = calculate_sparkline_widths(available, spacing);

            // 1. Total width of 3 cards plus the 2 inter-card gaps must match available width exactly
            let total = w0 + w1 + w2 + spacing * 2.0;
            assert!(
                (total - available).abs() < 0.001,
                "Total width ({total}) must match available width ({available})"
            );

            // 2. All cards must have strictly positive width
            assert!(w0 > 0.0 && w1 > 0.0 && w2 > 0.0);

            // 3. First two cards must be identical
            assert!((w0 - w1).abs() < 0.001);

            // 4. Last card may differ by at most 2.0px due to floor rounding of integer division
            assert!(
                (w0 - w2).abs() <= 2.0,
                "Card width divergence ({w0} vs {w2}) must not exceed 2.0px"
            );
        }
    }
    #[test]
    fn test_sparkline_title_and_peak_on_separate_lines_no_overlap() {
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        // Test at minimum supported card width (window width 760 -> card width ~240px)
        for &card_w in &[240.0, 320.0, 400.0] {
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(card_w + 50.0, 200.0),
                )),
                ..Default::default()
            };

            let mut output = ctx.run_ui(raw_input, |ui| {
                let cfg = SparklineConfig {
                    title: strings::SPARKLINE_DISK_TITLE,
                    current_label: "997.5 MB/s",
                    unit: " MB/s",
                    min_val: 0.0,
                    max_val: 1000.0,
                    line_color: Color32::GREEN,
                    size: Vec2::new(card_w, 68.0),
                };
                let values = [100.0, 500.0, 997.5];
                sparkline(ui, cfg, &values);
            });

            // Inspect text galleys to verify title and peak Y positions do not collide
            let mut text_rects = Vec::new();
            for shape in &output.shapes {
                if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                    text_rects.push((text_shape.galley.text().to_string(), text_shape.pos));
                }
            }

            let title_entry = text_rects.iter().find(|(t, _)| t.contains("Đĩa I/O"));
            let peak_entry = text_rects.iter().find(|(t, _)| t.contains("Đỉnh"));

            assert!(title_entry.is_some(), "Title text should be rendered");
            assert!(peak_entry.is_some(), "Peak text should be rendered");

            let (_, title_pos) = title_entry.unwrap();
            let (_, peak_pos) = peak_entry.unwrap();

            // Peak must be on its own line below the title (vertical separation >= 10px)
            assert!(
                peak_pos.y >= title_pos.y + 10.0,
                "Peak metric at y={} must be on its own line below title at y={}",
                peak_pos.y,
                title_pos.y
            );
            output.textures_delta.clear();
        }
    }
}
