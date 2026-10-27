# Ghi chú cho agent khi điều khiển Elden Ring

**Status 2026-10-27: sổ ghi chú sống. Agent đọc bằng tool `notes()` của MCP `eldenring-input` trước khi chơi, và ghi thêm bằng `add_note(topic, note, source)` khi người dùng sửa một điều sai hoặc khi thử nghiệm chứng minh được điều mới. Mỗi dòng có ngày và nguồn: `user` (người dùng nói) hoặc `observed` (agent tự thấy). Điều đã bị bác thì sửa hoặc xoá thẳng trong file, không để lại dòng sai.**

## Phím

- 2026-10-27 [user] W/A/S/D: di chuyển.
- 2026-10-27 [user] Space: **nhảy** (không phải lăn).
- 2026-10-27 [user] Shift: **lăn** (giữ để chạy nhanh).
- 2026-10-27 [user] M: mở và đóng bản đồ.
- 2026-10-27 [observed] E: tương tác và xác nhận (đóng hộp thoại, chọn Continue).
- 2026-10-27 [observed] Chuột trái: tấn công.

## Quy trình

- 2026-10-27 [observed] Từ màn hình tiêu đề vào world: `press e` (phím bất kỳ, ra hộp thoại cảnh báo), `press e` (đóng hộp thoại, ra menu chính với Continue), `press e` (Continue).
- 2026-10-27 [user] Tải nhân vật vào world mất khoảng 5 đến 10 giây sau khi chọn Continue.
- 2026-10-27 [observed] Game phải do người dùng bật offline (không Easy Anti-Cheat), và `InputBridge.dll` phải có trong `external_dlls` của `config_eldenring.toml`. Agent không tự bật được game.

## Thời gian

- 2026-10-27 [observed] Bản đồ và menu hiện sau khoảng 3 đến 5 giây; chụp ngay sau lệnh thì chưa thấy gì.

## Camera và chuột

- 2026-10-27 [observed] `camera(6, 0, 1)` xoay khoảng 25 đến 30 độ sang phải.

## Màn hình

- 2026-10-27 [observed] Kích thước cửa sổ game thay đổi (320x168, 800x450); `screenshot(max_width=...)` thu nhỏ khung. Cửa sổ nhỏ thì chữ trong menu rất khó đọc.
