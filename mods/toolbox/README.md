# Toolbox

Mod **nội bộ** (`audience = "internal"`): hộp đồ nghề gom những tính năng nhỏ dùng để chơi thử và kiểm tra mod khác, kèm GUI mở ngay trong game. Không phát hành, không CHANGELOG. Thêm tính năng mới khi cần.

## Tính năng

- **GUI trong game** (`MenuKey`, mặc định `F8`): cửa sổ Dear ImGui (dùng `common::overlay`) điều khiển bằng chuột, kéo/đổi cỡ và tự nhớ vị trí:
  - **7 nút Buildup riêng** (`Poison`, `Scarlet Rot`, `Hemorrhage (Bleed)`, `Death Blight`, `Frostbite`, `Sleep`, `Madness`): bấm vào nút nào thì cộng ngay buildup loại đó lên bản thân, hiện luôn số `đã đầy / tối đa` trên nút; mỗi dòng có nút `0` nhỏ để xoá riêng thanh đó, và nút `Clear All Buildup` xoá toàn bộ.
  - **Thanh trượt `Amount`**: chỉnh lượng buildup mỗi lần bấm (`0` = đầy thanh ngay lập tức), tự lưu vào `Toolbox.toml`.
  - **Công tắc `MapAnywhere` / `TorrentAnywhere`**: bật/tắt và lưu thẳng vào `Toolbox.toml` (báo cần khởi động lại game khi thay đổi vì hai tính năng này là patch mã lúc khởi động).
- **MapAnywhere**: mở bản đồ và dịch chuyển nhanh khi đang đánh boss và khi ở hang động / dungeon. Chép từ `sometweaks` (WarpAnywhere), xem [docs](../sometweaks/docs/misc_patches.md).
- **TorrentAnywhere**: gọi và cưỡi Torrent ở mọi nơi. Chép từ `sometweaks`; khác ở chỗ gọi `ApplySpEffect` bằng AOB vì API của `fromsoftware-rs` panic trên 2.7.2.0.
- **Input bridge** (`[InputBridge]`): đưa phím và chuột vào game đang ở nền để agent kiểm tra mod tự động, cả cho mod đọc phím kiểu Win32 (`GetAsyncKeyState`), và đặt con trỏ menu theo toạ độ cửa sổ. Lệnh đến qua file `Toolbox.cmd` cạnh DLL; bỏ qua bàn phím, chuột, tay cầm thật khi game không ở trước. MCP `eldenring-input` ở [mcp/server.py](mcp/server.py), sổ ghi chú cho agent ở [docs/game_notes.md](docs/game_notes.md). Hook DirectInput và con trỏ dùng chung với menu qua `common::overlay::InputExtension` (một hàm chỉ hook được một lần). Chi tiết: [docs/background_input.md](docs/background_input.md).
- **Resize cửa sổ** (`[WindowResize]`): kéo viền đổi cỡ cửa sổ game ở chế độ Windowed, khoá tỉ lệ, cỡ nhỏ nhất, cỡ lúc khởi động. Chi tiết: [docs/resize_approach.md](docs/resize_approach.md).
- **Đổi icon trên taskbar** (`[Taskbar] Icon`): thay hình icon của game ở nút taskbar bằng icon trống (`Blank`) hoặc icon mặc định của Windows (`Default`); `Game` giữ icon gốc. Dùng `WM_SETICON` và icon của lớp cửa sổ, nút vẫn còn. Chặn cả lúc game tự đặt lại icon của nó (không có khoảng hở). `StopFlashing` (mặc định bật) dừng nút taskbar nhấp nháy khi game đòi chú ý. Đổi bằng phím nạp lại cấu hình.
- **Nạp lại cấu hình**: phím `ReloadKey` (mặc định `F5`) nạp lại `Toolbox.toml` ngay trong game.

- **Trễ lúc tải (`LoadDelay`)**: menu chỉ móc vào đồ hoạ DX12 của game sau một khoảng chờ, để không đụng các mod overlay khác (JourneyHUD...) cũng móc vào đó. Các tính năng khác không phải chờ.

Tên key và giá trị mặc định nằm trong `Toolbox.toml` (cấu hình TOML qua `common::toml_config`, xem [toml_config](../../shared/docs/toml_config.md)).

## Thêm tính năng mới

1. Tạo `src/<tên>.rs` với hàm `pub fn run()`.
2. Thêm `mod` và một dòng `std::thread::spawn(<tên>::run)` ở `src/lib.rs`.
3. Thêm widget tương ứng vào `src/ui.rs` và một mục (kèm trường trong `src/config.rs`) vào `Toolbox.toml`.
4. Ghi một dòng vào [HISTORY.md](HISTORY.md).

## Giới hạn

- Input bridge chỉ dùng offline, không bật cùng EAC và không bật cùng SoulsTeleport (cùng hook DirectInput).
- Chưa chèn được tay cầm.

## Tài liệu

- [HISTORY.md](HISTORY.md): dòng thời gian thay đổi.
- [docs/](docs/): `background_input.md`, `resize_approach.md`, `game_notes.md`.
