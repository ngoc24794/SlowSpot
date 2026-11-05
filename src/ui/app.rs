//! Main SlowSpot application dashboard.
//!
//! Implements `eframe::App` with a reactive immediate-mode UI receiving
//! metrics snapshots from the background collector worker.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Margin, RichText, Sense, Stroke, Vec2};

use crate::model::{ActionCommand, PressureState, ScoredProcess, SystemSnapshot};
use crate::ui::actions::ActionRunner;
use crate::ui::state::{
    format_bottleneck_explanation, format_bps, format_bytes, format_percent, SortColumn, ViewState,
};
use crate::ui::strings;
use crate::ui::theme::{
    self, COLOR_BG_CARD, COLOR_BG_CARD_CULPRIT, COLOR_BG_CARD_SMOOTH, COLOR_CULPRIT, COLOR_NORMAL,
    COLOR_SUSPICIOUS, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};
use crate::ui::widgets;

/// Main SlowSpot desktop application.
pub struct SlowSpotApp {
    rx: Receiver<SystemSnapshot>,
    actions: Box<dyn ActionRunner>,
    state: ViewState,
    demo: bool,
    selected_pid: Option<u32>,
    confirm_kill: Option<ScoredProcess>,
}

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
fn render_cell<R>(
    ui: &mut egui::Ui,
    width: f32,
    layout: egui::Layout,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(Vec2::new(width, 20.0), layout, |ui| {
        ui.set_min_size(Vec2::new(width, 20.0));
        ui.set_max_size(Vec2::new(width, 20.0));
        ui.set_clip_rect(ui.max_rect());
        add_contents(ui)
    })
}

enum HeaderAlign {
    Left,
    Center,
    Right,
}

fn col_header_button(
    ui: &mut egui::Ui,
    label: &str,
    col: SortColumn,
    width: f32,
    align: HeaderAlign,
    current_col: SortColumn,
    current_dir: crate::ui::state::SortDirection,
) -> bool {
    let arrow = if current_col == col {
        match current_dir {
            crate::ui::state::SortDirection::Ascending => " ^",
            crate::ui::state::SortDirection::Descending => " v",
        }
    } else {
        ""
    };
    let text = format!("{label}{arrow}");
    let color = if current_col == col {
        COLOR_TEXT_PRIMARY
    } else {
        COLOR_TEXT_MUTED
    };
    let rich = RichText::new(text).color(color).size(12.0).strong();
    let btn = match align {
        HeaderAlign::Left => egui::Button::new("").left_text(rich).truncate(),
        HeaderAlign::Right => egui::Button::new("").right_text(rich).truncate(),
        HeaderAlign::Center => egui::Button::new(rich).truncate(),
    };
    ui.add_sized([width, 20.0], btn).clicked()
}

impl SlowSpotApp {
    /// Creates a new SlowSpot application instance.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        rx: Receiver<SystemSnapshot>,
        actions: Box<dyn ActionRunner>,
    ) -> Self {
        theme::install(&cc.egui_ctx);

        Self {
            rx,
            actions,
            state: ViewState::new(),
            demo: false,
            selected_pid: None,
            confirm_kill: None,
        }
    }

    /// Sets whether the application runs in simulated demo mode.
    pub fn with_demo(mut self, demo: bool) -> Self {
        self.demo = demo;
        self
    }
}

impl eframe::App for SlowSpotApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();

        // 1. Drain incoming snapshots from the collector thread
        while let Ok(snapshot) = self.rx.try_recv() {
            self.state.push_snapshot(snapshot, now);
        }

        // 2. Request a reactive repaint fallback (1 second cadence, avoids 60 FPS busy loop)
        ui.ctx().request_repaint_after(Duration::from_millis(1000));

        // 3. Render Dashboard Layout
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_space(4.0);

            self.render_header_banner(ui, now);
            ui.add_space(10.0);

            self.render_top_culprit_card(ui, now);
            ui.add_space(10.0);

            self.render_sparklines(ui);
            ui.add_space(10.0);

            self.render_process_table(ui, now);
            ui.add_space(10.0);

            self.render_toast_and_status(ui, now);
        });

        // 4. Modal confirmation dialog for forced kill
        self.render_confirm_kill_dialog(ui, now);
    }
}

impl SlowSpotApp {
    fn render_header_banner(&self, ui: &mut egui::Ui, now: Instant) {
        let card_frame = egui::Frame::new()
            .fill(COLOR_BG_CARD)
            .corner_radius(8.0)
            .inner_margin(Margin::same(12));

        card_frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // Title and Demo badge
                ui.heading(RichText::new("SlowSpot").color(COLOR_TEXT_PRIMARY).strong());
                if self.demo {
                    let demo_badge = egui::Frame::new()
                        .fill(Color32::from_rgb(180, 80, 30))
                        .corner_radius(4.0)
                        .inner_margin(Margin::symmetric(6, 2));
                    demo_badge.show(ui, |ui| {
                        ui.label(
                            RichText::new(strings::BADGE_DEMO)
                                .color(Color32::WHITE)
                                .size(11.0)
                                .strong(),
                        );
                    });
                }

                // System Pressure State Pill
                let (status_text, status_color) =
                    match self.state.latest_snapshot.as_ref().map(|s| s.pressure) {
                        Some(PressureState::Smooth) => (strings::PRESSURE_SMOOTH, COLOR_NORMAL),
                        Some(PressureState::Strained) => {
                            (strings::PRESSURE_STRAINED, COLOR_SUSPICIOUS)
                        }
                        Some(PressureState::Overloaded) => {
                            (strings::PRESSURE_OVERLOADED, COLOR_CULPRIT)
                        }
                        None => (strings::PRESSURE_WAITING, COLOR_TEXT_MUTED),
                    };

                let pill_frame = egui::Frame::new()
                    .fill(status_color.gamma_multiply(0.2))
                    .corner_radius(12.0)
                    .inner_margin(Margin::symmetric(10, 3))
                    .stroke(Stroke::new(1.0, status_color));

                pill_frame.show(ui, |ui| {
                    ui.label(
                        RichText::new(status_text)
                            .color(status_color)
                            .size(12.0)
                            .strong(),
                    );
                });

                // Cooldown notice if PC recovered from stress
                if self.state.is_cooldown_active(now) {
                    ui.label(
                        RichText::new(strings::NOTICE_COOLDOWN)
                            .color(COLOR_NORMAL)
                            .size(12.0)
                            .strong(),
                    );
                }

                // Push totals to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(snap) = &self.state.latest_snapshot {
                        let mem_pct = if snap.totals.memory_total_bytes > 0 {
                            (snap.totals.memory_used_bytes as f32
                                / snap.totals.memory_total_bytes as f32)
                                * 100.0
                        } else {
                            0.0
                        };

                        let text = format!(
                            "CPU: {}  |  RAM: {}/{} ({:.0}%)  |  Đĩa: {}",
                            format_percent(snap.totals.cpu_percent),
                            format_bytes(snap.totals.memory_used_bytes),
                            format_bytes(snap.totals.memory_total_bytes),
                            mem_pct,
                            format_bps(snap.totals.disk_bps),
                        );
                        ui.label(RichText::new(text).color(COLOR_TEXT_MUTED).size(12.0));
                    }
                });
            });
        });
    }

    fn render_top_culprit_card(&mut self, ui: &mut egui::Ui, now: Instant) {
        let culprit = self.state.top_culprit().cloned();

        let (fill_color, border_color) = if culprit.is_some() {
            (COLOR_BG_CARD_CULPRIT, COLOR_CULPRIT)
        } else {
            (COLOR_BG_CARD_SMOOTH, COLOR_NORMAL)
        };

        let card_frame = egui::Frame::new()
            .fill(fill_color)
            .corner_radius(8.0)
            .inner_margin(Margin::same(14))
            .stroke(Stroke::new(1.2, border_color));

        card_frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(target) = culprit {
                // Header row of hero card
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(strings::HERO_TITLE)
                            .color(COLOR_CULPRIT)
                            .size(12.0)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    widgets::severity_badge(ui, target.severity);
                    ui.add_space(4.0);
                    if !target.killable {
                        ui.label(
                            RichText::new(strings::HERO_PROTECTED_BADGE)
                                .color(Color32::from_rgb(220, 140, 200))
                                .size(11.0),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{}: {:.0}/100",
                                strings::HERO_SCORE_LABEL,
                                target.score
                            ))
                            .color(COLOR_CULPRIT)
                            .size(14.0)
                            .strong(),
                        );
                    });
                });

                ui.add_space(6.0);

                // Culprit process details
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&target.record.name)
                            .color(COLOR_TEXT_PRIMARY)
                            .size(18.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!("(PID {})", target.record.pid))
                            .color(COLOR_TEXT_MUTED)
                            .size(13.0),
                    );
                    ui.add_space(10.0);
                    let explanation = format_bottleneck_explanation(&target);
                    ui.label(
                        RichText::new(format!("— {explanation}"))
                            .color(COLOR_TEXT_PRIMARY)
                            .size(14.0),
                    );
                });

                ui.add_space(10.0);

                // Action buttons row
                ui.horizontal(|ui| {
                    let pid = target.record.pid;
                    let killable = target.killable;

                    // 1. Graceful close button
                    let close_btn = egui::Button::new(strings::BTN_GRACEFUL_CLOSE);
                    if ui.add_enabled(killable, close_btn).clicked() {
                        let res = self.actions.run(ActionCommand::GracefulClose(pid), &target);
                        self.state.record_graceful_close(pid, now);
                        match res {
                            Ok(msg) => {
                                self.state
                                    .add_toast(msg, false, now, Duration::from_secs(4))
                            }
                            Err(err) => {
                                self.state.add_toast(err, true, now, Duration::from_secs(5))
                            }
                        }
                    }

                    // 2. Force kill button (available after 5s graceful wait)
                    let can_kill = self.state.can_force_kill(pid, now);
                    let wait_rem = self.state.graceful_wait_remaining_secs(pid, now);

                    let kill_text = match wait_rem {
                        Some(s) if s > 0 => format!("{} ({}s)", strings::BTN_FORCE_KILL, s),
                        _ => strings::BTN_FORCE_KILL.to_string(),
                    };

                    let kill_btn = egui::Button::new(RichText::new(kill_text).color(if can_kill {
                        COLOR_CULPRIT
                    } else {
                        COLOR_TEXT_MUTED
                    }));
                    if ui.add_enabled(killable && can_kill, kill_btn).clicked() {
                        self.confirm_kill = Some(target.clone());
                    }

                    // 3. Open file location
                    if ui.button(strings::BTN_OPEN_LOCATION).clicked() {
                        let res = self.actions.run(ActionCommand::OpenLocation(pid), &target);
                        match res {
                            Ok(msg) => {
                                self.state
                                    .add_toast(msg, false, now, Duration::from_secs(4))
                            }
                            Err(err) => {
                                self.state.add_toast(err, true, now, Duration::from_secs(5))
                            }
                        }
                    }

                    // 4. Ignore culprit
                    if ui.button(strings::BTN_IGNORE_CULPRIT).clicked() {
                        let name = target.record.name.clone();
                        self.state.ignore_process(&name);
                        self.state.add_toast(
                            format!("{} '{}'", strings::MSG_IGNORED_SUCCESS, name),
                            false,
                            now,
                            Duration::from_secs(4),
                        );
                    }
                });
            } else {
                // No culprit banner with crisp painter checkmark
                ui.horizontal(|ui| {
                    widgets::draw_checkmark(ui, COLOR_NORMAL, 18.0);
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(strings::HERO_NO_CULPRIT)
                            .color(COLOR_NORMAL)
                            .size(15.0)
                            .strong(),
                    );
                });
            }
        });
    }

    fn render_sparklines(&self, ui: &mut egui::Ui) {
        let cpu_values: Vec<f32> = self.state.history.iter().map(|p| p.cpu_percent).collect();
        let ram_values: Vec<f32> = self
            .state
            .history
            .iter()
            .map(|p| p.memory_percent)
            .collect();
        let disk_values: Vec<f32> = self
            .state
            .history
            .iter()
            .map(|p| (p.disk_bps / (1024.0 * 1024.0)) as f32)
            .collect();

        let cur_cpu = cpu_values.last().copied().unwrap_or(0.0);
        let cur_ram = ram_values.last().copied().unwrap_or(0.0);
        let cur_disk = disk_values.last().copied().unwrap_or(0.0);

        let max_disk = disk_values.iter().fold(10.0f32, |m, &v| m.max(v * 1.2));

        // Distribute available width evenly minus egui's default item spacing
        let available_width = ui.available_width();
        let spacing = ui.spacing().item_spacing.x;
        let total_spacing = spacing * 2.0;
        let card_width = ((available_width - total_spacing) / 3.0).floor().max(1.0);
        let last_card_width = (available_width - card_width * 2.0 - total_spacing).max(1.0);

        ui.horizontal(|ui| {
            widgets::sparkline(
                ui,
                widgets::SparklineConfig {
                    title: strings::SPARKLINE_CPU_TITLE,
                    current_label: &format_percent(cur_cpu),
                    min_val: 0.0,
                    max_val: 100.0,
                    line_color: COLOR_CULPRIT,
                    size: Vec2::new(card_width, 80.0),
                },
                &cpu_values,
            );
            widgets::sparkline(
                ui,
                widgets::SparklineConfig {
                    title: strings::SPARKLINE_RAM_TITLE,
                    current_label: &format_percent(cur_ram),
                    min_val: 0.0,
                    max_val: 100.0,
                    line_color: COLOR_SUSPICIOUS,
                    size: Vec2::new(card_width, 80.0),
                },
                &ram_values,
            );
            widgets::sparkline(
                ui,
                widgets::SparklineConfig {
                    title: strings::SPARKLINE_DISK_TITLE,
                    current_label: &format!("{:.1} MB/s", cur_disk),
                    min_val: 0.0,
                    max_val: max_disk,
                    line_color: COLOR_NORMAL,
                    size: Vec2::new(last_card_width, 80.0),
                },
                &disk_values,
            );
        });
    }

    fn render_process_table(&mut self, ui: &mut egui::Ui, now: Instant) {
        let card_frame = egui::Frame::new()
            .fill(COLOR_BG_CARD)
            .corner_radius(8.0)
            .inner_margin(Margin::same(12));

        card_frame.show(ui, |ui| {
            ui.set_width(ui.available_width());

            // Header row with Title and flex Search box
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(strings::TABLE_TITLE)
                        .color(COLOR_TEXT_PRIMARY)
                        .size(14.0)
                        .strong(),
                );
                let remaining_width = ui.available_width();
                let search_width = remaining_width.max(100.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.state.search_query)
                            .hint_text(strings::SEARCH_PLACEHOLDER)
                            .desired_width(search_width),
                    );
                });
            });

            ui.add_space(8.0);

            // Table Header with Sort Triggers
            let card_available = ui.available_width();
            let spacing = ui.spacing().item_spacing.x;
            let layout = TableLayout::compute(card_available, spacing);

            let header_frame = egui::Frame::new().inner_margin(Margin::symmetric(4, 0));
            header_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let sort_col = self.state.sort_column;
                    let sort_dir = self.state.sort_direction;

                    // Column 0: Name (Left aligned)
                    render_cell(
                        ui,
                        layout.name_width,
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            if col_header_button(
                                ui,
                                strings::COL_NAME,
                                SortColumn::Name,
                                layout.name_width,
                                HeaderAlign::Left,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Name);
                            }
                        },
                    );

                    // Column 1: PID (Right aligned)
                    render_cell(
                        ui,
                        layout.pid_width,
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if col_header_button(
                                ui,
                                strings::COL_PID,
                                SortColumn::Pid,
                                layout.pid_width,
                                HeaderAlign::Right,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Pid);
                            }
                        },
                    );

                    // Column 2: CPU % (Right aligned)
                    render_cell(
                        ui,
                        layout.cpu_width,
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if col_header_button(
                                ui,
                                strings::COL_CPU,
                                SortColumn::Cpu,
                                layout.cpu_width,
                                HeaderAlign::Right,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Cpu);
                            }
                        },
                    );

                    // Column 3: RAM (Right aligned)
                    render_cell(
                        ui,
                        layout.ram_width,
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if col_header_button(
                                ui,
                                strings::COL_RAM,
                                SortColumn::Memory,
                                layout.ram_width,
                                HeaderAlign::Right,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Memory);
                            }
                        },
                    );

                    // Column 4: Disk I/O (Right aligned)
                    render_cell(
                        ui,
                        layout.disk_width,
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if col_header_button(
                                ui,
                                strings::COL_DISK,
                                SortColumn::Disk,
                                layout.disk_width,
                                HeaderAlign::Right,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Disk);
                            }
                        },
                    );

                    // Column 5: Score (Centered)
                    render_cell(
                        ui,
                        layout.score_width,
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            if col_header_button(
                                ui,
                                strings::COL_SCORE,
                                SortColumn::Score,
                                layout.score_width,
                                HeaderAlign::Center,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Score);
                            }
                        },
                    );

                    // Column 6: Kind (Centered)
                    render_cell(
                        ui,
                        layout.kind_width,
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            if col_header_button(
                                ui,
                                strings::COL_KIND,
                                SortColumn::Kind,
                                layout.kind_width,
                                HeaderAlign::Center,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Kind);
                            }
                        },
                    );
                });
            });

            ui.separator();

            // Table Rows (Top 20 filtered)
            let filtered = self.state.filtered_processes();
            let top_20: Vec<&ScoredProcess> = filtered.into_iter().take(20).collect();

            if top_20.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(strings::TABLE_EMPTY)
                        .color(COLOR_TEXT_MUTED)
                        .size(12.0),
                );
                ui.add_space(8.0);
            } else {
                for proc in &top_20 {
                    let pid = proc.record.pid;
                    let is_selected = self.selected_pid == Some(pid);

                    let row_bg = if is_selected {
                        Color32::from_rgb(38, 45, 60)
                    } else {
                        Color32::TRANSPARENT
                    };

                    let row_frame = egui::Frame::new()
                        .fill(row_bg)
                        .corner_radius(4.0)
                        .inner_margin(Margin::symmetric(4, 3));

                    let response = row_frame
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Column 0: Name + lock icon if system critical (truncated with ...)
                                render_cell(
                                    ui,
                                    layout.name_width,
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        if !proc.killable {
                                            widgets::draw_lock_icon(
                                                ui,
                                                Color32::from_rgb(220, 140, 200),
                                            );
                                        }
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&proc.record.name)
                                                    .color(COLOR_TEXT_PRIMARY)
                                                    .size(12.0),
                                            )
                                            .truncate(),
                                        )
                                        .on_hover_text(&proc.record.name);
                                    },
                                );

                                // Column 1: PID (Right-aligned)
                                render_cell(
                                    ui,
                                    layout.pid_width,
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(pid.to_string())
                                                .color(COLOR_TEXT_MUTED)
                                                .size(12.0),
                                        );
                                    },
                                );

                                // Column 2: CPU % (Right-aligned)
                                render_cell(
                                    ui,
                                    layout.cpu_width,
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format_percent(proc.record.cpu_percent))
                                                .color(COLOR_TEXT_PRIMARY)
                                                .size(12.0),
                                        );
                                    },
                                );

                                // Column 3: RAM (Right-aligned)
                                render_cell(
                                    ui,
                                    layout.ram_width,
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format_bytes(proc.record.memory_bytes))
                                                .color(COLOR_TEXT_PRIMARY)
                                                .size(12.0),
                                        );
                                    },
                                );

                                // Column 4: Disk I/O (Right-aligned)
                                let total_disk =
                                    proc.record.disk_read_bps + proc.record.disk_write_bps;
                                render_cell(
                                    ui,
                                    layout.disk_width,
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format_bps(total_disk))
                                                .color(COLOR_TEXT_PRIMARY)
                                                .size(12.0),
                                        );
                                    },
                                );

                                // Column 5: Score bar (Fixed width, centered)
                                render_cell(
                                    ui,
                                    layout.score_width,
                                    egui::Layout::centered_and_justified(
                                        egui::Direction::LeftToRight,
                                    ),
                                    |ui| {
                                        widgets::score_bar(ui, proc.score);
                                    },
                                );

                                // Column 6: Kind badge (Fixed width, centered)
                                render_cell(
                                    ui,
                                    layout.kind_width,
                                    egui::Layout::centered_and_justified(
                                        egui::Direction::LeftToRight,
                                    ),
                                    |ui| {
                                        widgets::process_kind_badge(ui, proc.kind);
                                    },
                                );
                            });
                        })
                        .response;

                    let row_sense =
                        ui.interact(response.rect, ui.scope_id().with(pid), Sense::click());
                    if row_sense.clicked() {
                        self.selected_pid = if is_selected { None } else { Some(pid) };
                    }
                }
            }

            ui.separator();

            // Selected process action bar
            self.render_selected_action_bar(ui, now);
        });
    }

    fn render_selected_action_bar(&mut self, ui: &mut egui::Ui, now: Instant) {
        let selected_proc = self.selected_pid.and_then(|pid| {
            self.state
                .latest_snapshot
                .as_ref()?
                .processes
                .iter()
                .find(|p| p.record.pid == pid)
                .cloned()
        });

        ui.horizontal(|ui| {
            if let Some(target) = selected_proc {
                let pid = target.record.pid;
                let killable = target.killable;

                ui.label(
                    RichText::new(strings::SELECTED_PROCESS_LABEL)
                        .color(COLOR_TEXT_MUTED)
                        .size(12.0),
                );
                ui.label(
                    RichText::new(format!("{} (PID {})", target.record.name, pid))
                        .color(COLOR_TEXT_PRIMARY)
                        .size(12.0)
                        .strong(),
                );
                if !killable {
                    ui.label(
                        RichText::new(strings::HERO_PROTECTED_BADGE)
                            .color(Color32::from_rgb(220, 140, 200))
                            .size(11.0),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Ignore button
                    if ui.button(strings::BTN_IGNORE_CULPRIT).clicked() {
                        let name = target.record.name.clone();
                        self.state.ignore_process(&name);
                        self.state.add_toast(
                            format!("{} '{}'", strings::MSG_IGNORED_SUCCESS, name),
                            false,
                            now,
                            Duration::from_secs(4),
                        );
                    }

                    // Open location button
                    if ui.button(strings::BTN_OPEN_LOCATION).clicked() {
                        let res = self.actions.run(ActionCommand::OpenLocation(pid), &target);
                        match res {
                            Ok(msg) => {
                                self.state
                                    .add_toast(msg, false, now, Duration::from_secs(4))
                            }
                            Err(err) => {
                                self.state.add_toast(err, true, now, Duration::from_secs(5))
                            }
                        }
                    }

                    // Force kill button
                    let can_kill = self.state.can_force_kill(pid, now);
                    let wait_rem = self.state.graceful_wait_remaining_secs(pid, now);
                    let kill_text = match wait_rem {
                        Some(s) if s > 0 => format!("{} ({}s)", strings::BTN_FORCE_KILL, s),
                        _ => strings::BTN_FORCE_KILL.to_string(),
                    };

                    let kill_btn = egui::Button::new(RichText::new(kill_text).color(if can_kill {
                        COLOR_CULPRIT
                    } else {
                        COLOR_TEXT_MUTED
                    }));
                    if ui.add_enabled(killable && can_kill, kill_btn).clicked() {
                        self.confirm_kill = Some(target.clone());
                    }

                    // Graceful close button
                    let close_btn = egui::Button::new(strings::BTN_GRACEFUL_CLOSE);
                    if ui.add_enabled(killable, close_btn).clicked() {
                        let res = self.actions.run(ActionCommand::GracefulClose(pid), &target);
                        self.state.record_graceful_close(pid, now);
                        match res {
                            Ok(msg) => {
                                self.state
                                    .add_toast(msg, false, now, Duration::from_secs(4))
                            }
                            Err(err) => {
                                self.state.add_toast(err, true, now, Duration::from_secs(5))
                            }
                        }
                    }
                });
            } else {
                ui.label(
                    RichText::new(strings::SELECTED_NONE)
                        .color(COLOR_TEXT_MUTED)
                        .size(12.0),
                );
            }
        });
    }

    fn render_toast_and_status(&self, ui: &mut egui::Ui, now: Instant) {
        if let Some(toast) = self.state.active_toast(now) {
            let (bg, border) = if toast.is_error {
                (Color32::from_rgb(55, 20, 20), COLOR_CULPRIT)
            } else {
                (Color32::from_rgb(20, 45, 30), COLOR_NORMAL)
            };

            let toast_frame = egui::Frame::new()
                .fill(bg)
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(12, 8))
                .stroke(Stroke::new(1.0, border));

            toast_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    if toast.is_error {
                        widgets::draw_warning_icon(ui, border, 14.0);
                    } else {
                        widgets::draw_checkmark(ui, border, 14.0);
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(&toast.message)
                            .color(COLOR_TEXT_PRIMARY)
                            .size(13.0),
                    );
                });
            });
        }
    }

    fn render_confirm_kill_dialog(&mut self, ui: &mut egui::Ui, now: Instant) {
        if let Some(target) = self.confirm_kill.clone() {
            let mut open = true;
            egui::Window::new(strings::CONFIRM_TITLE)
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.set_width(380.0);
                    ui.label(
                        RichText::new(format!(
                            "Bạn có chắc chắn muốn buộc tắt tiến trình '{}' (PID {})?\n\nHành động này sẽ ngắt tiến trình ngay lập tức và có thể làm mất dữ liệu chưa lưu.",
                            target.record.name, target.record.pid
                        ))
                        .color(COLOR_TEXT_PRIMARY)
                        .size(13.0),
                    );

                    ui.add_space(14.0);

                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(strings::CONFIRM_CANCEL).clicked() {
                                self.confirm_kill = None;
                            }

                            let kill_btn = egui::Button::new(
                                RichText::new(strings::CONFIRM_PROCEED)
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(COLOR_CULPRIT);

                            if ui.add(kill_btn).clicked() {
                                let res = self.actions.run(ActionCommand::ForceKill(target.record.pid), &target);
                                match res {
                                    Ok(msg) => self.state.add_toast(msg, false, now, Duration::from_secs(4)),
                                    Err(err) => self.state.add_toast(err, true, now, Duration::from_secs(5)),
                                }
                                self.confirm_kill = None;
                            }
                        });
                    });
                });

            if !open {
                self.confirm_kill = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets;
    use eframe::egui::{self, Color32};
    /// Pure computation layout verification across supported window dimensions.
    #[test]
    fn test_sparkline_layout_distribution_no_overflow() {
        let test_widths = [736.0, 976.0, 1200.0, 1896.0];
        let spacing = 8.0f32;
        let total_spacing = spacing * 2.0;

        for &available_width in &test_widths {
            let card_width = ((available_width - total_spacing) / 3.0).floor().max(1.0);
            let last_card_width = (available_width - card_width * 2.0 - total_spacing).max(1.0);

            let total_consumed = card_width * 2.0 + last_card_width + total_spacing;
            assert!(
                (total_consumed - available_width).abs() < 0.001,
                "Sparklines width {total_consumed} must match available width {available_width}"
            );
            assert!(
                total_consumed <= available_width,
                "Sparklines must not exceed available width"
            );
            assert!(card_width >= 180.0, "Card width should remain readable");
        }
    }

    #[test]
    fn test_process_table_columns_fill_exact_width_without_overflow() {
        let test_card_widths = [712.0f32, 952.0, 1176.0, 1872.0];
        let spacing = 8.0f32;

        for &card_available in &test_card_widths {
            let layout = TableLayout::compute(card_available, spacing);
            let other_cols_width = layout.pid_width
                + layout.cpu_width
                + layout.ram_width
                + layout.disk_width
                + layout.score_width
                + layout.kind_width;
            let total_col_spacing = 6.0 * spacing;
            let inner_row_width = (card_available - 8.0).max(other_cols_width + total_col_spacing);

            let total_row_consumed = layout.name_width + other_cols_width + total_col_spacing;
            assert!(
                (total_row_consumed - inner_row_width).abs() < 0.001,
                "Table columns width {total_row_consumed} must match inner row width {inner_row_width}"
            );
            assert!(
                total_row_consumed + 8.0 <= card_available,
                "Row must not exceed card available width"
            );
            assert!(
                layout.name_width >= 80.0,
                "Name column must maintain minimum width"
            );
        }
    }

    #[test]
    fn test_table_cell_exact_width_and_alignment() {
        let ctx = egui::Context::default();
        let card_available = 712.0;
        let spacing = 8.0;
        let layout = TableLayout::compute(card_available, spacing);

        let mut output = ctx.run_ui(Default::default(), |ui| {
            // 1. Render Header Row
            let mut header_x_coords = Vec::new();
            ui.horizontal(|ui| {
                let r0 = render_cell(
                    ui,
                    layout.name_width,
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label("Name");
                    },
                );
                header_x_coords.push(r0.response.rect.min.x);

                let r1 = render_cell(
                    ui,
                    layout.pid_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("PID");
                    },
                );
                header_x_coords.push(r1.response.rect.min.x);

                let r2 = render_cell(
                    ui,
                    layout.cpu_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("CPU");
                    },
                );
                header_x_coords.push(r2.response.rect.min.x);

                let r3 = render_cell(
                    ui,
                    layout.ram_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("RAM");
                    },
                );
                header_x_coords.push(r3.response.rect.min.x);

                let r4 = render_cell(
                    ui,
                    layout.disk_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("Disk");
                    },
                );
                header_x_coords.push(r4.response.rect.min.x);

                let r5 = render_cell(
                    ui,
                    layout.score_width,
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label("Score");
                    },
                );
                header_x_coords.push(r5.response.rect.min.x);

                let r6 = render_cell(
                    ui,
                    layout.kind_width,
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label("Kind");
                    },
                );
                header_x_coords.push(r6.response.rect.min.x);
            });

            // 2. Render Data Row with SystemCritical lock + long truncated name
            let mut row_x_coords = Vec::new();
            ui.horizontal(|ui| {
                let r0 = render_cell(
                    ui,
                    layout.name_width,
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        widgets::draw_lock_icon(ui, Color32::RED);
                        ui.add(
                            egui::Label::new(
                                "VeryLongProcessNameThatExceedsTheColumnWidthBoundaries.exe",
                            )
                            .truncate(),
                        );
                    },
                );
                row_x_coords.push(r0.response.rect.min.x);

                let r1 = render_cell(
                    ui,
                    layout.pid_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("4");
                    },
                );
                row_x_coords.push(r1.response.rect.min.x);

                let r2 = render_cell(
                    ui,
                    layout.cpu_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("99.9%");
                    },
                );
                row_x_coords.push(r2.response.rect.min.x);

                let r3 = render_cell(
                    ui,
                    layout.ram_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("1024.0 MB");
                    },
                );
                row_x_coords.push(r3.response.rect.min.x);

                let r4 = render_cell(
                    ui,
                    layout.disk_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("12.5 MB/s");
                    },
                );
                row_x_coords.push(r4.response.rect.min.x);

                let r5 = render_cell(
                    ui,
                    layout.score_width,
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        widgets::score_bar(ui, 90.0);
                    },
                );
                row_x_coords.push(r5.response.rect.min.x);

                let r6 = render_cell(
                    ui,
                    layout.kind_width,
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        widgets::process_kind_badge(ui, crate::model::ProcessKind::SystemCritical);
                    },
                );
                row_x_coords.push(r6.response.rect.min.x);
            });

            // 3. Render Another Row with short name
            let mut row2_x_coords = Vec::new();
            ui.horizontal(|ui| {
                let r0 = render_cell(
                    ui,
                    layout.name_width,
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add(egui::Label::new("cmd").truncate());
                    },
                );
                row2_x_coords.push(r0.response.rect.min.x);

                let r1 = render_cell(
                    ui,
                    layout.pid_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("12345");
                    },
                );
                row2_x_coords.push(r1.response.rect.min.x);

                let r2 = render_cell(
                    ui,
                    layout.cpu_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("0.0%");
                    },
                );
                row2_x_coords.push(r2.response.rect.min.x);

                let r3 = render_cell(
                    ui,
                    layout.ram_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("12.0 MB");
                    },
                );
                row2_x_coords.push(r3.response.rect.min.x);

                let r4 = render_cell(
                    ui,
                    layout.disk_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label("0 B/s");
                    },
                );
                row2_x_coords.push(r4.response.rect.min.x);

                let r5 = render_cell(
                    ui,
                    layout.score_width,
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        widgets::score_bar(ui, 10.0);
                    },
                );
                row2_x_coords.push(r5.response.rect.min.x);

                let r6 = render_cell(
                    ui,
                    layout.kind_width,
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        widgets::process_kind_badge(ui, crate::model::ProcessKind::UserApp);
                    },
                );
                row2_x_coords.push(r6.response.rect.min.x);
            });

            // Every column in row 1, row 2, and header must start at the exact same relative X offset
            assert_eq!(header_x_coords.len(), 7);
            assert_eq!(row_x_coords.len(), 7);
            assert_eq!(row2_x_coords.len(), 7);

            for i in 0..7 {
                let h_offset = header_x_coords[i] - header_x_coords[0];
                let r1_offset = row_x_coords[i] - row_x_coords[0];
                let r2_offset = row2_x_coords[i] - row2_x_coords[0];

                assert!(
                    (h_offset - r1_offset).abs() < 0.001,
                    "Column {i} offset in row 1 ({r1_offset}) must match header ({h_offset})"
                );
                assert!(
                    (h_offset - r2_offset).abs() < 0.001,
                    "Column {i} offset in row 2 ({r2_offset}) must match header ({h_offset})"
                );
            }
        });
        output.textures_delta.clear();
    }
}
