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
            let other_cols_width = 60.0 + 75.0 + 85.0 + 95.0 + 85.0 + 80.0; // 480.0
            let total_col_spacing = 6.0 * spacing;
            // Both header and rows use inner_margin(Margin::symmetric(4, ...)) so row inner width is card_available - 8.0
            let inner_row_width = (card_available - 8.0).max(other_cols_width + total_col_spacing);
            let name_col_width =
                (inner_row_width - other_cols_width - total_col_spacing).max(120.0);

            let header_frame = egui::Frame::new().inner_margin(Margin::symmetric(4, 0));
            header_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let sort_col = self.state.sort_column;
                    let sort_dir = self.state.sort_direction;

                    let arrow = |col: SortColumn| -> &str {
                        if sort_col == col {
                            match sort_dir {
                                crate::ui::state::SortDirection::Ascending => " ^",
                                crate::ui::state::SortDirection::Descending => " v",
                            }
                        } else {
                            ""
                        }
                    };

                    let col_btn =
                        |ui: &mut egui::Ui, label: &str, col: SortColumn, width: f32| -> bool {
                            let text = format!("{label}{}", arrow(col));
                            let color = if sort_col == col {
                                COLOR_TEXT_PRIMARY
                            } else {
                                COLOR_TEXT_MUTED
                            };
                            ui.add_sized(
                                [width, 20.0],
                                egui::Button::new(
                                    RichText::new(text).color(color).size(12.0).strong(),
                                ),
                            )
                            .clicked()
                        };

                    if col_btn(ui, strings::COL_NAME, SortColumn::Name, name_col_width) {
                        self.state.set_sort(SortColumn::Name);
                    }
                    if col_btn(ui, strings::COL_PID, SortColumn::Pid, 60.0) {
                        self.state.set_sort(SortColumn::Pid);
                    }
                    if col_btn(ui, strings::COL_CPU, SortColumn::Cpu, 75.0) {
                        self.state.set_sort(SortColumn::Cpu);
                    }
                    if col_btn(ui, strings::COL_RAM, SortColumn::Memory, 85.0) {
                        self.state.set_sort(SortColumn::Memory);
                    }
                    if col_btn(ui, strings::COL_DISK, SortColumn::Disk, 95.0) {
                        self.state.set_sort(SortColumn::Disk);
                    }
                    if col_btn(ui, strings::COL_SCORE, SortColumn::Score, 85.0) {
                        self.state.set_sort(SortColumn::Score);
                    }
                    if col_btn(ui, strings::COL_KIND, SortColumn::Kind, 80.0) {
                        self.state.set_sort(SortColumn::Kind);
                    }
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
                                // Name + lock icon if system critical
                                ui.allocate_ui_with_layout(
                                    Vec2::new(name_col_width, 18.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        if !proc.killable {
                                            widgets::draw_lock_icon(
                                                ui,
                                                Color32::from_rgb(220, 140, 200),
                                            );
                                        }
                                        ui.label(
                                            RichText::new(&proc.record.name)
                                                .color(COLOR_TEXT_PRIMARY)
                                                .size(12.0),
                                        );
                                    },
                                );

                                // PID
                                ui.add_sized(
                                    [60.0, 18.0],
                                    egui::Label::new(
                                        RichText::new(pid.to_string())
                                            .color(COLOR_TEXT_MUTED)
                                            .size(12.0),
                                    ),
                                );

                                // CPU %
                                ui.add_sized(
                                    [75.0, 18.0],
                                    egui::Label::new(
                                        RichText::new(format_percent(proc.record.cpu_percent))
                                            .color(COLOR_TEXT_PRIMARY)
                                            .size(12.0),
                                    ),
                                );

                                // RAM
                                ui.add_sized(
                                    [85.0, 18.0],
                                    egui::Label::new(
                                        RichText::new(format_bytes(proc.record.memory_bytes))
                                            .color(COLOR_TEXT_PRIMARY)
                                            .size(12.0),
                                    ),
                                );

                                // Disk I/O
                                let total_disk =
                                    proc.record.disk_read_bps + proc.record.disk_write_bps;
                                ui.add_sized(
                                    [95.0, 18.0],
                                    egui::Label::new(
                                        RichText::new(format_bps(total_disk))
                                            .color(COLOR_TEXT_PRIMARY)
                                            .size(12.0),
                                    ),
                                );

                                // Score bar
                                ui.allocate_ui_with_layout(
                                    Vec2::new(85.0, 18.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        widgets::score_bar(ui, proc.score);
                                    },
                                );

                                // Kind badge
                                ui.allocate_ui_with_layout(
                                    Vec2::new(80.0, 18.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
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
        let other_cols_width = 60.0 + 75.0 + 85.0 + 95.0 + 85.0 + 80.0; // 480.0
        let total_col_spacing = 6.0 * spacing; // 48.0

        for &card_available in &test_card_widths {
            let inner_row_width = (card_available - 8.0).max(other_cols_width + total_col_spacing);
            let name_col_width =
                (inner_row_width - other_cols_width - total_col_spacing).max(120.0);

            let total_row_consumed = name_col_width + other_cols_width + total_col_spacing;
            assert!(
                (total_row_consumed - inner_row_width).abs() < 0.001,
                "Table columns width {total_row_consumed} must match inner row width {inner_row_width}"
            );
            assert!(
                total_row_consumed + 8.0 <= card_available,
                "Row must not exceed card available width"
            );
            assert!(
                name_col_width >= 120.0,
                "Name column must maintain minimum width"
            );
        }
    }
}
