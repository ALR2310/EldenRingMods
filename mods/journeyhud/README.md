<div align="center">

# JourneyHUD

**HUD hành trình cho Elden Ring: minimap bản đồ thế giới, máu, posture, tích tụ trạng thái và hiệu ứng của mục tiêu đang khoá.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

JourneyHUD vẽ một overlay gọn ở góc trên bên phải: phía trên là **minimap** tròn với nền bản đồ thế giới của game, phía dưới là panel hiện **máu**, **posture** (thanh stagger), **tích tụ trạng thái** và **hiệu ứng đang chạy** của kẻ địch đang khoá mục tiêu; đồng thời hiện thời gian đếm ngược dưới hàng icon trạng thái của người chơi ở góc trên bên trái và dời thanh tích tụ trạng thái của người chơi về ngay dưới đó. Thiết kế chi tiết: [docs/design.md](docs/design.md).

## Tính năng

| Tính năng                 | Mô tả                                                                                                                                                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Panel mục tiêu**        | thanh máu và thanh posture của kẻ địch đang khoá, vẽ bằng khung và chất liệu thanh của game (số máu, posture và tích tụ bật/tắt được)                                                                                           |
| **Tích tụ trạng thái**    | độ tích tụ độc, thối đỏ, chảy máu, tử khí, đóng băng, ngủ, điên loạn của kẻ địch đang khoá, dưới thanh posture, bằng icon của game; kẻ địch miễn nhiễm thì không hiện, tắt được                                                 |
| **Hiệu ứng mục tiêu**     | các hiệu ứng đang chạy trên kẻ địch đang khoá (gồm cả độc, thối đỏ, đóng băng, ngủ sau khi nổ) với thời gian đếm ngược dưới thanh tích tụ, tắt được                                                                             |
| **Trạng thái người chơi** | thời gian đếm ngược ngay dưới từng icon trạng thái của game ở góc trên trái (kể cả khi xuống dòng), và dời thanh tích tụ trạng thái của người chơi về ngay dưới hàng icon đó (tương thích cả bản gốc lẫn Convergence), tắt được |
| **Minimap**               | ô tròn với đồng đội, spirit của mình và kẻ địch đang khoá, xoay theo camera hoặc giữ hướng Bắc ở trên, thu phóng bằng phím, tắt được                                                                                            |
| **Bản đồ**                | bản đồ thế giới của game (overworld, hầm ngầm, dungeon và DLC Land of Shadow) làm nền minimap, đọc từ thư mục game lúc chạy; chọn hiện toàn bộ hoặc chỉ vùng đã nhặt mảnh bản đồ, tắt được                                      |
| **Grace**                 | các điểm grace đã tìm thấy hiện trên minimap bằng chính icon của game, tắt được                                                                                                                                                 |
| **Địa điểm**              | di tích, nhà thờ, hang, pháo đài, boss... của bản đồ thế giới hiện trên minimap bằng chính icon của game, tắt được                                                                                                              |
| **Beacon**                | các beacon đã đặt trên bản đồ thế giới hiện trên minimap bằng icon của game, tắt được                                                                                                                                           |
| **Đồng hồ**               | giờ trong game (ngày/đêm) hiện ngay dưới minimap, giữa minimap và thanh mục tiêu, tắt được                                                                                                                                      |
| **Số lần chết**           | số lần chết của nhân vật hiện ngay phía trên minimap, tắt được                                                                                                                                                                  |
| **Đường đã đi**           | vệt đường bạn vừa đi trên minimap, mờ dần về phía cũ, tắt được                                                                                                                                                                  |
| **Map for Goblins**       | marker của mod Map for Goblins DLL (vật phẩm, địa điểm) hiện trên minimap bằng icon của nó, khi mod đó được cài; đọc qua API của nó, tắt được                                                                                   |
| **Tự ẩn**                 | ẩn dần khi mở menu hay màn hình tải; không khoá mục tiêu thì ẩn panel mục tiêu; tuỳ chọn ẩn minimap trong hang/dungeon hoặc khi đánh boss (panel mục tiêu tự trượt lên)                                                         |
| **Phím ẩn/hiện**          | một phím (`ToggleKey`, mặc định F6) ẩn hoặc hiện lại minimap và panel mục tiêu; để trống thì không có phím                                                                                                                      |

## Cấu hình

Vị trí của cột (lệch so với góc phải trên) và cỡ panel chỉnh trong `JourneyHUD.toml` cạnh DLL; tên key và
mặc định nằm ở chú thích file đó. Panel neo vào khung hình 16:9 của game (trong dải đen trên màn 16:10 hay
siêu rộng) và co giãn theo độ cao khung đó.

## Menu trong game

Có `ERNativeUI.dll` thì hiện hàng **Journey HUD** ở System > Game Options (hai trang con: Panel mục tiêu và Minimap, mỗi trang có các tuỳ chọn của nó); đổi
là áp dụng ngay và được ghi lại vào `JourneyHUD.toml`. Không có thì mod chạy bằng file đó như bình thường.

## Cách hoạt động

Task chạy mỗi frame đọc người chơi chính, mục tiêu đang khoá, đồng đội, spirit và hiệu ứng đang chạy rồi chép ra snapshot; luồng nền đọc bản đồ và sprite từ archive của game khi đã vào thế giới. Luồng render (ImGui qua hudhook, DX12) chỉ đọc snapshot để vẽ, không chạm đối tượng game. Phần trạng thái người chơi hook hàm cập nhật hàng icon trạng thái và thanh tích tụ của HUD để lấy slot icon đang hiển thị và đặt lại vị trí/tỉ lệ thanh tích tụ. Chi tiết: [docs/design.md](docs/design.md), [docs/map.md](docs/map.md), [docs/effects.md](docs/effects.md).

## Giới hạn

- Minimap chưa hiện các marker ghi chú (map marker xanh lá) do người chơi đặt hay kẻ địch thường xung quanh (ngoài mục tiêu đang khoá); panel không hiện tên kẻ địch.
- Trong co-op Seamless, thanh tích tụ và thời gian hiệu ứng của kẻ địch đọc từ bản kẻ địch trên máy mình nên có thể chỉ phản ánh phần máy mình biết.
- Chữ thời gian dưới hàng icon trạng thái của người chơi đặt theo vị trí HUD mặc định (nếu đổi cỡ HUD trong tuỳ chọn của game thì chữ có thể lệch).
- Màn hình HDR: overlay vẽ bằng màu SDR nên mod tự chuyển sang HDR khi game xuất HDR (chi tiết `docs/hdr.md`).
- Cần Elden Ring chạy DX12; có thể xung đột với phần mềm overlay khác hook DXGI (vd. MSI Afterburner).

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md)
- [HISTORY.md](HISTORY.md)
- [docs/design.md](docs/design.md): thiết kế, nguồn tham khảo, giai đoạn
- [docs/offsets.md](docs/offsets.md): offset và kiểm chứng theo bản exe
- [docs/map.md](docs/map.md): bản đồ thế giới dưới minimap, cách đọc archive của game
- [docs/radar.md](docs/radar.md): lớp chấm của minimap, học từ minimap của kubilaiswf
- [docs/buildup.md](docs/buildup.md): thanh tích tụ trạng thái của kẻ địch
- [docs/effects.md](docs/effects.md): hiệu ứng đang chạy của kẻ địch và người chơi
- [docs/names.md](docs/names.md): tra tên kẻ địch (đã bỏ, giữ kết quả dịch ngược)
