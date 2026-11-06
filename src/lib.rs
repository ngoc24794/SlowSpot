//! SlowSpot library root.
//!
//! Exposes the core modules for system metrics collection, analysis,
//! platform abstraction, and graphical user interface.

pub mod analyzer;
pub mod collector;
pub mod config;
pub mod model;
pub mod platform;
pub mod ui;

/// Application name constant.
pub const APP_NAME: &str = "SlowSpot";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_name() {
        assert_eq!(APP_NAME, "SlowSpot");
    }

    #[test]
    fn test_embedded_font_validity_and_vietnamese_glyphs() {
        let font_bytes = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
        assert!(
            font_bytes.len() > 100_000,
            "Font file is unexpectedly small"
        );
        // TrueType magic number: 0x00010000
        assert_eq!(&font_bytes[0..4], &[0x00, 0x01, 0x00, 0x00]);

        // Verify key Vietnamese diacritic characters exist in font cmap
        let vietnamese_sample = "Xin chào — Tiến trình đang làm chậm máy. Ơ ơ Ư ư Đ đ ế ệ ố ộ";
        for ch in vietnamese_sample.chars() {
            if ch.is_alphabetic() {
                assert!(
                    font_supports_char(font_bytes, ch),
                    "Font missing glyph for Vietnamese character: '{ch}' (U+{:04X})",
                    ch as u32
                );
            }
        }
    }

    fn font_supports_char(font_bytes: &[u8], ch: char) -> bool {
        let num_tables = u16::from_be_bytes([font_bytes[4], font_bytes[5]]) as usize;
        let mut cmap_offset = None;
        for i in 0..num_tables {
            let record_offset = 12 + i * 16;
            if &font_bytes[record_offset..record_offset + 4] == b"cmap" {
                let offset = u32::from_be_bytes([
                    font_bytes[record_offset + 8],
                    font_bytes[record_offset + 9],
                    font_bytes[record_offset + 10],
                    font_bytes[record_offset + 11],
                ]) as usize;
                cmap_offset = Some(offset);
                break;
            }
        }

        let Some(cmap_offset) = cmap_offset else {
            return false;
        };
        let num_subtables =
            u16::from_be_bytes([font_bytes[cmap_offset + 2], font_bytes[cmap_offset + 3]]) as usize;
        let code = ch as u32;

        for i in 0..num_subtables {
            let subtable_record = cmap_offset + 4 + i * 8;
            let sub_offset = cmap_offset
                + u32::from_be_bytes([
                    font_bytes[subtable_record + 4],
                    font_bytes[subtable_record + 5],
                    font_bytes[subtable_record + 6],
                    font_bytes[subtable_record + 7],
                ]) as usize;
            let format = u16::from_be_bytes([font_bytes[sub_offset], font_bytes[sub_offset + 1]]);

            if format == 4 && code <= 0xFFFF {
                let seg_count_x2 =
                    u16::from_be_bytes([font_bytes[sub_offset + 6], font_bytes[sub_offset + 7]])
                        as usize;
                let seg_count = seg_count_x2 / 2;
                let end_code_offset = sub_offset + 14;
                let start_code_offset = end_code_offset + seg_count_x2 + 2;

                for s in 0..seg_count {
                    let end = u16::from_be_bytes([
                        font_bytes[end_code_offset + s * 2],
                        font_bytes[end_code_offset + s * 2 + 1],
                    ]) as u32;
                    let start = u16::from_be_bytes([
                        font_bytes[start_code_offset + s * 2],
                        font_bytes[start_code_offset + s * 2 + 1],
                    ]) as u32;
                    if code >= start && code <= end {
                        return true;
                    }
                }
            } else if format == 12 {
                let num_groups = u32::from_be_bytes([
                    font_bytes[sub_offset + 12],
                    font_bytes[sub_offset + 13],
                    font_bytes[sub_offset + 14],
                    font_bytes[sub_offset + 15],
                ]) as usize;

                for g in 0..num_groups {
                    let group_offset = sub_offset + 16 + g * 12;
                    let start = u32::from_be_bytes([
                        font_bytes[group_offset],
                        font_bytes[group_offset + 1],
                        font_bytes[group_offset + 2],
                        font_bytes[group_offset + 3],
                    ]);
                    let end = u32::from_be_bytes([
                        font_bytes[group_offset + 4],
                        font_bytes[group_offset + 5],
                        font_bytes[group_offset + 6],
                        font_bytes[group_offset + 7],
                    ]);
                    if code >= start && code <= end {
                        return true;
                    }
                }
            }
        }

        false
    }
}
