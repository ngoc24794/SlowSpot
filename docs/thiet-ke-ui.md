# Tài liệu Thiết kế UI/UX & Hệ thống Giao diện — SlowSpot

| Phiên bản | Trạng thái | Ngày tạo | Tác giả |
| :--- | :--- | :--- | :--- |
| 2.0.0 | Đã nghiệm thu & Chuẩn hoá | 05/11/2026 | SlowSpot UI/UX Design & Engineering Team |

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
- **Chú thích (`FONT_SIZE_CAPTION`)**: `11.5 px` (Tiêu đề cột bảng, nhãn badge, đơn vị đo, phím tắt).
- **Số liệu Nhỏ (`FONT_SIZE_TINY`)**: `10.0 px` (Mốc trục thời gian -60s / nay, gợi ý thanh trạng thái).

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
- **Bên trái**: Tên ứng dụng `SlowSpot`, Huy hiệu `DEMO` (nếu chạy giả lập), Viên thuốc chỉ thị áp lực PC (`Mượt mà` ● / `Căng thẳng` ▲ / `Quá tải` 🛑), Nhãn `ĐANG TẠM DỪNG` (khi đóng băng hiển thị), và thông báo `Đã hạ nhiệt, PC đã mượt trở lại` ✔.
- **Bên phải**:
  - Nút **Tạm dừng / Tiếp tục** (kèm phím tắt `Space` và tooltip hướng dẫn a11y).
  - 3 Chip chỉ số tài nguyên mini (`CPU`, `RAM`, `Đĩa`) tích hợp thanh đo tỷ lệ trực quan ngay bên trong chip.

### 5.2. Thẻ Thủ Phạm Chính (Top Culprit Hero Card)
- **Khi có thủ phạm**:
  - Dòng 1: Tiêu đề `THỦ PHẠM CHÍNH` đỏ rực, Badge mức độ (`Thủ phạm` ◆ / `Đáng ngờ` ▲), Badge bảo vệ 🔒 (nếu là tiến trình cốt lõi), Điểm số góc phải `Điểm thủ phạm: 92/100`.
  - Dòng 2: Tên file thực thi to bản (18px), PID, và lý giải nguyên nhân bằng tiếng Việt (`— chiếm 88% CPU`).
  - Dòng 3: Dãy nút hành động phân cấp trực quan:
    1. **Nút Chính (Primary)**: `Đóng an toàn` (Tô nền xanh dương nổi bật, gửi `WM_CLOSE`).
    2. **Nút Nguy hiểm (Danger)**: `Bắt buộc tắt` (Chỉ tô nền đỏ khi đã qua 5s chờ đóng an toàn và tiến trình có thể tắt; khi chưa đủ điều kiện sẽ ở trạng thái vô hiệu hoá kèm tooltip giải thích lý do cụ thể).
    3. **Nút Phụ (Secondary)**: `Mở thư mục` và `Bỏ qua thủ phạm này` (Dạng viền tối giản).
- **Khi máy chạy mượt (No Culprit)**:
  - Hiển thị thẻ xanh lá thanh lịch với biểu tượng dấu kiểm tra ✔ vẽ bằng Painter, nhãn `Máy đang chạy mượt, không có thủ phạm` và phụ chú giải tỏa lo lắng của người dùng.

### 5.3. Biểu đồ Sparkline Thời gian Thực (Real-time Sparklines)
- Gồm 3 biểu đồ đồng thời theo dõi lịch sử 60 giây của **CPU**, **RAM** và **Tốc độ Đĩa I/O**.
- **Vùng tô nhạt (Area Shading)**: Vẽ đa giác mờ bên dưới đường biểu đồ với độ mờ 12%, mang lại cảm giác chiều sâu của dữ liệu theo trường phái Swiss.
- **Lưới tham chiếu mờ (Reference Grid)**: Các vạch kẻ ngang mờ nhạt tại các mốc 25%, 50%, 75% giúp ước lượng giá trị trực quan.
- **Thông số đỉnh & đơn vị**: Hiển thị đồng thời giá trị tức thời lớn bên phải và giá trị đỉnh cao nhất trong 60 giây (`Đỉnh: 94.0%`).
- **Trục thời gian**: Đánh dấu rõ ràng mốc `-60s` ở góc trái và `nay` ở góc phải.
- **Rà chuột soi dữ liệu (Hover Inspection)**: Khi di chuột lên biểu đồ, một đường chỉ dẫn dọc (hairline) xuất hiện kèm vòng tròn nhấn điểm và tooltip hiển thị số đo chính xác tại giây đó (ví dụ `-14s: 82.5%`).

### 5.4. Bảng Tiến Trình Tương Tác (Process Table)
- **Tiêu đề cột phẳng (Flat Headers)**: Loại bỏ giao diện nút nổi 3D xám cũ kỹ; thay bằng tiêu đề chữ hoa nhỏ tinh tế, khi rê chuột chuyển sang biểu tượng bàn tay (`PointingHand`). Mũi tên sắp xếp (Tăng dần/Giảm dần) được vẽ sắc nét trực tiếp bằng Painter.
- **Phân tách hàng trực quan**: Hàng cao 28px, đan xen màu zebra rất nhẹ (độ mờ 4% trên hàng lẻ) cùng đường phân cách mảnh màu border subtle.
- **Hiệu ứng chọn hàng (Selection State)**: Hàng đang chọn đổi nền sang tông xanh thẫm và có thanh nhấn màu xanh dương (`COLOR_INFO`) dày 3px ở cạnh trái.
- **Cột Điểm tiến trình**: Thanh tiến trình mini 38px bo góc kết hợp số điểm 0..100 đặt bên ngoài cạnh phải thanh, không còn tình trạng chữ đè lên vạch màu gây khó đọc.
- **Nhận diện Tiến trình Hệ thống**: Các tiến trình được bảo vệ (`SystemCritical`) có biểu tượng ổ khoá 🔒 vẽ bằng Painter cạnh tên, tooltip hiển thị đầy đủ đường dẫn tệp thực thi (`exe_path`).
- **Trạng thái Tìm kiếm Rỗng**: Khi không có tiến trình khớp với từ khoá tìm kiếm, bảng hiển thị thông điệp thân thiện `Không tìm thấy tiến trình nào phù hợp.`

### 5.5. Thanh Trạng Thái Đáy (Bottom Status Bar)
- Cố định ở cuối giao diện:
  - Cánh trái: Bảng hướng dẫn phím tắt súc tích (`Ctrl+F: Tìm kiếm | Esc: Xoá / Đóng | Lên/Xuống: Chọn | Space: Tạm dừng`).
  - Cánh phải: Thời gian cập nhật gần nhất (`Cập nhật: vừa xong` hoặc `X s trước`), Tổng số tiến trình thu thập (`Tổng số: 120 tiến trình`), và huy hiệu chế độ giả lập nếu có.

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

## 8. Bố Cục Thích Ứng (Responsive Layout)

Giao diện tự động co giãn sắc nét trên mọi tỷ lệ High-DPI Windows (100%, 125%, 150%, 200%) và giữ tỷ lệ hoàn hảo tại kích thước cửa sổ tối thiểu được định nghĩa trong kiến trúc:

```
┌────────────────────────────────────────────────────────────────────────┐
│ [SlowSpot] [DEMO] (● Mượt mà) (⏸ ĐANG TẠM DỪNG)      [Tạm dừng] [Chips]│
├────────────────────────────────────────────────────────────────────────┤
│ THỦ PHẠM CHÍNH  [◆ Thủ phạm]                        Điểm thủ phạm: 92  │
│ heavy_render.exe (PID 1420) — chiếm 88% CPU                            │
│ [ Đóng an toàn ]  [ Bắt buộc tắt (4s) ]  [ Mở thư mục ]  [ Bỏ qua ]     │
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
- Độ rộng tối thiểu: `760px` x `520px` không bao giờ bị vỡ khung hay xuất hiện thanh cuộn ngang ngoài ý muốn.
- Các cột bảng được co giãn thông minh, cột tên tiến trình chiếm toàn bộ không gian còn lại và tự động cắt bằng dấu ba chấm `…` nếu quá dài.
