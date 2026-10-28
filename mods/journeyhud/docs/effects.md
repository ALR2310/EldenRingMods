# JourneyHUD: hiệu ứng đang chạy của kẻ địch (SpEffect) và thời gian còn lại

**Status 2026-10-28: nghiên cứu, chưa có code. Ý tưởng lấy từ mod ERBuffTimer 1.2.1 (hiện buff của người chơi); ta áp cho kẻ địch đang khoá: hiện icon và thời gian còn lại của các hiệu ứng nó đang dính, dưới panel buildup. Layout `SpecialEffect` mới đối chiếu với `fromsoftware-rs`, chưa kiểm IDA.**

## ERBuffTimer làm thế nào (phân tích tĩnh `ERBuffTimer.dll`, không chạy nó)

Mod đã tải về ở `C:\Users\Admin\Downloads\ERBuffTimer` (README, ini, tên, 172 icon, DLL x64 938 KB). Chỉ xem, chưa cài.

- **Nguồn dữ liệu:** danh sách SpEffect đang hoạt động trên nhân vật cùng thời gian còn lại; thống kê (`show_stats`) đọc từ các dòng `SpEffectParam` (tự tìm bảng trong repository: log `SpEffectParam found at repository+0x..`, kiểm cỡ dòng).
- **Tên:** tra kho text của game theo ngôn ngữ đang chạy, qua bảng `game_names.txt` (`ID effect=Loại:ID vật phẩm`, loại G vật phẩm/phép, A talisman, S kỹ năng, W vũ khí, P giáp); effect không có tên dùng `names_en.txt`/`names_ru.txt` (từ Paramdex).
- **Icon:** icon trạng thái của game đọc thẳng từ archive `Data*` (`menu\hi\01_common`, dùng `BCrypt`/`CRYPT32` giải mã header, như `map/archive.rs` của ta), giải nén atlas rồi vẽ bằng shader riêng; mỡ vũ khí và buff vũ khí dùng 172 PNG lấy từ wiki Fextralife (ta không dùng lại, chỉ dùng icon của game).
- **Vẽ và ẩn:** hook `Present` và `ExecuteCommandLists` bằng cách sửa vtable (không hook inline); đọc một cờ HUD của game để ẩn khi mở menu (log `HUD flag found at [exe+..]+0x..`).
- **Lọc:** chỉ buff có lợi (hiệu ứng xấu để thanh buildup của game); `hide_ids` bỏ id nội bộ; `show_unknown` hiện cả effect không tên.
- **An toàn:** không thấy import mạng nào; có `ShellExecuteW`, `VirtualProtect/Alloc` (hook), `GetCursorPos/SetCursorPos` (kéo panel). Mod nói rõ chỉ dùng offline.

## Dữ liệu ta có sẵn cho kẻ địch

`fromsoftware-rs` (rev của workspace) đã gõ kiểu cho danh sách này (`cs/sp_effect.rs`), nên không cần offset thô như phần resist:

| Trường | Offset (từ `offset_of!`) | Ý nghĩa |
|---|---|---|
| `ChrIns.special_effect` | +0x178 | con trỏ tới `SpecialEffect` |
| `SpecialEffect.head` | +0x08 | đầu danh sách liên kết đôi (`entries()` duyệt) |
| `SpecialEffectEntry.param_data` / `param_id` | +0x00 / +0x08 | dòng `SpEffectParam` và id của hiệu ứng |
| `removal_timer` | +0x40 | **thời gian còn lại** (giây) |
| `duration` | +0x48 | tổng thời gian |
| `interval_timer` | +0x4C | bộ đếm chu kỳ (độc, chảy máu theo nhịp) |
| cỡ một entry | 0x78 | |

Cùng cách đọc như `target.rs` đã làm với kẻ địch khoá: `chr_by_handle` rồi `chr.special_effect.entries()`.

`SpEffectParam` có các cột (tên ở `tmp/csv/base/SpEffectParam.csv`) để lọc: `iconId` (icon trạng thái; 0 là không hiện icon), `effectEndurance` (thời gian hiệu lực), `enableLifeTime`, `stateInfo` (loại trạng thái: độc, chảy máu, buff...) và `dispIconNonactive`.

## Hướng làm (khi làm)

1. **Đọc** mỗi frame trong task `Target`, cho kẻ địch khoá: các entry có `param_data.icon_id != 0` và `removal_timer > 0`, bỏ loại đã có thanh buildup (độc, thối đỏ, chảy máu, tử khí, đóng băng, ngủ, điên loạn qua `stateInfo` hoặc id), giữ tối đa vài cái, sắp theo thời gian còn lại.
2. **Icon:** `iconId` của `SpEffectParam` ứng với sprite trạng thái trong menu atlas (`SB_Condition` đã dùng cho buildup, các sheet icon trạng thái khác); đọc bằng `map/sprites.rs` như các icon hiện có. Cần tìm bảng ánh xạ `iconId` sang tên sprite (`MENU_..`).
3. **Vẽ** dưới các thanh buildup: icon, thời gian còn lại dạng `12.3s` (cỡ chữ 10 px như số trên thanh), tối dần khi sắp hết như ERBuffTimer; thêm khoá `Target.Effects`, mặc định cân nhắc.
4. **Tên** (nếu cần) tra theo `game_names.txt`-kiểu hoặc bỏ qua: panel chỉ hiện icon và thời gian cho gọn.

## Việc cần kiểm trước khi tin

- Layout `SpecialEffect`/`SpecialEffectEntry` trên 2.6.2.0 đến 2.7.2.0 bằng IDA (offset trên chỉ lấy từ `fromsoftware-rs`, chưa đối chiếu code của game; cần tìm hàm giảm `removal_timer` mỗi frame).
- Kẻ địch có rất nhiều SpEffect nội bộ (AI, pha boss, kháng sát thương) không có icon: kiểm `iconId` có đủ để lọc ra các hiệu ứng đáng hiện không, và bộ lọc theo `stateInfo`.
- Thời gian còn lại của hiệu ứng trên kẻ địch trong co-op (Seamless) có thể chỉ đúng với phần máy mình biết, như buildup (`docs/buildup.md`).
- Các hiệu ứng không hết hạn (vĩnh viễn, `removal_timer` bằng 0 hoặc âm) hiện thế nào.
