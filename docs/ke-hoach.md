# Kế hoạch triển khai — SlowSpot

Nguyên tắc: các task làm **tuần tự**, một worker (agy), mỗi task trên worktree `../SlowSpot-wt-<tên>` và nhánh `agent/<tên>`, sau khi review thì merge vào `agent/integration`. Không push, không merge vào `main`.

Dependency đã được duyệt: `eframe` 0.37 (feature `glow`, tắt `wgpu`), `sysinfo` 0.40, `windows-sys` 0.61. Dùng `std::sync::mpsc`, không thêm crate channel. Thêm dependency khác phải hỏi lại.

## Danh sách task

| # | Task (nhánh) | File được phép sửa | Tiêu chí xong | Trạng thái |
|---|---|---|---|---|
| 1 | `scaffold` | `Cargo.toml`, `.cargo/config.toml`, `src/main.rs`, `src/lib.rs`, `src/*/mod.rs` (khung rỗng), `assets/fonts/*`, `rustfmt.toml` | Có crate `slowspot` (lib + bin), `+crt-static`, profile release theo `docs/kien-truc.md`, font Noto Sans nhúng + OFL.txt, mở được cửa sổ egui trống hiển thị tiếng Việt. build/test/clippy/fmt pass | **ĐÃ XONG** |
| 2 | `model-analyzer` | `src/model.rs`, `src/analyzer/**` | Kiểu dữ liệu `ProcessRecord`, `SystemSnapshot`...; `SafetyFilter`; `CulpritEngine` đúng công thức trong kiến trúc; damping 3 chu kỳ. Thuần logic, không gọi OS. Unit test đủ các case ở mục 7.1 kiến trúc | **ĐÃ XONG** |
| 3 | `collector` | `src/collector/**`, `src/config.rs` | Thread nền 1 giây/lần dùng `sysinfo`, tính delta CPU/disk, gửi `SystemSnapshot` qua `mpsc`; có `MockSystemCollector` cho test; không crash khi Access Denied; test bằng mock | **ĐÃ XONG** |
| 4 | `platform` | `src/platform/**` | `graceful_close` (WM_CLOSE), `force_kill` (TerminateProcess), `open_location`, `show_error_box` (MessageBoxW); từ chối kill tiến trình trong whitelist ở tầng này; unit test cho phần logic không cần OS (bản thật kiểm thủ công) | **ĐÃ XONG** |
| 5 | `ui` | `src/ui/**` | Giao diện: banner trạng thái, thẻ thủ phạm chính, bảng tiến trình có tìm kiếm và sắp xếp ổn định, sparkline 60s, nút hành động (2 cấp), "Bỏ qua thủ phạm này"; tiếng Việt; chạy được với `MockSystemCollector` (`--demo`) | **ĐÃ XONG** |
| 6 | `wiring` | `src/main.rs`, `src/app.rs` | Nối collector, analyzer, ui, platform; lỗi OpenGL thì hiện MessageBox tiếng Việt rồi thoát; repaint theo sự kiện; ứng dụng chạy thật được | **ĐÃ XONG** |
| 7 | `package` | `Cargo.toml` (metadata), `scripts/package.ps1`, `scripts/verify.ps1`, `README.md`, `docs/*` | `cargo build --release` ra `dist/slowspot.exe` (+ zip), script đo kiểm; cập nhật README và docs theo thực tế | **ĐÃ XONG** |

## Giai đoạn bổ sung sau task 7 (UI/UX và theo dõi quá tải)

| Nhánh | Nội dung | Trạng thái |
|---|---|---|
| `uiux`, `uiux2`, `uiux3` | Nâng cấp UI/UX (palette, biểu đồ, bảng, thanh trạng thái, vùng cuộn cắt gọn) | ĐÃ XONG |
| `watch` | Theo dõi tiến trình quá tải theo đợt, phân tích theo nhóm | ĐÃ XONG |
| `demo` | Demo theo giai đoạn, màu khung theo mức độ | ĐÃ XONG |
| `polish3`, `polish4` | Bố cục ổn định (khung cố định), huy hiệu theo mức độ | ĐÃ XONG |
| `selfshot` | Cờ `--screenshot`, kiểm tra bố cục tự động | ĐÃ XONG |
| `polish5` | Sửa cắt khung, chồng chữ ở 760x520 | ĐÃ XONG |

## Cách review mỗi task (orchestrator)
1. Đọc `.agents/reports/<task>.md` và `git diff main...agent/<task>`; xác nhận không có file ngoài phạm vi, secret, lệnh lạ; code và comment bằng tiếng Anh.
2. Chạy trong worktree: `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
3. Lỗi thì gửi feedback cụ thể, tối đa 2 vòng, sau đó báo người dùng.
4. Đạt thì merge `--no-ff` vào `agent/integration`.

## Các bước kiểm chứng cuối (sau task 7)
1. Trên `agent/integration`: `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` đều pass.
2. `cargo build --release` thành công; ghi kích thước `slowspot.exe` (mục tiêu < 15 MB).
3. **Không phụ thuộc DLL ngoài**: `dumpbin /dependents slowspot.exe` chỉ có DLL hệ thống Windows (kernel32, user32, gdi32, opengl32, ...), không có `vcruntime140*`, `msvcp140*`, `api-ms-win-crt-*`.
4. **Chạy độc lập**: copy `slowspot.exe` vào thư mục trống (không có gì khác), chạy bằng `Start-Process`; xác nhận tiến trình sống ≥ 10 giây, cửa sổ chính có tiêu đề, không sinh file phụ trong thư mục.
5. **Hiển thị đúng**: chụp ảnh màn hình cửa sổ và kiểm tra tiếng Việt có dấu không lỗi ô vuông, bố cục không vỡ.
6. **Nhận diện thủ phạm (AC-04)**: chạy script PowerShell tạo tải CPU (vòng lặp) và tải disk (ghi file lớn); xác nhận SlowSpot gắn cờ đúng tiến trình trong ≤ 3–5 giây.
7. **An toàn (AC-05)**: nút tắt bị khoá với `csrss.exe`/`lsass.exe`; thử đóng an toàn một Notepad thật.
8. **Tài nguyên**: đo CPU/RAM của `slowspot.exe` trong 2–3 phút khi chạy nền, ghi số đo thật vào docs.
9. Tài liệu `docs/` được cập nhật nếu thực tế lệch thiết kế.
