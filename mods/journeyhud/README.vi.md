<div align="center">

# JourneyHUD

**Minimap trên bản đồ thế giới, cùng máu, posture và tích tụ trạng thái của mục tiêu đang khoá.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11543-orange)](https://www.nexusmods.com/eldenring/mods/11543)

</div>

---

## Giới thiệu

JourneyHUD vẽ một HUD gọn ở góc trên bên phải: một minimap tròn trên chính bản đồ thế giới của game, và
bên dưới là máu, posture, tích tụ trạng thái và hiệu ứng đang chạy của kẻ địch bạn đang khoá. Mod cũng hiện
thời gian còn lại dưới các icon trạng thái của bạn. Chạy được ở chế độ offline và Seamless Co-op, cần game
chạy DX12.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/11543).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `JourneyHUD.dll` vào thư mục mod của loader và đăng ký nó.
4. Chạy game. `JourneyHUD.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [JourneyHUD.toml](JourneyHUD.toml); sửa rồi nhấn `F5` (mặc định) để áp dụng mà không cần khởi động lại game.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Journey HUD** cũng
xuất hiện trong System > Game Options. Không bắt buộc.

Mod khác hook cùng swap chain DX12 có thể xung đột với overlay: nếu game thoát ngay lúc khởi động, hãy tăng
`LoadDelay` trong `JourneyHUD.toml`.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/11543): tải về và bình luận.
- [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt): giấy phép của mã và ý tưởng mod dựa vào.
- [docs/design.md](docs/design.md): thiết kế HUD, bố cục và texture.
- [docs/offsets.md](docs/offsets.md): offset và kiểm chứng theo bản exe.
- [docs/radar.md](docs/radar.md): các chấm trên minimap.
- [docs/map.md](docs/map.md): bản đồ thế giới, grace, địa điểm, beacon và DLC.
- [docs/buildup.md](docs/buildup.md): các thanh tích tụ trạng thái.
- [docs/effects.md](docs/effects.md): hiệu ứng đang chạy, hàng icon trạng thái và thanh tích tụ của người chơi.
- [docs/layout_drag.md](docs/layout_drag.md): kéo thả minimap và thanh mục tiêu.
- [docs/hdr.md](docs/hdr.md): overlay trên màn hình HDR.
- [docs/player_marker.md](docs/player_marker.md): kim người chơi thay thế (Blue Markers).
- [docs/questpath.md](docs/questpath.md): crash lúc khởi động với QuestPath.
- [docs/heap_corruption.md](docs/heap_corruption.md): crash hỏng heap chưa rõ nguyên nhân.
