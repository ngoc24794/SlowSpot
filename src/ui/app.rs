//! Main SlowSpot application dashboard.
//!
//! Implements `eframe::App` with a reactive immediate-mode UI receiving
//! metrics snapshots from the background collector worker.
//! Features Swiss minimalism design, accessible geometric indicators (WCAG AA),
//! real-time area charts, flat table headers, keyboard navigation, and live pause controls.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui::{self, Margin, Pos2, RichText, Sense, Stroke, Vec2};

use crate::analyzer::{format_breakdown_banner, format_pressure_badge_tooltip};
use crate::model::{ActionCommand, Episode, PressureBreakdown, ScoredProcess, SystemSnapshot};
use crate::ui::actions::ActionRunner;
use crate::ui::charts::{self, SparklineConfig};
use crate::ui::palette::{
    evaluate_header_badge, level_tokens, COLOR_ACCENT_OK, COLOR_BG, COLOR_BG_CARD_SMOOTH,
    COLOR_BORDER, COLOR_BORDER_SUBTLE, COLOR_CARD, COLOR_DESTRUCTIVE, COLOR_FOREGROUND, COLOR_INFO,
    COLOR_MUTED, COLOR_MUTED_FOREGROUND, COLOR_WARNING, FONT_SIZE_BODY, FONT_SIZE_CAPTION,
    FONT_SIZE_HEADING, FONT_SIZE_SUBHEADING, FONT_SIZE_TINY, RADIUS_LG, RADIUS_MD, RADIUS_SM,
};
use crate::ui::state::{
    format_bottleneck_explanation, format_bps, format_bytes, format_episode_time_str,
    format_percent, SortColumn, ViewState,
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

/// Fixed height of the "Đang theo dõi" (Watched) panel slot.
pub const WATCHED_SLOT_HEIGHT: f32 = 50.0;

/// Fixed height of the status area (Hero culprit card / smooth calm banner) slot.
pub const STATUS_SLOT_HEIGHT: f32 = 104.0;

/// Active visual presentation state of the hero card.
#[derive(Debug, Clone, PartialEq)]
pub enum HeroBannerState {
    /// A single dominant culprit was identified.
    Culprit(ScoredProcess),
    /// System pressure is high (Strained or Overloaded) but no single culprit is identified.
    HighPressureNoCulprit {
        title: String,
        subtitle: String,
        group_culprit_name: Option<String>,
        color: egui::Color32,
    },
    /// System is healthy and smooth with no culprits.
    SmoothHealthy {
        title: &'static str,
        subtitle: &'static str,
        color: egui::Color32,
    },
}

/// Evaluates hero banner presentation state based on culprit presence and overall system pressure.
pub fn evaluate_hero_banner_state(
    culprit: Option<&ScoredProcess>,
    pressure: crate::model::PressureState,
    breakdown: Option<&PressureBreakdown>,
) -> HeroBannerState {
    if let Some(target) = culprit {
        HeroBannerState::Culprit(target.clone())
    } else {
        match pressure {
            crate::model::PressureState::Overloaded | crate::model::PressureState::Strained => {
                let color = COLOR_WARNING;

                if let Some(bd) = breakdown {
                    let subtitle = format_breakdown_banner(bd);
                    if let Some(gc) = &bd.group_culprit {
                        HeroBannerState::HighPressureNoCulprit {
                            title: format!(
                                "Nhóm thủ phạm: {} ({} tiến trình)",
                                gc.name, gc.process_count
                            ),
                            subtitle,
                            group_culprit_name: Some(gc.name.clone()),
                            color,
                        }
                    } else {
                        let title = if pressure == crate::model::PressureState::Overloaded {
                            strings::HERO_PRESSURE_OVERLOADED.to_string()
                        } else {
                            strings::HERO_PRESSURE_STRAINED.to_string()
                        };
                        HeroBannerState::HighPressureNoCulprit {
                            title,
                            subtitle,
                            group_culprit_name: None,
                            color,
                        }
                    }
                } else {
                    HeroBannerState::HighPressureNoCulprit {
                        title: if pressure == crate::model::PressureState::Overloaded {
                            strings::HERO_PRESSURE_OVERLOADED.to_string()
                        } else {
                            strings::HERO_PRESSURE_STRAINED.to_string()
                        },
                        subtitle: strings::HERO_PRESSURE_NO_SINGLE_CULPRIT_SUB.to_string(),
                        group_culprit_name: None,
                        color,
                    }
                }
            }
            crate::model::PressureState::Smooth => HeroBannerState::SmoothHealthy {
                title: strings::HERO_NO_CULPRIT,
                subtitle: strings::HERO_NO_CULPRIT_SUB,
                color: COLOR_ACCENT_OK,
            },
        }
    }
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

        // 5. Central Dashboard Content with 12px horizontal and 4px vertical outer margin
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(COLOR_BG)
                    .inner_margin(Margin::symmetric(12, 4)),
            )
            .show(ui, |ui| {
                // Clip strictly to central panel bounds to prevent painting over status bar
                let central_clip = ui.clip_rect().intersect(ui.max_rect());
                ui.set_clip_rect(central_clip);

                // Ephemeral Toast Notifications (if any)
                self.render_toast_and_status(ui, now);

                // Header Banner (Title, Demo, Status Pill, Mini Chips, Pause Button)
                self.render_header_banner(ui, now);
                ui.add_space(2.0);

                // Watched Panel ("Đang theo dõi" - Fixed slot above status card)
                self.render_watched_panel(ui, now);
                ui.add_space(2.0);

                // Top Culprit Hero Card / Smooth Status Banner - Fixed slot
                self.render_top_culprit_card(ui, now);
                ui.add_space(2.0);

                // Real-time Sparklines (CPU, RAM, Disk)
                self.render_sparklines(ui);
                ui.add_space(2.0);

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
            let card_rect = ui.available_rect_before_wrap();

            let right_spacing = ui.spacing().item_spacing.x;
            // Right cluster layout budget (Pause button + 3 resource chips + item spacing)
            let right_cluster_w = if self.state.latest_snapshot.is_some() {
                64.0 + 3.0 * 126.0 + 3.0 * right_spacing
            } else {
                64.0
            };
            let pill_area_left = card_rect.right() - right_cluster_w - 10.0;

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

                // 3. System Pressure Status Pill with unified level mapping (Single Source of Truth)
                let snap_ref = self.state.latest_snapshot.as_ref();
                let pressure = snap_ref
                    .map(|s| s.pressure)
                    .unwrap_or(crate::model::PressureState::Smooth);
                let culprit = self.state.top_culprit();
                let totals = snap_ref.map(|s| &s.totals);
                let badge_info = evaluate_header_badge(culprit, pressure, totals);
                let status_shape = badge_info.shape;
                let status_color = badge_info.color;
                let status_text = badge_info.text;
                let tooltip_text = snap_ref
                    .map(|s| format_pressure_badge_tooltip(&s.totals))
                    .unwrap_or_default();

                let pill_frame = egui::Frame::new()
                    .fill(status_color.gamma_multiply(0.18))
                    .corner_radius(RADIUS_MD)
                    .inner_margin(Margin::symmetric(8, 4))
                    .stroke(Stroke::new(1.0, status_color.gamma_multiply(0.4)));

                pill_frame.show(ui, |ui| {
                    let resp = ui
                        .horizontal(|ui| {
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
                        })
                        .response;
                    if !tooltip_text.is_empty() {
                        resp.on_hover_text(tooltip_text);
                    }
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

                // 5. Cooldown notice if PC recovered from stress (strictly non-overlapping before right pills)
                let current_x = ui.cursor().min.x;
                let available_for_notice = (pill_area_left - current_x).max(0.0);

                if self.state.is_cooldown_active(now) && available_for_notice >= 40.0 {
                    let notice_w = available_for_notice.min(220.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(notice_w, ui.available_height()),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_max_width(notice_w);
                            widgets::draw_checkmark(ui, COLOR_ACCENT_OK, 14.0);
                            ui.add_space(2.0);
                            let text_w = (notice_w - 18.0).max(10.0);
                            ui.add_sized(
                                Vec2::new(text_w, ui.available_height()),
                                egui::Label::new(
                                    RichText::new(strings::NOTICE_COOLDOWN)
                                        .color(COLOR_ACCENT_OK)
                                        .size(FONT_SIZE_CAPTION)
                                        .strong(),
                                )
                                .truncate(),
                            );
                        },
                    );
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

                    let pause_resp = ui.add_sized(Vec2::new(64.0, 24.0), pause_btn);
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
        let pressure = self
            .state
            .latest_snapshot
            .as_ref()
            .map(|s| s.pressure)
            .unwrap_or(crate::model::PressureState::Smooth);
        let breakdown = self
            .state
            .latest_snapshot
            .as_ref()
            .and_then(|s| s.breakdown.as_ref());

        let banner_state = evaluate_hero_banner_state(culprit.as_ref(), pressure, breakdown);

        let (fill_color, border_color) = match &banner_state {
            HeroBannerState::Culprit(target) => {
                let tokens = level_tokens(target.severity);
                (tokens.bg, tokens.border)
            }
            HeroBannerState::HighPressureNoCulprit { color, .. } => {
                (COLOR_MUTED, color.gamma_multiply(0.4))
            }
            HeroBannerState::SmoothHealthy { color, .. } => {
                (COLOR_BG_CARD_SMOOTH, color.gamma_multiply(0.3))
            }
        };

        let card_frame = egui::Frame::new()
            .fill(fill_color)
            .corner_radius(RADIUS_MD)
            .inner_margin(Margin::symmetric(10, 4))
            .stroke(Stroke::new(1.0, border_color));

        let (slot_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), STATUS_SLOT_HEIGHT),
            Sense::hover(),
        );
        let slot_clip = ui.clip_rect().intersect(slot_rect);
        let mut slot_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(slot_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        slot_ui.set_clip_rect(slot_clip);

        card_frame.show(&mut slot_ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(STATUS_SLOT_HEIGHT - 8.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            match banner_state {
                HeroBannerState::Culprit(target) => {
                    let tokens = level_tokens(target.severity);
                    let pid = target.record.pid;
                    let killable = target.killable;

                    // Row 1: Header (Title, Severity Badge, Protected Badge, Culprit Score)
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tokens.title)
                                .color(tokens.border)
                                .size(FONT_SIZE_CAPTION)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        widgets::severity_badge(ui, target.severity);
                        ui.add_space(4.0);

                        if !killable {
                            widgets::process_kind_badge(
                                ui,
                                crate::model::ProcessKind::SystemCritical,
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{}: {:.0}/100",
                                    strings::HERO_SCORE_LABEL,
                                    target.score
                                ))
                                .color(tokens.border)
                                .size(FONT_SIZE_CAPTION)
                                .strong(),
                            );
                        });
                    });

                    ui.add_space(1.0);

                    // Row 2: Culprit Process Name & Bottleneck Explanation
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&target.record.name)
                                .color(COLOR_FOREGROUND)
                                .size(FONT_SIZE_BODY)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("(PID {})", pid))
                                .color(COLOR_MUTED_FOREGROUND)
                                .size(FONT_SIZE_CAPTION),
                        );
                        ui.add_space(4.0);
                        let explanation = format_bottleneck_explanation(&target);
                        ui.label(
                            RichText::new(format!("— {explanation}"))
                                .color(COLOR_FOREGROUND)
                                .size(FONT_SIZE_CAPTION),
                        );
                    });

                    ui.add_space(2.0);

                    // Row 3: Compact single row of actions with shorter note
                    let can_kill = self.state.can_force_kill(pid, now);
                    let wait_rem = self.state.graceful_wait_remaining_secs(pid, now);

                    ui.horizontal(|ui| {
                        // 1. Primary Button: Graceful Close ("Đóng an toàn")
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

                        // 2. Danger Button: Force Kill ("Bắt buộc tắt")
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

                    // Row 4: Status note explaining when force-kill is available or system protection
                    if !can_kill || !killable {
                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            let note_text = if !killable {
                                strings::TOOLTIP_SYSTEM_PROTECTED
                            } else if let Some(s) = wait_rem {
                                &format!(
                                    "{} {s} {}",
                                    strings::TOOLTIP_WAIT_GRACEFUL_PREFIX,
                                    strings::TOOLTIP_WAIT_GRACEFUL_SUFFIX
                                )
                            } else {
                                strings::NOTICE_FORCE_KILL_DISABLED_DEFAULT
                            };

                            widgets::draw_warning_icon(ui, COLOR_MUTED_FOREGROUND, 11.0);
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(note_text)
                                    .color(COLOR_MUTED_FOREGROUND)
                                    .size(FONT_SIZE_CAPTION),
                            );
                        });
                    }
                }
                HeroBannerState::HighPressureNoCulprit {
                    title,
                    subtitle,
                    group_culprit_name,
                    color,
                } => {
                    let content_h = if group_culprit_name.is_some() {
                        50.0
                    } else {
                        24.0
                    };
                    let pad_top = ((STATUS_SLOT_HEIGHT - 8.0 - content_h) / 2.0).max(0.0);
                    ui.add_space(pad_top);

                    ui.horizontal(|ui| {
                        widgets::draw_warning_icon(ui, color, 16.0);
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(title)
                                .color(color)
                                .size(FONT_SIZE_SUBHEADING)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("— {subtitle}"))
                                .color(COLOR_MUTED_FOREGROUND)
                                .size(FONT_SIZE_CAPTION),
                        );
                    });

                    if let Some(ref group_name) = group_culprit_name {
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            let filter_btn = egui::Button::new(
                                RichText::new("Lọc trong bảng")
                                    .color(COLOR_FOREGROUND)
                                    .size(FONT_SIZE_CAPTION),
                            )
                            .fill(COLOR_CARD)
                            .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));
                            if ui.add(filter_btn).clicked() {
                                self.state.search_query = group_name.clone();
                            }

                            let ignore_btn = egui::Button::new(
                                RichText::new("Bỏ qua nhóm này")
                                    .color(COLOR_FOREGROUND)
                                    .size(FONT_SIZE_CAPTION),
                            )
                            .fill(COLOR_CARD)
                            .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));
                            if ui.add(ignore_btn).clicked() {
                                self.state.ignore_process(group_name);
                                self.state.add_toast(
                                    format!("{} '{}'", strings::MSG_IGNORED_SUCCESS, group_name),
                                    false,
                                    now,
                                    Duration::from_secs(4),
                                );
                            }

                            ui.label(
                                RichText::new("Can thiệp từng tiến trình trong bảng bên dưới")
                                    .color(COLOR_MUTED_FOREGROUND)
                                    .size(FONT_SIZE_TINY),
                            );
                        });
                    }
                }
                HeroBannerState::SmoothHealthy {
                    title,
                    subtitle,
                    color,
                } => {
                    let pad_top = ((STATUS_SLOT_HEIGHT - 8.0 - 24.0) / 2.0).max(0.0);
                    ui.add_space(pad_top);

                    ui.horizontal(|ui| {
                        widgets::draw_checkmark(ui, color, 16.0);
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(title)
                                .color(color)
                                .size(FONT_SIZE_SUBHEADING)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("— {subtitle}"))
                                .color(COLOR_MUTED_FOREGROUND)
                                .size(FONT_SIZE_CAPTION),
                        );
                    });
                }
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
                    size: Vec2::new(w0, 50.0),
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
                    size: Vec2::new(w1, 50.0),
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
                    size: Vec2::new(w2, 50.0),
                },
                &disk_values,
            );
        });
    }

    /// Renders the compact "Đang theo dõi" panel above the status area in a fixed-height slot.
    fn render_watched_panel(&mut self, ui: &mut egui::Ui, now: Instant) {
        let episodes: Vec<Episode> = self.state.watched_episodes().to_vec();

        let (slot_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), WATCHED_SLOT_HEIGHT),
            Sense::hover(),
        );
        let slot_clip = ui.clip_rect().intersect(slot_rect);
        let mut slot_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(slot_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        slot_ui.set_clip_rect(slot_clip);

        let card_frame = egui::Frame::new()
            .fill(COLOR_CARD)
            .corner_radius(RADIUS_MD)
            .inner_margin(Margin::symmetric(10, 3))
            .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));

        card_frame.show(&mut slot_ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(WATCHED_SLOT_HEIGHT - 6.0);

            // Header row
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(strings::WATCHED_PANEL_TITLE)
                        .color(COLOR_FOREGROUND)
                        .size(FONT_SIZE_SUBHEADING)
                        .strong(),
                );

                let active_count = self.state.actively_watched_count();
                let count_badge = egui::Frame::new()
                    .fill(if active_count > 0 {
                        COLOR_WARNING.gamma_multiply(0.2)
                    } else {
                        COLOR_MUTED
                    })
                    .corner_radius(RADIUS_SM)
                    .inner_margin(Margin::symmetric(6, 1))
                    .stroke(Stroke::new(
                        1.0,
                        if active_count > 0 {
                            COLOR_WARNING.gamma_multiply(0.5)
                        } else {
                            COLOR_BORDER_SUBTLE
                        },
                    ));

                count_badge.show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("{}", episodes.len()))
                            .color(if active_count > 0 {
                                COLOR_WARNING
                            } else {
                                COLOR_MUTED_FOREGROUND
                            })
                            .size(FONT_SIZE_TINY)
                            .strong(),
                    );
                });
            });

            ui.add_space(2.0);

            // Content area: fixed height (~22.0px), exactly 1 row visible.
            // Shows muted placeholder if 0 episodes; scrolls internally without growing if > 1 episodes.
            let content_height = 22.0;
            if episodes.is_empty() {
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), content_height),
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label(
                            RichText::new(strings::WATCHED_EMPTY_PLACEHOLDER)
                                .color(COLOR_MUTED_FOREGROUND)
                                .size(FONT_SIZE_CAPTION),
                        );
                    },
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("watched_panel_scroll")
                    .max_height(content_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for ep in &episodes {
                            self.render_watched_row(ui, ep, now);
                        }
                    });
            }
        });
    }

    /// Renders an individual watched process row in the watched panel.
    fn render_watched_row(&mut self, ui: &mut egui::Ui, ep: &Episode, now: Instant) {
        let now_ms = self
            .state
            .latest_snapshot
            .as_ref()
            .map(|s| s.timestamp_ms)
            .unwrap_or(0);
        let retention_secs = if self.demo { 20 } else { 120 };
        let row_frame = egui::Frame::new()
            .fill(COLOR_BG)
            .corner_radius(RADIUS_SM)
            .inner_margin(Margin::symmetric(6, 1))
            .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));

        row_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // Expand toggle for group episodes
                if ep.is_group() {
                    let group_key = ep.name.to_lowercase();
                    let is_expanded = self.state.expanded_groups.contains(&group_key);
                    let toggle_sym = if is_expanded { "▼" } else { "▶" };
                    if ui
                        .small_button(toggle_sym)
                        .on_hover_text("Bấm để mở rộng / thu gọn danh sách PID")
                        .clicked()
                    {
                        if is_expanded {
                            self.state.expanded_groups.remove(&group_key);
                        } else {
                            self.state.expanded_groups.insert(group_key);
                        }
                    }
                }

                // 1. State badge with geometric icon and text
                let cur_score = ep.score_history.last().copied().unwrap_or(ep.peak_score);
                widgets::watched_state_badge(
                    ui,
                    ep.state,
                    cur_score,
                    ep.cooldown_remaining_secs(now_ms, retention_secs),
                );
                ui.add_space(4.0);

                // 2. Name & PID (or group count)
                if ep.is_group() {
                    ui.label(
                        RichText::new(format!("{} ×{}", ep.name, ep.process_count))
                            .color(COLOR_FOREGROUND)
                            .size(FONT_SIZE_BODY)
                            .strong(),
                    );
                } else {
                    ui.label(
                        RichText::new(&ep.name)
                            .color(COLOR_FOREGROUND)
                            .size(FONT_SIZE_BODY)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!("({})", ep.pid))
                            .color(COLOR_MUTED_FOREGROUND)
                            .size(FONT_SIZE_CAPTION),
                    );
                }

                ui.add_space(6.0);

                // 3. Time info: "Từ HH:MM:SS (đã MM:SS)"
                ui.label(
                    RichText::new(format_episode_time_str(ep))
                        .color(COLOR_MUTED_FOREGROUND)
                        .size(FONT_SIZE_CAPTION),
                );

                ui.add_space(6.0);

                // 4. Peak stats: CPU / RAM / Disk (show only non-zero peaks labeled 'Đỉnh')
                let mut peaks = Vec::new();
                if ep.peak_cpu > 0.05 {
                    peaks.push(format!("CPU: {}", format_percent(ep.peak_cpu)));
                }
                if ep.peak_ram_bytes > 0 {
                    peaks.push(format!("RAM: {}", format_bytes(ep.peak_ram_bytes)));
                }
                if ep.peak_disk_bps > 0.05 {
                    peaks.push(format!("Đĩa: {}", format_bps(ep.peak_disk_bps)));
                }

                if !peaks.is_empty() {
                    let peak_str = format!("Đỉnh {}", peaks.join(" • "));
                    ui.label(
                        RichText::new(peak_str)
                            .color(COLOR_MUTED_FOREGROUND)
                            .size(FONT_SIZE_CAPTION),
                    );
                }

                // 5. Mini sparkline of score history
                ui.add_space(4.0);
                widgets::mini_sparkline(ui, &ep.score_history, 48.0, 16.0);

                // 6. Action button (right-aligned) respecting two-tier safety & per-process closing
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ep.is_group() {
                        let filter_btn = egui::Button::new(
                            RichText::new("Xem trong bảng")
                                .color(COLOR_FOREGROUND)
                                .size(FONT_SIZE_TINY),
                        )
                        .fill(COLOR_CARD)
                        .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));
                        let resp = ui.add(filter_btn);
                        if resp
                            .on_hover_text("Can thiệp từng tiến trình trong bảng bên dưới")
                            .clicked()
                        {
                            self.state.search_query = ep.name.clone();
                        }
                        return;
                    }

                    let active_proc =
                        self.state.latest_snapshot.as_ref().and_then(|s| {
                            s.processes.iter().find(|p| p.record.pid == ep.pid).cloned()
                        });

                    if let Some(target) = active_proc {
                        let pid = target.record.pid;
                        let killable = target.killable;

                        if !killable {
                            let prot_btn = egui::Button::new(
                                RichText::new(strings::BTN_GRACEFUL_CLOSE)
                                    .color(COLOR_MUTED_FOREGROUND)
                                    .size(FONT_SIZE_TINY),
                            )
                            .fill(COLOR_MUTED)
                            .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));
                            let resp = ui.add_enabled(false, prot_btn);
                            resp.on_hover_text(strings::TOOLTIP_SYSTEM_PROTECTED);
                        } else {
                            let can_kill = self.state.can_force_kill(pid, now);
                            let wait_rem = self.state.graceful_wait_remaining_secs(pid, now);

                            if can_kill {
                                let kill_btn = egui::Button::new(
                                    RichText::new(strings::BTN_FORCE_KILL)
                                        .color(COLOR_FOREGROUND)
                                        .size(FONT_SIZE_TINY)
                                        .strong(),
                                )
                                .fill(COLOR_DESTRUCTIVE)
                                .stroke(Stroke::NONE);

                                let resp = ui.add(kill_btn);
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                if resp.clicked() {
                                    self.confirm_kill = Some(target.clone());
                                }
                            } else if let Some(s) = wait_rem {
                                let wait_btn = egui::Button::new(
                                    RichText::new(format!("{} ({}s)", strings::BTN_FORCE_KILL, s))
                                        .color(COLOR_MUTED_FOREGROUND)
                                        .size(FONT_SIZE_TINY),
                                )
                                .fill(COLOR_MUTED)
                                .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));
                                let resp = ui.add_enabled(false, wait_btn);
                                resp.on_hover_text(format!(
                                    "{} {s} {}",
                                    strings::TOOLTIP_WAIT_GRACEFUL_PREFIX,
                                    strings::TOOLTIP_WAIT_GRACEFUL_SUFFIX
                                ));
                            } else {
                                let close_btn = egui::Button::new(
                                    RichText::new(strings::BTN_GRACEFUL_CLOSE)
                                        .color(COLOR_BG)
                                        .size(FONT_SIZE_TINY)
                                        .strong(),
                                )
                                .fill(COLOR_INFO)
                                .stroke(Stroke::NONE);

                                let resp = ui.add(close_btn);
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                if resp.clicked() {
                                    let res = self
                                        .actions
                                        .run(ActionCommand::GracefulClose(pid), &target);
                                    self.state.record_graceful_close(pid, now);
                                    match res {
                                        Ok(msg) => self.state.add_toast(
                                            msg,
                                            false,
                                            now,
                                            Duration::from_secs(4),
                                        ),
                                        Err(err) => self.state.add_toast(
                                            err,
                                            true,
                                            now,
                                            Duration::from_secs(5),
                                        ),
                                    }
                                }
                            }
                        }
                    } else {
                        ui.label(
                            RichText::new("—")
                                .color(COLOR_MUTED_FOREGROUND)
                                .size(FONT_SIZE_CAPTION),
                        );
                    }
                });
            });

            // Expandable member PIDs list for group episodes
            if ep.is_group() && self.state.expanded_groups.contains(&ep.name.to_lowercase()) {
                ui.add_space(3.0);
                let member_pids: Vec<u32> = self
                    .state
                    .latest_snapshot
                    .as_ref()
                    .map(|s| {
                        s.processes
                            .iter()
                            .filter(|p| p.record.name.eq_ignore_ascii_case(&ep.name))
                            .map(|p| p.record.pid)
                            .collect()
                    })
                    .unwrap_or_default();

                ui.horizontal_wrapped(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("Các PID thành viên:")
                            .color(COLOR_MUTED_FOREGROUND)
                            .size(FONT_SIZE_TINY),
                    );
                    for pid in member_pids {
                        let badge = egui::Frame::new()
                            .fill(COLOR_MUTED)
                            .corner_radius(RADIUS_SM)
                            .inner_margin(Margin::symmetric(4, 1))
                            .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE));
                        badge.show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("PID: {pid}"))
                                    .color(COLOR_FOREGROUND)
                                    .monospace()
                                    .size(FONT_SIZE_TINY),
                            );
                        });
                    }
                });
            }
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

            // Table body region computation (strictly below headers and search bar)
            let table_body_top = ui.cursor().top();
            let bottom_bar_height = if self.state.selected_pid.is_some() {
                42.0
            } else {
                0.0
            };
            let card_bottom_margin = 5.0;
            let table_body_bottom =
                (ui.max_rect().bottom() - bottom_bar_height - card_bottom_margin)
                    .max(table_body_top + 40.0);
            let table_body_height = table_body_bottom - table_body_top;
            let table_body_rect = egui::Rect::from_min_max(
                Pos2::new(ui.max_rect().left(), table_body_top),
                Pos2::new(ui.max_rect().right(), table_body_bottom),
            );

            // Bounded child UI with strictly enforced clipping matching the table body region.
            // Guarantees no scrolled row can ever paint over header, search bar, charts, or top banner.
            let body_clip = ui.clip_rect().intersect(table_body_rect);
            let (allocated_body_rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), table_body_height),
                Sense::hover(),
            );

            let mut body_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(allocated_body_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            body_ui.set_clip_rect(body_clip);

            egui::ScrollArea::vertical()
                .id_salt("process_table_scroll")
                .max_height(table_body_height)
                .auto_shrink([false, false])
                .show(&mut body_ui, |scroll_ui| {
                    scroll_ui.spacing_mut().item_spacing.y = 0.0;
                    let scroll_clip = scroll_ui.clip_rect().intersect(body_clip);
                    scroll_ui.set_clip_rect(scroll_clip);

                    let table_procs: Vec<ScoredProcess> =
                        self.state.table_processes().into_iter().cloned().collect();

                    let mut clicked_pid = None;

                    if table_procs.is_empty() {
                        table::render_empty_table_state(scroll_ui);
                    } else {
                        for (idx, proc) in table_procs.iter().enumerate() {
                            let pid = proc.record.pid;
                            let is_selected = self.state.selected_pid == Some(pid);
                            let is_watched = self.state.is_watched_pid(pid);
                            let group_count = self.state.is_group_culprit_member(&proc.record.name);

                            let row_resp = table::render_process_row(
                                scroll_ui,
                                proc,
                                &layout,
                                is_selected,
                                is_watched,
                                group_count,
                                idx,
                            );
                            if row_resp.clicked() {
                                clicked_pid = Some(if is_selected { None } else { Some(pid) });
                            }
                        }
                    }

                    if let Some(sel) = clicked_pid {
                        self.state.select_pid(sel);
                    }
                });

            // Selected Process Action Bar (anchored at bottom of table card)
            if self.state.selected_pid.is_some() {
                ui.add_space(2.0);
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
                    let ts = self
                        .state
                        .latest_snapshot
                        .as_ref()
                        .map(|s| s.timestamp_ms)
                        .unwrap_or(crate::collector::MockSource::BASE_TIMESTAMP_MS);
                    let (phase_name, rem_secs) = crate::collector::MockSource::demo_phase_at(ts);
                    let demo_str = format!("[DEMO] Giai đoạn: {} · còn {}s", phase_name, rem_secs);
                    ui.label(
                        RichText::new(demo_str)
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

    /// Verifies visible rows requirement: >= 8 rows at 1000x680 and >= 4 at 760x520 (Defect 1 targets).
    #[test]
    fn test_visible_rows_count_at_supported_resolutions() {
        use crate::collector::SampleSource;
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        let test_cases = [(1000.0f32, 680.0f32, 8usize), (760.0, 520.0, 4)];

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

    /// Verifies hero banner state evaluation and state consistency (Addendum B).
    #[test]
    fn test_hero_banner_state_mapping() {
        use crate::model::{
            Bottleneck, PressureState, ProcessKind, ProcessRecord, ScoredProcess, Severity,
        };
        use crate::ui::palette::{COLOR_ACCENT_OK, COLOR_WARNING};

        let sample_record = ProcessRecord {
            pid: 1234,
            parent_pid: None,
            name: "heavy.exe".to_string(),
            exe_path: None,
            cpu_percent: 85.0,
            memory_bytes: 4 * 1024 * 1024 * 1024,
            disk_read_bps: 10.0 * 1024.0 * 1024.0,
            disk_write_bps: 20.0 * 1024.0 * 1024.0,
        };
        let culprit = ScoredProcess {
            record: sample_record,
            kind: ProcessKind::UserApp,
            score: 95.0,
            severity: Severity::Culprit,
            main_bottleneck: Bottleneck::Cpu,
            killable: true,
        };

        // 1. Culprit present -> Always HeroBannerState::Culprit regardless of pressure
        assert_eq!(
            evaluate_hero_banner_state(Some(&culprit), PressureState::Overloaded, None),
            HeroBannerState::Culprit(culprit.clone())
        );
        assert_eq!(
            evaluate_hero_banner_state(Some(&culprit), PressureState::Strained, None),
            HeroBannerState::Culprit(culprit.clone())
        );
        assert_eq!(
            evaluate_hero_banner_state(Some(&culprit), PressureState::Smooth, None),
            HeroBannerState::Culprit(culprit.clone())
        );

        // 2. No culprit + Overloaded -> HighPressureNoCulprit with warning color & Overloaded message
        match evaluate_hero_banner_state(None, PressureState::Overloaded, None) {
            HeroBannerState::HighPressureNoCulprit {
                title,
                subtitle,
                color,
                ..
            } => {
                assert_eq!(title, strings::HERO_PRESSURE_OVERLOADED);
                assert_eq!(subtitle, strings::HERO_PRESSURE_NO_SINGLE_CULPRIT_SUB);
                assert_eq!(color, COLOR_WARNING);
            }
            other => panic!("Expected HighPressureNoCulprit, got {:?}", other),
        }

        // 3. No culprit + Strained -> HighPressureNoCulprit with warning color & Strained message
        match evaluate_hero_banner_state(None, PressureState::Strained, None) {
            HeroBannerState::HighPressureNoCulprit {
                title,
                subtitle,
                color,
                ..
            } => {
                assert_eq!(title, strings::HERO_PRESSURE_STRAINED);
                assert_eq!(subtitle, strings::HERO_PRESSURE_NO_SINGLE_CULPRIT_SUB);
                assert_eq!(color, COLOR_WARNING);
            }
            other => panic!("Expected HighPressureNoCulprit, got {:?}", other),
        }

        // 4. No culprit + Smooth -> SmoothHealthy with green OK color
        match evaluate_hero_banner_state(None, PressureState::Smooth, None) {
            HeroBannerState::SmoothHealthy {
                title,
                subtitle,
                color,
            } => {
                assert_eq!(title, strings::HERO_NO_CULPRIT);
                assert_eq!(subtitle, strings::HERO_NO_CULPRIT_SUB);
                assert_eq!(color, COLOR_ACCENT_OK);
            }
            other => panic!("Expected SmoothHealthy, got {:?}", other),
        }
    }

    /// Verifies UI render consistency between header badge and hero banner under high pressure (Addendum B).
    #[test]
    fn test_hero_banner_and_header_consistency_under_high_pressure() {
        use crate::model::{PressureState, SystemSnapshot, SystemTotals};
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        let (_tx, rx) = std::sync::mpsc::channel();
        let mut app = SlowSpotApp {
            rx,
            actions: Box::new(crate::ui::DemoActions),
            state: ViewState::new(),
            demo: true,
            confirm_kill: None,
            last_update_instant: None,
        };

        // Snapshot with Overloaded pressure but NO culprit
        let snap = SystemSnapshot {
            timestamp_ms: 1000,
            totals: SystemTotals {
                cpu_percent: 98.0,
                memory_used_bytes: 30 * 1024 * 1024 * 1024,
                memory_total_bytes: 32 * 1024 * 1024 * 1024,
                disk_bps: 250.0 * 1024.0 * 1024.0,
            },
            processes: vec![],
            pressure: PressureState::Overloaded,
            watched: Vec::new(),
            breakdown: None,
        };
        app.state.push_snapshot(snap, Instant::now());

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(1000.0, 680.0),
            )),
            ..Default::default()
        };

        let mut output = ctx.run_ui(raw_input, |ui| {
            app.render_ui(ui, Instant::now());
        });

        let mut rendered_texts = Vec::new();
        for shape in &output.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                rendered_texts.push(text_shape.galley.text().to_string());
            }
        }

        // Header badge shows 'Quá tải [resource]' (Addendum 1)
        assert!(
            rendered_texts.iter().any(|t| t.starts_with("Quá tải")),
            "Header badge must name overloaded resource(s)"
        );

        // Banner shows 'Hệ thống đang quá tải'
        assert!(
            rendered_texts
                .iter()
                .any(|t| t == strings::HERO_PRESSURE_OVERLOADED),
            "Banner must show 'Hệ thống đang quá tải' under high pressure"
        );

        // Banner must NEVER show 'Máy đang chạy mượt, không có thủ phạm'
        assert!(
            !rendered_texts.iter().any(|t| t == strings::HERO_NO_CULPRIT),
            "Banner must NOT show 'Máy đang chạy mượt' when pressure is Overloaded!"
        );
        output.textures_delta.clear();
    }

    /// Verifies that scrolling the process table never bleeds rows outside the table body (Addendum A).
    #[test]
    fn test_process_table_scrolled_clipping_bounds() {
        use crate::collector::SampleSource;
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

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

        let win_w = 1000.0;
        let win_h = 680.0;

        // Frame 1: Initial layout
        let raw_input_1 = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(win_w, win_h),
            )),
            ..Default::default()
        };
        let mut out1 = ctx.run_ui(raw_input_1, |ui| {
            app.render_ui(ui, Instant::now());
        });
        out1.textures_delta.clear();

        // Frame 2: Scroll downward so top rows scroll past the top of the table body
        // In egui, scroll area state can be set or scrolled with MouseWheel
        let raw_input_2 = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(win_w, win_h),
            )),
            events: vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -350.0),
                modifiers: Default::default(),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
            }],
            ..Default::default()
        };

        let mut out2 = ctx.run_ui(raw_input_2, |ui| {
            app.render_ui(ui, Instant::now());
        });

        // Locate header row bottom and status bar top
        let mut header_y = None;
        for shape in &out2.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                if text_shape.galley.text().to_uppercase() == strings::COL_NAME.to_uppercase() {
                    header_y = Some(text_shape.pos.y + 16.0);
                    break;
                }
            }
        }
        let header_bottom = header_y.expect("Header column name must be found");
        let status_bar_top = win_h - 32.0;

        // Every process table text shape must have its clip_rect strictly between header_bottom and status_bar_top
        let mut verified_rows = 0;
        for shape in &out2.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                let text = text_shape.galley.text();
                if text.contains(".exe") {
                    verified_rows += 1;
                    assert!(
                        shape.clip_rect.min.y >= header_bottom - 2.0,
                        "Row text '{}' clip min.y ({}) must be >= header_bottom ({}) to avoid painting over header/charts/banner",
                        text,
                        shape.clip_rect.min.y,
                        header_bottom
                    );
                    assert!(
                        shape.clip_rect.max.y <= status_bar_top + 2.0,
                        "Row text '{}' clip max.y ({}) must be <= status_bar_top ({}) to avoid painting over status bar",
                        text,
                        shape.clip_rect.max.y,
                        status_bar_top
                    );
                }
            }
        }
        assert!(
            verified_rows > 0,
            "Must have verified at least one row text shape"
        );
        out2.textures_delta.clear();

        // Frame 3: Scroll all the way to the bottom
        let raw_input_3 = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(win_w, win_h),
            )),
            events: vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -5000.0),
                modifiers: Default::default(),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
            }],
            ..Default::default()
        };
        let mut out3 = ctx.run_ui(raw_input_3, |ui| {
            app.render_ui(ui, Instant::now());
        });
        let mut verified_bottom = 0;
        for shape in &out3.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                let text = text_shape.galley.text();
                if text.contains(".exe") {
                    verified_bottom += 1;
                    assert!(
                        shape.clip_rect.min.y >= header_bottom - 2.0,
                        "At bottom scroll, row '{}' clip min.y ({}) must be >= header_bottom ({})",
                        text,
                        shape.clip_rect.min.y,
                        header_bottom
                    );
                    assert!(
                        shape.clip_rect.max.y <= status_bar_top + 2.0,
                        "At bottom scroll, row '{}' clip max.y ({}) must be <= status_bar_top ({})",
                        text,
                        shape.clip_rect.max.y,
                        status_bar_top
                    );
                }
            }
        }
        assert!(
            verified_bottom > 0,
            "Must have verified rows when scrolled to bottom"
        );
        out3.textures_delta.clear();

        // Frame 4: Scroll back all the way to the top
        let raw_input_4 = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(win_w, win_h),
            )),
            events: vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 5000.0),
                modifiers: Default::default(),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
            }],
            ..Default::default()
        };
        let mut out4 = ctx.run_ui(raw_input_4, |ui| {
            app.render_ui(ui, Instant::now());
        });
        let mut verified_top = 0;
        for shape in &out4.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                let text = text_shape.galley.text();
                if text.contains(".exe") {
                    verified_top += 1;
                    assert!(
                        shape.clip_rect.min.y >= header_bottom - 2.0,
                        "At top scroll, row '{}' clip min.y ({}) must be >= header_bottom ({})",
                        text,
                        shape.clip_rect.min.y,
                        header_bottom
                    );
                    assert!(
                        shape.clip_rect.max.y <= status_bar_top + 2.0,
                        "At top scroll, row '{}' clip max.y ({}) must be <= status_bar_top ({})",
                        text,
                        shape.clip_rect.max.y,
                        status_bar_top
                    );
                }
            }
        }
        assert!(
            verified_top > 0,
            "Must have verified rows when scrolled back to top"
        );
        out4.textures_delta.clear();
    }

    #[test]
    fn test_group_culprit_banner_and_table_tag() {
        let ctx = egui::Context::default();
        let (_tx, rx) = std::sync::mpsc::channel();
        let mut app = SlowSpotApp {
            rx,
            actions: Box::new(crate::ui::DemoActions),
            state: ViewState::new(),
            demo: true,
            confirm_kill: None,
            last_update_instant: None,
        };

        // Create 3 rustc.exe processes (24%, 21%, 7% -> 52%)
        let p1 = ScoredProcess {
            record: crate::model::ProcessRecord {
                pid: 101,
                parent_pid: None,
                name: "rustc.exe".to_string(),
                exe_path: None,
                cpu_percent: 24.0,
                memory_bytes: 1024 * 1024 * 500,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            kind: crate::model::ProcessKind::UserApp,
            score: 24.0,
            severity: crate::model::Severity::Normal,
            main_bottleneck: crate::model::Bottleneck::Cpu,
            killable: true,
        };
        let p2 = ScoredProcess {
            record: crate::model::ProcessRecord {
                pid: 102,
                parent_pid: None,
                name: "rustc.exe".to_string(),
                exe_path: None,
                cpu_percent: 21.0,
                memory_bytes: 1024 * 1024 * 500,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            kind: crate::model::ProcessKind::UserApp,
            score: 21.0,
            severity: crate::model::Severity::Normal,
            main_bottleneck: crate::model::Bottleneck::Cpu,
            killable: true,
        };
        let p3 = ScoredProcess {
            record: crate::model::ProcessRecord {
                pid: 103,
                parent_pid: None,
                name: "rustc.exe".to_string(),
                exe_path: None,
                cpu_percent: 7.0,
                memory_bytes: 1024 * 1024 * 500,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            kind: crate::model::ProcessKind::UserApp,
            score: 7.0,
            severity: crate::model::Severity::Normal,
            main_bottleneck: crate::model::Bottleneck::Cpu,
            killable: true,
        };

        let group_share = crate::model::ProcessGroupShare {
            name: "rustc.exe".to_string(),
            process_count: 3,
            total_metric: 52.0,
            share_percent: 52.0,
            pids: vec![101, 102, 103],
        };

        let bd = crate::model::PressureBreakdown {
            resource: crate::model::OverloadedResource::Cpu,
            system_value: 99.7,
            resource_unit: "%",
            top_groups: vec![group_share.clone()],
            remainder_share_percent: 48.0,
            group_culprit: Some(group_share),
        };

        let snap = SystemSnapshot {
            timestamp_ms: 1000,
            totals: crate::model::SystemTotals {
                cpu_percent: 99.7,
                memory_used_bytes: 8 * 1024 * 1024 * 1024,
                memory_total_bytes: 16 * 1024 * 1024 * 1024,
                disk_bps: 0.0,
            },
            processes: vec![p1, p2, p3],
            pressure: crate::model::PressureState::Overloaded,
            watched: Vec::new(),
            breakdown: Some(bd),
        };

        app.state.push_snapshot(snap, Instant::now());

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(1000.0, 680.0),
            )),
            ..Default::default()
        };

        let mut output = ctx.run_ui(raw_input, |ui| {
            app.render_ui(ui, Instant::now());
        });

        let mut rendered_texts = Vec::new();
        for shape in &output.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                rendered_texts.push(text_shape.galley.text().to_string());
            }
        }

        // 1. Header pill names the overloaded resource: 'Quá tải CPU'
        assert!(
            rendered_texts.iter().any(|t| t == "Quá tải CPU"),
            "Header status pill must be 'Quá tải CPU', got: {:?}",
            rendered_texts
        );

        // 2. Banner names the group culprit
        assert!(
            rendered_texts
                .iter()
                .any(|t| t.contains("Nhóm thủ phạm: rustc.exe (3 tiến trình)")),
            "Banner must name group culprit rustc.exe with 3 processes"
        );

        // 3. Banner subtitle shows specific resource breakdown
        assert!(
            rendered_texts
                .iter()
                .any(|t| t.contains("CPU đang 99.7% — rustc.exe (3 tiến trình) chiếm 52%")),
            "Banner subtitle must show detailed breakdown string"
        );

        // 4. Table renders group member tags '×3'
        assert!(
            rendered_texts.iter().any(|t| t == "×3"),
            "Table must render group badge '×3' for member rows"
        );

        output.textures_delta.clear();
    }

    /// Verifies that when a process is Suspicious, the header status badge shows
    /// 'Có dấu hiệu chậm' (amber) and does NOT show green 'Mượt mà' (Defect 1).
    #[test]
    fn test_hero_banner_and_header_badge_suspicious_level() {
        use crate::model::{
            Bottleneck, PressureState, ProcessKind, ProcessRecord, Severity, SystemTotals,
        };
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        let (_tx, rx) = std::sync::mpsc::channel();
        let mut app = SlowSpotApp {
            rx,
            actions: Box::new(crate::ui::DemoActions),
            state: ViewState::new(),
            demo: false,
            confirm_kill: None,
            last_update_instant: None,
        };

        // Snapshot with Smooth pressure but 1 suspicious process
        let proc = ScoredProcess {
            record: ProcessRecord {
                pid: 2024,
                parent_pid: None,
                name: "indexer.exe".to_string(),
                exe_path: None,
                cpu_percent: 45.0,
                memory_bytes: 1024 * 1024 * 500,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            kind: ProcessKind::UserApp,
            score: 55.0,
            severity: Severity::Suspicious,
            main_bottleneck: Bottleneck::Cpu,
            killable: true,
        };

        let snap = SystemSnapshot {
            timestamp_ms: 1000,
            totals: SystemTotals {
                cpu_percent: 48.0,
                memory_used_bytes: 4 * 1024 * 1024 * 1024,
                memory_total_bytes: 16 * 1024 * 1024 * 1024,
                disk_bps: 0.0,
            },
            processes: vec![proc],
            pressure: PressureState::Smooth,
            watched: Vec::new(),
            breakdown: None,
        };

        app.state.push_snapshot(snap, Instant::now());

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(1000.0, 680.0),
            )),
            ..Default::default()
        };

        let mut output = ctx.run_ui(raw_input, |ui| {
            app.render_ui(ui, Instant::now());
        });

        let mut rendered_texts = Vec::new();
        for shape in &output.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                rendered_texts.push(text_shape.galley.text().to_string());
            }
        }

        // Main card shows "ĐÁNG NGỜ"
        assert!(
            rendered_texts
                .iter()
                .any(|t| t == strings::HERO_TITLE_SUSPICIOUS),
            "Card must display 'ĐÁNG NGỜ' for suspicious process"
        );

        // Header badge shows 'Có dấu hiệu chậm'
        assert!(
            rendered_texts
                .iter()
                .any(|t| t == strings::STATUS_SUSPICIOUS),
            "Header badge must show 'Có dấu hiệu chậm' when card is Suspicious, got: {:?}",
            rendered_texts
        );

        // Header badge must NOT show 'Mượt mà'
        assert!(
            !rendered_texts.iter().any(|t| t == strings::PRESSURE_SMOOTH),
            "Header badge must NOT show 'Mượt mà' when card is Suspicious"
        );

        output.textures_delta.clear();
    }

    /// Verifies that during Cooling phase at min width 760, header left cluster and right pills do not overlap (Defect 2).
    #[test]
    fn test_cooling_header_layout_no_overlap_at_760() {
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        let now = Instant::now();

        // Test both with 0 watched processes and with >= 1 watched processes
        for has_watched in [false, true] {
            let (_tx, rx) = std::sync::mpsc::channel();
            let mut app = SlowSpotApp {
                rx,
                actions: Box::new(crate::ui::DemoActions),
                state: ViewState::new(),
                demo: true,
                confirm_kill: None,
                last_update_instant: None,
            };

            // Trigger cooldown mode
            app.state.cooldown_until = Some(now + Duration::from_secs(10));
            assert!(app.state.is_cooldown_active(now));

            let watched = if has_watched {
                vec![crate::model::Episode {
                    pid: 1234,
                    name: "heavy.exe".to_string(),
                    process_count: 1,
                    started_at: 1000,
                    peak_score: 85.0,
                    peak_cpu: 85.0,
                    peak_ram_bytes: 1024 * 1024 * 1024,
                    peak_disk_bps: 100.0,
                    overloaded_seconds: 10,
                    last_overloaded_at: 1000,
                    state: crate::model::EpisodeState::Cooling,
                    kind: crate::model::ProcessKind::UserApp,
                    killable: true,
                    score_history: vec![85.0],
                }]
            } else {
                Vec::new()
            };

            let snap = SystemSnapshot {
                timestamp_ms: 1000,
                totals: crate::model::SystemTotals {
                    cpu_percent: 25.0,
                    memory_used_bytes: 4 * 1024 * 1024 * 1024,
                    memory_total_bytes: 16 * 1024 * 1024 * 1024,
                    disk_bps: 100.0,
                },
                processes: vec![],
                pressure: crate::model::PressureState::Smooth,
                watched,
                breakdown: None,
            };
            app.state.push_snapshot(snap, now);

            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(760.0, 520.0),
                )),
                ..Default::default()
            };

            let mut output = ctx.run_ui(raw_input, |ui| {
                app.render_ui(ui, now);
            });

            // Locate CPU label min_x and rightmost x coordinate of left-cluster header texts
            let mut cpu_label_x = None;
            let mut left_cluster_max_x = 0.0f32;
            let mut has_watched_badge_in_header = false;

            for shape in &output.shapes {
                if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                    let text = text_shape.galley.text();
                    let text_rect =
                        egui::Rect::from_min_size(text_shape.pos, text_shape.galley.size());

                    // Header elements in top region (y < 60)
                    if text_rect.min.y < 60.0 {
                        if text == "CPU" {
                            cpu_label_x = Some(text_rect.min.x);
                        } else if text.starts_with(strings::BADGE_WATCHING_HEADER_PREFIX) {
                            has_watched_badge_in_header = true;
                        } else if text != "RAM"
                            && text != "Đĩa"
                            && !text.contains('%')
                            && !text.contains("B/s")
                            && text != strings::BTN_PAUSE
                            && text != strings::BTN_RESUME
                        {
                            left_cluster_max_x = left_cluster_max_x.max(text_rect.max.x);
                        }
                    }
                }
            }

            assert!(
                !has_watched_badge_in_header,
                "Redundant 'Theo dõi: N' chip must NOT be rendered in the header banner"
            );

            let cpu_x = cpu_label_x.expect("CPU chip label must be rendered in header");
            assert!(
                left_cluster_max_x < cpu_x,
                "Left header cluster text (max x: {left_cluster_max_x}) must NOT overlap or touch CPU pill (min x: {cpu_x}) when has_watched={has_watched}"
            );

            output.textures_delta.clear();
        }
    }

    /// Verifies that a single-process group does NOT show a '×1' tag in the table row (Defect 4).
    #[test]
    fn test_single_process_group_does_not_render_tag() {
        use crate::model::{
            Bottleneck, PressureState, ProcessGroupShare, ProcessKind, ProcessRecord, Severity,
            SystemTotals,
        };
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        let (_tx, rx) = std::sync::mpsc::channel();
        let mut app = SlowSpotApp {
            rx,
            actions: Box::new(crate::ui::DemoActions),
            state: ViewState::new(),
            demo: false,
            confirm_kill: None,
            last_update_instant: None,
        };

        let proc = ScoredProcess {
            record: ProcessRecord {
                pid: 5001,
                parent_pid: None,
                name: "heavy_single.exe".to_string(),
                exe_path: None,
                cpu_percent: 85.0,
                memory_bytes: 1024 * 1024 * 500,
                disk_read_bps: 0.0,
                disk_write_bps: 0.0,
            },
            kind: ProcessKind::UserApp,
            score: 85.0,
            severity: Severity::Culprit,
            main_bottleneck: Bottleneck::Cpu,
            killable: true,
        };

        // Breakdown with 1 process
        let group_share = ProcessGroupShare {
            name: "heavy_single.exe".to_string(),
            process_count: 1,
            total_metric: 85.0,
            share_percent: 85.0,
            pids: vec![5001],
        };

        let bd = crate::model::PressureBreakdown {
            resource: crate::model::OverloadedResource::Cpu,
            system_value: 90.0,
            resource_unit: "%",
            top_groups: vec![group_share.clone()],
            remainder_share_percent: 15.0,
            group_culprit: Some(group_share),
        };

        let snap = SystemSnapshot {
            timestamp_ms: 1000,
            totals: SystemTotals {
                cpu_percent: 90.0,
                memory_used_bytes: 4 * 1024 * 1024 * 1024,
                memory_total_bytes: 16 * 1024 * 1024 * 1024,
                disk_bps: 0.0,
            },
            processes: vec![proc],
            pressure: PressureState::Overloaded,
            watched: Vec::new(),
            breakdown: Some(bd),
        };

        app.state.push_snapshot(snap, Instant::now());

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(1000.0, 680.0),
            )),
            ..Default::default()
        };

        let mut output = ctx.run_ui(raw_input, |ui| {
            app.render_ui(ui, Instant::now());
        });

        let mut rendered_texts = Vec::new();
        for shape in &output.shapes {
            if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                rendered_texts.push(text_shape.galley.text().to_string());
            }
        }

        // Table must NOT render group tag '×1'
        assert!(
            !rendered_texts.iter().any(|t| t == "×1"),
            "Table must NOT render group badge '×1' for single-process group"
        );

        output.textures_delta.clear();
    }

    /// Verifies that the fixed slot heights (Watched and Status Area) remain constant
    /// across varying states (Empty/Populated/Scrolling watchlist, Smooth/Culprit/HighPressure),
    /// guaranteeing that Sparklines and Table anchors never move (Addendum requirements 1, 2, 4).
    #[test]
    fn test_layout_slots_constant_height_across_states() {
        use crate::model::*;
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        let win_w = 1000.0f32;
        let win_h = 680.0f32;

        let base_totals = SystemTotals {
            cpu_percent: 20.0,
            memory_used_bytes: 4 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_bps: 1024.0 * 1024.0,
        };

        // State 1: SmoothHealthy, 0 watched items (placeholder shown)
        let snap1 = SystemSnapshot {
            timestamp_ms: 1000,
            totals: base_totals.clone(),
            processes: vec![],
            pressure: PressureState::Smooth,
            watched: vec![],
            breakdown: None,
        };

        // State 2: Culprit state with active killable culprit, 0 watched items
        let culprit_proc = ScoredProcess {
            record: ProcessRecord {
                pid: 101,
                parent_pid: None,
                name: "heavy_task.exe".to_string(),
                exe_path: None,
                cpu_percent: 85.0,
                memory_bytes: 2 * 1024 * 1024 * 1024,
                disk_read_bps: 1024.0,
                disk_write_bps: 1024.0,
            },
            kind: ProcessKind::UserApp,
            score: 95.0,
            severity: Severity::Culprit,
            main_bottleneck: Bottleneck::Cpu,
            killable: true,
        };
        let snap2 = SystemSnapshot {
            timestamp_ms: 2000,
            totals: SystemTotals {
                cpu_percent: 85.0,
                ..base_totals.clone()
            },
            processes: vec![culprit_proc.clone()],
            pressure: PressureState::Overloaded,
            watched: vec![],
            breakdown: None,
        };

        // State 3: High pressure without single culprit, with 1 watched episode
        let ep1 = Episode {
            pid: 201,
            name: "watched_service.exe".to_string(),
            process_count: 1,
            started_at: 1000,
            peak_score: 80.0,
            peak_cpu: 75.0,
            peak_ram_bytes: 1024 * 1024 * 1024,
            peak_disk_bps: 5.0 * 1024.0 * 1024.0,
            overloaded_seconds: 10,
            last_overloaded_at: 2000,
            state: EpisodeState::Cooling,
            kind: ProcessKind::BackgroundService,
            killable: true,
            score_history: vec![80.0, 60.0, 40.0],
        };
        let snap3 = SystemSnapshot {
            timestamp_ms: 3000,
            totals: SystemTotals {
                cpu_percent: 92.0,
                ..base_totals.clone()
            },
            processes: vec![],
            pressure: PressureState::Overloaded,
            watched: vec![ep1],
            breakdown: None,
        };

        // State 4: Culprit with 5 watched episodes (exceeding 2 rows, triggers scroll clipping)
        let mut watched_many = Vec::new();
        for i in 1..=5 {
            watched_many.push(Episode {
                pid: 300 + i,
                name: format!("daemon_{i}.exe"),
                process_count: 1,
                started_at: 1000,
                peak_score: 75.0,
                peak_cpu: 40.0,
                peak_ram_bytes: 512 * 1024 * 1024,
                peak_disk_bps: 0.0,
                overloaded_seconds: 15,
                last_overloaded_at: 2500,
                state: EpisodeState::Overloaded,
                kind: ProcessKind::BackgroundService,
                killable: true,
                score_history: vec![75.0],
            });
        }
        let snap4 = SystemSnapshot {
            timestamp_ms: 4000,
            totals: SystemTotals {
                cpu_percent: 88.0,
                ..base_totals
            },
            processes: vec![culprit_proc],
            pressure: PressureState::Overloaded,
            watched: watched_many,
            breakdown: None,
        };

        let snapshots = vec![snap1, snap2, snap3, snap4];
        let mut sparkline_anchors = Vec::new();
        let mut table_anchors = Vec::new();

        for (idx, snap) in snapshots.into_iter().enumerate() {
            let (_tx, rx) = std::sync::mpsc::channel();
            let mut app = SlowSpotApp {
                rx,
                actions: Box::new(crate::ui::DemoActions),
                state: ViewState::new(),
                demo: false,
                confirm_kill: None,
                last_update_instant: None,
            };
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

            let mut spark_y = None;
            let mut table_y = None;

            for shape in &output.shapes {
                if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                    let text = text_shape.galley.text();
                    if text == strings::SPARKLINE_CPU_TITLE {
                        spark_y = Some(text_shape.pos.y);
                    } else if text == strings::TABLE_TITLE {
                        table_y = Some(text_shape.pos.y);
                    }
                }
            }

            output.textures_delta.clear();

            let s_y = spark_y.unwrap_or_else(|| panic!("State {idx}: Sparklines title not found"));
            let t_y = table_y.unwrap_or_else(|| panic!("State {idx}: Table title not found"));

            sparkline_anchors.push(s_y);
            table_anchors.push(t_y);
        }

        // Verify that across ALL states, Sparklines and Table Y positions are completely constant!
        for (i, (&s_y, &t_y)) in sparkline_anchors.iter().zip(&table_anchors).enumerate() {
            assert!(
                (s_y - sparkline_anchors[0]).abs() < 0.001,
                "Sparklines Y anchor shifted in state {i}: {} vs initial {}",
                s_y,
                sparkline_anchors[0]
            );
            assert!(
                (t_y - table_anchors[0]).abs() < 0.001,
                "Table Y anchor shifted in state {i}: {} vs initial {}",
                t_y,
                table_anchors[0]
            );
        }

        // Verify constant slot height constants
        assert_eq!(WATCHED_SLOT_HEIGHT, 50.0);
        assert_eq!(STATUS_SLOT_HEIGHT, 104.0);
    }

    /// Regression test for Defect 1 & Defect 4:
    /// Computes the required height of the culprit card content and asserts STATUS_SLOT_HEIGHT >= that value.
    /// Also asserts every button is fully visible and clickable without clipping in both Suspicious and Culprit states
    /// at both 1000x680 and 760x520.
    #[test]
    fn test_culprit_card_content_height_fits_in_status_slot() {
        use crate::model::{
            Bottleneck, PressureState, ProcessKind, ProcessRecord, Severity, SystemTotals,
        };
        let ctx = egui::Context::default();
        crate::ui::theme::install(&ctx);

        let test_cases = [
            // (severity, score, is_killable, window_size)
            (
                Severity::Culprit,
                92.0,
                true,
                egui::Vec2::new(1000.0, 680.0),
            ),
            (Severity::Culprit, 92.0, true, egui::Vec2::new(760.0, 520.0)),
            (
                Severity::Suspicious,
                55.0,
                true,
                egui::Vec2::new(1000.0, 680.0),
            ),
            (
                Severity::Suspicious,
                55.0,
                true,
                egui::Vec2::new(760.0, 520.0),
            ),
            (
                Severity::Culprit,
                95.0,
                false,
                egui::Vec2::new(760.0, 520.0),
            ),
        ];

        for (severity, score, killable, win_size) in test_cases {
            let (_tx, rx) = std::sync::mpsc::channel();
            let mut app = SlowSpotApp {
                rx,
                actions: Box::new(crate::ui::DemoActions),
                state: ViewState::new(),
                demo: false,
                confirm_kill: None,
                last_update_instant: None,
            };

            let culprit = ScoredProcess {
                record: ProcessRecord {
                    pid: 1420,
                    parent_pid: None,
                    name: "heavy_task.exe".to_string(),
                    exe_path: None,
                    cpu_percent: 88.0,
                    memory_bytes: 1024 * 1024 * 1024,
                    disk_read_bps: 100.0,
                    disk_write_bps: 100.0,
                },
                kind: if killable {
                    ProcessKind::UserApp
                } else {
                    ProcessKind::SystemCritical
                },
                score,
                main_bottleneck: Bottleneck::Cpu,
                severity,
                killable,
            };

            let snap = SystemSnapshot {
                timestamp_ms: 1000,
                totals: SystemTotals {
                    cpu_percent: 90.0,
                    memory_used_bytes: 8 * 1024 * 1024 * 1024,
                    memory_total_bytes: 16 * 1024 * 1024 * 1024,
                    disk_bps: 1000.0,
                },
                processes: vec![culprit.clone()],
                pressure: PressureState::Overloaded,
                watched: Vec::new(),
                breakdown: None,
            };
            app.state.push_snapshot(snap, Instant::now());

            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, win_size)),
                ..Default::default()
            };

            let mut output = ctx.run_ui(raw_input, |ui| {
                app.render_ui(ui, Instant::now());
            });

            // Find all shapes belonging to the culprit card:
            let mut card_min_y = f32::MAX;
            let mut card_max_y = f32::MIN;
            let mut found_buttons = Vec::new();
            let mut found_note = false;

            let is_culprit_element = |t: &str| -> bool {
                t == level_tokens(severity).title
                    || t == "Thủ phạm"
                    || t == "Đáng ngờ"
                    || t == "Hệ thống"
                    || t.starts_with(strings::HERO_SCORE_LABEL)
                    || t == "heavy_task.exe"
                    || t.starts_with("(PID ")
                    || t.starts_with("— ")
                    || t == strings::BTN_GRACEFUL_CLOSE
                    || t == strings::BTN_FORCE_KILL
                    || (t.starts_with(strings::BTN_FORCE_KILL) && t.ends_with(')'))
                    || t == strings::BTN_OPEN_LOCATION
                    || t == strings::BTN_IGNORE_CULPRIT
                    || t == strings::TOOLTIP_SYSTEM_PROTECTED
                    || t == strings::NOTICE_FORCE_KILL_DISABLED_DEFAULT
                    || t.starts_with(strings::TOOLTIP_WAIT_GRACEFUL_PREFIX)
            };

            let slot_start_y = 118.0;
            let slot_end_y = slot_start_y + STATUS_SLOT_HEIGHT;

            for shape in &output.shapes {
                if let egui::epaint::Shape::Text(text_shape) = &shape.shape {
                    let text = text_shape.galley.text();
                    let pos = text_shape.pos;
                    let size = text_shape.galley.size();

                    // Elements inside the status card slot
                    if pos.y >= slot_start_y
                        && (pos.y + size.y) <= slot_end_y + 10.0
                        && is_culprit_element(text)
                    {
                        card_min_y = card_min_y.min(pos.y);
                        card_max_y = card_max_y.max(pos.y + size.y);

                        let is_force_kill_btn = text == strings::BTN_FORCE_KILL
                            || (text.starts_with(strings::BTN_FORCE_KILL) && text.ends_with(')'));

                        if text == strings::BTN_GRACEFUL_CLOSE
                            || is_force_kill_btn
                            || text == strings::BTN_OPEN_LOCATION
                            || text == strings::BTN_IGNORE_CULPRIT
                        {
                            found_buttons.push((text.to_string(), pos, size));
                        } else if text == strings::TOOLTIP_SYSTEM_PROTECTED
                            || text == strings::NOTICE_FORCE_KILL_DISABLED_DEFAULT
                            || text.starts_with(strings::TOOLTIP_WAIT_GRACEFUL_PREFIX)
                        {
                            found_note = true;
                        }
                    }
                }
            }

            output.textures_delta.clear();

            // Assert that all 4 action buttons were rendered
            assert_eq!(
                found_buttons.len(),
                4,
                "All 4 buttons must be rendered in culprit card at {:?}, found: {:?}",
                win_size,
                found_buttons
            );

            // Assert that the safety note was rendered when force-kill is not yet active
            assert!(
                found_note,
                "Safety note must be rendered in culprit card at {:?}",
                win_size
            );

            // Frame inner margin is 4px top + 4px bottom = 8px
            let content_span = card_max_y - card_min_y;
            let required_height = content_span + 8.0;

            assert!(
                STATUS_SLOT_HEIGHT >= required_height,
                "STATUS_SLOT_HEIGHT ({STATUS_SLOT_HEIGHT}) must be >= required content height ({required_height}) for {:?} at {:?}",
                severity,
                win_size
            );

            // Ensure every button's bottom edge is comfortably within the card span
            for (btn_name, btn_pos, btn_size) in found_buttons {
                let btn_bottom = btn_pos.y + btn_size.y;
                assert!(
                    btn_bottom <= card_max_y,
                    "Button '{btn_name}' bottom ({btn_bottom}) must not exceed card bottom ({card_max_y})"
                );
            }
        }
    }
}
