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
pub const NOTICE_COOLDOWN: &str = "✨ Đã hạ nhiệt, PC đã mượt trở lại";

// System Resource Headers
pub const RES_CPU: &str = "CPU Hệ thống";
pub const RES_RAM: &str = "Bộ nhớ RAM";
pub const RES_DISK: &str = "Tốc độ Đĩa";

// Top Culprit Hero Card
pub const HERO_TITLE: &str = "THỦ PHẠM CHÍNH";
pub const HERO_NO_CULPRIT: &str = "Máy đang chạy mượt, không có thủ phạm";
pub const HERO_SCORE_LABEL: &str = "Điểm thủ phạm";
pub const HERO_PROTECTED_BADGE: &str = "🔒 Được bảo vệ";
pub const HERO_REASON_PREFIX: &str = "Nguyên nhân:";

// Action Buttons
pub const BTN_GRACEFUL_CLOSE: &str = "Đóng an toàn";
pub const BTN_FORCE_KILL: &str = "Bắt buộc tắt";
pub const BTN_OPEN_LOCATION: &str = "Mở thư mục";
pub const BTN_IGNORE_CULPRIT: &str = "Bỏ qua thủ phạm này";

// Sparklines
pub const SPARKLINE_CPU_TITLE: &str = "CPU (60 giây)";
pub const SPARKLINE_RAM_TITLE: &str = "RAM (60 giây)";
pub const SPARKLINE_DISK_TITLE: &str = "Đĩa I/O (60 giây)";

// Process Table
pub const TABLE_TITLE: &str = "Danh sách tiến trình (Top 20)";
pub const SEARCH_PLACEHOLDER: &str = "🔍 Tìm kiếm theo tên hoặc PID...";
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
