<div align="center">

# JourneyHUD

**HUD gọn ở góc phải màn hình: máu và posture của kẻ địch đang khoá mục tiêu.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

JourneyHUD vẽ một overlay nhỏ ở góc trên bên phải: khi khoá mục tiêu một kẻ địch, panel hiện
**máu** và **posture** (thanh stagger) của nó, tắt đi khi bỏ khoá; phía trên nó là một **minimap** tròn, cùng một cột ở góc phải. Mod được thiết kế để mở rộng
thành HUD hành trình (minimap có nền bản đồ), xem `docs/design.md`.

## Tính năng

| Tính năng | Mô tả |
|---|---|
| **Panel mục tiêu** | thanh máu và thanh posture của kẻ địch đang khoá, vẽ giống thanh của game (số máu bật được) |
| **Minimap** | ô tròn xoay theo camera với đồng đội, spirit của mình và kẻ địch đang khoá, tắt được |
| **Bản đồ** | bản đồ thế giới của game làm nền minimap, đọc từ thư mục game lúc chạy, tắt được |
| **Grace** | các điểm grace đã tìm thấy hiện trên minimap bằng chính icon của game, tắt được |
| **Địa điểm** | di tích, nhà thờ, hang, pháo đài, boss... của bản đồ thế giới hiện trên minimap bằng chính icon của game, tắt được |
| **Beacon** | các beacon đã đặt trên bản đồ thế giới hiện trên minimap bằng icon của game, tắt được |
| **Tự ẩn** | không khoá thì không vẽ gì; kẻ địch không có thanh posture thì chỉ hiện máu |

## Cấu hình

Vị trí của cột (lệch so với góc phải trên) và cỡ panel chỉnh trong `JourneyHUD.toml` cạnh DLL; tên key và
mặc định nằm ở chú thích file đó. Panel neo vào khung hình 16:9 của game (trong dải đen trên màn 16:10 hay
siêu rộng) và co giãn theo độ cao khung đó.

## Menu trong game

Có `ERNativeUI.dll` thì hiện hàng **Journey HUD** ở System > Game Options (bật tắt, số máu, độ dài, cỡ, hai lề); đổi
là áp dụng ngay và được ghi lại vào `JourneyHUD.toml`. Không có thì mod chạy bằng file đó như bình thường.

## Cách hoạt động

Task chạy mỗi frame đọc handle mục tiêu khoá của người chơi chính, đổi sang nhân vật tương ứng
rồi chép máu và posture ra một snapshot. Luồng render (ImGui qua hudhook, DX12) chỉ đọc snapshot,
không chạm đối tượng game. Chi tiết: [docs/design.md](docs/design.md).

## Giới hạn

- Bản đồ chỉ có overworld, hầm ngầm và dungeon (chưa có Shadow of the Erdtree), luôn hiện đủ, chưa có marker ghi chú do người chơi đặt; minimap chưa hiện kẻ địch xung quanh (ngoài mục tiêu khoá); chưa có tên kẻ địch hay trạng thái (độc, chảy máu...).
- Cần Elden Ring chạy DX12; có thể xung đột với phần mềm overlay khác hook DXGI (vd. MSI Afterburner).

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md)
- [HISTORY.md](HISTORY.md)
- [docs/design.md](docs/design.md): thiết kế, nguồn tham khảo, giai đoạn
- [docs/offsets.md](docs/offsets.md): offset và kiểm chứng theo bản exe
- [docs/map.md](docs/map.md): bản đồ thế giới dưới minimap, cách đọc archive của game
- [docs/radar.md](docs/radar.md): lớp chấm của minimap, học từ minimap của kubilaiswf
- [docs/names.md](docs/names.md): tra tên kẻ địch (đã bỏ, giữ kết quả dịch ngược)
