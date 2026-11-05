# AGENTS.md — SlowSpot

SlowSpot là ứng dụng desktop Windows viết bằng Rust, giám sát tiến trình làm chậm PC (CPU, RAM, disk...) và chỉ ra thủ phạm.

## Quy ước ngôn ngữ
- **Tài liệu** (`docs/`, README, báo cáo): tiếng Việt.
- **Code**: tên biến/hàm/module, comment, commit message, log, thông báo lỗi trong code: **tiếng Anh**.

## Lệnh chuẩn (chạy ở gốc repo / worktree)
```
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo build --release      # bản portable: target/release/slowspot.exe
```
Toolchain: `stable-x86_64-pc-windows-msvc`. Bản release link tĩnh CRT (không cần DLL ngoài).

## Quy ước git
- Nhánh chính: `main`. Mỗi task làm trên worktree riêng, nhánh `agent/<tên-task>`.
- Commit nhỏ, message tiếng Anh dạng `type: summary` (feat, fix, docs, refactor, test, chore).
- Không push, không merge vào `main`. Không commit `target/`, `dist/`, thư mục/tệp của AI agent.
- Chỉ sửa các file được liệt kê trong phạm vi task. Không thêm dependency ngoài phạm vi task mà chưa được duyệt.
- Không đưa secret, token, đường dẫn cá nhân vào repo.

## Giao thức báo cáo
Khi xong task, ghi báo cáo vào `.agents/reports/<task>.md` (file cục bộ, bị gitignore — KHÔNG commit) gồm:
1. Đã làm gì (danh sách file đổi).
2. Lệnh đã chạy và kết quả (build/test/clippy).
3. Vấn đề tồn đọng / quyết định cần orchestrator xem.
Sau đó commit code trên nhánh task và dừng, chờ orchestrator review.
