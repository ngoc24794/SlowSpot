# Tài liệu Thiết kế UI/UX & Hệ thống Giao diện — SlowSpot

| Phiên bản | Trạng thái | Ngày tạo | Tác giả |
| :--- | :--- | :--- | :--- |
| 2.3.1 | Hoàn thiện Ổn định Bố cục Dọc & Slot Cố định (Task polish3 Addendum) | 06/11/2026 | SlowSpot UI/UX Design & Engineering Team |
| 2.3.0 | Hoàn thiện Khắc phục Lỗi UI/UX Vòng 4 (Task polish3) | 06/11/2026 | SlowSpot UI/UX Design & Engineering Team |
| 2.2.0 | Hoàn thiện Khắc phục Lỗi UI/UX Vòng 3 (Task uiux3) | 05/11/2026 | SlowSpot UI/UX Design & Engineering Team |

---

## 1. Triết lý Thiết kế (Design Philosophy)

SlowSpot áp dụng trường phái **Swiss Minimalism & High-Density Dashboard**:
- **Tập trung cao độ (Information-Dense & Purpose-Driven)**: Mật độ hiển thị đạt mức 8/10, tối ưu hoá không gian hiển thị để đưa những thông tin quan trọng nhất (Thủ phạm gây nghẽn, áp lực tài nguyên, tiến trình tốn tài nguyên) đến mắt người dùng chỉ trong 3 giây đầu tiên.
- **Tối giản Thuần khiết (Swiss Dark Theme)**: Nền tối đậm màu xanh đá bảng phiến (#0F172A), các thẻ thông tin bo tròn nhẹ góc, phân tách bằng độ tương phản bề mặt thay vì đường viền dày rối mắt.
- **Tiếp cận Đa phương thức (Accessible / WCAG 2.1 AA Compliant)**: Không bao giờ dựa vào màu sắc đơn thuần để phân định mức độ nghiêm trọng. Mọi trạng thái luôn đi kèm hình học nhận diện (Hình tròn, Tam giác cảnh báo, Hình thoi, Bát giác) và văn bản tiếng Việt rõ ràng.
- **Chuyển động Tinh tế (Subtle Micro-Interactions)**: Mức chuyển động tinh tế 3/10 (150–250ms), sử dụng các hàm chuyển động chuyển tiếp nhẹ nhàng của egui, tự động ngắt hoạt ảnh khi kích hoạt chế độ `SLOWSPOT_REDUCED_MOTION=1`, không tiêu tốn chu kỳ vẽ 60 FPS khi tĩnh.

---

## 2. Bảng Mã Màu & Design Tokens (Color Palette)

Toàn bộ màu sắc trong SlowSpot được định nghĩa tập trung trong module `src/ui/palette.rs` dưới dạng hằng số strongly-typed `egui::Color32`, tuyệt đối không dùng mã hex rải rác trong các hàm widget.

### 2.1. Danh mục Token Màu

| Tên Token | Mã Hex / RGB | Mô tả mục đích sử dụng | Tỷ lệ tương phản trên nền tối |
| :--- | :--- | :--- | :--- |
| `COLOR_BG` | `#0F172A` (15, 23, 42) | Nền chính của toàn bộ cửa sổ ứng dụng (Slate 900) | — (Nền cơ sở) |
| `COLOR_CARD` | `#1B2336` (27, 35, 54) | Bề mặt các thẻ chứa nội dung (Banner, Card, Bảng) | 1.15:1 so với nền |
| `COLOR_BG_CARD_CULPRIT` | `#261014` (38, 16, 20) | Nền thẻ thủ phạm chính (dark crimson), tương phản 4.81:1 với viền đỏ | **4.8:1** (Vượt chuẩn WCAG AA >= 4.5) |
| `COLOR_BG_CARD_SUSPICIOUS` | `#281808` (40, 24, 8) | Nền thẻ tiến trình đáng ngờ (dark amber), tương phản 7.99:1 với viền vàng | **8.0:1** (Vượt chuẩn WCAG AA >= 4.5) |
| `COLOR_MUTED` | `#272F42` (39, 47, 66) | Bề mặt hàng hover, ô tìm kiếm, chip phụ (Slate 800) | 1.35:1 so với nền |
| `COLOR_SELECTED_ROW` | `#24334D` (36, 51, 77) | Nền của hàng đang được người dùng chọn | 1.45:1 so với nền |
| `COLOR_BORDER` | `#475569` (71, 85, 105) | Viền hộp thoại, khung phân cách rõ nét (Slate 600) | 3.2:1 (Đạt chuẩn đồ hoạ) |
| `COLOR_BORDER_SUBTLE`| Alpha 60 trên `#475569` | Đường kẻ mờ phân tách hàng bảng và vạch lưới biểu đồ | Tinh tế, không gây nhiễu |
| `COLOR_FOREGROUND` | `#F8FAFC` (248, 250, 252) | Chữ chính, tiêu đề, số liệu quan trọng (Slate 50) | **16.0:1** (Vượt chuẩn WCAG AAA) |
| `COLOR_MUTED_FOREGROUND`| `#94A3B8` (148, 163, 184) | Chữ phụ, nhãn đơn vị, gợi ý phím tắt (Slate 400) | **6.8:1** (Vượt chuẩn WCAG AA >= 4.5) |
| `COLOR_ACCENT_OK` | `#22C55E` (34, 197, 94) | Trạng thái Mượt mà, Bình thường, Đĩa I/O (Emerald 500)| **8.0:1** (Vượt chuẩn WCAG AA) |
| `COLOR_WARNING` | `#F59E0B` (245, 158, 11) | Trạng thái Căng thẳng, Đáng ngờ, RAM (Amber 500) | **7.8:1** (Vượt chuẩn WCAG AA) |
| `COLOR_DESTRUCTIVE` | `#EF4444` (239, 68, 68) | Trạng thái Quá tải, Thủ phạm, Nút Huỷ (Red 500) | 3.7:1 (Dùng làm nền cảnh báo) |
| `COLOR_DESTRUCTIVE_TEXT`| `#F87171` (248, 113, 113) | Chữ cảnh báo thủ phạm trên nền tối (Red 400) | **5.0:1** (Vượt chuẩn WCAG AA >= 4.5) |
| `COLOR_INFO` | `#38BDF8` (56, 189, 248) | Ứng dụng người dùng, thanh nhấn chọn (Sky 400) | **8.2:1** (Vượt chuẩn WCAG AA) |
| `COLOR_PROTECTED` | `#E879F9` (232, 121, 249) | Tiến trình cốt lõi hệ thống được bảo vệ (Fuchsia 400) | **7.4:1** (Vượt chuẩn WCAG AA) |

### 2.2. Kiểm định Tương phản WCAG 2.1 AA (Contrast Ratio Test)
Thuật toán tính độ sáng tương đối ($L$) và tỉ số tương phản được kiểm thử tự động tại `ui::palette::tests::test_wcag_contrast_ratios_for_normal_text`:
$$L = 0.2126 \times R_{\text{linear}} + 0.7152 \times G_{\text{linear}} + 0.0722 \times B_{\text{linear}}$$
$$\text{Contrast Ratio} = \frac{L_1 + 0.05}{L_2 + 0.05}$$
Toàn bộ chữ hiển thị cho người dùng đều vượt ngưỡng yêu cầu tối thiểu 4.5:1 dành cho chữ thường theo chuẩn WCAG 2.1 cấp độ AA.
Bộ token màu theo cấp độ (Suspicious vs Culprit) được kiểm chứng tự động tại `ui::palette::tests::test_level_color_token_mapping`:
- Viền và chữ màu hổ phách `#F59E0B` trên nền tối hổ phách `#281808` đạt tỉ số tương phản 7.99:1.
- Viền và chữ màu đỏ `#EF4444` trên nền tối đỏ thẫm `#261014` đạt tỉ số tương phản 4.81:1.
- Chữ chính `COLOR_FOREGROUND` và chữ phụ `COLOR_MUTED_FOREGROUND` trên cả hai nền đều đạt tương phản > 6.6:1.

---

## 3. Hệ thống Typography & Phông chữ

### 3.1. Phông chữ Nhúng Sẵn
- Sử dụng phông chữ mở **Noto Sans Regular** (`assets/fonts/NotoSans-Regular.ttf`) với giấy phép SIL Open Font License 1.1 (OFL 1.1).
- 100% văn bản giao diện tiếng Việt có dấu được kiểm chứng không bị lỗi ô vuông (`tofu`) thông qua bài kiểm thử tĩnh `ui::strings::tests::test_all_ui_strings_supported_by_noto_sans`.

### 3.2. Thang Kích thước Chữ (Type Scale)
- **Tiêu đề Chính (`FONT_SIZE_HEADING`)**: `18.0 px` (In đậm, dùng cho tên ứng dụng SlowSpot và tên tiến trình thủ phạm).
- **Tiêu đề Phụ (`FONT_SIZE_SUBHEADING`)**: `15.0 px` (In đậm, dùng cho tiêu đề bảng tiến trình và kết luận mượt mà).
- **Thân bài Lớn (`FONT_SIZE_BODY_LARGE`)**: `14.0 px` (Giải thích nguyên nhân nghẽn hệ thống, hộp thoại xác nhận).
- **Thân bài Chuẩn (`FONT_SIZE_BODY`)**: `13.0 px` (Hàng tiến trình, tên tiến trình trong bảng, thông báo toast).
- **Chú thích (`FONT_SIZE_CAPTION`)**: `12.0 px` (Tiêu đề cột bảng, nhãn badge, đơn vị đo, phím tắt, mốc trục thời gian, đảm bảo không có văn bản nào < 12px và luôn đạt tương phản >= 4.5:1).
- **Số liệu Nhỏ (`FONT_SIZE_TINY`)**: `12.0 px` (Đồng bộ mức tối thiểu 12px theo quy chuẩn trợ năng a11y).

### 3.3. Định dạng Số Liệu (Tabular Figures)
Các cột số (PID, CPU %, RAM, Đĩa I/O, Điểm) được hiển thị với kiểu định dạng chữ số canh lề phải và cố định độ rộng (`monospace` / `tabular`) để đảm bảo các con số luôn thẳng cột khi quét dọc danh sách.

---

## 4. Quy chuẩn Khoảng cách (Spacing) & Bo góc (Radius)

### 4.1. Thang Khoảng cách (4-Point Spacing Grid)
- `SPACING_XS = 4.0 px`: Khoảng cách giữa biểu tượng và nhãn, đệm bên trong badge.
- `SPACING_SM = 8.0 px`: Khoảng cách giữa các cột trong hàng bảng, khoảng cách giữa các khối thành phần nhỏ.
- `SPACING_MD = 12.0 px`: Đệm bên trong thẻ Card, khoảng cách giữa các phần giao diện lớn.
- `SPACING_LG = 16.0 px`: Đệm bên trong hộp thoại xác nhận modal dialog.

### 4.2. Quy chuẩn Bo góc (Corner Radius)
- `RADIUS_SM = 4.0 px`: Nút bấm nhỏ, badge phân loại, thanh tiến trình điểm, ô tìm kiếm.
- `RADIUS_MD = 6.0 px`: Thẻ biểu đồ sparkline, thông báo toast, thanh tiến trình được chọn.
- `RADIUS_LG = 8.0 px`: Thẻ Card lớn (Banner đỉnh, Thẻ thủ phạm chính, Khung bao bảng tiến trình, Hộp thoại).

---

## 5. Thiết kế Thành phần Giao diện (Components)

### 5.1. Khung Tiêu Đề Đỉnh (Header Banner)
- **Bên trái**:
  - Tên ứng dụng `SlowSpot`, Huy hiệu `DEMO` (nếu chạy giả lập).
  - **Huy hiệu trạng thái thống nhất (Single Source of Truth)** đồng bộ cấp độ với Hero Card:
    - **Mức Bình thường (Normal)**: Hình tròn đặc (●) màu xanh lá `COLOR_ACCENT_OK` + nhãn `Mượt mà`.
    - **Mức Đáng ngờ (Suspicious)**: Hình tam giác cảnh báo (▲) màu vàng hổ phách `COLOR_WARNING` + nhãn `Có dấu hiệu chậm`.
    - **Mức Thủ phạm / Quá tải (Culprit / Overload)**: Hình bát giác nguy hiểm (🛑) màu đỏ `COLOR_DESTRUCTIVE` + nhãn tài nguyên quá tải (ví dụ: `Quá tải CPU`, `Quá tải RAM`, `Quá tải đĩa`, `Quá tải CPU + RAM`).
  - Nhãn `ĐANG TẠM DỪNG` (khi đóng băng hiển thị).
  - Thông báo làm mát `Đã hạ nhiệt, PC đã mượt trở lại` ✔ với cơ chế tự động co giãn / cắt gọn dấu ba chấm (`…`) trước cụm pill tài nguyên bên phải ở mọi độ rộng $\ge 760\text{px}$ (đo lường không gian khả dụng = tổng chiều rộng thẻ - chiều rộng cụm chip phải - lề an toàn 10px). Huy hiệu thừa `Theo dõi: N` trên thanh tiêu đề đã được lược bỏ để nhường không gian cho thông báo này.
- **Bên phải**:
  - Nút **Tạm dừng / Tiếp tục** (kèm phím tắt `Space` và tooltip hướng dẫn a11y, kích thước cố định gọn gàng 64px x 24px).
  - 3 Chip chỉ số tài nguyên mini (`CPU`, `RAM`, `Đĩa`):
    - Cấu trúc phân bổ độ rộng cố định nghiêm ngặt: Nhãn bên trái (`CPU`/`RAM`/`Đĩa`), khoảng cách đệm 6px (bảo đảm nhãn chữ và vạch bar không bao giờ chạm nhau), thanh đo tỷ lệ trực quan (12px), và giá trị số liệu monospace căn lề phải.
    - Ranh giới thanh bar và số liệu hoàn toàn tách rời, triệt tiêu triệt để hiện tượng vạch bar vẽ đè/che khuất các ký tự đầu tiên của số liệu với mọi giá trị lên đến `999.9 MB/s` và `100%`.

### 5.2. Khu Vực Trạng Thái & Thẻ Thủ Phạm Chính (Status Area & Hero Card)
- **Slot Cố Định Bất Biến (`STATUS_SLOT_HEIGHT = 104px`)**: Chiều cao khu vực trạng thái là một hằng số cố định tuyệt đối, nâng từ 64px lên 104px để hiển thị đầy đủ thẻ thủ phạm/đáng ngờ với 4 dòng hoàn chỉnh: tiêu đề, thông tin tiến trình, hàng 4 nút hành động, và dòng ghi chú an toàn. Thiết kế này bảo đảm mọi nút bấm luôn hiển thị trọn vẹn và bấm được ở cả trạng thái Đáng ngờ (Suspicious) và Thủ phạm (Culprit) tại cả 1000x680 và 760x520 mà không bao giờ bị cắt xén (clip). Các khối thành phần bên dưới (Biểu đồ Sparklines và Bảng tiến trình) không bao giờ bị xê dịch khi trạng thái hệ thống thay đổi (Zero Cumulative Layout Shift).
- **Khi có thủ phạm hoặc tiến trình đáng ngờ**:
  - Khung viền (Border), màu nền (Background), tiêu đề (Title), màu điểm số và huy hiệu cấp độ tuân thủ nghiêm ngặt theo cấp độ nghiêm trọng:
    - **Mức Đáng ngờ (Suspicious - điểm 40–69)**: Viền màu vàng hổ phách `#F59E0B` (`COLOR_WARNING`), nền tối ánh hổ phách `#281808` (`COLOR_BG_CARD_SUSPICIOUS`), tiêu đề `ĐÁNG NGỜ` màu hổ phách, điểm số hiển thị màu hổ phách, huy hiệu cấp độ hình tam giác cảnh báo ▲ + nhãn `Đáng ngờ`.
    - **Mức Thủ phạm (Culprit - điểm >= 70)**: Viền màu đỏ `#EF4444` (`COLOR_DESTRUCTIVE`), nền tối ánh đỏ thẫm `#261014` (`COLOR_BG_CARD_CULPRIT`), tiêu đề `THỦ PHẠM CHÍNH` màu đỏ, điểm số hiển thị màu đỏ, huy hiệu cấp độ hình thoi nguy hiểm ◆ + nhãn `Thủ phạm`.
  - Luôn kết hợp biểu tượng hình học + nhãn văn bản (icon + text label) song hành cùng màu sắc để màu sắc không bao giờ là tín hiệu thị giác duy nhất (WCAG 1.4.1).
  - Tỷ lệ tương phản chữ trên nền tối màu luôn đạt >= 4.5:1.
  - Bố cục 4 dòng tinh gọn vừa vặn hoàn hảo trong slot 104px:
    - **Dòng 1**: Tiêu đề thẻ, huy hiệu cấp độ, huy hiệu bảo vệ hệ thống và điểm số thủ phạm căn lề phải.
    - **Dòng 2**: Tên file thực thi (14px in đậm), PID, và lý giải nguyên nhân bằng tiếng Việt (`— chiếm 88% CPU`).
    - **Dòng 3 (Hàng 4 nút hành động đầy đủ)**:
      1. **Nút Chính (Primary)**: `Đóng an toàn` (Tô đặc màu xanh dương `COLOR_INFO`, chữ tối màu `COLOR_BG` đậm nét).
      2. **Nút Nguy hiểm (Danger)**: `Bắt buộc tắt` (Tô nền đỏ khi đã qua 5s chờ đóng an toàn và tiến trình có thể tắt; khi chưa đủ điều kiện ở trạng thái vô hiệu hoá kèm tooltip).
      3. **Nút Phụ (Secondary)**: `Mở thư mục` và `Bỏ qua thủ phạm này` (Dạng viền tối giản).
    - **Dòng 4 (Ghi chú an toàn)**: Khi nút `Bắt buộc tắt` chưa khả dụng hoặc tiến trình được bảo vệ, hiển thị biểu tượng cảnh báo ⚠️ kèm thông điệp giải thích: `Tiến trình hệ thống bảo vệ`, `Chờ an toàn (Xs)` hoặc `Bắt buộc tắt sẽ khả dụng nếu ứng dụng không đóng sau 5 giây`. Tooltip chi tiết vẫn được hiển thị đầy đủ khi rà chuột.
- **Khi không có thủ phạm duy nhất (No Single Culprit)**:
  - **Trường hợp máy mượt mà (`PressureState::Smooth`)**:
    - Biểu tượng dấu kiểm tra ✔ xanh lá (`COLOR_ACCENT_OK`), nhãn `Máy đang chạy mượt, không có thủ phạm` và phụ chú `— Hệ thống đang hoạt động ổn định, các tiến trình trong ngưỡng an toàn.` Toàn bộ thông điệp được **căn giữa theo chiều dọc** trong slot 104px, tạo cảm giác thoáng đãng và bình yên.
  - **Trường hợp hệ thống chịu tải cao hoặc quá tải (`PressureState::Strained` / `Overloaded`)**:
    - Đồng bộ cảnh báo màu vàng hổ phách (`COLOR_WARNING`) với biểu tượng tam giác cảnh báo ▲, tiêu đề `Hệ thống đang quá tải` (hoặc `Hệ thống đang chịu tải cao`) và phụ chú `— Chưa xác định được một thủ phạm rõ ràng, nhiều tiến trình cùng chiếm tài nguyên.`
    - Nếu có nhóm tiến trình nổi bật, cung cấp thêm hàng nút `Lọc trong bảng` và `Bỏ qua nhóm này`. Toàn bộ nội dung được căn giữa theo chiều dọc trong cùng slot 104px.

### 5.3. Biểu đồ Sparkline Thời gian Thực (Real-time Sparklines)
- Gồm 3 biểu đồ đồng thời theo dõi lịch sử 60 giây của **CPU**, **RAM** và **Tốc độ Đĩa I/O**.
- **Chiều rộng chia đều không tràn**: Chiều rộng 3 thẻ được tính toán chính xác bằng hàm thuần `calculate_sparkline_widths(available_width, spacing)` sao cho tổng 3 thẻ cộng 2 khoảng trống spacing bằng đúng 100% `available_width`.
- **Vùng tô nhạt (Area Shading)**: Vẽ đa giác mờ bên dưới đường biểu đồ với độ mờ 12%, mang lại cảm giác chiều sâu của dữ liệu theo trường phái Swiss.
- **Lưới tham chiếu mờ (Reference Grid)**: Các vạch kẻ ngang mờ nhạt tại các mốc 25%, 50%, 75% giúp ước lượng giá trị trực quan.
- **Thông số đỉnh & đơn vị**: Tiêu đề thẻ hiển thị ở dòng trên góc trái (`Đĩa I/O (60s)`), giá trị tức thời lớn ở góc phải (`997.5 MB/s`), và giá trị đỉnh cao nhất trong 60 giây (`Đỉnh: 997.5 MB/s`) được đặt riêng ở dòng thứ hai ngay dưới tiêu đề. Thiết kế phân dòng này loại bỏ hoàn toàn nguy cơ đè chữ/va chạm nhãn ở mọi độ rộng cửa sổ >= 760px.
- **Trục thời gian**: Đánh dấu rõ ràng mốc `-60s` ở góc trái và `nay` ở góc phải với chữ 12px không bị mờ.
- **Rà chuột soi dữ liệu (Hover Inspection)**: Khi di chuột lên biểu đồ, một đường chỉ dẫn dọc (hairline) xuất hiện kèm vòng tròn nhấn điểm và tooltip hiển thị số đo chính xác tại giây đó (ví dụ `-14s: 82.5%`).

### 5.4. Bảng Tiến Trình Tương Tác (Process Table)
- **Tiêu đề cột phẳng (Flat Headers)**: Tiêu đề chữ hoa nhỏ tinh tế (~20px), khi rê chuột chuyển sang biểu tượng bàn tay (`PointingHand`). Mũi tên sắp xếp vẽ sắc nét trực tiếp bằng Painter. Đường phân cách 1px đặt ngay dưới tiêu đề cột, triệt tiêu khoảng trống thừa.
- **Cấu trúc Vùng cuộn & Cắt xén Nghiêm ngặt (Strict Viewport & Child UI Clipping)**:
  - Thanh tìm kiếm và hàng tiêu đề cột cố định (`Flat Headers`) nằm hoàn toàn bên ngoài vùng cuộn, luôn hiển thị bên trên thân bảng.
  - Phần thân bảng được phân bổ một vùng hình chữ nhật chính xác (`table_body_rect`) tạo thành một child UI (`body_ui`) với đường cắt xén nghiêm ngặt `body_clip = ui.clip_rect().intersect(table_body_rect)`.
  - Thành phần `ScrollArea::vertical().id_salt("process_table_scroll")` được nhúng hoàn toàn bên trong child UI này.
  - Toàn bộ ô dữ liệu (`render_cell`) và bề mặt hàng bảng áp dụng cơ chế giao cắt `ui.clip_rect().intersect(...)`, đảm bảo khi cuộn lên đỉnh hay cuộn xuống đáy, tuyệt đối không một dòng chữ hay đồ hoạ tiến trình nào có thể vẽ đè lên tiêu đề cột, thanh tìm kiếm, các biểu đồ sparklines, banner đỉnh phía trên, hoặc thanh trạng thái đáy.
- **Mật độ hiển thị cao & Chiều cao hàng ~28px**: Mỗi hàng có chiều cao chuẩn 28px với khoảng cách đệm `item_spacing.y = 0.0px`, đảm bảo hiển thị đồng thời >= 12 hàng ở kích thước 1000x680 và >= 8 hàng ở kích thước tối thiểu 760x520.
- **Cột Loại tiến trình (Giảm nhiễu thị giác)**: Tiến trình ứng dụng thông thường (`UserApp`) được hiển thị bằng chữ màu xám mờ (`COLOR_MUTED_FOREGROUND`), không có khung viền dạng nút bấm để tránh nhiễu và không gây hiểu lầm là nút bấm được. Chỉ tiến trình dịch vụ nền (`BackgroundService`) và tiến trình hệ thống cốt lõi (`SystemCritical` kèm ổ khoá 🔒) mới hiển thị dạng huy hiệu.
- **Làm nổi bật hàng theo cấp độ (Level Highlight)**:
  - Hàng thủ phạm (Điểm >= 70): Nền hàng được phủ dải màu đỏ nhạt `COLOR_DESTRUCTIVE` (alpha 12%), huy hiệu theo dõi viền đỏ, khi được chọn có thanh nhấn cạnh trái màu đỏ.
  - Hàng đáng ngờ (Điểm 40–69): Nền hàng được phủ dải màu vàng hổ phách `COLOR_WARNING` (alpha 10%), huy hiệu theo dõi viền vàng, khi được chọn có thanh nhấn cạnh trái màu vàng.
- **Hiệu ứng chọn hàng (Selection State)**: Hàng đang chọn đổi nền sang tông xanh thẫm (`COLOR_SELECTED_ROW`) và có thanh nhấn dày 3px ở cạnh trái phản ánh màu cấp độ (Đỏ cho Thủ phạm, Vàng cho Đáng ngờ, Xanh dương `COLOR_INFO` cho tiến trình bình thường).
- **Cột Điểm tiến trình**: Thanh tiến trình mini 38px bo góc kết hợp số điểm 0..100 đặt bên ngoài cạnh phải thanh, không còn tình trạng chữ đè lên vạch màu gây khó đọc.
- **Nhận diện Tiến trình Hệ thống**: Các tiến trình được bảo vệ (`SystemCritical`) có biểu tượng ổ khoá 🔒 vẽ bằng Painter cạnh tên, tooltip hiển thị đầy đủ đường dẫn tệp thực thi (`exe_path`).
- **Trạng thái Tìm kiếm Rỗng**: Khi không có tiến trình khớp với từ khoá tìm kiếm, bảng hiển thị thông điệp thân thiện `Không tìm thấy tiến trình nào phù hợp.`

### 5.5. Thanh Trạng Thái Đáy Cố Định (Bottom Status Bar)
- Cố định bằng `Panel::bottom` (vẽ trước CentralPanel), luôn luôn hiển thị ở mọi kích thước cửa sổ >= 760x520:
  - Viền phân cách: Viền đỉnh 1px rõ nét màu `COLOR_BORDER` (`#475569`) ngăn cách hoàn toàn với nội dung bảng phía trên.
  - Cánh trái: Bảng hướng dẫn phím tắt súc tích (`Ctrl+F: Tìm kiếm | Esc: Xoá / Đóng | Lên/Xuống: Chọn | Space: Tạm dừng`) với cỡ chữ >= 12px, màu tương phản cao `#94A3B8` (`COLOR_MUTED_FOREGROUND`).
  - Cánh phải: Thời gian cập nhật gần nhất (`Cập nhật: vừa xong` hoặc `X s trước`), Tổng số tiến trình thu thập (`Tổng số: 120 tiến trình`), các dấu gạch đứng phân cách màu `#475569`, huy hiệu `[DEMO]` và `[ĐANG TẠM DỪNG]` khi kích hoạt.

---

## 6. Tương Tác Bàn Phím & Phím Tắt (Keyboard Navigation)

Ứng dụng hỗ trợ thao tác hoàn toàn bằng bàn phím theo tiêu chuẩn trợ năng máy tính để bàn:
- `Ctrl + F`: Đặt con trỏ focus ngay lập tức vào ô tìm kiếm tiến trình.
- `Escape (Esc)`:
  - Nếu đang mở hộp thoại xác nhận Bắt buộc tắt -> Đóng hộp thoại an toàn.
  - Nếu đang có từ khoá trong ô tìm kiếm -> Xoá sạch từ khoá tìm kiếm.
  - Nếu đang chọn một hàng trong bảng -> Huỷ chọn hàng.
- `Mũi tên Lên (↑)` / `Mũi tên Xuống (↓)`: Di chuyển nhanh hàng chọn lên trên / xuống dưới trong bảng tiến trình đã lọc, tự động kẹp biên an toàn khi chạm đầu hoặc cuối danh sách.
- `Phím Cách (Space)`: Đóng băng (Tạm dừng) hoặc Tiếp tục cập nhật luồng hiển thị thời gian thực (khi không gõ trong ô tìm kiếm).
- Vòng tròn chỉ thị focus của bàn phím (`focus ring`) được giữ nguyên theo cơ chế mặc định của `egui`.

---

## 7. Khả Năng Tiếp Cận (Accessibility & a11y)

1. **Phi phụ thuộc màu sắc (Shape + Text Co-design)**:
   - Thay vì chỉ đổi màu xanh/vàng/đỏ, ứng dụng sử dụng bộ hình học nhận diện:
     - Mức Bình thường / Mượt mà: Hình tròn đặc (●) + chữ tiếng Việt.
     - Mức Đáng ngờ / Căng thẳng: Hình tam giác cảnh báo (▲) + chữ tiếng Việt.
     - Mức Thủ phạm / Quá tải: Hình thoi nguy hiểm (◆) hoặc Bát giác dừng (🛑) + chữ tiếng Việt.
   - Các hình vẽ được vẽ trực tiếp bằng `Painter::convex_polygon`, tuyệt đối không dùng ký tự Unicode ngoài font Noto Sans để loại bỏ triệt để nguy cơ xuất hiện ô vuông `tofu`.
2. **Kiểm soát Dữ liệu Động (Live Data Control)**:
   - Theo khuyến nghị WCAG 2.2 cho nội dung cập nhật tự động (Real-time metrics), ứng dụng cung cấp nút Tạm dừng đóng băng hiển thị. Luồng thu thập ngầm vẫn chạy đều đặn nhưng giao diện được giữ yên tĩnh để người dùng đọc thông số và thao tác không bị trượt chuột.
3. **Giảm Hoạt Ảnh (Reduced Motion)**:
   - Hỗ trợ biến môi trường `SLOWSPOT_REDUCED_MOTION=1`. Khi bật cờ này, mọi chuyển động chuyển tiếp (animation) được ngắt để bảo vệ người dùng nhạy cảm với chuyển động hình ảnh.

---

## 8. Bố Cục Thích Ứng & Ổn Định Vị Trí (Responsive & Stable Vertical Layout)

Giao diện áp dụng cơ chế phân bổ vị trí dọc nghiêm ngặt:
1. **Header Banner**: Tên ứng dụng, Huy hiệu trạng thái, Nút Tạm dừng, Mini Chips tài nguyên.
2. **Đang theo dõi (Watched Panel)**: Slot cố định 50px (`WATCHED_SLOT_HEIGHT`).
3. **Khu vực Trạng thái / Thẻ thủ phạm (Status Area)**: Slot cố định 104px (`STATUS_SLOT_HEIGHT`).
4. **Biểu đồ Sparklines**: Chiều cao 50px cố định cho 3 tài nguyên (CPU, RAM, Đĩa).
5. **Bảng tiến trình (Process Table)**: Chiếm toàn bộ phần chiều cao còn lại.
6. **Thanh trạng thái đáy (Bottom Status Bar)**: Cố định bằng `Panel::bottom`.

Nhờ cơ chế slot cố định bất biến cho cả 2 khối Đang theo dõi và Thẻ trạng thái, các khối Biểu đồ và Bảng tiến trình **tuyệt đối không bao giờ bị xê dịch khi trạng thái hệ thống thay đổi** (không bị đẩy lên hay hạ xuống).

```
┌────────────────────────────────────────────────────────────────────────┐
│ [SlowSpot] [DEMO] (● Mượt mà) (⏸ ĐANG TẠM DỪNG)      [Tạm dừng] [Chips]│
├────────────────────────────────────────────────────────────────────────┤
│ ĐANG THEO DÕI [0]             Chưa có tiến trình nào cần theo dõi      │
├────────────────────────────────────────────────────────────────────────┤
│ THỦ PHẠM CHÍNH  [◆ Thủ phạm]                        Điểm thủ phạm: 92  │
│ heavy_render.exe (PID 1420) — chiếm 88% CPU                            │
│ [ Đóng an toàn ] [ Bắt buộc tắt (4s) ] [ Mở thư mục ] [ Bỏ qua ]       │
│ ⚠️ Bắt buộc tắt sẽ khả dụng nếu ứng dụng không đóng sau 5 giây         │
├────────────────────────────────────────────────────────────────────────┤
│ [ CPU 60s: 88.0%  ]    [ RAM 60s: 42.1%  ]    [ Đĩa I/O: 12.5 MB/s ]   │
│ ~~~~~~~/\~~~~~         ~~~~~~~~~---------     _/\___/\____________     │
│ -60s            nay    -60s            nay    -60s                 nay │
├────────────────────────────────────────────────────────────────────────┤
│ DANH SÁCH TIẾN TRÌNH (TOP 20)           [ Tìm kiếm tên/PID (Ctrl+F) ]  │
│ TÊN TIẾN TRÌNH ▲   PID      CPU %     RAM        ĐĨA I/O    ĐIỂM   LOẠI │
│ ▌ heavy_render.exe 1420     88.0%     1.2 GB     12.5 MB/s  [██] 92 Ứng dụng│
│   svchost.exe 🔒   890       0.2%    120.0 MB     0.0 B/s   [░░] 10 Hệ thống│
│ Đang chọn: heavy_render.exe (PID 1420)    [Đóng an toàn] [Bắt buộc tắt]│
├────────────────────────────────────────────────────────────────────────┤
│ Ctrl+F: Tìm kiếm | Esc: Xoá | Lên/Xuống: Chọn | Space: Tạm dừng        │
│                                      Cập nhật: vừa xong | 120 tiến trình│
└────────────────────────────────────────────────────────────────────────┘
```
- **Cam kết số hàng hiển thị của bảng tiến trình**:
  - Tại độ phân giải tiêu chuẩn `1000px` x `680px`: Luôn hiển thị $\ge 8$ hàng tiến trình đồng thời (thực tế hiển thị 10 hàng).
  - Tại độ phân giải tối thiểu `760px` x `520px`: Luôn hiển thị $\ge 4$ hàng tiến trình đồng thời (thực tế hiển thị 4 hàng).
  - Đã được bảo vệ bằng kiểm thử tự động `ui::app::tests::test_visible_rows_count_at_supported_resolutions` và `ui::app::tests::test_layout_slots_constant_height_across_states`.
- Các cột bảng được co giãn thông minh, cột tên tiến trình chiếm toàn bộ không gian còn lại và tự động cắt bằng dấu ba chấm `…` nếu quá dài.


---

## 9. Thiết kế Giao diện Giám sát Đợt Quá tải & Nhóm Thủ phạm

### 9.1. Khung "Đang theo dõi" (Watched Panel)
- **Vị trí**: Đặt cố định ngay phía trên Thẻ trạng thái/Thủ phạm chính (dưới Header Banner) theo đúng phân tầng nhận thức.
- **Slot Cố Định Bất Biến (`WATCHED_SLOT_HEIGHT = 50px`)**: Chiều cao khung luôn được bảo lưu cố định (tiêu đề + 1 hàng tiến trình). Slot được thu gọn từ 64px xuống 50px để bù đắp không gian cho thẻ thủ phạm 104px mà không làm giảm số lượng hàng bảng tiến trình.
- **Trạng thái rỗng (0 tiến trình)**: Hiển thị dòng chữ mờ căn giữa `Chưa có tiến trình nào cần theo dõi` (`WATCHED_EMPTY_PLACEHOLDER`). Tuyệt đối không co cụm hay ẩn đi để không gây xê dịch các thành phần phía dưới.
- **Trạng thái có 1 tiến trình**: Hiển thị 1 hàng vừa khít bên trong slot.
- **Trạng thái có > 1 tiến trình**: Vùng danh sách tự động kích hoạt thanh cuộn dọc nội bộ (`ScrollArea::vertical()`), cắt xén nghiêm ngặt bên trong chiều cao nội dung 22px và tuyệt đối không bao giờ phình to (never grow), giữ vị trí ổn định 100% cho biểu đồ và bảng tiến trình.
- **Thẻ dòng tiến trình theo dõi**:
  - Huy hiệu trạng thái kết hợp hình học và văn bản (chuẩn WCAG AA):
    - `Thủ phạm` (Diamond ◆ đỏ `#EF4444`): Đang trong đợt quá tải mức thủ phạm (điểm >= 70).
    - `Đáng ngờ` (Triangle ▲ vàng hổ phách `#F59E0B`): Đang trong đợt quá tải mức nghi ngờ (điểm 40–69).
    - `Hạ nhiệt (còn Xs)` (Circle ● vàng): Đang trong giai đoạn làm mát (20s ở chế độ demo, 120s ở chế độ thường) kèm đếm lùi thời gian cập nhật mỗi giây.
    - `Đã hồi phục` (Circle ● xanh lá): Đã qua giai đoạn làm mát an toàn.
    - `Đã thoát` (Diamond ◆ xám): Tiến trình đã dừng hoạt động.
  - Tên tiến trình, PID (hoặc nhãn `rustc.exe ×3` nếu là nhóm tiến trình).
  - Thời gian: "Từ HH:MM:SS (đã MM:SS)".
  - Thống kê đỉnh điểm: "Đỉnh CPU: 88.0% • RAM: 1.2 GB • Đĩa: 45.0 MB/s".
  - Mini Sparkline: Biểu đồ xu hướng điểm số/tỷ trọng trong đợt quá tải.
  - Nút hành động nhanh: Tuân thủ an toàn 2 cấp độ ("Đóng an toàn" hoặc "Bắt buộc tắt"). Đối với nhóm tiến trình, cung cấp nút "Xem trong bảng" để can thiệp từng tiến trình riêng rẽ.

### 9.2. Huy hiệu Áp lực Hệ thống & Tooltip Chi tiết
- **Huy hiệu Header**: Minh bạch tài nguyên bị quá tải: "Quá tải CPU", "Quá tải RAM", "Quá tải đĩa" hoặc kết hợp "Quá tải CPU + RAM".
- **Tooltip so sánh ngưỡng**: Khi rê chuột lên huy hiệu trạng thái, hiển thị giá trị thực tế đo được so với ngưỡng cấu hình:
  `CPU: 99.7% (ngưỡng 90%) | RAM: 72.0% (ngưỡng 95%) | Đĩa: 12.0 MB/s (ngưỡng 50 MB/s)`.
- **Lược bỏ huy hiệu trùng lặp**: Huy hiệu `Theo dõi: N` trước đây trên header đã được loại bỏ vì bản thân khung "Đang theo dõi" ngay bên dưới đã hiển thị huy hiệu đếm số lượng, giúp tránh dư thừa thị giác và dành trọn không gian cho thông báo hạ nhiệt.

### 9.3. Trình bày Nhóm Thủ phạm (Group Culprit Presentation)
- **Hero Card**: Khi áp lực cao nhưng không có một tiến trình đơn lẻ áp đảo, hệ thống hiển thị thẻ Nhóm thủ phạm:
  - Tiêu đề: `Nhóm thủ phạm: rustc.exe (3 tiến trình)`
  - Diễn giải: `CPU đang 99.7% — rustc.exe (3 tiến trình) chiếm 52%, chrome.exe (4) 8%, còn lại 40% từ hệ thống/tiến trình khác`
- **Bảng tiến trình**: Gắn nhãn nhóm nhỏ `×N` bên cạnh tên tiến trình thành viên khi nhóm có $\ge 2$ tiến trình thực thi (không hiển thị nhãn `×1` cho tiến trình đơn lẻ) và làm nổi bật nền hàng bằng dải màu xanh nhẹ để người dùng dễ nhận diện trực quan.
