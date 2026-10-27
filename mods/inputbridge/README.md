<div align="center">

# InputBridge

Công cụ nội bộ để agent điều khiển Elden Ring đang chạy ở nền.

</div>

## Giới thiệu

Gửi phím vào game khi cửa sổ không được focus, để agent kiểm tra mod tự động trong lúc người dùng làm việc khác. Không phát hành.

## Tính năng

- Chèn bàn phím và chuột vào game đang ở nền.
- Bỏ qua bàn phím, chuột, tay cầm thật khi game không ở trước.
- Nhận lệnh qua file `InputBridge.cmd` cạnh DLL.
- Ghi log thống kê các đường đọc input của game.

## Cấu hình

Các tuỳ chọn nằm ở chú thích trong `InputBridge.ini`.

## Cách hoạt động

Game mất thiết bị DirectInput, bỏ qua input và coi mình không focus khi ở nền. Mod gỡ ba lớp chặn đó rồi thêm phím giả vào dữ liệu bàn phím game đọc. Chi tiết, bằng chứng và địa chỉ: [docs/background_input.md](docs/background_input.md).

## Giới hạn

- Chưa chèn được tay cầm.
- Chỉ dùng offline, không bật cùng EAC.
- Không bật cùng SoulsTeleport (cùng hook DirectInput).

## MCP cho agent

`mcp/server.py` là MCP (Python, cần gói `mcp` và `pillow`, không phụ thuộc MCP nào khác) cho agent điều khiển game qua file lệnh của mod và chụp cửa sổ nền bằng `PrintWindow`. Chạy: `python mods/inputbridge/mcp/server.py`; thư mục `mod` của game lấy từ biến môi trường `ER_MOD_DIR`.

## Sổ ghi chú cho agent

`docs/game_notes.md` giữ những gì agent đã học khi điều khiển game (phím của người dùng, cách vào world, thời gian chờ). MCP đọc bằng `notes()` và ghi thêm bằng `add_note()`; agent phải gọi `notes()` trước khi chơi và ghi lại khi người dùng sửa hoặc khi thử nghiệm chứng minh điều mới.

## Tài liệu

- [HISTORY.md](HISTORY.md)
- [docs/background_input.md](docs/background_input.md)
- [docs/game_notes.md](docs/game_notes.md)
