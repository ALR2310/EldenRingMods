# Toolbox

Mod **nội bộ** (`audience = "internal"`): hộp đồ nghề gom những tính năng nhỏ dùng để chơi thử và kiểm tra mod khác, kèm GUI mở ngay trong game. Không phát hành, không CHANGELOG. Thêm tính năng mới khi cần.

## Tính năng

- **GUI trong game** (`MenuKey`, mặc định `F8`): cửa sổ Dear ImGui (dùng `common::overlay`) điều khiển bằng chuột, kéo/đổi cỡ và tự nhớ vị trí:
  - **7 nút Buildup riêng** (`Poison`, `Scarlet Rot`, `Hemorrhage (Bleed)`, `Death Blight`, `Frostbite`, `Sleep`, `Madness`): bấm vào nút nào thì cộng ngay buildup loại đó lên bản thân, hiện luôn số `đã đầy / tối đa` trên nút; mỗi dòng có nút `0` nhỏ để xoá riêng thanh đó, và nút `Clear All Buildup` xoá toàn bộ.
  - **Thanh trượt `Amount`**: chỉnh lượng buildup mỗi lần bấm (`0` = đầy thanh ngay lập tức), tự lưu vào `Toolbox.ini`.
  - **Công tắc `MapAnywhere` / `TorrentAnywhere`**: bật/tắt và lưu thẳng vào `Toolbox.ini` (báo cần khởi động lại game khi thay đổi vì hai tính năng này là patch mã lúc khởi động).
- **MapAnywhere**: mở bản đồ và dịch chuyển nhanh khi đang đánh boss và khi ở hang động / dungeon. Chép từ `sometweaks` (WarpAnywhere), xem [docs](../sometweaks/docs/misc_patches.md).
- **TorrentAnywhere**: gọi và cưỡi Torrent ở mọi nơi. Chép từ `sometweaks`; khác ở chỗ gọi `ApplySpEffect` bằng AOB vì API của `fromsoftware-rs` panic trên 2.7.2.0.
- **Nạp lại cấu hình**: phím `ReloadKey` (mặc định `F5`) nạp lại `Toolbox.ini` ngay trong game.

Tên key và giá trị mặc định nằm trong `Toolbox.ini`.

## Thêm tính năng mới

1. Tạo `src/<tên>.rs` với hàm `pub fn run()`.
2. Thêm `mod` và một dòng `std::thread::spawn(<tên>::run)` ở `src/lib.rs`.
3. Thêm widget tương ứng vào `src/ui.rs` và một mục vào `Toolbox.ini` (khóa phẳng, không trùng).
4. Ghi một dòng vào [HISTORY.md](HISTORY.md).

## Tài liệu

- [HISTORY.md](HISTORY.md): dòng thời gian thay đổi.
