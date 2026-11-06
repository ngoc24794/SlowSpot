//! Process table layout, flat headers, sort indicators, row styling,
//! zebra striping, selection accent bars, and accessible empty states.

use eframe::egui::{self, Color32, Pos2, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};

use crate::model::{ScoredProcess, Severity};
use crate::ui::palette::{
    COLOR_BORDER_SUBTLE, COLOR_DESTRUCTIVE, COLOR_FOREGROUND, COLOR_INFO, COLOR_MUTED,
    COLOR_MUTED_FOREGROUND, COLOR_PROTECTED, COLOR_SELECTED_ROW, COLOR_WARNING, FONT_SIZE_BODY,
    FONT_SIZE_CAPTION, RADIUS_SM,
};
use crate::ui::state::{format_bps, format_bytes, format_percent, SortColumn, SortDirection};
use crate::ui::strings;
use crate::ui::widgets;

/// Calculated column widths for the process table ensuring consistent alignment.
#[derive(Clone, Copy, Debug)]
pub struct TableLayout {
    pub name_width: f32,
    pub pid_width: f32,
    pub cpu_width: f32,
    pub ram_width: f32,
    pub disk_width: f32,
    pub score_width: f32,
    pub kind_width: f32,
    pub spacing: f32,
}

impl TableLayout {
    /// Computes column widths based on available width and horizontal item spacing.
    pub fn compute(card_available: f32, spacing: f32) -> Self {
        let pid_width = 55.0;
        let cpu_width = 70.0;
        let ram_width = 80.0;
        let disk_width = 90.0;
        let score_width = 75.0;
        let kind_width = 80.0;

        let fixed_total = pid_width + cpu_width + ram_width + disk_width + score_width + kind_width;
        let total_spacing = 6.0 * spacing;
        let inner_width = (card_available - 8.0).max(fixed_total + total_spacing + 80.0);
        let name_width = (inner_width - fixed_total - total_spacing).max(80.0);

        Self {
            name_width,
            pid_width,
            cpu_width,
            ram_width,
            disk_width,
            score_width,
            kind_width,
            spacing,
        }
    }
}

/// Allocates a table cell with fixed width and layout, clipping content to prevent overflow.
pub fn render_cell<R>(
    ui: &mut egui::Ui,
    width: f32,
    layout: egui::Layout,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(Vec2::new(width, 24.0), layout, |ui| {
        ui.set_min_size(Vec2::new(width, 24.0));
        ui.set_max_size(Vec2::new(width, 24.0));
        let cell_clip = ui.clip_rect().intersect(ui.max_rect());
        ui.set_clip_rect(cell_clip);
        add_contents(ui)
    })
}

/// Renders a flat, unbordered column header button with integrated Painter sort arrow.
pub fn render_flat_header(
    ui: &mut egui::Ui,
    title: &str,
    col: SortColumn,
    width: f32,
    align: egui::Align,
    active_col: SortColumn,
    active_dir: SortDirection,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let is_active = active_col == col;
    let text_color = if is_active || resp.hovered() {
        COLOR_FOREGROUND
    } else {
        COLOR_MUTED_FOREGROUND
    };

    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(
            rect,
            RADIUS_SM,
            Color32::from_rgba_unmultiplied(255, 255, 255, 10),
        );
    }

    let font_id = egui::FontId::proportional(FONT_SIZE_CAPTION);
    let title_upper = title.to_uppercase();
    let galley = p.layout_no_wrap(title_upper, font_id, text_color);

    let arrow_width = if is_active { 12.0 } else { 0.0 };
    let total_content_w = galley.size().x + arrow_width;

    let start_x = match align {
        egui::Align::Min => rect.left() + 4.0,
        egui::Align::Center => rect.center().x - (total_content_w / 2.0),
        egui::Align::Max => rect.right() - 4.0 - total_content_w,
    };

    let text_y = rect.center().y - (galley.size().y / 2.0);
    p.galley(Pos2::new(start_x, text_y), galley, text_color);

    if is_active {
        let arrow_center = Pos2::new(start_x + total_content_w - 5.0, rect.center().y);
        let ascending = active_dir == SortDirection::Ascending;
        let arrow_color = COLOR_INFO;
        let h = 3.5;
        let w = 3.5;

        if ascending {
            let top = Pos2::new(arrow_center.x, arrow_center.y - h);
            let left = Pos2::new(arrow_center.x - w, arrow_center.y + h);
            let right = Pos2::new(arrow_center.x + w, arrow_center.y + h);
            p.add(egui::Shape::convex_polygon(
                vec![top, right, left],
                arrow_color,
                Stroke::NONE,
            ));
        } else {
            let bottom = Pos2::new(arrow_center.x, arrow_center.y + h);
            let left = Pos2::new(arrow_center.x - w, arrow_center.y - h);
            let right = Pos2::new(arrow_center.x + w, arrow_center.y - h);
            p.add(egui::Shape::convex_polygon(
                vec![bottom, left, right],
                arrow_color,
                Stroke::NONE,
            ));
        }
    }

    resp.clicked()
}

/// Renders a single data row with zebra background, hover highlight, selected accent bar,
/// and optional monitored episode indicator.
pub fn render_process_row(
    ui: &mut egui::Ui,
    proc: &ScoredProcess,
    layout: &TableLayout,
    is_selected: bool,
    is_watched: bool,
    group_count: Option<usize>,
    row_idx: usize,
) -> Response {
    let row_height = 28.0;
    let total_width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(total_width, row_height), Sense::click());

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let row_clip = ui.clip_rect().intersect(rect);
    let p = ui.painter().with_clip_rect(row_clip);

    // 1. Background styling: selection > hover > culprit/suspicious level highlight > group member > zebra
    let bg_color = if is_selected {
        COLOR_SELECTED_ROW
    } else if resp.hovered() {
        COLOR_MUTED
    } else if proc.severity == Severity::Culprit {
        COLOR_DESTRUCTIVE.gamma_multiply(0.12)
    } else if proc.severity == Severity::Suspicious {
        COLOR_WARNING.gamma_multiply(0.10)
    } else if group_count.is_some_and(|c| c >= 2) {
        COLOR_INFO.gamma_multiply(0.08)
    } else if row_idx % 2 == 1 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 4)
    } else {
        Color32::TRANSPARENT
    };

    if bg_color != Color32::TRANSPARENT {
        p.rect_filled(rect, RADIUS_SM, bg_color);
    }

    // 2. Left accent bar for selected row, colored by level
    if is_selected {
        let accent_color = match proc.severity {
            Severity::Culprit => COLOR_DESTRUCTIVE,
            Severity::Suspicious => COLOR_WARNING,
            Severity::Normal => COLOR_INFO,
        };
        let accent_rect = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
        p.rect_filled(accent_rect, 1.5, accent_color);
    }

    // 3. Subtle bottom row divider
    p.line_segment(
        [
            Pos2::new(rect.left(), rect.bottom()),
            Pos2::new(rect.right(), rect.bottom()),
        ],
        Stroke::new(1.0, COLOR_BORDER_SUBTLE),
    );

    // 4. Render cell columns inside row rect
    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child_ui.set_clip_rect(row_clip);
    child_ui.spacing_mut().item_spacing.x = layout.spacing;

    // Column 0: Name (with lock icon if SystemCritical, tag if watched)
    render_cell(
        &mut child_ui,
        layout.name_width,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            if !proc.killable {
                widgets::draw_lock_icon(ui, COLOR_PROTECTED);
            }
            if is_watched {
                widgets::watched_tag(ui, proc.severity);
            }
            if let Some(count) = group_count {
                if count >= 2 {
                    widgets::group_tag(ui, count);
                }
            }

            let label = egui::Label::new(
                RichText::new(&proc.record.name)
                    .color(COLOR_FOREGROUND)
                    .size(FONT_SIZE_BODY),
            )
            .truncate();

            let label_resp = ui.add(label);

            // Rich hover tooltip with full path if available
            let tip = match &proc.record.exe_path {
                Some(path) => format!(
                    "{}\n{} {}\n(PID: {})",
                    proc.record.name,
                    strings::TOOLTIP_EXE_PATH,
                    path,
                    proc.record.pid
                ),
                None => format!("{} (PID: {})", proc.record.name, proc.record.pid),
            };
            label_resp.on_hover_text(tip);
        },
    );

    // Column 1: PID
    render_cell(
        &mut child_ui,
        layout.pid_width,
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.label(
                RichText::new(proc.record.pid.to_string())
                    .color(COLOR_MUTED_FOREGROUND)
                    .monospace()
                    .size(FONT_SIZE_CAPTION),
            );
        },
    );

    // Column 2: CPU %
    render_cell(
        &mut child_ui,
        layout.cpu_width,
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.label(
                RichText::new(format_percent(proc.record.cpu_percent))
                    .color(COLOR_FOREGROUND)
                    .monospace()
                    .size(FONT_SIZE_BODY),
            );
        },
    );

    // Column 3: RAM
    render_cell(
        &mut child_ui,
        layout.ram_width,
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.label(
                RichText::new(format_bytes(proc.record.memory_bytes))
                    .color(COLOR_FOREGROUND)
                    .monospace()
                    .size(FONT_SIZE_BODY),
            );
        },
    );

    // Column 4: Disk I/O
    let total_disk = proc.record.disk_read_bps + proc.record.disk_write_bps;
    render_cell(
        &mut child_ui,
        layout.disk_width,
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.label(
                RichText::new(format_bps(total_disk))
                    .color(COLOR_FOREGROUND)
                    .monospace()
                    .size(FONT_SIZE_BODY),
            );
        },
    );

    // Column 5: Score Bar
    render_cell(
        &mut child_ui,
        layout.score_width,
        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        |ui| {
            widgets::score_bar(ui, proc.score);
        },
    );

    // Column 6: Kind Badge
    render_cell(
        &mut child_ui,
        layout.kind_width,
        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        |ui| {
            widgets::process_kind_badge(ui, proc.kind);
        },
    );

    resp
}

/// Renders a friendly empty state message when search yields no matches.
pub fn render_empty_table_state(ui: &mut Ui) {
    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(strings::TABLE_EMPTY)
                .color(COLOR_MUTED_FOREGROUND)
                .size(FONT_SIZE_BODY),
        );
    });
    ui.add_space(16.0);
}
