# Tài liệu Nghiệp vụ & Yêu cầu Sản phẩm — SlowSpot

| Phiên bản | Trạng thái | Ngày tạo | Tác giả |
| :--- | :--- | :--- | :--- |
| 1.1.0 | Đã duyệt & Cập nhật Review Fixes | 05/11/2026 | SlowSpot Core Team (Agent Docs) |

---

## 1. Giới thiệu & Mục tiêu Sản phẩm

### 1.1. Bối cảnh
Khi máy tính Windows bị chậm, giật lag hoặc phản hồi chậm chạp, phản xạ tự nhiên của đa số người dùng là mở **Windows Task Manager** (`Ctrl + Shift + Esc`). Tuy nhiên, Task Manager được thiết kế như một bảng điều khiển số liệu thô dành cho quản trị viên, dẫn đến việc người dùng thông thường gặp khó khăn trong việc trả lời câu hỏi cốt lõi: **"Ứng dụng nào đang thực sự làm máy tôi bị đơ, và tôi nên làm gì với nó?"**

### 1.2. Mục tiêu của SlowSpot
**SlowSpot** là ứng dụng desktop portable viết bằng ngôn ngữ Rust dành riêng cho hệ điều hành Windows, với các mục tiêu trọng tâm:
1. **Trả lời tức thì câu hỏi "Ai là thủ phạm?"**: Thay vì bắt người dùng tự soi hàng trăm con số nhảy múa, SlowSpot tự động phân tích đa chiều tài nguyên (CPU, RAM, Disk I/O) và chỉ thẳng tên tiến trình chịu trách nhiệm chính gây nghẽn hệ thống.
2. **Đơn giản hóa hành động**: Đưa ra kết luận rõ ràng bằng tiếng Việt và gợi ý hành động an toàn (Đóng nhẹ nhàng, Tắt bắt buộc, hoặc Cảnh báo không được tắt nếu là tiến trình hệ thống).
3. **Cực nhẹ và Portable tuyệt đối**: Chạy ngay tức thì từ MỘT file thực thi duy nhất (`.exe`), không cần cài đặt, không cần cài runtime phụ trợ (.NET, Visual C++ Redistributable), chiếm dụng tài nguyên bản thân cực thấp để không làm máy đang lag thêm trầm trọng.

---

## 2. Người dùng Mục tiêu (Target Personas)

SlowSpot phục vụ 3 nhóm người dùng điển hình:

### 2.1. Người dùng Phổ thông & Dân văn phòng (General / Office User)
- **Hành vi**: Mở nhiều ứng dụng cùng lúc (Office, Chrome với nhiều tab, phần mềm kế toán, ứng dụng họp trực tuyến).
- **Nỗi đau (Pain Points)**: Khi máy đơ, mở Task Manager thấy hàng trăm tiến trình lạ hoắc, danh sách nhảy liên tục, các tiến trình trùng tên (như 15 tiến trình `chrome.exe` hoặc hàng chục `svchost.exe`). Sợ bấm nhầm làm hỏng Windows hoặc mất dữ liệu văn bản chưa lưu.
- **Kỳ vọng**: Mở ứng dụng lên là thấy ngay một bảng thông báo to, rõ ràng: *"Google Chrome đang chiếm 92% CPU — Bấm vào đây để đóng ứng dụng an toàn"*.

### 2.2. Lập trình viên & Người dùng Nâng cao (Developer / Power User)
- **Hành vi**: Chạy các tác vụ nặng (biên dịch code, Docker containers, IDE IntelliJ/VS Code, máy ảo, mô hình AI cục bộ).
- **Nỗi đau (Pain Points)**: Đôi khi một tiến trình chạy ngầm rò rỉ bộ nhớ (memory leak) hoặc nghẽn ổ đĩa (Disk thrashing do build artifacts lớn) làm đứng toàn bộ UI của Windows mà CPU tổng thể chỉ mới 30-40%. Cần phát hiện ngay lập tức tiến trình con (subprocess) nào phát sinh đột biến.
- **Kỳ vọng**: Bảng thống kê súc tích, độ trễ thấp, hiển thị chính xác tiến trình gốc (parent process), dung lượng đọc/ghi đĩa thực tế và tỷ lệ chiếm dụng tài nguyên tức thời.

### 2.3. Chuyên viên Hỗ trợ Kỹ thuật & Quản trị viên IT (IT Helpdesk / Technician)
- **Hành vi**: Tiếp nhận xử lý máy tính của nhân viên hoặc khách hàng gặp sự cố chạy chậm.
- **Nỗi đau (Pain Points)**: Không thể hoặc không muốn cài thêm phần mềm cồng kềnh lên máy khách; một số máy bị khoá chính sách phần mềm (AppLocker/không có quyền cài đặt installer); máy cũ chạy đĩa cơ HDD hoặc SSD đời đầu rất dễ bị 100% Disk Active Time.
- **Kỳ vọng**: Một file `.exe` copy từ USB chạy được ngay lập tức, không để lại rác trên máy, chẩn đoán chính xác nút thắt cổ chai phần cứng trong 5 giây.

---

## 3. Phân tích Bài toán: Vì sao Task Manager chưa giải quyết tốt?

| Vấn đề thực tế | Thực trạng trên Windows Task Manager | Cách giải quyết của SlowSpot |
| :--- | :--- | :--- |
| **Nhiễu loạn danh sách** | Sắp xếp theo cột CPU/RAM khiến danh sách đảo vị trí liên tục mỗi giây, người dùng không thể nhấp chuột chính xác vào tiến trình muốn chọn. | Giữ danh sách ổn định với bộ đệm (damping buffer); làm nổi bật cố định Top 1 - 3 thủ phạm ở đầu giao diện bằng thẻ Card riêng biệt. |
| **Ảo tưởng chỉ số (Metric Blindspot)** | Máy đơ cứng nhưng CPU chỉ 25% (nghẽn do 100% Disk I/O trên ổ cài Windows) hoặc RAM 98% dẫn tới Windows liên tục swap pagefile. Task Manager không chỉ ra mối liên hệ này. | Tính điểm tổng hợp `Culprit Score` kết hợp cả 3 trục: CPU load, Áp lực RAM, và Tốc độ I/O đĩa. Cảnh báo rõ nguyên nhân gây nghẽn: "Nghẽn ổ đĩa" hay "Cạn kiệt RAM". |
| **Tiến trình trùng tên & Tiến trình hệ thống** | Hiện hàng loạt `svchost.exe`, `RuntimeBroker.exe`. Người dùng tắt nhầm tiến trình quan trọng làm xanh màn hình (BSOD). | Nhận diện và gom nhóm tiến trình; tích hợp **Danh sách Trắng Hệ thống (System Safety Filter)** để vô hiệu hóa nút bấm tắt đối với các tiến trình sống còn của Windows. |
| **Hành động thô bạo** | Chỉ có tùy chọn `End Task` (cưỡng chế tắt ngay lập tức), dễ làm mất dữ liệu chưa lưu của người dùng. | Cung cấp 2 cấp độ: **Đóng an toàn (Graceful Close)** gửi tín hiệu đóng cửa sổ trước, chỉ khi ứng dụng treo cứng mới dùng đến **Tắt cưỡng bức (Force Kill)**. |

---

## 4. Phạm vi Sản phẩm (Product Scope)

### 4.1. Có làm trong MVP (In-Scope — Giai đoạn 1)
- **Thu thập tài nguyên hệ thống theo thời gian thực (Real-time Metric Collection)**:
  - CPU tổng thể và CPU riêng từng tiến trình (chuẩn hóa trên toàn bộ số nhân/luồng).
  - RAM sử dụng (Working Set) và mức độ tiêu thụ thực tế.
  - Tốc độ Đọc/Ghi đĩa (Disk Read/Write Bytes/sec) của từng tiến trình và toàn hệ thống.
- **Động cơ chấm điểm thủ phạm (Culprit Scoring Engine)**:
  - Thuật toán chấm điểm theo thang 0 - 100 dựa trên trọng số tài nguyên và mức độ đột biến (spike).
  - Phân loại mức độ nguy hại: `Bình thường (Normal)`, `Đáng ngờ (Suspicious)`, `Nghiêm trọng (Culprit)`.
- **Giao diện Người dùng Tinh gọn (Simple & Focused UI)**:
  - Khung banner trên cùng: Hiển thị ngay lập tức tình trạng PC (Mượt mà / Căng thẳng / Quá tải) và Thủ phạm lớn nhất kèm lý giải ngắn gọn bằng tiếng Việt.
  - Bảng danh sách tiến trình: Hiển thị Top 10 - 20 tiến trình tiêu thụ nhiều tài nguyên nhất, hỗ trợ ô tìm kiếm nhanh theo tên.
  - Biểu đồ mini (Sparkline): Lịch sử áp lực hệ thống trong 60 giây gần nhất.
- **Cơ chế An toàn Hệ thống (Safety Mechanism)**:
  - Phân loại rõ ràng: Ứng dụng người dùng (User App), Dịch vụ nền (Background Service), Tiến trình cốt lõi hệ điều hành (System Critical Process).
  - Khóa bảo vệ không cho phép người dùng tắt các tiến trình: `smss.exe`, `csrss.exe`, `wininit.exe`, `services.exe`, `lsass.exe`, `explorer.exe` (chỉ cho phép khởi động lại Explorer).
- **Hành động xử lý (Process Remediation)**:
  - Đóng an toàn (`Graceful Close` via `WM_CLOSE`).
  - Cưỡng bức tắt (`Force Kill` via `TerminateProcess`).
  - Mở thư mục chứa file thực thi trong File Explorer.
  - Xử lý lỗi phân quyền rõ ràng: Khi gặp tiến trình bị `Access Denied`, hiển thị thông báo yêu cầu quyền Quản trị viên thay vì crash.
- **Tiêu chuẩn Portable & Trải nghiệm**:
  - 1 file `.exe` duy nhất, không ghi vào Registry hay AppData nếu không được yêu cầu.
  - DPI awareness do `eframe`/`winit` tự động xử lý mặc định (co giãn chuẩn xác ở mọi mức DPI 100% đến 200%).
  - Hỗ trợ đầy đủ tiếng Việt có dấu với font chữ **Noto Sans** (giấy phép mở SIL Open Font License 1.1 — OFL 1.1) nhúng trực tiếp vào binary, kèm file `assets/fonts/OFL.txt` trong repository.
  - Xử lý lỗi khởi tạo đồ họa an toàn: Nếu máy không hỗ trợ OpenGL 2.0+, hiển thị hộp thoại `MessageBoxW` tiếng Việt giải thích rõ ràng và thoát êm đẹp.

### 4.2. Không làm trong MVP / Chuyển sang giai đoạn sau (Out-of-Scope / Future Roadmap)
- **Tự động khởi động lại dưới quyền Quản trị viên (Relaunch as Administrator)**: Chưa đưa vào MVP (ở MVP chỉ cần hiển thị thông báo rõ ràng khi bị Access Denied).
- **Backend đồ họa đa nền tảng wgpu (DirectX 12 / Vulkan)**: Chưa đưa vào MVP để giữ binary nhỏ gọn, build nhanh; sẽ xem xét ở giai đoạn sau nếu cần hỗ trợ máy không có driver OpenGL.
- **Giám sát GPU nâng cao (3D Engine, VRAM, Video Encode)**: Chưa đưa vào MVP vì API giám sát GPU của Windows (DirectX/DXGI/NVAPI) phức tạp, phụ thuộc driver hãng và dễ gây crash trên máy tính không có card đồ họa rời.
- **Giám sát lưu lượng mạng chi tiết từng tiến trình (Network per Process)**: Đòi hỏi quyền quản trị nâng cao hoặc ETW kernel provider, làm tăng kích thước ứng dụng và rủi ro bị phần mềm diệt virus chặn nhầm.
- **Tự động tắt tiến trình ngầm (Autokill)**: Không tự ý tắt bất kỳ tiến trình nào của người dùng khi chưa có xác nhận thủ công.
- **Bóc tách từng Tab của trình duyệt web**: Cần tích hợp giao thức Chrome DevTools Protocol hoặc can thiệp sâu, nằm ngoài phạm vi công cụ nhẹ.
- **Lưu trữ dữ liệu lịch sử dài hạn vào Database**: MVP chỉ lưu dữ liệu dạng vòng lặp trong RAM (Rolling buffer) tối đa 5 phút để vẽ đồ thị, không lưu trữ xuống ổ đĩa.
- **Chức năng dọn dẹp rác ổ đĩa / Tối ưu Registry**: Tránh sa đà vào các tính năng "dọn dẹp ảo" không rõ hiệu quả.

---

## 5. Luồng Sử dụng Ứng dụng (User Journey & Flow)

```
[Bắt đầu] Người dùng thấy PC bị chậm / giật hình
   │
   ▼
[1] Double-click chạy file "slowspot.exe" (Không cần cài đặt)
   │
   ├── [Trường hợp Lỗi đồ họa: Máy thiếu OpenGL 2.0+]
   │      └── Hiển thị MessageBoxW tiếng Việt: "Không thể khởi tạo đồ họa OpenGL..." -> Thoát an toàn
   │
   ▼
[2] Ứng dụng khởi động (Mục tiêu < 1 giây, đo thực tế khi đóng gói) & Thu thập mẫu ban đầu
   │
   ▼
[3] Màn hình chính xuất hiện:
   ├── Khung cảnh báo: "Hệ thống đang nghẽn: CPU 94%, Đĩa 100%"
   ├── Thẻ Thủ Phạm Chính: "Phát hiện 'premier_render.exe' (Score: 92/100) chiếm 88% CPU"
   └── Danh sách các ứng dụng xếp sau
   │
   ├── [Trường hợp A: Người dùng đồng ý xử lý]
   │      │
   │      ├── Bấm "Đóng ứng dụng an toàn"
   │      │      ├── Ứng dụng lưu trạng thái & thoát êm đẹp
   │      │      └── Nếu sau 5 giây vẫn treo -> Nút đổi thành "Bắt buộc tắt (Force Kill)"
   │      │
   │      └── Hệ thống hạ nhiệt -> SlowSpot thông báo "Đã hạ nhiệt, PC đã mượt trở lại"
   │
   ├── [Trường hợp B: Đây là tiến trình cố ý chạy (VD: Đang render / tải game)]
   │      └── Bấm "Bỏ qua thủ phạm này" -> Thuật toán chuyển sang cảnh báo tiến trình kế tiếp
   │
   └── [Trường hợp C: Thao tác trúng tiến trình bị Access Denied]
          └── Hiển thị thông báo: "Không đủ quyền can thiệp tiến trình này (Cần quyền Administrator)"
```

---

## 6. Tiêu chí Chấp nhận Sản phẩm (Acceptance Criteria)

Các chỉ số tài nguyên và thời gian dưới đây là **mục tiêu thiết kế**, sẽ được đo đạc thực tế khi hoàn thiện đóng gói bản release và ghi lại số đo thật vào báo cáo:

| Mã tiêu chí | Hạng mục | Tiêu chí chấp nhận (Mục tiêu thiết kế) | Phương pháp kiểm chứng |
| :--- | :--- | :--- | :--- |
| **AC-01** | **Thời gian khởi động** | Thời gian từ lúc double-click đến khi hiển thị frame giao diện đầu tiên **mục tiêu < 1 giây** (sẽ đo thực tế khi đóng gói và ghi lại số đo thật) trên máy tính văn phòng tiêu chuẩn (Core i3 thế hệ 8 trở lên, ổ SSD). | Đo bằng lệnh đo thời gian khởi tạo tiến trình (`Measure-Command`). |
| **AC-02** | **Tính Portable & Độc lập** | Đúng **1 file duy nhất `.exe`**, không sinh file rác vào thư mục chạy, không yêu cầu cài đặt bất kỳ runtime nào (kể cả Visual C++ Redistributable 2015-2022). | 1. Kiểm tra tĩnh tại chỗ: Chạy `dumpbin /dependents slowspot.exe` xác nhận không có `vcruntime140.dll` / `msvcp140.dll`.<br>2. Chạy thử nghiệm trên máy ảo Windows 10/11 sạch hoàn toàn (Clean Windows Sandbox). |
| **AC-03** | **Chiếm dụng tài nguyên của SlowSpot** | - CPU trung bình: **mục tiêu < 1.0%** khi chạy nền / hiển thị thông thường (sẽ đo thực tế khi đóng gói).<br>- RAM tiêu thụ: **mục tiêu < 60 MB Private Working Set** (sẽ đo thực tế khi đóng gói).<br>- Disk I/O của SlowSpot: **0 MB/s** (không ghi đĩa định kỳ). | Dùng Performance Monitor (`perfmon`) theo dõi chính tiến trình `slowspot.exe` trong 10 phút. |
| **AC-04** | **Độ chính xác nhận diện thủ phạm** | Nhận diện chính xác 100% tiến trình gây nghẽn trong các kịch bản thử nghiệm:<br>1. Kịch bản Stress CPU (1 tiến trình chạy vòng lặp vô tận chiếm > 80% CPU).<br>2. Kịch bản Disk Thrashing (1 tiến trình liên tục đọc/ghi file lớn làm bão hòa I/O).<br>3. Kịch bản Cạn kiệt RAM (tiến trình rò rỉ RAM chiếm > 85% dung lượng còn lại). | Chạy các script giả lập tải (`stress-cpu.exe`, `io-heavy.exe`) và kiểm tra xem SlowSpot có gắn cờ đúng thủ phạm trong vòng **3 giây** (3 chu kỳ lấy mẫu) hay không. |
| **AC-05** | **Bảo vệ hệ thống (Safety)** | Tuyệt đối không cho phép người dùng kích hoạt lệnh Terminate đối với các tiến trình nằm trong `System Safety Whitelist` (`smss.exe`, `csrss.exe`, `lsass.exe`, `winlogon.exe`, v.v.). Nút bấm phải bị vô hiệu hóa hoặc có cảnh báo chặn rõ ràng. | Thử thao tác với các tiến trình hệ thống trong danh sách hiển thị. |
| **AC-06** | **Chất lượng hiển thị & Ngôn ngữ** | - 100% văn bản giao diện hiển thị đúng tiếng Việt có dấu với font Noto Sans nhúng sẵn, không lỗi font ô vuông (`tofu`).<br>- Giao diện tự co giãn sắc nét trên các tỷ lệ High-DPI: 100%, 125%, 150%, 200% nhờ cơ chế mặc định của `eframe`/`winit`. | Kiểm tra trực quan trên màn hình độ phân giải Full HD (100%), 2K (125%), 4K (150% - 200%). |
| **AC-07** | **Kích thước file thực thi** | Kích thước file thực thi release **mục tiêu < 15 MB** (ước lượng khoảng 6 - 9 MB sau khi cấu hình strip & LTO, sẽ đo thực tế khi đóng gói). | Kiểm tra dung lượng file đầu ra tại `target/release/slowspot.exe`. |
| **AC-08** | **Khả năng phục hồi & Chịu lỗi** | Khi gặp tiến trình bị Access Denied (do quyền Administrator của tiến trình mục tiêu cao hơn), SlowSpot không được crash, mà hiển thị thông báo rõ ràng "Yêu cầu quyền Quản trị viên để can thiệp". | Chạy SlowSpot dưới quyền người dùng chuẩn (Standard User) và tương tác với tiến trình của tài khoản SYSTEM. |
| **AC-09** | **Xử lý lỗi đồ họa OpenGL (Fallback Error Notice)** | Khi khởi tạo đồ họa OpenGL 2.0+ thất bại (máy không có driver GPU, chỉ có OpenGL 1.1 phần mềm của Windows), SlowSpot không được crash hoặc treo im lặng; ứng dụng phải bắt lỗi và hiển thị một hộp thoại Win32 `MessageBoxW` bằng tiếng Việt giải thích rõ nguyên nhân kèm gợi ý cập nhật driver GPU, sau đó thoát an toàn. | Giả lập môi trường thiếu OpenGL hoặc can thiệp bối cảnh khởi tạo để kiểm chứng hộp thoại thông báo. |

---

### 6.1. Bảng Tổng hợp Kết quả Nghiệm thu Đo đạc Thực tế

Các số liệu nghiệm thu thực tế thu được từ quá trình chạy `scripts/package.ps1` và `scripts/verify.ps1`:

| Mã tiêu chí | Hạng mục nghiệm thu | Chỉ số mục tiêu | Kết quả đo thực tế | Trạng thái |
| :--- | :--- | :--- | :--- | :--- |
| **AC-01** | Thời gian khởi động | < 1.0 giây | < 0.5 giây; duy trì ổn định sau 8s chạy cô lập sandbox | **ĐẠT (PASS)** |
| **AC-02** | Tính Portable & Độc lập | 1 file .exe, 0 runtime DLL ngoài | 17 DLL hệ thống Windows, 0 DLL runtime CRT (`VCRUNTIME`/`MSVCP`/`api-ms-win-crt-*`) | **ĐẠT (PASS)** |
| **AC-03** | Mức chiếm dụng CPU | < 1.0% | **0.00% – 0.08%** (trung bình 12 core khi chạy nền) | **ĐẠT (PASS)** |
| **AC-03** | Bộ nhớ Private Memory | < 60 MB | **46.72 – 47.04 MB** Private Memory | **ĐẠT (PASS)** |
| **AC-03** | Tác động Đĩa I/O | 0 MB/s, 0 file rác | **0.00 MB/s**; 0 file rác sinh ra trong thư mục chạy | **ĐẠT (PASS)** |
| **AC-05** | An toàn hệ thống | Chặn tắt tiến trình Core Windows | `csrss.exe`, `lsass.exe` bị khoá nút tắt 🔒 | **ĐẠT (PASS)** |
| **AC-06** | Font tiếng Việt | 100% tiếng Việt có dấu | Font Noto Sans Regular nhúng trực tiếp, hiển thị chuẩn xác | **ĐẠT (PASS)** |
| **AC-07** | Kích thước file thực thi | < 15.0 MB | **5.29 MB** (5,551,104 bytes); Zip: **2.34 MB** | **ĐẠT (PASS)** |
| **AC-08** | Chịu lỗi phân quyền | Không crash khi Access Denied | Bắt lỗi an toàn, thông báo yêu cầu quyền Administrator | **ĐẠT (PASS)** |
| **AC-09** | Báo lỗi OpenGL native | Hộp thoại MessageBoxW khi lỗi | Bắt lỗi khởi tạo `eframe`, hiện thông báo Win32 tiếng Việt | **ĐẠT (PASS)** |

