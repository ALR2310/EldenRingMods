# InputBridge: lịch sử

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-10-27 | Tạo mod: chèn bàn phím vào game đang ở nền (hook DirectInput, gỡ cờ foreground của game, giả `GetForegroundWindow`), điều khiển bằng file `InputBridge.cmd`; E đóng được hộp thoại khi game ở nền. Chưa test trong lúc chơi | [docs/background_input.md](docs/background_input.md) |
| 2026-10-27 | Lọc bàn phím, chuột, tay cầm thật khi game ở nền (`IsolateInput`) và thêm chèn chuột (di chuyển, nút, con lăn); vào world, đi, mở bản đồ, xoay camera, đánh đều chạy ở nền. Chưa commit | [docs/background_input.md](docs/background_input.md) |
| 2026-10-27 | Thêm `mcp/server.py`: MCP `eldenring-input` (press, hold, camera, click, wheel, release_all, screenshot...) ghi lệnh vào `InputBridge.cmd`; chụp cửa sổ nền tự viết (`ctypes` + Pillow), không phụ thuộc MCP khác; chưa thử với agent | [README.md](README.md) |
| 2026-10-27 | MCP thêm `notes`/`add_note` và `docs/game_notes.md`: sổ ghi chú dùng chung để agent nhớ phím, quy trình, thời gian giữa các phiên; người dùng sửa phím (Space nhảy, Shift lăn) thì ghi lại | [docs/game_notes.md](docs/game_notes.md) |
| 2026-10-27 | Bảng phím đầy đủ trong `src/keys.txt` (cả mod lẫn MCP đọc, gồm `-`, `=`/`plus`, phím số, ngoặc, Home/End...); hook `GetAsyncKeyState` để mod đọc phím kiểu Win32 (phím zoom của JourneyHud, F5 reload) thấy phím và nút chuột giả; hook `GetCursorPos` + lệnh `moveto x y` / `cursor off` đặt con trỏ menu theo toạ độ cửa sổ; MCP thêm `mouse_to`, `click_at`, `cursor_off` (toạ độ theo ảnh chụp gần nhất). Thử: `-` đổi tầm minimap, `moveto` hiện nhãn grace đúng chỗ trên bản đồ; chưa thử click | [docs/background_input.md](docs/background_input.md) |
