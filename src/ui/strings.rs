//! Centralized user-facing Vietnamese strings for SlowSpot UI.
//!
//! Keeping all localized strings in one module ensures consistency and simplifies maintenance.

// Window & General
pub const APP_SUBTITLE: &str = "Giám sát & Tìm thủ phạm làm chậm máy";
pub const BADGE_DEMO: &str = "DEMO";

// System Pressure States
pub const PRESSURE_SMOOTH: &str = "Mượt mà";
pub const PRESSURE_STRAINED: &str = "Căng thẳng";
pub const PRESSURE_OVERLOADED: &str = "Quá tải";
pub const PRESSURE_WAITING: &str = "Đang chờ dữ liệu...";

// Cooldown Notice
pub const NOTICE_COOLDOWN: &str = "Đã hạ nhiệt, PC đã mượt trở lại";

// System Resource Headers
pub const RES_CPU: &str = "CPU Hệ thống";
pub const RES_RAM: &str = "Bộ nhớ RAM";
pub const RES_DISK: &str = "Tốc độ Đĩa";

// Live Update & Pause Controls
pub const BTN_PAUSE: &str = "Tạm dừng";
pub const BTN_RESUME: &str = "Tiếp tục";
pub const BADGE_PAUSED: &str = "ĐANG TẠM DỪNG";
pub const TOOLTIP_PAUSE_BUTTON: &str = "Đóng băng hiển thị để dễ dàng xem và chọn (Phím Space)";
pub const TOOLTIP_RESUME_BUTTON: &str =
    "Tiếp tục cập nhật dữ liệu hiển thị thời gian thực (Phím Space)";

// Top Culprit Hero Card
pub const HERO_TITLE: &str = "THỦ PHẠM CHÍNH";
pub const HERO_TITLE_CULPRIT: &str = "THỦ PHẠM CHÍNH";
pub const HERO_TITLE_SUSPICIOUS: &str = "ĐÁNG NGỜ";
pub const HERO_NO_CULPRIT: &str = "Máy đang chạy mượt, không có thủ phạm";
pub const HERO_NO_CULPRIT_SUB: &str =
    "Hệ thống đang hoạt động ổn định, các tiến trình trong ngưỡng an toàn.";
pub const HERO_PRESSURE_OVERLOADED: &str = "Hệ thống đang quá tải";
pub const HERO_PRESSURE_STRAINED: &str = "Hệ thống đang chịu tải cao";
pub const HERO_PRESSURE_NO_SINGLE_CULPRIT_SUB: &str =
    "Chưa xác định được một thủ phạm rõ ràng, nhiều tiến trình cùng chiếm tài nguyên.";
pub const HERO_SCORE_LABEL: &str = "Điểm thủ phạm";
pub const HERO_PROTECTED_BADGE: &str = "Bảo vệ";
pub const HERO_REASON_PREFIX: &str = "Nguyên nhân:";

// Watched Processes Panel ("Đang theo dõi")
pub const WATCHED_PANEL_TITLE: &str = "Đang theo dõi";
pub const WATCHED_BADGE_OVERLOADED: &str = "Quá tải";
pub const WATCHED_BADGE_COOLING_PREFIX: &str = "Hạ nhiệt (còn ";
pub const WATCHED_BADGE_COOLING_SUFFIX: &str = "s)";
pub const WATCHED_BADGE_EXITED: &str = "Đã thoát";
pub const WATCHED_TIME_PREFIX: &str = "Từ ";
pub const WATCHED_DURATION_PREFIX: &str = "(đã ";
pub const WATCHED_DURATION_SUFFIX: &str = ")";
pub const WATCHED_PEAK_PREFIX: &str = "Đỉnh:";
pub const BADGE_WATCHING_HEADER_PREFIX: &str = "Theo dõi:";
pub const BADGE_WATCHED_TAG: &str = "Theo dõi";

// Action Buttons
pub const BTN_GRACEFUL_CLOSE: &str = "Đóng an toàn";
pub const BTN_FORCE_KILL: &str = "Bắt buộc tắt";
pub const BTN_OPEN_LOCATION: &str = "Mở thư mục";
pub const BTN_IGNORE_CULPRIT: &str = "Bỏ qua thủ phạm này";

// Sparklines
pub const SPARKLINE_CPU_TITLE: &str = "CPU (60s)";
pub const SPARKLINE_RAM_TITLE: &str = "RAM (60s)";
pub const SPARKLINE_DISK_TITLE: &str = "Đĩa I/O (60s)";
pub const SPARKLINE_PEAK_PREFIX: &str = "Đỉnh:";
pub const SPARKLINE_TIME_START: &str = "-60s";
pub const SPARKLINE_TIME_NOW: &str = "nay";

// Process Table
pub const TABLE_TITLE: &str = "Danh sách tiến trình (Top 20)";
pub const SEARCH_PLACEHOLDER: &str = "Tìm kiếm theo tên hoặc PID (Ctrl+F)...";
pub const COL_NAME: &str = "Tên tiến trình";
pub const COL_PID: &str = "PID";
pub const COL_CPU: &str = "CPU %";
pub const COL_RAM: &str = "RAM";
pub const COL_DISK: &str = "Đĩa I/O";
pub const COL_SCORE: &str = "Điểm";
pub const COL_KIND: &str = "Loại";
pub const TABLE_EMPTY: &str = "Không tìm thấy tiến trình nào phù hợp.";

// Process Kinds
pub const KIND_USER_APP: &str = "Ứng dụng";
pub const KIND_BACKGROUND: &str = "Dịch vụ nền";
pub const KIND_SYSTEM_CRITICAL: &str = "Hệ thống";

// Process Severity
pub const SEVERITY_NORMAL: &str = "Bình thường";
pub const SEVERITY_SUSPICIOUS: &str = "Đáng ngờ";
pub const SEVERITY_CULPRIT: &str = "Thủ phạm";

// Selected Process Bar
pub const SELECTED_PROCESS_LABEL: &str = "Đang chọn:";
pub const SELECTED_NONE: &str = "Chọn một tiến trình trong bảng để thao tác";

// Tooltips & Inline Notices for Actions
pub const TOOLTIP_SYSTEM_PROTECTED: &str = "Tiến trình hệ thống được bảo vệ và không thể can thiệp";
pub const TOOLTIP_WAIT_GRACEFUL_PREFIX: &str = "Chờ";
pub const TOOLTIP_WAIT_GRACEFUL_SUFFIX: &str = "giây sau khi gửi lệnh đóng an toàn để bắt buộc tắt";
pub const TOOLTIP_EXE_PATH: &str = "Đường dẫn:";
pub const NOTICE_FORCE_KILL_DISABLED_DEFAULT: &str =
    "Bắt buộc tắt sẽ khả dụng nếu ứng dụng không đóng sau 5 giây";
pub const NOTICE_SYSTEM_PROTECTED: &str = "Tiến trình hệ thống được bảo vệ — không thể tắt";
pub const NOTICE_WAIT_GRACEFUL_PREFIX: &str = "Chờ";
pub const NOTICE_WAIT_GRACEFUL_SUFFIX: &str = "giây sau Đóng an toàn để Bắt buộc tắt";

// Bottom Status Bar & Shortcuts
pub const STATUS_HINT_SHORTCUTS: &str =
    "Ctrl+F: Tìm kiếm | Esc: Xoá / Đóng | Lên/Xuống: Chọn | Space: Tạm dừng";
pub const STATUS_UPDATED_PREFIX: &str = "Cập nhật:";
pub const STATUS_TOTAL_PROCESSES: &str = "Tổng số:";
pub const STATUS_PROCESS_UNIT: &str = "tiến trình";

// Confirmation Dialog
pub const CONFIRM_TITLE: &str = "Xác nhận bắt buộc tắt";
pub const CONFIRM_CANCEL: &str = "Hủy bỏ";
pub const CONFIRM_PROCEED: &str = "Bắt buộc tắt ngay";

// Action Feedback Messages (Vietnamese)
pub const MSG_ACCESS_DENIED: &str = "Cần quyền Quản trị viên để can thiệp tiến trình này";
pub const MSG_SYSTEM_PROTECTED: &str = "Tiến trình hệ thống được bảo vệ và không thể can thiệp";
pub const MSG_NO_WINDOW: &str =
    "Tiến trình không có cửa sổ hiển thị để đóng an toàn (hãy dùng Bắt buộc tắt)";
pub const MSG_NOT_FOUND: &str = "Không tìm thấy tiến trình hoặc tiến trình đã kết thúc";
pub const MSG_NO_EXE_PATH: &str = "Không xác định được đường dẫn tệp thực thi";
pub const MSG_IGNORED_SUCCESS: &str = "Đã thêm vào danh sách bỏ qua";

// Startup & Runtime Errors (Native Dialogs)
pub const ERR_OPENGL_INIT: &str =
    "Không thể khởi tạo giao diện đồ hoạ OpenGL 2.0+.\n\nVui lòng cập nhật driver card đồ hoạ hoặc chạy ứng dụng trên máy tính có hỗ trợ OpenGL 2.0 trở lên.";
pub const ERR_UNEXPECTED_PANIC: &str =
    "Đã xảy ra sự cố nghiêm trọng không mong muốn khiến ứng dụng phải dừng lại.\n\nỨng dụng sẽ tự động đóng để bảo vệ an toàn cho hệ thống.";

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_STRINGS: &[&str] = &[
        APP_SUBTITLE,
        BADGE_DEMO,
        PRESSURE_SMOOTH,
        PRESSURE_STRAINED,
        PRESSURE_OVERLOADED,
        PRESSURE_WAITING,
        NOTICE_COOLDOWN,
        RES_CPU,
        RES_RAM,
        RES_DISK,
        BTN_PAUSE,
        BTN_RESUME,
        BADGE_PAUSED,
        TOOLTIP_PAUSE_BUTTON,
        TOOLTIP_RESUME_BUTTON,
        HERO_TITLE,
        HERO_TITLE_CULPRIT,
        HERO_TITLE_SUSPICIOUS,
        HERO_NO_CULPRIT,
        HERO_NO_CULPRIT_SUB,
        HERO_PRESSURE_OVERLOADED,
        HERO_PRESSURE_STRAINED,
        HERO_PRESSURE_NO_SINGLE_CULPRIT_SUB,
        HERO_SCORE_LABEL,
        HERO_PROTECTED_BADGE,
        HERO_REASON_PREFIX,
        WATCHED_PANEL_TITLE,
        WATCHED_BADGE_OVERLOADED,
        WATCHED_BADGE_COOLING_PREFIX,
        WATCHED_BADGE_COOLING_SUFFIX,
        WATCHED_BADGE_EXITED,
        WATCHED_TIME_PREFIX,
        WATCHED_DURATION_PREFIX,
        WATCHED_DURATION_SUFFIX,
        WATCHED_PEAK_PREFIX,
        BADGE_WATCHING_HEADER_PREFIX,
        BADGE_WATCHED_TAG,
        BTN_GRACEFUL_CLOSE,
        BTN_FORCE_KILL,
        BTN_OPEN_LOCATION,
        BTN_IGNORE_CULPRIT,
        SPARKLINE_CPU_TITLE,
        SPARKLINE_RAM_TITLE,
        SPARKLINE_DISK_TITLE,
        SPARKLINE_PEAK_PREFIX,
        SPARKLINE_TIME_START,
        SPARKLINE_TIME_NOW,
        TABLE_TITLE,
        SEARCH_PLACEHOLDER,
        COL_NAME,
        COL_PID,
        COL_CPU,
        COL_RAM,
        COL_DISK,
        COL_SCORE,
        COL_KIND,
        TABLE_EMPTY,
        KIND_USER_APP,
        KIND_BACKGROUND,
        KIND_SYSTEM_CRITICAL,
        SEVERITY_NORMAL,
        SEVERITY_SUSPICIOUS,
        SEVERITY_CULPRIT,
        SELECTED_PROCESS_LABEL,
        SELECTED_NONE,
        TOOLTIP_SYSTEM_PROTECTED,
        TOOLTIP_WAIT_GRACEFUL_PREFIX,
        TOOLTIP_WAIT_GRACEFUL_SUFFIX,
        TOOLTIP_EXE_PATH,
        NOTICE_FORCE_KILL_DISABLED_DEFAULT,
        NOTICE_SYSTEM_PROTECTED,
        NOTICE_WAIT_GRACEFUL_PREFIX,
        NOTICE_WAIT_GRACEFUL_SUFFIX,
        STATUS_HINT_SHORTCUTS,
        STATUS_UPDATED_PREFIX,
        STATUS_TOTAL_PROCESSES,
        STATUS_PROCESS_UNIT,
        CONFIRM_TITLE,
        CONFIRM_CANCEL,
        CONFIRM_PROCEED,
        MSG_ACCESS_DENIED,
        MSG_SYSTEM_PROTECTED,
        MSG_NO_WINDOW,
        MSG_NOT_FOUND,
        MSG_NO_EXE_PATH,
        MSG_IGNORED_SUCCESS,
        ERR_OPENGL_INIT,
        ERR_UNEXPECTED_PANIC,
    ];

    #[test]
    fn test_all_ui_strings_supported_by_noto_sans() {
        let font_bytes = include_bytes!("../../assets/fonts/NotoSans-Regular.ttf");
        for &s in ALL_STRINGS {
            for ch in s.chars() {
                if ch.is_whitespace() {
                    continue;
                }
                assert!(
                    font_supports_char(font_bytes, ch),
                    "String '{s}' contains character '{ch}' (U+{:04X}) not supported by embedded Noto Sans font",
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
