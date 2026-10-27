# AutoPickup: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng. Chuyện nào có phân tích dài thì
link sang `docs/`.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-10-26 | Mod mới: hook `ExecActionButton` trả "đã kích hoạt" cho 4 nhóm ID (nguyên liệu, vật phẩm, đồ trên xác, rune rơi) và nhân tầm ActionButtonParam. Cơ chế lấy từ Elden_Ring_game_tools, ErdTools (Convergence), Solid Uncapper; hook đặt ở hàm lõi vì AOB của Erd-Tools khớp 2 hàm bọc | [docs](docs/exec_action_button.md) |
| 2026-10-26 | Đối chiếu ID với bảng ActionButtonParam vanilla và Convergence: tách rương thành nhóm `Chests` (mặc định tắt), chuyển "Pillage" sang CorpseLoot, thêm nguyên liệu DLC `207850-207894` và vài "Pick up item" còn thiếu. | [docs](docs/exec_action_button.md) |
| 2026-10-26 | Sửa crash `illegal instruction` ở lần gọi pass-through đầu tiên: nhảy về hàm gốc bằng `rax` làm hỏng `mov [rax+8], rbx` ở nửa sau hàm (ghi đè lên code của chính nó), đổi sang `r11`. | [docs](docs/exec_action_button.md) |
| 2026-10-26 | Người dùng thử trong game và xác nhận chạy đúng; viết `nexus_page.bbcode`, chuẩn bị phát hành 1.0.0 | [docs](docs/exec_action_button.md) |
| 2026-10-26 | Nếu mod khác đã hook đầu hàm `ExecActionButton` thì báo xung đột (log và hộp thoại, qua `common::conflict`) và tắt hook, thay vì chờ 120 giây rồi báo không tìm thấy mẫu. Mặc định `Range` là 2.0 cho mọi nhóm | [docs](docs/exec_action_button.md) |
| 2026-10-27 | Ghi giới hạn `Range` (0.1 đến 10) vào chú thích file config và gợi ý trong menu; trước đó giới hạn chỉ có trong code | [docs](docs/exec_action_button.md) |
