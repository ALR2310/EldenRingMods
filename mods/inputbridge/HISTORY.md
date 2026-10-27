# InputBridge: lịch sử

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-10-27 | Tạo mod: chèn bàn phím vào game đang ở nền (hook DirectInput, gỡ cờ foreground của game, giả `GetForegroundWindow`), điều khiển bằng file `InputBridge.cmd`; E đóng được hộp thoại khi game ở nền. Chưa test trong lúc chơi | [docs/background_input.md](docs/background_input.md) |
| 2026-10-27 | Lọc bàn phím, chuột, tay cầm thật khi game ở nền (`IsolateInput`) và thêm chèn chuột (di chuyển, nút, con lăn); vào world, đi, mở bản đồ, xoay camera, đánh đều chạy ở nền. Chưa commit | [docs/background_input.md](docs/background_input.md) |
| 2026-10-27 | Thêm `mcp/server.py`: MCP `eldenring-input` (press, hold, camera, click, wheel, release_all, screenshot...) ghi lệnh vào `InputBridge.cmd`; chụp cửa sổ nền tự viết (`ctypes` + Pillow), không phụ thuộc MCP khác; chưa thử với agent | [README.md](README.md) |
| 2026-10-27 | MCP thêm `notes`/`add_note` và `docs/game_notes.md`: sổ ghi chú dùng chung để agent nhớ phím, quy trình, thời gian giữa các phiên; người dùng sửa phím (Space nhảy, Shift lăn) thì ghi lại | [docs/game_notes.md](docs/game_notes.md) |
