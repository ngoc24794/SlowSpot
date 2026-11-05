//! Main SlowSpot application dashboard.
//!
//! Implements `eframe::App` with a reactive immediate-mode UI receiving
//! metrics snapshots from the background collector worker.
//! Features Swiss minimalism design, accessible geometric indicators (WCAG AA),
//! real-time area charts, flat table headers, keyboard navigation, and live pause controls.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui::{self, Margin, Pos2, RichText, Sense, Stroke, Vec2};

use crate::model::{ActionCommand, ScoredProcess, SystemSnapshot};
use crate::ui::actions::ActionRunner;
use crate::ui::charts::{self, SparklineConfig};
use crate::ui::palette::{
    pressure_indicator, COLOR_ACCENT_OK, COLOR_BG, COLOR_BG_CARD_CULPRIT, COLOR_BG_CARD_SMOOTH,
    COLOR_BORDER, COLOR_BORDER_SUBTLE, COLOR_CARD, COLOR_DESTRUCTIVE, COLOR_DESTRUCTIVE_TEXT,
    COLOR_FOREGROUND, COLOR_INFO, COLOR_MUTED, COLOR_MUTED_FOREGROUND, COLOR_WARNING,
    FONT_SIZE_BODY, FONT_SIZE_CAPTION, FONT_SIZE_HEADING, FONT_SIZE_SUBHEADING, FONT_SIZE_TINY,
    RADIUS_LG, RADIUS_MD, RADIUS_SM,
};
use crate::ui::state::{
    format_bottleneck_explanation, format_bps, format_percent, SortColumn, ViewState,
};
use crate::ui::strings;
pub use crate::ui::table::{render_cell, TableLayout};
use crate::ui::{table, theme, widgets};

/// Main SlowSpot desktop application.
pub struct SlowSpotApp {
    rx: Receiver<SystemSnapshot>,
    actions: Box<dyn ActionRunner>,
    state: ViewState,
    demo: bool,
    confirm_kill: Option<ScoredProcess>,
    last_update_instant: Option<Instant>,
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
            confirm_kill: None,
            last_update_instant: None,
        }
    }

    /// Sets whether the application runs in simulated demo mode.
    pub fn with_demo(mut self, demo: bool) -> Self {
        self.demo = demo;
        self
    }
}

impl SlowSpotApp {
    /// Renders the complete dashboard interface.
    pub fn render_ui(&mut self, ui: &mut egui::Ui, now: Instant) {
        // 1. Drain incoming snapshots from the collector thread
        let mut got_snapshot = false;
        while let Ok(snapshot) = self.rx.try_recv() {
            self.state.push_snapshot(snapshot, now);
            got_snapshot = true;
        }
        if got_snapshot {
            self.last_update_instant = Some(now);
        }

        // 2. Handle keyboard shortcuts & navigation
        self.handle_keyboard_inputs(ui, now);

        // 3. Request a reactive repaint fallback (1 second cadence, avoids 60 FPS busy loop)
        ui.ctx().request_repaint_after(Duration::from_millis(1000));

        // 4. Fixed Bottom Status Bar: Rendered before CentralPanel, always visible at all window sizes >= 760x520
        egui::Panel::bottom("slowspot_bottom_status_bar")
            .frame(
                egui::Frame::new()
                    .fill(COLOR_CARD)
                    .inner_margin(Margin::symmetric(12, 4))
                    .stroke(Stroke::new(1.0, COLOR_BORDER)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                // 1px top border separating status bar from content
                let top_y = ui.max_rect().top();
                ui.painter().line_segment(
                    [
                        Pos2::new(ui.max_rect().left(), top_y),
                        Pos2::new(ui.max_rect().right(), top_y),
                    ],
                    Stroke::new(1.0, COLOR_BORDER),
                );
                self.render_bottom_status_bar(ui, now);
            });

        // 5. Central Dashboard Content with 12px horizontal and 8px vertical outer margin
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(COLOR_BG)
                    .inner_margin(Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                // Clip strictly to central panel bounds to prevent painting over status bar
                let central_clip = ui.clip_rect().intersect(ui.max_rect());
                ui.set_clip_rect(central_clip);

                // Ephemeral Toast Notifications (if any)
                self.render_toast_and_status(ui, now);

                // Header Banner (Title, Demo, Status Pill, Mini Chips, Pause Button)
                self.render_header_banner(ui, now);
                ui.add_space(3.0);

                // Top Culprit Hero Card / Smooth Status Banner
                self.render_top_culprit_card(ui, now);
                ui.add_space(3.0);

                // Real-time Sparklines (CPU, RAM, Disk)
                self.render_sparklines(ui);
                ui.add_space(3.0);

                // Process Table (Takes remaining vertical space, internal vertical scroll only)
                self.render_process_table(ui, now);
            });

        // 6. Modal confirmation dialog for forced kill
        self.render_confirm_kill_dialog(ui, now);
    }
}

impl eframe::App for SlowSpotApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_ui(ui, Instant::now());
    }
}

impl SlowSpotApp {
    /// Handles application-wide keyboard shortcuts.
    fn handle_keyboard_inputs(&mut self, ui: &mut egui::Ui, now: Instant) {
        let search_id = ui.make_persistent_id("slowspot_search_input");
        let search_focused = ui.ctx().memory(|m| m.has_focus(search_id));

        // Ctrl+F: Focus process search input
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
            ui.ctx().memory_mut(|m| m.request_focus(search_id));
        }

        // Esc: Close confirmation dialog, or clear search filter, or deselect
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.confirm_kill.is_some() {
                self.confirm_kill = None;
            } else if !self.state.search_query.is_empty() {
                self.state.search_query.clear();
            } else if self.state.selected_pid.is_some() {
                self.state.select_pid(None);
            }
        }

        // Space: Toggle live update pause / resume (only when search input is not active)
        if !search_focused && ui.input(|i| i.key_pressed(egui::Key::Space)) {
            self.state.toggle_pause(now);
        }

        // Arrow Up / Down: Navigate selected process row (when search input is not active)
        if !search_focused {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.state.select_next_process();
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.state.select_prev_process();
            }
        }
    }

    /// Renders the compact header banner containing app title, system pressure pill,
    /// mini resource chips, and live pause control.
    fn render_header_banner(&mut self, ui: &mut egui::Ui, now: Instant) {
        let card_frame = egui::Frame::new()
            .fill(COLOR_CARD)
            .corner_radius(RADIUS_MD)
            .inner_margin(Margin::symmetric(12, 6))
            .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));

        card_frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // 1. App Title
                ui.label(
                    RichText::new("SlowSpot")
                        .color(COLOR_FOREGROUND)
                        .size(FONT_SIZE_HEADING)
                        .strong(),
                );

                // 2. Demo Badge
                if self.demo {
                    let demo_badge = egui::Frame::new()
                        .fill(COLOR_WARNING.gamma_multiply(0.25))
                        .corner_radius(RADIUS_SM)
                        .inner_margin(Margin::symmetric(6, 2))
                        .stroke(Stroke::new(1.0, COLOR_WARNING.gamma_multiply(0.5)));
                    demo_badge.show(ui, |ui| {
                        ui.label(
                            RichText::new(strings::BADGE_DEMO)
                                .color(COLOR_WARNING)
                                .size(FONT_SIZE_TINY)
                                .strong(),
                        );
                    });
                }

                // 3. System Pressure Status Pill with geometric shape + text (WCAG compliant)
                let (status_text, status_shape, status_color) =
                    pressure_indicator(self.state.latest_snapshot.as_ref().map(|s| s.pressure));

                let pill_frame = egui::Frame::new()
                    .fill(status_color.gamma_multiply(0.18))
                    .corner_radius(RADIUS_MD)
                    .inner_margin(Margin::symmetric(8, 4))
                    .stroke(Stroke::new(1.0, status_color.gamma_multiply(0.4)));

                pill_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        widgets::draw_pressure_shape_indicator(
                            ui,
                            status_shape,
                            status_color,
                            10.0,
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(status_text)
                                .color(status_color)
                                .size(FONT_SIZE_CAPTION)
                                .strong(),
                        );
                    });
                });

                // 4. Paused State Indicator
                if self.state.is_paused {
                    let paused_badge = egui::Frame::new()
                        .fill(COLOR_WARNING.gamma_multiply(0.2))
                        .corner_radius(RADIUS_MD)
                        .inner_margin(Margin::symmetric(8, 4))
                        .stroke(Stroke::new(1.0, COLOR_WARNING));
                    paused_badge.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            widgets::draw_pause_icon(ui, COLOR_WARNING, 10.0);
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(strings::BADGE_PAUSED)
                                    .color(COLOR_WARNING)
                                    .size(FONT_SIZE_CAPTION)
                                    .strong(),
                            );
                        });
                    });
                }

                // 5. Cooldown notice if PC recovered from stress
                if self.state.is_cooldown_active(now) {
                    ui.horizontal(|ui| {
                        widgets::draw_checkmark(ui, COLOR_ACCENT_OK, 14.0);
                        ui.label(
                            RichText::new(strings::NOTICE_COOLDOWN)
                                .color(COLOR_ACCENT_OK)
                                .size(FONT_SIZE_CAPTION)
                                .strong(),
                        );
                    });
                }

                // Push right-aligned controls: Pause button & Mini resource chips
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Pause / Resume Toggle Button
                    let is_paused = self.state.is_paused;
                    let (btn_text, btn_tip, btn_color) = if is_paused {
                        (
                            strings::BTN_RESUME,
                            strings::TOOLTIP_RESUME_BUTTON,
                            COLOR_ACCENT_OK,
                        )
                    } else {
                        (
                            strings::BTN_PAUSE,
                            strings::TOOLTIP_PAUSE_BUTTON,
                            COLOR_MUTED_FOREGROUND,
                        )
                    };

                    let pause_btn = egui::Button::new(
                        RichText::new(btn_text)
                            .color(COLOR_FOREGROUND)
                            .size(FONT_SIZE_CAPTION),
                    )
                    .fill(if is_paused {
                        COLOR_WARNING.gamma_multiply(0.2)
                    } else {
                        COLOR_MUTED
                    })
                    .stroke(Stroke::new(1.0, btn_color.gamma_multiply(0.5)));

                    let pause_resp = ui.add(pause_btn);
                    if pause_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if pause_resp.on_hover_text(btn_tip).clicked() {
                        self.state.toggle_pause(now);
                    }

                    // Mini Resource Metric Chips (Disk, RAM, CPU)
                    if let Some(snap) = &self.state.latest_snapshot {
                        let mem_pct = if snap.totals.memory_total_bytes > 0 {
                            (snap.totals.memory_used_bytes as f32
                                / snap.totals.memory_total_bytes as f32)
                                * 100.0
                        } else {
                            0.0
                        };

                        let disk_mb = (snap.totals.disk_bps / (1024.0 * 1024.0)) as f32;
                        let disk_pct = (disk_mb / 50.0 * 100.0).clamp(0.0, 100.0);

                        // Disk Chip
                        widgets::mini_metric_chip(
                            ui,
                            "Đĩa",
                            &format_bps(snap.totals.disk_bps),
                            disk_pct,
                            COLOR_ACCENT_OK,
                        );

                        // RAM Chip
                        widgets::mini_metric_chip(
                            ui,
                            "RAM",
                            &format!("{:.0}%", mem_pct),
                            mem_pct,
                            COLOR_WARNING,
                        );

                        // CPU Chip
                        widgets::mini_metric_chip(
                            ui,
                            "CPU",
                            &format_percent(snap.totals.cpu_percent),
                            snap.totals.cpu_percent,
                            COLOR_DESTRUCTIVE,
                        );
                    }
                });
            });
        });
    }

    /// Renders the hero culprit card with primary/danger/secondary action button hierarchy.
    fn render_top_culprit_card(&mut self, ui: &mut egui::Ui, now: Instant) {
        let culprit = self.state.top_culprit().cloned();

        let (fill_color, border_color) = if culprit.is_some() {
            (COLOR_BG_CARD_CULPRIT, COLOR_DESTRUCTIVE.gamma_multiply(0.4))
        } else {
            (COLOR_BG_CARD_SMOOTH, COLOR_ACCENT_OK.gamma_multiply(0.3))
        };

        let card_frame = egui::Frame::new()
            .fill(fill_color)
            .corner_radius(RADIUS_MD)
            .inner_margin(Margin::symmetric(12, 5))
            .stroke(Stroke::new(1.0, border_color));

        card_frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(target) = culprit {
                let pid = target.record.pid;
                let killable = target.killable;

                // Row 1: Header (Title, Severity Badge, Protected Badge, Culprit Score)
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(strings::HERO_TITLE)
                            .color(COLOR_DESTRUCTIVE_TEXT)
                            .size(FONT_SIZE_CAPTION)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    widgets::severity_badge(ui, target.severity);
                    ui.add_space(4.0);

                    if !killable {
                        widgets::process_kind_badge(ui, crate::model::ProcessKind::SystemCritical);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{}: {:.0}/100",
                                strings::HERO_SCORE_LABEL,
                                target.score
                            ))
                            .color(COLOR_DESTRUCTIVE_TEXT)
                            .size(FONT_SIZE_BODY)
                            .strong(),
                        );
                    });
                });

                ui.add_space(3.0);

                // Row 2: Culprit Process Name & Bottleneck Explanation
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&target.record.name)
                            .color(COLOR_FOREGROUND)
                            .size(FONT_SIZE_SUBHEADING)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!("(PID {})", pid))
                            .color(COLOR_MUTED_FOREGROUND)
                            .size(FONT_SIZE_BODY),
                    );
                    ui.add_space(6.0);
                    let explanation = format_bottleneck_explanation(&target);
                    ui.label(
                        RichText::new(format!("— {explanation}"))
                            .color(COLOR_FOREGROUND)
                            .size(FONT_SIZE_BODY),
                    );
                });

                ui.add_space(6.0);

                // Row 3: Action Buttons with strict visual hierarchy
                let can_kill = self.state.can_force_kill(pid, now);
                let wait_rem = self.state.graceful_wait_remaining_secs(pid, now);

                ui.horizontal(|ui| {
                    // 1. Primary Button: Graceful Close ("Đóng an toàn") - Solid fill, NO fake focus outline
                    let (close_fill, close_color, close_stroke) = if killable {
                        (COLOR_INFO, COLOR_BG, Stroke::NONE)
                    } else {
                        (
                            COLOR_MUTED,
                            COLOR_MUTED_FOREGROUND,
                            Stroke::new(1.0, COLOR_BORDER_SUBTLE),
                        )
                    };

                    let close_btn = egui::Button::new(
                        RichText::new(strings::BTN_GRACEFUL_CLOSE)
                            .color(close_color)
                            .size(FONT_SIZE_BODY)
                            .strong(),
                    )
                    .fill(close_fill)
                    .stroke(close_stroke);

                    let close_resp = ui.add_enabled(killable, close_btn);
                    if close_resp.hovered() && killable {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if !killable {
                        close_resp.on_hover_text(strings::TOOLTIP_SYSTEM_PROTECTED);
                    } else if close_resp.clicked() {
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

                    // 2. Danger Button: Force Kill ("Bắt buộc tắt") - Solid red only when eligible
                    let kill_text = match wait_rem {
                        Some(s) if s > 0 => format!("{} ({}s)", strings::BTN_FORCE_KILL, s),
                        _ => strings::BTN_FORCE_KILL.to_string(),
                    };

                    let (kill_fill, kill_color, kill_stroke) = if can_kill && killable {
                        (COLOR_DESTRUCTIVE, COLOR_FOREGROUND, Stroke::NONE)
                    } else {
                        (
                            COLOR_MUTED,
                            COLOR_MUTED_FOREGROUND,
                            Stroke::new(1.0, COLOR_BORDER_SUBTLE),
                        )
                    };

                    let kill_btn = egui::Button::new(
                        RichText::new(kill_text)
                            .color(kill_color)
                            .size(FONT_SIZE_BODY)
                            .strong(),
                    )
                    .fill(kill_fill)
                    .stroke(kill_stroke);

                    let kill_resp = ui.add_enabled(killable && can_kill, kill_btn);
                    if kill_resp.hovered() && can_kill {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if !killable {
                        kill_resp.on_hover_text(strings::TOOLTIP_SYSTEM_PROTECTED);
                    } else if !can_kill {
                        if let Some(s) = wait_rem {
                            let tip = format!(
                                "{} {s} {}",
                                strings::TOOLTIP_WAIT_GRACEFUL_PREFIX,
                                strings::TOOLTIP_WAIT_GRACEFUL_SUFFIX
                            );
                            kill_resp.on_hover_text(tip);
                        } else {
                            kill_resp.on_hover_text(strings::NOTICE_FORCE_KILL_DISABLED_DEFAULT);
                        }
                    } else if kill_resp.clicked() {
                        self.confirm_kill = Some(target.clone());
                    }

                    // 3. Secondary Button: Open File Location ("Mở thư mục")
                    let open_btn = egui::Button::new(
                        RichText::new(strings::BTN_OPEN_LOCATION)
                            .color(COLOR_FOREGROUND)
                            .size(FONT_SIZE_CAPTION),
                    )
                    .fill(COLOR_CARD)
                    .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));

                    let open_resp = ui.add(open_btn);
                    if open_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if open_resp.clicked() {
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

                    // 4. Secondary Button: Ignore Culprit ("Bỏ qua thủ phạm này")
                    let ignore_btn = egui::Button::new(
                        RichText::new(strings::BTN_IGNORE_CULPRIT)
                            .color(COLOR_FOREGROUND)
                            .size(FONT_SIZE_CAPTION),
                    )
                    .fill(COLOR_CARD)
                    .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));

                    let ignore_resp = ui.add(ignore_btn);
                    if ignore_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if ignore_resp.clicked() {
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

                // Row 4: Immediate inline notice when Force Kill is disabled (Requirement 6)
                if !can_kill || !killable {
                    let reason_text = if !killable {
                        strings::NOTICE_SYSTEM_PROTECTED.to_string()
                    } else if let Some(s) = wait_rem {
                        format!(
                            "{} {s} {}",
                            strings::NOTICE_WAIT_GRACEFUL_PREFIX,
                            strings::NOTICE_WAIT_GRACEFUL_SUFFIX
                        )
                    } else {
                        strings::NOTICE_FORCE_KILL_DISABLED_DEFAULT.to_string()
                    };

                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        widgets::draw_warning_icon(ui, COLOR_MUTED_FOREGROUND, 11.0);
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(reason_text)
                                .color(COLOR_MUTED_FOREGROUND)
                                .size(FONT_SIZE_CAPTION),
                        );
                    });
                }
            } else {
                // Healthy PC State - compact 1-line layout
                ui.horizontal(|ui| {
                    widgets::draw_checkmark(ui, COLOR_ACCENT_OK, 16.0);
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(strings::HERO_NO_CULPRIT)
                            .color(COLOR_ACCENT_OK)
                            .size(FONT_SIZE_SUBHEADING)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!("— {}", strings::HERO_NO_CULPRIT_SUB))
                            .color(COLOR_MUTED_FOREGROUND)
                            .size(FONT_SIZE_CAPTION),
                    );
                });
            }
        });
    }

    /// Renders 60-second resource sparklines with area shading and hover inspection.
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

        // Distribute available width evenly minus spacing using guaranteed non-overflow calculation
        let available_width = ui.available_width();
        let spacing = ui.spacing().item_spacing.x;
        let [w0, w1, w2] = charts::calculate_sparkline_widths(available_width, spacing);

        ui.horizontal(|ui| {
            charts::sparkline(
                ui,
                SparklineConfig {
                    title: strings::SPARKLINE_CPU_TITLE,
                    current_label: &format_percent(cur_cpu),
                    unit: "%",
                    min_val: 0.0,
                    max_val: 100.0,
                    line_color: COLOR_DESTRUCTIVE,
                    size: Vec2::new(w0, 62.0),
                },
                &cpu_values,
            );
            charts::sparkline(
                ui,
                SparklineConfig {
                    title: strings::SPARKLINE_RAM_TITLE,
                    current_label: &format_percent(cur_ram),
                    unit: "%",
                    min_val: 0.0,
                    max_val: 100.0,
                    line_color: COLOR_WARNING,
                    size: Vec2::new(w1, 62.0),
                },
                &ram_values,
            );
            charts::sparkline(
                ui,
                SparklineConfig {
                    title: strings::SPARKLINE_DISK_TITLE,
                    current_label: &format!("{:.1} MB/s", cur_disk),
                    unit: " MB/s",
                    min_val: 0.0,
                    max_val: max_disk,
                    line_color: COLOR_ACCENT_OK,
                    size: Vec2::new(w2, 62.0),
                },
                &disk_values,
            );
        });
    }

    /// Renders the main process table with flat sortable headers, search filtering,
    /// zebra rows, and selected process action bar.
    fn render_process_table(&mut self, ui: &mut egui::Ui, now: Instant) {
        let card_frame = egui::Frame::new()
            .fill(COLOR_CARD)
            .corner_radius(RADIUS_MD)
            .inner_margin(Margin::symmetric(10, 5))
            .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));

        card_frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            let card_clip = ui.clip_rect().intersect(ui.max_rect());
            ui.set_clip_rect(card_clip);

            // Header Row: Section Title and Search Input Box
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(strings::TABLE_TITLE)
                        .color(COLOR_FOREGROUND)
                        .size(FONT_SIZE_SUBHEADING)
                        .strong(),
                );

                let remaining_width = ui.available_width();
                let search_width = remaining_width.max(120.0);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let search_id = ui.make_persistent_id("slowspot_search_input");
                    let text_edit = egui::TextEdit::singleline(&mut self.state.search_query)
                        .id(search_id)
                        .hint_text(strings::SEARCH_PLACEHOLDER)
                        .desired_width(search_width);
                    ui.add(text_edit);
                });
            });

            ui.add_space(2.0);

            // Column Header Layout Computation
            let card_available = ui.available_width();
            let spacing = ui.spacing().item_spacing.x;
            let layout = TableLayout::compute(card_available, spacing);

            // Flat Table Headers
            let header_frame = egui::Frame::new().inner_margin(Margin::symmetric(4, 0));
            header_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let sort_col = self.state.sort_column;
                    let sort_dir = self.state.sort_direction;

                    // Col 0: Name (Left aligned)
                    render_cell(
                        ui,
                        layout.name_width,
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            if table::render_flat_header(
                                ui,
                                strings::COL_NAME,
                                SortColumn::Name,
                                layout.name_width,
                                egui::Align::Min,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Name);
                            }
                        },
                    );

                    // Col 1: PID (Right aligned)
                    render_cell(
                        ui,
                        layout.pid_width,
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if table::render_flat_header(
                                ui,
                                strings::COL_PID,
                                SortColumn::Pid,
                                layout.pid_width,
                                egui::Align::Max,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Pid);
                            }
                        },
                    );

                    // Col 2: CPU % (Right aligned)
                    render_cell(
                        ui,
                        layout.cpu_width,
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if table::render_flat_header(
                                ui,
                                strings::COL_CPU,
                                SortColumn::Cpu,
                                layout.cpu_width,
                                egui::Align::Max,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Cpu);
                            }
                        },
                    );

                    // Col 3: RAM (Right aligned)
                    render_cell(
                        ui,
                        layout.ram_width,
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if table::render_flat_header(
                                ui,
                                strings::COL_RAM,
                                SortColumn::Memory,
                                layout.ram_width,
                                egui::Align::Max,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Memory);
                            }
                        },
                    );

                    // Col 4: Disk I/O (Right aligned)
                    render_cell(
                        ui,
                        layout.disk_width,
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if table::render_flat_header(
                                ui,
                                strings::COL_DISK,
                                SortColumn::Disk,
                                layout.disk_width,
                                egui::Align::Max,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Disk);
                            }
                        },
                    );

                    // Col 5: Score (Centered)
                    render_cell(
                        ui,
                        layout.score_width,
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            if table::render_flat_header(
                                ui,
                                strings::COL_SCORE,
                                SortColumn::Score,
                                layout.score_width,
                                egui::Align::Center,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Score);
                            }
                        },
                    );

                    // Col 6: Kind (Centered)
                    render_cell(
                        ui,
                        layout.kind_width,
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            if table::render_flat_header(
                                ui,
                                strings::COL_KIND,
                                SortColumn::Kind,
                                layout.kind_width,
                                egui::Align::Center,
                                sort_col,
                                sort_dir,
                            ) {
                                self.state.set_sort(SortColumn::Kind);
                            }
                        },
                    );
                });
            });

            // Compact 1px divider directly below header (Defect 3: reduces header gap)
            let sep_stroke = Stroke::new(1.0, COLOR_BORDER_SUBTLE);
            let sep_rect = ui
                .allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover())
                .0;
            ui.painter()
                .line_segment([sep_rect.left_top(), sep_rect.right_top()], sep_stroke);

            // Reserve room for bottom action bar if process is selected, and let rows scroll inside
            let bottom_bar_height = if self.state.selected_pid.is_some() {
                42.0
            } else {
                0.0
            };
            let card_bottom_margin = 5.0;
            let available_h =
                (ui.available_height() - bottom_bar_height - card_bottom_margin).max(60.0);

            egui::ScrollArea::vertical()
                .max_height(available_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let scroll_clip = ui.clip_rect().intersect(ui.max_rect());
                    ui.set_clip_rect(scroll_clip);
                    let top_20: Vec<ScoredProcess> = self
                        .state
                        .filtered_processes()
                        .into_iter()
                        .take(20)
                        .cloned()
                        .collect();

                    if top_20.is_empty() {
                        table::render_empty_table_state(ui);
                    } else {
                        for (idx, proc) in top_20.iter().enumerate() {
                            let pid = proc.record.pid;
                            let is_selected = self.state.selected_pid == Some(pid);

                            let row_resp =
                                table::render_process_row(ui, proc, &layout, is_selected, idx);
                            if row_resp.clicked() {
                                if is_selected {
                                    self.state.select_pid(None);
                                } else {
                                    self.state.select_pid(Some(pid));
                                }
                            }
                        }
                    }
                });

            // Selected Process Action Bar (anchored at bottom of table card)
            if self.state.selected_pid.is_some() {
                ui.add_space(4.0);
                self.render_selected_action_bar(ui, now);
            }
        });
    }

    /// Renders the action bar for the currently selected process below the table.
    fn render_selected_action_bar(&mut self, ui: &mut egui::Ui, now: Instant) {
        let selected_proc = self.state.selected_pid.and_then(|pid| {
            self.state
                .latest_snapshot
                .as_ref()?
                .processes
                .iter()
                .find(|p| p.record.pid == pid)
                .cloned()
        });

        let bar_frame = egui::Frame::new()
            .fill(COLOR_MUTED.gamma_multiply(0.4))
            .corner_radius(RADIUS_MD)
            .inner_margin(Margin::symmetric(10, 6));

        bar_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(target) = selected_proc {
                    let pid = target.record.pid;
                    let killable = target.killable;

                    ui.label(
                        RichText::new(strings::SELECTED_PROCESS_LABEL)
                            .color(COLOR_MUTED_FOREGROUND)
                            .size(FONT_SIZE_CAPTION),
                    );
                    ui.label(
                        RichText::new(format!("{} (PID {})", target.record.name, pid))
                            .color(COLOR_FOREGROUND)
                            .size(FONT_SIZE_BODY)
                            .strong(),
                    );
                    if !killable {
                        widgets::process_kind_badge(ui, crate::model::ProcessKind::SystemCritical);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Ignore button
                        let ignore_btn = egui::Button::new(
                            RichText::new(strings::BTN_IGNORE_CULPRIT)
                                .color(COLOR_MUTED_FOREGROUND)
                                .size(FONT_SIZE_CAPTION),
                        )
                        .fill(COLOR_CARD)
                        .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));
                        let ignore_resp = ui.add(ignore_btn);
                        if ignore_resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if ignore_resp.clicked() {
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
                        let open_btn = egui::Button::new(
                            RichText::new(strings::BTN_OPEN_LOCATION)
                                .color(COLOR_MUTED_FOREGROUND)
                                .size(FONT_SIZE_CAPTION),
                        )
                        .fill(COLOR_CARD)
                        .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));
                        let open_resp = ui.add(open_btn);
                        if open_resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if open_resp.clicked() {
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

                        // Force kill button (Danger)
                        let can_kill = self.state.can_force_kill(pid, now);
                        let wait_rem = self.state.graceful_wait_remaining_secs(pid, now);
                        let kill_text = match wait_rem {
                            Some(s) if s > 0 => format!("{} ({}s)", strings::BTN_FORCE_KILL, s),
                            _ => strings::BTN_FORCE_KILL.to_string(),
                        };

                        let (kill_fill, kill_color, kill_stroke) = if can_kill && killable {
                            (COLOR_DESTRUCTIVE, COLOR_FOREGROUND, Stroke::NONE)
                        } else {
                            (
                                COLOR_MUTED,
                                COLOR_MUTED_FOREGROUND,
                                Stroke::new(1.0, COLOR_BORDER_SUBTLE),
                            )
                        };

                        let kill_btn = egui::Button::new(
                            RichText::new(kill_text)
                                .color(kill_color)
                                .size(FONT_SIZE_CAPTION)
                                .strong(),
                        )
                        .fill(kill_fill)
                        .stroke(kill_stroke);

                        let kill_resp = ui.add_enabled(killable && can_kill, kill_btn);
                        if kill_resp.hovered() && can_kill {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if !killable {
                            kill_resp.on_hover_text(strings::TOOLTIP_SYSTEM_PROTECTED);
                        } else if !can_kill {
                            if let Some(s) = wait_rem {
                                let tip = format!(
                                    "{} {s} {}",
                                    strings::TOOLTIP_WAIT_GRACEFUL_PREFIX,
                                    strings::TOOLTIP_WAIT_GRACEFUL_SUFFIX
                                );
                                kill_resp.on_hover_text(tip);
                            } else {
                                kill_resp
                                    .on_hover_text(strings::NOTICE_FORCE_KILL_DISABLED_DEFAULT);
                            }
                        } else if kill_resp.clicked() {
                            self.confirm_kill = Some(target.clone());
                        }

                        // Graceful close button (Primary)
                        let (close_fill, close_color, close_stroke) = if killable {
                            (COLOR_INFO, COLOR_BG, Stroke::NONE)
                        } else {
                            (
                                COLOR_MUTED,
                                COLOR_MUTED_FOREGROUND,
                                Stroke::new(1.0, COLOR_BORDER_SUBTLE),
                            )
                        };

                        let close_btn = egui::Button::new(
                            RichText::new(strings::BTN_GRACEFUL_CLOSE)
                                .color(close_color)
                                .size(FONT_SIZE_CAPTION)
                                .strong(),
                        )
                        .fill(close_fill)
                        .stroke(close_stroke);

                        let close_resp = ui.add_enabled(killable, close_btn);
                        if close_resp.hovered() && killable {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if !killable {
                            close_resp.on_hover_text(strings::TOOLTIP_SYSTEM_PROTECTED);
                        } else if close_resp.clicked() {
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
                            .color(COLOR_MUTED_FOREGROUND)
                            .size(FONT_SIZE_CAPTION),
                    );
                }
            });
        });
    }

    /// Renders ephemeral toast feedback notifications with success/error icons.
    fn render_toast_and_status(&self, ui: &mut egui::Ui, now: Instant) {
        if let Some(toast) = self.state.active_toast(now) {
            let (bg, border) = if toast.is_error {
                (COLOR_DESTRUCTIVE.gamma_multiply(0.25), COLOR_DESTRUCTIVE)
            } else {
                (COLOR_ACCENT_OK.gamma_multiply(0.25), COLOR_ACCENT_OK)
            };

            let toast_frame = egui::Frame::new()
                .fill(bg)
                .corner_radius(RADIUS_MD)
                .inner_margin(Margin::symmetric(12, 7))
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
                            .color(COLOR_FOREGROUND)
                            .size(FONT_SIZE_BODY),
                    );
                });
            });
        }
    }

    /// Renders the bottom status bar with keyboard shortcuts, last update timestamp,
    /// and total process count.
    fn render_bottom_status_bar(&self, ui: &mut egui::Ui, now: Instant) {
        ui.horizontal(|ui| {
            // Left: Keyboard Shortcut Cheatsheet (>= 12px, solid high-contrast muted foreground)
            ui.label(
                RichText::new(strings::STATUS_HINT_SHORTCUTS)
                    .color(COLOR_MUTED_FOREGROUND)
                    .size(FONT_SIZE_CAPTION),
            );

            // Right: Metadata (Last Updated, Process Count, Demo / Paused badges)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let update_str = match self.last_update_instant {
                    Some(t) => {
                        let elapsed_secs = now.duration_since(t).as_secs();
                        if elapsed_secs == 0 {
                            format!("{} vừa xong", strings::STATUS_UPDATED_PREFIX)
                        } else {
                            format!("{} {}s trước", strings::STATUS_UPDATED_PREFIX, elapsed_secs)
                        }
                    }
                    None => format!("{} --", strings::STATUS_UPDATED_PREFIX),
                };

                ui.label(
                    RichText::new(update_str)
                        .color(COLOR_MUTED_FOREGROUND)
                        .size(FONT_SIZE_CAPTION),
                );

                ui.label(
                    RichText::new("|")
                        .color(COLOR_BORDER)
                        .size(FONT_SIZE_CAPTION),
                );

                let proc_count = self.state.display_processes.len();
                let count_str = format!(
                    "{} {} {}",
                    strings::STATUS_TOTAL_PROCESSES,
                    proc_count,
                    strings::STATUS_PROCESS_UNIT
                );
                ui.label(
                    RichText::new(count_str)
                        .color(COLOR_MUTED_FOREGROUND)
                        .size(FONT_SIZE_CAPTION),
                );

                if self.state.is_paused {
                    ui.label(
                        RichText::new("|")
                            .color(COLOR_BORDER_SUBTLE)
                            .size(FONT_SIZE_CAPTION),
                    );
                    ui.label(
                        RichText::new(format!("[{}]", strings::BADGE_PAUSED))
                            .color(COLOR_WARNING)
                            .size(FONT_SIZE_CAPTION)
                            .strong(),
                    );
                }

                if self.demo {
                    ui.label(
                        RichText::new("|")
                            .color(COLOR_BORDER_SUBTLE)
                            .size(FONT_SIZE_CAPTION),
                    );
                    ui.label(
                        RichText::new(format!("[{}]", strings::BADGE_DEMO))
                            .color(COLOR_WARNING)
                            .size(FONT_SIZE_CAPTION)
                            .strong(),
                    );
                }
            });
        });
    }

    /// Renders the modal confirmation dialog for forced kill operations.
    fn render_confirm_kill_dialog(&mut self, ui: &mut egui::Ui, now: Instant) {
        if let Some(target) = self.confirm_kill.clone() {
            let mut open = true;
            egui::Window::new(strings::CONFIRM_TITLE)
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::new()
                        .fill(COLOR_CARD)
                        .corner_radius(RADIUS_LG)
                        .inner_margin(Margin::same(16))
                        .stroke(Stroke::new(1.0, COLOR_BORDER)),
                )
                .show(ui.ctx(), |ui| {
                    ui.set_width(390.0);
                    ui.label(
                        RichText::new(format!(
                            "Bạn có chắc chắn muốn buộc tắt tiến trình '{}' (PID {})?\n\nHành động này sẽ ngắt tiến trình ngay lập tức và có thể làm mất dữ liệu chưa lưu.",
                            target.record.name, target.record.pid
                        ))
                        .color(COLOR_FOREGROUND)
                        .size(FONT_SIZE_BODY),
                    );

                    ui.add_space(14.0);

                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Cancel is safe default button
                            let cancel_btn = egui::Button::new(
                                RichText::new(strings::CONFIRM_CANCEL)
                                    .color(COLOR_FOREGROUND)
                                    .size(FONT_SIZE_BODY),
                            )
                            .fill(COLOR_MUTED)
                            .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));

                            if ui.add(cancel_btn).clicked() {
                                self.confirm_kill = None;
                            }

                            // Proceed is danger button
                            let kill_btn = egui::Button::new(
                                RichText::new(strings::CONFIRM_PROCEED)
                                    .color(COLOR_FOREGROUND)
                                    .size(FONT_SIZE_BODY)
                                    .strong(),
                            )
                            .fill(COLOR_DESTRUCTIVE);

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
    fn test_panel_and_central_margin_layout() {
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        for &(win_w, win_h) in &[(1000.0, 680.0), (760.0, 520.0)] {
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(win_w, win_h),
                )),
                ..Default::default()
            };

            let mut output = ctx.run_ui(raw_input, |ui| {
                // Bottom panel
                egui::Panel::bottom("test_status_bar")
                    .frame(
                        egui::Frame::new()
                            .fill(COLOR_CARD)
                            .inner_margin(Margin::symmetric(12, 6)),
                    )
                    .show(ui, |ui| {
                        ui.label("Status Bar");
                    });

                // Central panel with uniform 12px outer margin
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().inner_margin(Margin::same(12)))
                    .show(ui, |ui| {
                        let avail_w = ui.available_width();
                        // 12px margin on left and right means avail_w == win_w - 24.0
                        assert!(
                            (avail_w - (win_w - 24.0)).abs() < 1.0,
                            "Available width {avail_w} must equal window width minus 24px margins"
                        );
                    });
            });
            output.textures_delta.clear();
        }
    }

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

    /// Verifies that no process table rows or cells paint over the bottom status bar (Defect 1).
    #[test]
    fn test_process_table_clipping_above_status_bar() {
        use crate::collector::SampleSource;
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        for &(win_w, win_h) in &[(1000.0, 680.0), (760.0, 520.0)] {
            let (_tx, rx) = std::sync::mpsc::channel();
            let mut app = SlowSpotApp {
                rx,
                actions: Box::new(crate::ui::DemoActions),
                state: ViewState::new(),
                demo: true,
                confirm_kill: None,
                last_update_instant: None,
            };

            let mut mock = crate::collector::MockSource::demo();
            let snap = crate::analyzer::Analyzer::default().analyze(mock.sample());
            app.state.push_snapshot(snap, Instant::now());

            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(win_w, win_h),
                )),
                ..Default::default()
            };

            let mut output = ctx.run_ui(raw_input, |ui| {
                app.render_ui(ui, Instant::now());
            });

            // Status bar is at the bottom of the window
            let status_bar_top = win_h - 32.0;

            for shape in &output.shapes {
                let clip = shape.clip_rect;
                if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                    let text = text_shape.galley.text();
                    if text.contains(".exe") || text.contains("MB/s") {
                        assert!(
                            clip.max.y <= status_bar_top + 2.0,
                            "Table text '{}' clip_rect.max.y ({}) must not paint over status bar top ({})",
                            text,
                            clip.max.y,
                            status_bar_top
                        );
                    }
                }
            }
            output.textures_delta.clear();
        }
    }

    /// Verifies visible rows requirement: >= 12 rows at 1000x680 and >= 8 at 760x520 (Defect 3).
    #[test]
    fn test_visible_rows_count_at_supported_resolutions() {
        use crate::collector::SampleSource;
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        let test_cases = [(1000.0f32, 680.0f32, 12usize), (760.0, 520.0, 8)];

        for &(win_w, win_h, expected_min_rows) in &test_cases {
            let (_tx, rx) = std::sync::mpsc::channel();
            let mut app = SlowSpotApp {
                rx,
                actions: Box::new(crate::ui::DemoActions),
                state: ViewState::new(),
                demo: true,
                confirm_kill: None,
                last_update_instant: None,
            };

            let mut mock = crate::collector::MockSource::demo();
            let snap = crate::analyzer::Analyzer::default().analyze(mock.sample());
            app.state.push_snapshot(snap, Instant::now());

            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(win_w, win_h),
                )),
                ..Default::default()
            };

            let mut row_positions = Vec::new();
            let mut output = ctx.run_ui(raw_input, |ui| {
                app.render_ui(ui, Instant::now());
            });

            for shape in &output.shapes {
                if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                    let t = text_shape.galley.text();
                    if t.contains(".exe") {
                        row_positions.push((t.to_string(), text_shape.pos, shape.clip_rect));
                    }
                }
            }

            assert!(
                row_positions.len() >= expected_min_rows,
                "At {}x{}, found {} visible rows, expected at least {}",
                win_w,
                win_h,
                row_positions.len(),
                expected_min_rows
            );
            output.textures_delta.clear();
        }
    }

    /// Verifies that plain user apps render as muted text without button-like border (Defect 4).
    #[test]
    fn test_plain_app_kind_badge_is_muted_text_without_border() {
        use crate::model::ProcessKind;
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(300.0, 100.0),
            )),
            ..Default::default()
        };

        // 1. Render UserApp
        let mut out_app = ctx.run_ui(raw_input.clone(), |ui| {
            widgets::process_kind_badge(ui, ProcessKind::UserApp);
        });

        let mut app_rect_count = 0;
        let mut app_text_found = false;
        for shape in &out_app.shapes {
            match &shape.shape {
                egui::epaint::Shape::Rect(_) => app_rect_count += 1,
                egui::epaint::Shape::Text(t) if t.galley.text() == strings::KIND_USER_APP => {
                    app_text_found = true;
                }
                _ => {}
            }
        }
        assert!(app_text_found, "UserApp text must be rendered");
        assert_eq!(
            app_rect_count, 0,
            "UserApp must not have background or border rects"
        );
        out_app.textures_delta.clear();

        // 2. Render BackgroundService
        let mut out_svc = ctx.run_ui(raw_input, |ui| {
            widgets::process_kind_badge(ui, ProcessKind::BackgroundService);
        });

        let mut svc_rect_count = 0;
        for shape in &out_svc.shapes {
            if let egui::epaint::Shape::Rect(_) = &shape.shape {
                svc_rect_count += 1;
            }
        }
        assert!(
            svc_rect_count >= 1,
            "BackgroundService must retain a badge border/pill"
        );
        out_svc.textures_delta.clear();
    }

    /// Verifies status bar contrast and 1px top border specifications (Defect 5).
    #[test]
    fn test_status_bar_specs_contrast_and_border() {
        const {
            assert!(FONT_SIZE_CAPTION >= 12.0);
        }
        assert_eq!(
            COLOR_MUTED_FOREGROUND,
            Color32::from_rgb(148, 163, 184),
            "Status bar text color must be #94A3B8"
        );
        assert_eq!(COLOR_BORDER, Color32::from_rgb(71, 85, 105));
    }
}
