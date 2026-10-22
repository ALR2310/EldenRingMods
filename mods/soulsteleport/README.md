<div align="center">

# SoulsTeleport

**Dịch chuyển tới đúng chỗ đồng đội đang đứng trong Seamless Co-op.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

SoulsTeleport là mod độc lập cho **Seamless Co-op** (cần cài Seamless Co-op). Bấm phím mở menu,
menu liệt kê mọi người trong phiên kèm vai trò (Host, Co-op, Hunter...), bấm một người để
**dịch chuyển tới đúng chỗ họ đang đứng** qua màn hình tải, dù ở xa đến đâu.

Vị trí được hỏi trực tiếp người đó qua Steam P2P, nên **người được tới cũng phải cài mod**.

## Tính năng

| Tính năng | Mô tả |
|---|---|
| **Menu đồng đội** | liệt kê mọi người trong phiên với vai trò, kéo và đổi cỡ được, có nút Reset vị trí |
| **Dịch chuyển chính xác** | tới đúng block và toạ độ của đồng đội, qua màn hình tải |
| **Bảo vệ khỏi người thù địch** | invader, Bloody Finger, Recusant không teleport được, và không ai teleport tới họ được |
| **Menu tự co giãn** | theo độ phân giải của game, hệ số phóng to chỉnh được |

Menu điều khiển bằng bàn phím và chuột; khi menu mở, chuột không tác động vào game (bàn phím và
tay cầm vẫn tới game để bạn tiếp tục di chuyển).

## Cấu hình

Cấu hình nằm trong `SoulsTeleport.ini` cạnh DLL, tự tạo ở lần chạy đầu. Chỉnh được phím mở
menu, hệ số phóng to menu (đổi xong phải khởi động lại game), vị trí và kích thước menu (mod tự
ghi khi bạn kéo hoặc đổi cỡ) và ghi log. Tên key, giá trị mặc định và ý nghĩa nằm ở chú thích
trong [SoulsTeleport.ini](SoulsTeleport.ini), file mẫu nhúng sẵn trong DLL. Mod không có hot
reload.

## Cách hoạt động

| Phần | Cách làm |
|---|---|
| **Dịch chuyển** | Dùng lại đúng trình tự game tự dùng khi kết thúc phiên multiplayer: yêu cầu chuyển map tới block đích, đặt điểm spawn tuỳ chỉnh, rồi đặt cờ yêu cầu warp. Cờ được ghi thẳng thay vì gọi hàm kích hoạt của game, để không làm rớt phiên co-op. Các hàm của game tìm bằng AOB, không khoá theo phiên bản: AOB hụt thì mod tự tắt. ([docs/warp_mechanism.md](docs/warp_mechanism.md)) |
| **Lấy vị trí** | Trong Seamless Co-op vị trí đồng đội ở xa không đọc được từ game, nên mod hỏi qua Steam P2P: bên hỏi gửi `WHERE`, bên kia trả `HERE` kèm block và toạ độ rồi mới warp. Chỉ trả lời khi đã ở trong thế giới, và người gửi được kiểm tra theo danh tính Steam xác thực. ([docs/position_exchange.md](docs/position_exchange.md)) |
| **Vai trò** | Mỗi người trong phiên hiện kèm vai trò; teleport bị chặn giữa hai bên nếu một trong hai thù địch, cả khi hỏi lẫn khi trả lời. ([docs/roles_and_invaders.md](docs/roles_and_invaders.md)) |
| **Menu** | Dear ImGui qua hudhook (DirectX 12), font Noto Sans nhúng trong DLL. ([docs/menu_ui.md](docs/menu_ui.md)) |

## Giới hạn

- **Cả hai bên phải cài mod:** người được tới là bên trả lời vị trí.
- Ai trong phiên cài mod (và không thù địch) cũng hỏi được vị trí của bạn; nên đặt
  `cooppassword` riêng cho Seamless Co-op để người lạ không vào được phiên.
- Menu chưa hỗ trợ tay cầm.
- Chưa chặn teleport khi người kia đang cưỡi Torrent trên không, trong trận boss hay đang chết.
- Đọc danh sách phiên, vị trí và trạng thái phiên qua fromsoftware-rs nên phụ thuộc bố cục bộ
  nhớ từng bản game; chưa kiểm tra phiên từ 3 người trở lên.
- Font nhúng không có chữ Hàn (chữ Hán và kana chỉ hiện khi Windows có sẵn font tương ứng).
- Phím mở menu mặc định trùng phím mở bảng lobby mặc định của SoulsChat, nên ai dùng cả hai mod
  cần đổi một trong hai.

## Nguồn

Font Noto Sans (SIL OFL 1.1, kèm file giấy phép trong gói phát hành), hudhook, Dear ImGui; tham
khảo cách làm menu và chặn input của QuestPath.

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/warp_mechanism.md](docs/warp_mechanism.md) | Cơ chế dịch chuyển, AOB, tương thích |
| [docs/position_exchange.md](docs/position_exchange.md) | WHERE / HERE qua Steam P2P, chống giả gói tin |
| [docs/roles_and_invaders.md](docs/roles_and_invaders.md) | Vai trò và chặn teleport giữa người thù địch |
| [docs/menu_ui.md](docs/menu_ui.md) | Menu ImGui, chuột, input, font |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/warp.rs](src/warp.rs), [src/net.rs](src/net.rs) (gửi nhận), [src/wire.rs](src/wire.rs)
(định dạng gói tin), [src/steam.rs](src/steam.rs), [src/party.rs](src/party.rs),
[src/ui.rs](src/ui.rs), [src/input_block.rs](src/input_block.rs).
