# SlowSpot — Giám sát & Chỉ điểm Tiến trình Làm chậm PC Windows

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011%20(64--bit)-blue.svg)](https://microsoft.com)
[![CRT](https://img.shields.io/badge/CRT-Static%20Linked%20(Zero%20DLL)-success.svg)](#tính-năng-nổi-bật)

**SlowSpot** là ứng dụng desktop Windows độc lập (standalone portable) siêu nhẹ viết bằng Rust, giúp người dùng giải quyết tức thì câu hỏi khi máy tính bị đơ hoặc giật lag: **"Tiến trình nào đang thực sự làm chậm PC của tôi, và tôi nên xử lý thế nào?"**

Thay vì hiển thị hàng trăm dòng số liệu thô và đảo vị trí liên tục như Windows Task Manager, SlowSpot tự động tổng hợp tải đa chiều (CPU, RAM, Disk I/O) theo mô hình **Tài nguyên chi phối (Dominant Resource)** kết hợp bộ giảm xóc ổn định (damping), chỉ thẳng "thủ phạm" và đưa ra hành động xử lý an toàn bằng tiếng Việt.

---

## 📸 Minh hoạ Giao diện

```
+========================================================================================+
| SlowSpot - Trợ lý Giám sát Hiệu năng Windows                                  [_][口][X]|
+----------------------------------------------------------------------------------------+
| ⚠️  HỆ THỐNG ĐANG CĂNG THẲNG: CPU 91.5% | ĐĨA 64.2 MB/s | RAM 82.4%                     |
+----------------------------------------------------------------------------------------+
| [ THỦ PHẠM CHÍNH ]                                                                     |
|  🔴 premiere_render.exe (PID: 14208) — Điểm tác hại: 91 / 100 [Nghiêm trọng]          |
|  Nguyên nhân: Chiếm dụng CPU tới 89.2% làm nghẽn năng lực xử lý                        |
|                                                                                        |
|  [Đóng ứng dụng an toàn]     [Bỏ qua thủ phạm này]     [Mở thư mục chứa file]          |
+----------------------------------------------------------------------------------------+
| LỊCH SỬ ÁP LỰC HỆ THỐNG (60s gần nhất)                                                 |
| CPU  [ ▂▃▅▆▇██████████████████████████████████████████████████████████████████ ]  91.5% |
| ĐĨA  [        ▂▃▅▆▇███████████████████████████████████████████████████████████ ] 64.2M/s|
| RAM  [ ▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅ ]  82.4% |
+----------------------------------------------------------------------------------------+
| DANH SÁCH TIẾN TRÌNH CHI TIẾT                [ Tìm kiếm tiến trình...          🔍 ]    |
| Tên tiến trình          PID     Điểm      CPU%        Đọc/Ghi Đĩa         Bộ nhớ (RAM) |
|----------------------------------------------------------------------------------------|
| premiere_render.exe    14208     91      89.2%         1.2 MB/s              4.85 GB   |
| chrome.exe             11044     35       4.1%         0.4 MB/s              1.20 GB   |
| code.exe                8920     18       1.5%         0.1 MB/s            820.00 MB   |
| explorer.exe            4112      8       0.5%         0.0 MB/s            145.00 MB   |
| csrss.exe                760      0       0.0%         0.0 MB/s             12.50 MB 🔒|
+========================================================================================+
```

### Tính năng tuỳ chọn "Bỏ qua thủ phạm này" (Ignore Culprit)
Khi một tiến trình đang chạy nặng là tác vụ bạn **chủ động thực hiện** (ví dụ bạn đang render video Adobe Premiere, biên dịch dự án lớn, hoặc tải game qua Steam), bạn chỉ cần nhấp vào nút **[Bỏ qua thủ phạm này]**. SlowSpot sẽ ghi nhận tiến trình vào danh sách bỏ qua tạm thời trong phiên làm việc và ngay lập tức chỉ điểm tiến trình đứng thứ hai nếu hệ thống vẫn bị nghẽn.

---

## ⚡ Tính năng Nổi bật

1. **Standalone & Zero-Dependency**:
   - File thực thi duy nhất `slowspot.exe` kích thước chỉ **~5.29 MB** (gói nén zip chỉ **~2.34 MB**).
   - Link tĩnh toàn bộ C Runtime (`+crt-static`), **không yêu cầu** cài đặt Microsoft Visual C++ Redistributable (`vcruntime140.dll` / `msvcp140.dll`).
   - Nhúng sẵn font chữ tiếng Việt **Noto Sans** chất lượng cao qua macro `include_bytes!`, hiển thị tiếng Việt sắc nét không lỗi ô vuông.
2. **Không tự làm đơ máy**:
   - Mức chiếm dụng CPU khi giám sát **< 0.1%** nhờ cơ chế Reactive Event Loop (chỉ render khi có dữ liệu mới hoặc tương tác chuột).
   - Tiêu thụ RAM cực thấp (**~47 MB Private Memory**).
   - **0 MB/s** Disk I/O (không ghi log hay file tạm vào ổ đĩa trong quá trình chạy).
3. **Thuật toán Dominant-Resource & Damping**:
   - Tự động nhận diện cổ chai dù là nghẽn CPU (95%), nghẽn Đĩa I/O (đĩa HDD/SSD 100%), hay áp lực cạn kiệt RAM (> 80%).
   - Bộ đệm 3 chu kỳ ngăn chặn hiện tượng danh sách nhảy giật liên tục.
4. **Cơ chế An toàn Tuyệt đối**:
   - Tích hợp **System Safety Filter** khóa cứng nút tắt đối với các tiến trình cốt lõi của Windows (`csrss.exe`, `lsass.exe`, `smss.exe`, ...) nhằm triệt tiêu rủi ro màn hình xanh (BSOD).
   - Hành động 2 cấp độ: Ưu tiên **Đóng an toàn** (`WM_CLOSE`), nếu sau 5 giây ứng dụng treo cứng mới chuyển sang **Bắt buộc tắt** (`TerminateProcess`).
   - Xử lý lỗi đồ họa thân thiện: Nếu máy hoàn toàn không có driver GPU (chỉ có OpenGL 1.1 phần mềm của Windows), ứng dụng hiện hộp thoại Win32 tiếng Việt hướng dẫn và thoát êm đẹp, không bao giờ crash im lặng.

---

## 🚀 Cách Dùng Bản Portable

1. Tải file `slowspot-portable.zip` hoặc `slowspot.exe` từ thư mục phát hành.
2. Giải nén vào bất kỳ thư mục nào (hoặc copy trực tiếp file `slowspot.exe` vào USB).
3. Double-click chạy trực tiếp `slowspot.exe`:
   - Ứng dụng khởi động ngay lập tức (< 1 giây).
   - Không cần chạy quyền Administrator đối với nhu cầu thông thường.
   - Không sinh thêm bất kỳ file tạm hay khóa Registry nào trên máy.

---

## 🧪 Chế độ Mô phỏng / Thuyết trình (`--demo`)

SlowSpot tích hợp sẵn chế độ mô phỏng độc lập (`MockSource` & `DemoActions`), cho phép trải nghiệm và kiểm thử toàn bộ tính năng giao diện mà không tác động đến tiến trình thật trên hệ thống:

```powershell
.\slowspot.exe --demo
```

Trong chế độ demo:
- Dữ liệu tiến trình được giả lập tự động xoay vòng qua 3 kịch bản: **Đột biến CPU (Heavy CPU Hog)**, **Xung đột đĩa (Disk Thrashing)**, và **Hệ thống bình thường (Normal Idle)**.
- Các nút tương tác ([Đóng ứng dụng], [Bỏ qua]) hoạt động an toàn và ghi nhận phản hồi giao diện trực tiếp mà không dừng bất kỳ tiến trình thật nào của hệ điều hành.

---

## 🛠️ Build Từ Nguồn (Build from Source)

### Yêu cầu Tiền đề
- Hệ điều hành: **Windows 10 / Windows 11 (64-bit)**.
- Rust Toolchain: `stable-x86_64-pc-windows-msvc` (khuyến nghị Rust 1.80+).
- Cài đặt Visual Studio Build Tools (C++ Workload).

### Các lệnh biên dịch chuẩn

```powershell
# 1. Kiểm tra cấu hình và mã nguồn
cargo check

# 2. Biên dịch bản thử nghiệm (Debug)
cargo build

# 3. Biên dịch bản thương phẩm tối ưu (Release Standalone)
cargo build --release
```

File thực thi đầu ra nằm tại: `target\release\slowspot.exe`.

### Đóng gói tự động (Packaging Script)

Dự án cung cấp script PowerShell tự động hoá toàn bộ quy trình biên dịch release, làm sạch thư mục `dist/`, copy binary, đóng gói `slowspot-portable.zip` và in mã băm SHA256:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package.ps1
```

Kết quả đóng gói tạo ra trong thư mục `dist/`:
- `dist/slowspot.exe`
- `dist/slowspot-portable.zip`

---

## 🔬 Kiểm Thử & Kiểm Định Chất Lượng

Dự án tuân thủ nghiêm ngặt tiêu chuẩn kiểm thử tự động của Rust và kiểm chứng độc lập trên môi trường Windows:

```powershell
# Chạy toàn bộ 49+ bài Unit test nội bộ
cargo test

# Kiểm tra định dạng mã nguồn chuẩn
cargo fmt --check

# Kiểm tra chất lượng và phát hiện cảnh báo
cargo clippy --all-targets -- -D warnings
```

### Script Kiểm chứng Độc lập (`scripts/verify.ps1`)

Script kiểm tra 6 tiêu chí chấp nhận kỹ thuật của bản portable:
1. **Kiểm tra phụ thuộc DLL**: Dùng `dumpbin` (hoặc bộ giải mã PE header PowerShell thuần) xác nhận 100% DLL là thư viện hệ thống Windows; **không có** `vcruntime140*`, `msvcp140*`, hoặc `api-ms-win-crt-*`.
2. **Khởi chạy cô lập**: Copy file exe vào thư mục sandbox tạm hoàn toàn trống và chạy với cờ `--demo`.
3. **Tiến trình sống và tiêu đề cửa sổ**: Xác nhận tiến trình duy trì ổn định sau 8 giây và có `MainWindowTitle` khớp `"SlowSpot"`.
4. **Đo đạc tài nguyên thực tế**: Theo dõi liên tục trong ~10 giây, tính toán mức chiếm CPU% và Private Memory trung bình.
5. **Kiểm tra file rác**: Xác nhận ứng dụng không tự ý sinh file mới trong thư mục chạy.
6. **Dọn dẹp tiến trình**: Tắt chính xác PID đã tạo và thu hồi thư mục sandbox tạm thời.

Chạy lệnh:
```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify.ps1
```

---

## 📂 Cấu Trúc Thư Mục & Module

```
SlowSpot/
├── .cargo/
│   └── config.toml         # Cấu hình cờ rustflags link tĩnh CRT (+crt-static)
├── assets/
│   └── fonts/
│       ├── NotoSans-Regular.ttf # Font tiếng Việt nhúng trực tiếp vào binary
│       └── OFL.txt              # Giấy phép Open Font License 1.1
├── docs/                   # Tài liệu kỹ thuật dự án (Tiếng Việt)
│   ├── ke-hoach.md         # Kế hoạch và tiến độ thực thi từng task
│   ├── kien-truc.md        # Thiết kế kiến trúc, công thức tính điểm & số đo thật
│   └── nghiep-vu.md        # Yêu cầu nghiệp vụ và tiêu chí chấp nhận (AC)
├── scripts/                # Scripts tự động hoá (PowerShell)
│   ├── package.ps1         # Đóng gói release ra dist/ và tính SHA256
│   └── verify.ps1          # Kiểm chứng binary độc lập, đo đạc tài nguyên
├── src/                    # Mã nguồn chính ứng dụng (Rust)
│   ├── analyzer/           # Động cơ phân tích, lọc an toàn & tính điểm
│   │   ├── damping.rs      # Bộ đệm giảm xóc làm dịu đột biến
│   │   ├── mod.rs          # Điểm kết nối tầng phân tích
│   │   ├── safety.rs       # Danh sách trắng bảo vệ tiến trình hệ thống
│   │   └── scoring.rs      # Thuật toán điểm Dominant-Resource & Spike bonus
│   ├── collector/          # Tầng thu thập số liệu phần cứng định kỳ
│   │   ├── mock.rs         # Nguồn dữ liệu giả lập phục vụ demo/test
│   │   ├── mod.rs          # Giao diện SampleSource & quản lý luồng
│   │   ├── system.rs       # Nguồn thu thập sysinfo thật từ Windows
│   │   └── worker.rs       # Worker thread chạy ngầm 1000ms gửi qua channel
│   ├── platform/           # Trừu tượng hoá Win32 API qua windows-sys
│   │   ├── dialog.rs       # Hiển thị MessageBoxW native khi có lỗi
│   │   ├── mod.rs          # Xuất khẩu giao diện tầng nền tảng
│   │   └── process.rs      # Đóng an toàn (WM_CLOSE), cưỡng chế (TerminateProcess)
│   ├── ui/                 # Giao diện người dùng đồ hoạ (eframe / egui)
│   │   ├── actions.rs      # Xử lý tương tác nút bấm giao diện
│   │   ├── app.rs          # Vòng lặp chính eframe và kết xuất khung nhìn
│   │   ├── mod.rs          # Xuất khẩu giao diện UI
│   │   ├── state.rs        # Quản lý trạng thái bảng dữ liệu, lọc, sparkline
│   │   ├── strings.rs      # Tập trung toàn bộ chuỗi hiển thị tiếng Việt
│   │   ├── theme.rs        # Bảng màu sắc thái trực quan
│   │   └── widgets.rs      # Các widget tuỳ biến (thẻ hero, sparkline)
│   ├── config.rs           # Cấu hình chu kỳ lấy mẫu và ngưỡng điểm
│   ├── lib.rs              # Thư viện gốc của SlowSpot
│   ├── main.rs             # Điểm khởi nhập binary, bắt cờ dòng lệnh & lỗi khởi tạo
│   └── model.rs            # Định nghĩa toàn bộ schema dữ liệu & trạng thái
├── Cargo.toml              # Khai báo metadata và profile tối ưu release
└── README.md               # Hướng dẫn tổng quan dự án (Tiếng Việt)
```

---

## ⚠️ Giới Hạn Đã Biết (Known Limitations)

1. **Yêu cầu Khởi tạo Đồ hoạ OpenGL 2.0+**:
   - Giao diện SlowSpot xây dựng trên backend `glow` (OpenGL siêu nhẹ). Máy tính cần có driver card đồ hoạ hỗ trợ tối thiểu OpenGL 2.0 trở lên.
   - Nếu máy hoàn toàn không có driver GPU (chỉ sử dụng driver phần mềm OpenGL 1.1 cổ điển mặc định của Windows), ứng dụng sẽ hiện hộp thoại Win32 thông báo rõ ràng nguyên nhân và thoát an toàn thay vì crash.
2. **Quyền Quản trị viên (Administrator Privilege)**:
   - Khi chạy ở quyền người dùng thông thường (Standard Non-Admin), SlowSpot vẫn chẩn đoán và hiển thị đầy đủ tài nguyên của toàn bộ hệ thống.
   - Tuy nhiên, nếu người dùng muốn can thiệp thao tác đóng/tắt các tiến trình chạy dưới tài khoản đặc quyền cấp cao (`SYSTEM`, `LOCAL SERVICE`), Windows sẽ từ chối truy cập (`Access Denied`). Khi đó giao diện sẽ hiển thị thông báo yêu cầu người dùng khởi chạy SlowSpot dưới quyền Quản trị viên để can thiệp.
3. **Phạm vi Nền tảng**:
   - Bản phát hành hiện tại được tối ưu chuyên biệt cho hệ điều hành Windows 10 và Windows 11 (kiến trúc 64-bit `x86_64`).

---

## 📜 Giấy Phép Bản Quyền (License)

- **Mã nguồn SlowSpot**: chưa chọn giấy phép (chờ chủ dự án quyết định).
- **Phông chữ Noto Sans**: Được phân phối theo giấy phép mở [SIL Open Font License 1.1](assets/fonts/OFL.txt) (xem chi tiết tại `assets/fonts/OFL.txt`).
