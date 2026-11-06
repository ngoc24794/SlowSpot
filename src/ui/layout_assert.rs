//! Machine-checkable UI layout assertions.
//!
//! Validates structural layout stability, non-overlapping text elements,
//! button containment within designated slots, and table row density
//! without requiring physical pixel rendering or screen captures.

use crate::collector::SampleSource;
use eframe::egui::{Rect, Vec2};

/// Snapshot of bounding rectangles for key structural elements and widgets in a rendered frame.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSnapshot {
    /// Total viewport/window dimension for this frame.
    pub window_size: Vec2,
    /// Bounding rectangle of the top header card.
    pub header_card_rect: Rect,
    /// Named bounding rectangles of widgets positioned in the header card.
    pub header_elements: Vec<(&'static str, Rect)>,
    /// Bounding rectangle of the fixed "Đang theo dõi" (Watched) panel slot.
    pub watched_slot_rect: Rect,
    /// Bounding rectangle of the fixed status hero/calm card slot.
    pub status_slot_rect: Rect,
    /// Named bounding rectangles of action buttons rendered within the status slot.
    pub action_buttons: Vec<(&'static str, Rect)>,
    /// Bounding rectangle of the real-time sparkline charts card.
    pub charts_rect: Rect,
    /// Bounding rectangle of the process table card.
    pub table_card_rect: Rect,
    /// Bounding rectangle of the interactive table body (rows viewport).
    pub table_body_rect: Rect,
    /// Number of completely visible process table rows.
    pub table_visible_rows: usize,
}

impl LayoutSnapshot {
    /// Validates internal containment and non-overlap constraints for this individual frame snapshot.
    pub fn assert_valid(&self, phase_name: &str, expected_min_rows: usize) -> Result<(), String> {
        // 1. No header widget extends outside the header card
        for (name, rect) in &self.header_elements {
            if rect.min.x < self.header_card_rect.min.x - 1.5
                || rect.max.x > self.header_card_rect.max.x + 1.5
                || rect.min.y < self.header_card_rect.min.y - 1.5
                || rect.max.y > self.header_card_rect.max.y + 1.5
            {
                return Err(format!(
                    "In phase '{phase_name}': header widget '{name}' rect ({rect:?}) extends outside header card ({:?})",
                    self.header_card_rect
                ));
            }
        }

        // 2. No text/element rect in the header overlaps another text/element rect
        for i in 0..self.header_elements.len() {
            for j in (i + 1)..self.header_elements.len() {
                let (name1, r1) = self.header_elements[i];
                let (name2, r2) = self.header_elements[j];

                let inter = r1.intersect(r2);
                if inter.is_positive() && inter.width() > 1.0 && inter.height() > 1.0 {
                    return Err(format!(
                        "In phase '{phase_name}': header element '{name1}' ({r1:?}) overlaps with '{name2}' ({r2:?}) by {:.1}px horizontally and {:.1}px vertically!",
                        inter.width(),
                        inter.height()
                    ));
                }
            }
        }

        // 3. Every action button is fully inside the status slot
        for (name, rect) in &self.action_buttons {
            if rect.min.x < self.status_slot_rect.min.x - 1.5
                || rect.max.x > self.status_slot_rect.max.x + 1.5
                || rect.min.y < self.status_slot_rect.min.y - 1.5
                || rect.max.y > self.status_slot_rect.max.y + 1.5
            {
                return Err(format!(
                    "In phase '{phase_name}': action button '{name}' rect ({rect:?}) extends outside status slot ({:?})",
                    self.status_slot_rect
                ));
            }
        }

        // 4. Table body is inside the table card
        if self.table_body_rect.min.y < self.table_card_rect.min.y - 1.5
            || self.table_body_rect.max.y > self.table_card_rect.max.y + 1.5
        {
            return Err(format!(
                "In phase '{phase_name}': table body rect ({:?}) extends outside table card ({:?})",
                self.table_body_rect, self.table_card_rect
            ));
        }

        // 5. Table visible rows count meets required threshold
        if self.table_visible_rows < expected_min_rows {
            return Err(format!(
                "In phase '{phase_name}' at {:.0}x{:.0}: table visible rows ({}) is less than required minimum ({})!",
                self.window_size.x, self.window_size.y, self.table_visible_rows, expected_min_rows
            ));
        }

        // 6. Vertical gaps between consecutive sections must equal SECTION_GAP (8.0px)
        let gap_header_watched = self.watched_slot_rect.min.y - self.header_card_rect.max.y;
        if (gap_header_watched - crate::ui::app::SECTION_GAP).abs() > 0.5 {
            return Err(format!(
                "In phase '{phase_name}': vertical gap between header and watched card is {:.1}px, expected {:.1}px",
                gap_header_watched, crate::ui::app::SECTION_GAP
            ));
        }

        let gap_watched_status = self.status_slot_rect.min.y - self.watched_slot_rect.max.y;
        if (gap_watched_status - crate::ui::app::SECTION_GAP).abs() > 0.5 {
            return Err(format!(
                "In phase '{phase_name}': vertical gap between watched and status slot is {:.1}px, expected {:.1}px",
                gap_watched_status, crate::ui::app::SECTION_GAP
            ));
        }

        let gap_status_charts = self.charts_rect.min.y - self.status_slot_rect.max.y;
        if (gap_status_charts - crate::ui::app::SECTION_GAP).abs() > 0.5 {
            return Err(format!(
                "In phase '{phase_name}': vertical gap between status and charts is {:.1}px, expected {:.1}px",
                gap_status_charts, crate::ui::app::SECTION_GAP
            ));
        }

        let gap_charts_table = self.table_card_rect.min.y - self.charts_rect.max.y;
        if (gap_charts_table - crate::ui::app::SECTION_GAP).abs() > 0.5 {
            return Err(format!(
                "In phase '{phase_name}': vertical gap between charts and table card is {:.1}px, expected {:.1}px",
                gap_charts_table, crate::ui::app::SECTION_GAP
            ));
        }

        Ok(())
    }
}

/// Asserts that element rects of header, watched, status, charts, and table are IDENTICAL
/// across all operational phases (layout stability).
pub fn assert_layout_stability(phases: &[(&str, &LayoutSnapshot)]) -> Result<(), String> {
    if phases.len() < 2 {
        return Ok(());
    }

    let (base_name, base) = phases[0];

    for &(other_name, other) in &phases[1..] {
        if base.header_card_rect != other.header_card_rect {
            return Err(format!(
                "Layout instability: header_card_rect changed between {base_name} ({:?}) and {other_name} ({:?})",
                base.header_card_rect, other.header_card_rect
            ));
        }
        if base.watched_slot_rect != other.watched_slot_rect {
            return Err(format!(
                "Layout instability: watched_slot_rect changed between {base_name} ({:?}) and {other_name} ({:?})",
                base.watched_slot_rect, other.watched_slot_rect
            ));
        }
        if base.status_slot_rect != other.status_slot_rect {
            return Err(format!(
                "Layout instability: status_slot_rect changed between {base_name} ({:?}) and {other_name} ({:?})",
                base.status_slot_rect, other.status_slot_rect
            ));
        }
        if base.charts_rect != other.charts_rect {
            return Err(format!(
                "Layout instability: charts_rect changed between {base_name} ({:?}) and {other_name} ({:?})",
                base.charts_rect, other.charts_rect
            ));
        }
        if base.table_card_rect != other.table_card_rect {
            return Err(format!(
                "Layout instability: table_card_rect changed between {base_name} ({:?}) and {other_name} ({:?})",
                base.table_card_rect, other.table_card_rect
            ));
        }
    }

    Ok(())
}

/// Runs the layout assertion suite for a specific viewport size, stepping through
/// all 4 demo phases (Calm, Suspicious, Culprit, Cooling) and verifying layout stability,
/// containment, non-overlap, button positions, and table row counts.
pub fn verify_layout_for_size(
    width: f32,
    height: f32,
    expected_min_rows: usize,
) -> Result<(), String> {
    let ctx = eframe::egui::Context::default();
    crate::ui::theme::install(&ctx);

    let (_tx, rx) = std::sync::mpsc::channel();
    let mut app =
        crate::ui::SlowSpotApp::for_testing(rx, Box::new(crate::ui::DemoActions)).with_demo(true);

    let mut mock = crate::collector::MockSource::demo();
    let mut analyzer = crate::analyzer::Analyzer::default();

    // The 4 key deterministic demo phases and their corresponding target ticks
    let phases_ticks = [
        ("calm", 4),
        ("suspicious", 15),
        ("culprit", 30),
        ("cooling", 45),
    ];

    let mut current_tick = 0;
    let mut recorded_phases = Vec::new();

    for (phase_name, target_tick) in phases_ticks {
        while current_tick <= target_tick {
            let raw = mock.sample();
            let snap = analyzer.analyze(raw);
            app.push_snapshot(snap, std::time::Instant::now());
            current_tick += 1;
        }

        // Run one egui frame to calculate layout
        let raw_input = eframe::egui::RawInput {
            screen_rect: Some(eframe::egui::Rect::from_min_size(
                eframe::egui::Pos2::ZERO,
                eframe::egui::Vec2::new(width, height),
            )),
            ..Default::default()
        };

        let mut full_output = ctx.run_ui(raw_input, |ui| {
            app.render_ui(ui, std::time::Instant::now());
        });
        full_output.textures_delta.clear();

        let snapshot = app
            .last_layout
            .clone()
            .ok_or_else(|| format!("Failed to capture layout snapshot for phase '{phase_name}'"))?;

        snapshot.assert_valid(phase_name, expected_min_rows)?;
        recorded_phases.push((phase_name, snapshot));
    }

    // Now verify stability across all 4 phases
    let phase_refs: Vec<(&str, &LayoutSnapshot)> =
        recorded_phases.iter().map(|(p, s)| (*p, s)).collect();
    assert_layout_stability(&phase_refs)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_assertions_1000x680() {
        verify_layout_for_size(1000.0, 680.0, 8).expect("Layout assertion failed at 1000x680");
    }

    #[test]
    fn test_layout_assertions_760x520() {
        verify_layout_for_size(760.0, 520.0, 4).expect("Layout assertion failed at 760x520");
    }
}
